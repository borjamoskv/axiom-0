//! Turbine: Lock-free asynchronous parallel inference engine for AXIOM-0.
//!
//! Integrates the Axiom-MM memory model ([`SeqlockCell`]) into the deep
//! elaborator pipeline, allowing multi-threaded inference across CPU cores
//! with zero locks, wait-free snapshot reads, and cache-line aligned telemetry.

use crate::ast::{Ast, ExprId, Quantity};
use crate::elaborator::{Elaboration, Error, check_with_turbine, synthesize_with_turbine};
use crate::eval::Value;
use crate::seqlock::{ReadError, SeqlockCell, Snapshot, WriteError};

/// Status of an elaboration node within the turbine matrix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum ElabStatus {
    Unelaborated = 0,
    InProgress = 1,
    CertifiedValid = 2,
    CertifiedError = 3,
}

/// Discriminant of a canonical value or type in atomic silicio.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum TypeTag {
    Unit = 0,
    UnitType = 1,
    Universe = 2,
    Pi = 3,
    Neutral = 4,
    Other = 5,
}

/// A 4-word atomic snapshot representing the state of an AST node in silicio.
/// Fits within a single 64-byte L1 cache line (32 bytes payload + sequence metadata).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtomicElabSnapshot {
    pub status: ElabStatus,
    pub type_tag: TypeTag,
    pub level: usize,
    pub quantity: Quantity,
}

impl AtomicElabSnapshot {
    pub const EMPTY: Self = Self {
        status: ElabStatus::Unelaborated,
        type_tag: TypeTag::Unit,
        level: 0,
        quantity: Quantity::Zero,
    };

    pub fn to_words(self) -> [usize; 4] {
        [
            self.status as usize,
            self.type_tag as usize,
            self.level,
            match self.quantity {
                Quantity::Zero => 0,
                Quantity::One => 1,
                Quantity::Omega => 2,
            },
        ]
    }

    pub fn from_words(words: [usize; 4]) -> Self {
        let status = match words[0] {
            1 => ElabStatus::InProgress,
            2 => ElabStatus::CertifiedValid,
            3 => ElabStatus::CertifiedError,
            _ => ElabStatus::Unelaborated,
        };
        let type_tag = match words[1] {
            0 => TypeTag::Unit,
            1 => TypeTag::UnitType,
            2 => TypeTag::Universe,
            3 => TypeTag::Pi,
            4 => TypeTag::Neutral,
            _ => TypeTag::Other,
        };
        let level = words[2];
        let quantity = match words[3] {
            1 => Quantity::One,
            2 => Quantity::Omega,
            _ => Quantity::Zero,
        };
        Self {
            status,
            type_tag,
            level,
            quantity,
        }
    }

    pub fn from_value(val: &Value, quantity: Quantity) -> Self {
        match val {
            Value::Unit => Self {
                status: ElabStatus::CertifiedValid,
                type_tag: TypeTag::Unit,
                level: 0,
                quantity,
            },
            Value::UnitType => Self {
                status: ElabStatus::CertifiedValid,
                type_tag: TypeTag::UnitType,
                level: 0,
                quantity,
            },
            Value::Universe(u) => Self {
                status: ElabStatus::CertifiedValid,
                type_tag: TypeTag::Universe,
                level: *u as usize,
                quantity,
            },
            Value::Neutral(crate::eval::Neutral::Var(l)) => Self {
                status: ElabStatus::CertifiedValid,
                type_tag: TypeTag::Neutral,
                level: l.0,
                quantity,
            },
            Value::Pi(q, _, _) => Self {
                status: ElabStatus::CertifiedValid,
                type_tag: TypeTag::Pi,
                level: 0,
                quantity: *q,
            },
            _ => Self {
                status: ElabStatus::CertifiedValid,
                type_tag: TypeTag::Other,
                level: 0,
                quantity,
            },
        }
    }

    pub fn to_value(&self) -> Option<Value> {
        match self.type_tag {
            TypeTag::Unit => Some(Value::Unit),
            TypeTag::UnitType => Some(Value::UnitType),
            TypeTag::Universe => Some(Value::Universe(self.level as u32)),
            TypeTag::Neutral => Some(Value::Neutral(crate::eval::Neutral::Var(
                crate::ast::Level(self.level),
            ))),
            _ => None,
        }
    }
}

/// The Turbine Engine: A lock-free concurrent memoization and parallel inference turbine.
///
/// Houses an indexed vector of [`SeqlockCell<usize, 4>`] aligned with the AST arena.
pub struct TurbineEngine {
    slots: Vec<SeqlockCell<usize, 4>>,
}

