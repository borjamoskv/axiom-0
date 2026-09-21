//! Turbine: parallel elaboration with bounded-attempt atomic telemetry.
//!
//! Integrates the Axiom-MM memory model ([`SeqlockCell`]) into the elaborator
//! pipeline. Snapshots summarize a completed elaboration; they omit its typing
//! context and cannot be used as a memoization cache or a typing certificate.
//! Snapshot operations can fail under contention. Batch elaboration uses scoped
//! worker threads and waits for them to finish.

use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

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

/// Discriminant of a value or type summarized by telemetry.
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

/// Four words of telemetry summarizing the state of an AST node.
///
/// Slot storage also includes sequence metadata. Neither this type nor its
/// storage guarantees cache-line alignment or a particular cache-line size.
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
            Value::Pi(_, q, _, _) => Self {
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

    /// Reconstructs the simple value summary of a successful elaboration.
    ///
    /// The result is telemetry, not evidence that the value is well typed in a
    /// caller's context. Compound values cannot be reconstructed from these words.
    pub fn to_value(&self) -> Option<Value> {
        if self.status != ElabStatus::CertifiedValid {
            return None;
        }
        match self.type_tag {
            TypeTag::Unit => Some(Value::Unit),
            TypeTag::UnitType => Some(Value::UnitType),
            TypeTag::Universe => u32::try_from(self.level).ok().map(Value::Universe),
            TypeTag::Neutral => Some(Value::Neutral(crate::eval::Neutral::Var(
                crate::ast::Level(self.level),
            ))),
            _ => None,
        }
    }
}

/// Failure to read elaboration telemetry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TurbineReadError {
    /// A capacity-only engine has not yet received an in-range publication.
    Unbound,
    /// The expression belongs to a different AST arena.
    ForeignArena,
    /// The expression has no slot in this engine.
    OutOfBounds { index: usize, capacity: usize },
    /// No coherent snapshot was obtained within the requested attempt budget.
    Read(ReadError),
}

impl fmt::Display for TurbineReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unbound => f.write_str("turbine is not yet bound to an AST arena"),
            Self::ForeignArena => f.write_str("expression belongs to a different turbine arena"),
            Self::OutOfBounds { index, capacity } => {
                write!(
                    f,
                    "expression index {index} exceeds turbine capacity {capacity}"
                )
            }
            Self::Read(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for TurbineReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read(error) => Some(error),
            _ => None,
        }
    }
}

/// Failure to publish elaboration telemetry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TurbineWriteError {
    /// The expression belongs to a different AST arena.
    ForeignArena,
    /// The expression has no slot in this engine.
    OutOfBounds { index: usize, capacity: usize },
    /// The slot was contended or its sequence counter was exhausted.
    Write(WriteError),
}

impl fmt::Display for TurbineWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForeignArena => f.write_str("expression belongs to a different turbine arena"),
            Self::OutOfBounds { index, capacity } => {
                write!(
                    f,
                    "expression index {index} exceeds turbine capacity {capacity}"
                )
            }
            Self::Write(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for TurbineWriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Write(error) => Some(error),
            _ => None,
        }
    }
}

// Arena allocation stops before issuing usize::MAX, so it cannot be a valid ID.
const UNBOUND_ARENA: usize = usize::MAX;

use std::sync::RwLock;

/// Parallel elaboration and atomic telemetry for exactly one AST arena.
///
/// Houses a fixed-size vector of [`SeqlockCell<usize, 4>`]. Slot indices are valid
/// only for the arena to which this engine is bound. Publications are best effort;
/// inference always uses the AST and its typing context, not these summaries.
pub struct TurbineEngine {
    arena: AtomicUsize,
    slots: Vec<SeqlockCell<usize, 4>>,
    meta_ctx: RwLock<Vec<Option<Value>>>,
    pub expr_to_meta: RwLock<std::collections::HashMap<crate::ast::ExprId, crate::ast::MetaId>>,
    pub inserted_implicits: RwLock<std::collections::HashMap<crate::ast::ExprId, Vec<crate::ast::MetaId>>>,
}

