use std::{
    collections::BTreeMap,
    fmt,
    sync::atomic::{AtomicUsize, Ordering},
};

/// The usage semiring {0, 1, ω}. In particular, 1 + 1 = ω and 0 * ω = 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Quantity {
    Zero,
    One,
    Omega,
}

impl Quantity {
    pub const fn plus(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Self::Zero, q) | (q, Self::Zero) => q,
            _ => Self::Omega,
        }
    }

    pub const fn times(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Self::Zero, _) | (_, Self::Zero) => Self::Zero,
            (Self::One, q) | (q, Self::One) => q,
            _ => Self::Omega,
        }
    }

    /// Exact erasure/linearity; ω permits weakening and contraction.
    /// This is not the numeric ordering 0 <= 1 <= ω: 1 does not permit 0 uses.
    pub const fn permits(self, observed: Self) -> bool {
        matches!(
            (self, observed),
            (Self::Zero, Self::Zero) | (Self::One, Self::One) | (Self::Omega, _)
        )
    }
}

impl fmt::Display for Quantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Zero => "0",
            Self::One => "1",
            Self::Omega => "ω",
        })
    }
}

// Tokens are process-local and never recycled, including after an arena drops.
// Refusing exhaustion avoids turning a wrapped counter into a valid old handle.
static NEXT_ARENA: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ArenaId(usize);

impl ArenaId {
    fn fresh() -> Self {
        Self(
            NEXT_ARENA
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
                .expect("arena identity space exhausted"),
        )
    }
}

/// Opaque, arena-owned handle. Foreign handles are rejected, even at equal indices.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TypeId {
    arena: ArenaId,
    index: usize,
}

/// Arena-local handle. Child nodes must be allocated before their parent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExprId {
    pub(crate) arena: ArenaId,
    pub(crate) index: usize,
}

impl fmt::Display for TypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "t{}:{}", self.arena.0, self.index)
    }
}

impl fmt::Display for ExprId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "e{}:{}", self.arena.0, self.index)
    }
}

/// Half-open byte range in the source associated with this AST.
/// Source text and UTF-8 boundary validation belong to the future frontend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    start: usize,
    end: usize,
}

impl Span {
    pub const fn new(start: usize, end: usize) -> Option<Self> {
        if start <= end {
            Some(Self { start, end })
        } else {
            None
        }
    }

    pub const fn start(self) -> usize {
        self.start
    }
    pub const fn end(self) -> usize {
        self.end
    }
}

/// Absolute De Bruijn level: the outermost binder has level 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Level(pub usize);

/// Non-dependent function space; the grade describes argument demand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    Unit,
    Function {
        quantity: Quantity,
        domain: TypeId,
        codomain: TypeId,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Expr {
    Var(Level),
    Unit,
    Lambda { quantity: Quantity, body: ExprId },
    App { function: ExprId, argument: ExprId },
    Ann { term: ExprId, ty: TypeId },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AstError {
    ForeignType(TypeId),
    ForeignExpr(ExprId),
    UnknownType(TypeId),
    UnknownExpr(ExprId),
}

impl fmt::Display for AstError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForeignType(id) => write!(f, "type {id} belongs to another arena"),
            Self::ForeignExpr(id) => write!(f, "expression {id} belongs to another arena"),
            Self::UnknownType(id) => write!(f, "unknown type {id}"),
            Self::UnknownExpr(id) => write!(f, "unknown expression {id}"),
        }
    }
}

impl std::error::Error for AstError {}

/// Canonical types and an acyclic expression arena. Elaborating shared term
/// nodes is rejected: each occurrence must have its own `ExprId`.
///
/// Type construction uses a deterministic ordered map: O(log T) per function
/// constructor. During elaboration, canonical type equality is O(1).
pub struct Ast {
    arena: ArenaId,
    types: Vec<Type>,
    functions: BTreeMap<(Quantity, TypeId, TypeId), TypeId>,
    expressions: Vec<Node>,
}

struct Node {
    expression: Expr,
    span: Option<Span>,
}

impl Default for Ast {
    fn default() -> Self {
        Self {
            arena: ArenaId::fresh(),
            types: vec![Type::Unit],
            functions: BTreeMap::new(),
            expressions: Vec::new(),
        }
    }
}

impl Ast {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn unit_type(&self) -> TypeId {
        TypeId {
            arena: self.arena,
            index: 0,
        }
    }

    pub fn function_type(
        &mut self,
        quantity: Quantity,
        domain: TypeId,
        codomain: TypeId,
    ) -> Result<TypeId, AstError> {
        self.ty(domain)?;
        self.ty(codomain)?;
        let key = (quantity, domain, codomain);
        if let Some(&id) = self.functions.get(&key) {
            return Ok(id);
        }
        let id = TypeId {
            arena: self.arena,
            index: self.types.len(),
        };
        self.types.push(Type::Function {
            quantity,
            domain,
            codomain,
        });
        self.functions.insert(key, id);
        Ok(id)
    }

    pub fn push(&mut self, expression: Expr) -> Result<ExprId, AstError> {
        self.push_node(expression, None)
    }

    pub fn push_spanned(&mut self, expression: Expr, span: Span) -> Result<ExprId, AstError> {
        self.push_node(expression, Some(span))
    }

    fn push_node(&mut self, expression: Expr, span: Option<Span>) -> Result<ExprId, AstError> {
        match expression {
            Expr::Lambda { body, .. } => {
                self.expr(body)?;
            }
            Expr::App { function, argument } => {
                self.expr(function)?;
                self.expr(argument)?;
            }
            Expr::Ann { term, ty } => {
                self.expr(term)?;
                self.ty(ty)?;
            }
            Expr::Var(_) | Expr::Unit => {}
        }
        let id = ExprId {
            arena: self.arena,
            index: self.expressions.len(),
        };
        self.expressions.push(Node { expression, span });
        Ok(id)
    }

    pub fn ty(&self, id: TypeId) -> Result<Type, AstError> {
        if id.arena != self.arena {
            return Err(AstError::ForeignType(id));
        }
        self.types
            .get(id.index)
            .copied()
            .ok_or(AstError::UnknownType(id))
    }

    pub fn expr(&self, id: ExprId) -> Result<Expr, AstError> {
        self.node(id).map(|node| node.expression)
    }

    pub fn span(&self, id: ExprId) -> Result<Option<Span>, AstError> {
        self.node(id).map(|node| node.span)
    }

    fn node(&self, id: ExprId) -> Result<&Node, AstError> {
        if id.arena != self.arena {
            return Err(AstError::ForeignExpr(id));
        }
        self.expressions
            .get(id.index)
            .ok_or(AstError::UnknownExpr(id))
    }

    pub fn expression_count(&self) -> usize {
        self.expressions.len()
    }
}