impl TurbineEngine {
    /// Creates a new turbine with the specified slot capacity.
    pub fn new(capacity: usize) -> Self {
        let mut slots = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            slots.push(SeqlockCell::new(AtomicElabSnapshot::EMPTY.to_words()));
        }
        Self { slots }
    }

    /// Creates a turbine sized precisely to an AST arena's node count.
    pub fn for_ast(ast: &Ast) -> Self {
        Self::new(ast.expression_count().max(16))
    }

    /// Number of atomic slots in the turbine.
    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    /// Attempts to read a coherent snapshot of the elaboration state of `expr`.
    /// Wait-free: takes at most `attempts * (4 + 2)` atomic loads.
    pub fn try_get_cached(
        &self,
        expr: ExprId,
        attempts: usize,
    ) -> Result<Snapshot<usize, 4>, ReadError> {
        let idx = expr.index();
        if let Some(cell) = self.slots.get(idx) {
            cell.try_read(attempts)
        } else {
            Err(ReadError::RetryBudgetExhausted { attempts })
        }
    }

    /// Publishes a certified elaboration snapshot for `expr`.
    pub fn publish(&self, expr: ExprId, snapshot: AtomicElabSnapshot) -> Result<usize, WriteError> {
        let idx = expr.index();
        if let Some(cell) = self.slots.get(idx) {
            cell.try_write(snapshot.to_words())
        } else {
            Err(WriteError::Contended)
        }
    }

    /// Parallel multi-threaded batch elaboration.
    ///
    /// Distributes `roots` across available hardware threads using `std::thread::scope`.
    /// Zero external dependencies, pure standard library concurrency.
    pub fn elaborate_batch_parallel(
        &self,
        ast: &Ast,
        roots: &[ExprId],
        types: &[Value],
    ) -> Vec<Result<Elaboration, Error>> {
        if roots.is_empty() {
            return Vec::new();
        }

        let num_threads = std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(4)
            .min(roots.len());

        let chunk_size = roots.len().div_ceil(num_threads);
        let mut results = Vec::with_capacity(roots.len());
        // Initialize placeholder
        for _ in 0..roots.len() {
            results.push(Err(Error::UnknownExpr(crate::ast::AstError::ForeignExpr(
                roots[0],
            ))));
        }

        std::thread::scope(|s| {
            let chunks: Vec<_> = roots.chunks(chunk_size).enumerate().collect();
            let mut handles = Vec::with_capacity(chunks.len());

            for (chunk_idx, chunk) in chunks {
                let start_idx = chunk_idx * chunk_size;
                let handle = s.spawn(move || {
                    let mut chunk_res = Vec::with_capacity(chunk.len());
                    for &root in chunk {
                        let res = synthesize_with_turbine(ast, root, types, self);
                        chunk_res.push((start_idx, res));
                    }
                    chunk_res
                });
                handles.push(handle);
            }

            for handle in handles {
                let chunk_res = handle.join().expect("Worker thread panicked in turbine");
                for (offset, (base, res)) in chunk_res.into_iter().enumerate() {
                    results[base + offset] = res;
                }
            }
        });

        results
    }

    /// Parallel multi-threaded batch type-checking against an expected value.
    pub fn check_batch_parallel(
        &self,
        ast: &Ast,
        roots: &[ExprId],
        expected: Value,
        types: &[Value],
    ) -> Vec<Result<Elaboration, Error>> {
        if roots.is_empty() {
            return Vec::new();
        }

        let num_threads = std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(4)
            .min(roots.len());

        let chunk_size = roots.len().div_ceil(num_threads);
        let mut results = Vec::with_capacity(roots.len());
        for _ in 0..roots.len() {
            results.push(Err(Error::UnknownExpr(crate::ast::AstError::ForeignExpr(
                roots[0],
            ))));
        }

        std::thread::scope(|s| {
            let chunks: Vec<_> = roots.chunks(chunk_size).enumerate().collect();
            let mut handles = Vec::with_capacity(chunks.len());

            for (chunk_idx, chunk) in chunks {
                let exp = expected.clone();
                let handle = s.spawn(move || {
                    let mut chunk_res = Vec::with_capacity(chunk.len());
                    for &root in chunk {
                        let res = check_with_turbine(ast, root, exp.clone(), types, self);
                        chunk_res.push(res);
                    }
                    (chunk_idx * chunk_size, chunk_res)
                });
                handles.push(handle);
            }

            for handle in handles {
                let (start_idx, chunk_res) =
                    handle.join().expect("Worker thread panicked in turbine");
                for (offset, res) in chunk_res.into_iter().enumerate() {
                    results[start_idx + offset] = res;
                }
            }
        });

        results
    }
}