impl TurbineEngine {
    /// Creates a new turbine with the specified slot capacity.
    ///
    /// The first in-range publication atomically binds the engine to its arena.
    /// Reads cannot bind the engine and return [`TurbineReadError::Unbound`] until
    /// this happens. Simultaneous publishers from other arenas are rejected.
    pub fn new(capacity: usize) -> Self {
        let mut slots = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            slots.push(SeqlockCell::new(AtomicElabSnapshot::EMPTY.to_words()));
        }
        Self {
            arena: AtomicUsize::new(UNBOUND_ARENA),
            slots,
            meta_ctx: RwLock::new(Vec::new()),
            expr_to_meta: RwLock::new(std::collections::HashMap::new()),
            inserted_implicits: RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Creates a turbine bound to this arena and its current node count.
    ///
    /// Later additions to the AST have no slot in this engine.
    pub fn for_ast(ast: &Ast) -> Self {
        let mut engine = Self::new(ast.expression_count());
        *engine.arena.get_mut() = ast.arena_id().raw();
        engine
    }

    /// Instantiates a new metavariable.
    pub fn new_meta(&self) -> crate::ast::MetaId {
        let mut ctx = self.meta_ctx.write().unwrap();
        let id = ctx.len();
        ctx.push(None);
        crate::ast::MetaId(id)
    }

    /// Resolves a metavariable with a value.
    pub fn solve_meta(&self, meta: crate::ast::MetaId, value: Value) {
        let mut ctx = self.meta_ctx.write().unwrap();
        if ctx[meta.0].is_none() {
            ctx[meta.0] = Some(value);
        }
    }

    /// Forces a value, substituting solved metavariables recursively at the head.
    pub fn force(&self, ast: &Ast, val: Value) -> Value {
        match val {
            Value::Meta(id, spine) => {
                let solution = self.meta_ctx.read().unwrap()[id.0].clone();
                if let Some(sol) = solution {
                    self.force(ast, sol)
                } else {
                    Value::Meta(id, spine)
                }
            }
            _ => val,
        }
    }

    /// Number of atomic slots in the turbine.
    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    /// Attempts to read coherent telemetry for `expr` in this engine's arena.
    ///
    /// Performs one arena load and at most six atomic loads per read attempt.
    /// The budget bounds attempts, not the time until a successful snapshot.
    pub fn try_snapshot(
        &self,
        expr: ExprId,
        attempts: usize,
    ) -> Result<Snapshot<usize, 4>, TurbineReadError> {
        let idx = expr.index();
        let cell = self.slots.get(idx).ok_or(TurbineReadError::OutOfBounds {
            index: idx,
            capacity: self.slots.len(),
        })?;
        let arena = self.arena.load(Ordering::Acquire);
        if arena == UNBOUND_ARENA {
            return Err(TurbineReadError::Unbound);
        }
        if arena != expr.arena.raw() {
            return Err(TurbineReadError::ForeignArena);
        }
        cell.try_read(attempts).map_err(TurbineReadError::Read)
    }

    /// Compatibility alias for [`Self::try_snapshot`].
    ///
    /// Despite this historical name, snapshots are telemetry, not cached
    /// elaborations, and must not be used to skip inference or checking.
    pub fn try_get_cached(
        &self,
        expr: ExprId,
        attempts: usize,
    ) -> Result<Snapshot<usize, 4>, TurbineReadError> {
        self.try_snapshot(expr, attempts)
    }

    /// Publishes telemetry for `expr`, validating its arena and slot first.
    pub fn publish(
        &self,
        expr: ExprId,
        snapshot: AtomicElabSnapshot,
    ) -> Result<usize, TurbineWriteError> {
        let idx = expr.index();
        let cell = self.slots.get(idx).ok_or(TurbineWriteError::OutOfBounds {
            index: idx,
            capacity: self.slots.len(),
        })?;
        let requested_arena = expr.arena.raw();
        let arena = self.arena.load(Ordering::Acquire);
        if arena == UNBOUND_ARENA {
            match self.arena.compare_exchange(
                UNBOUND_ARENA,
                requested_arena,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {}
                Err(bound_arena) if bound_arena == requested_arena => {}
                Err(_) => return Err(TurbineWriteError::ForeignArena),
            }
        } else if arena != requested_arena {
            return Err(TurbineWriteError::ForeignArena);
        }
        cell.try_write(snapshot.to_words())
            .map_err(TurbineWriteError::Write)
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
