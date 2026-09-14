use std::{
    fmt,
    sync::atomic::{AtomicUsize, Ordering},
};

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

static NEXT_ARENA: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ArenaId(usize);

impl ArenaId {
    fn fresh() -> Self {
        Self(
            NEXT_ARENA
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
                .expect("arena exhaust"),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExprId {
    pub(crate) arena: ArenaId,
    pub(crate) index: usize,
}

impl fmt::Display for ExprId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "e{}:{}", self.arena.0, self.index)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Level(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Expr {
    Var(Level),
    Universe(u32),
    UnitType,
    Unit,
    Pi {
        quantity: Quantity,
        domain: ExprId,
        codomain: ExprId,
    },
    Lambda {
        quantity: Quantity,
        body: ExprId,
    },
    App {
        function: ExprId,
        argument: ExprId,
    },
    Ann {
        term: ExprId,
        ty: ExprId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Eval(ExprId),
    Let {
        name: String,
        ty: Option<ExprId>,
        term: ExprId,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AstError {
    ForeignExpr(ExprId),
    UnknownExpr(ExprId),
}

impl fmt::Display for AstError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForeignExpr(id) => write!(f, "expr {id} foreign"),
            Self::UnknownExpr(id) => write!(f, "expr {id} unknown"),
        }
    }
}
impl std::error::Error for AstError {}

pub struct Ast {
    arena: ArenaId,
    expressions: Vec<Node>,
}

struct Node {
    expression: Expr,
    #[allow(dead_code)]
    span: Option<Span>,
}

impl Default for Ast {
    fn default() -> Self {
        Self {
            arena: ArenaId::fresh(),
            expressions: Vec::new(),
        }
    }
}

impl Ast {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, expression: Expr) -> Result<ExprId, AstError> {
        self.push_spanned(expression, None)
    }

    pub fn push_spanned_exact(&mut self, expression: Expr, span: Span) -> Result<ExprId, AstError> {
        self.push_spanned(expression, Some(span))
    }

    fn push_spanned(&mut self, expression: Expr, span: Option<Span>) -> Result<ExprId, AstError> {
        let id = ExprId {
            arena: self.arena,
            index: self.expressions.len(),
        };
        self.expressions.push(Node { expression, span });
        Ok(id)
    }

    pub fn expr(&self, id: ExprId) -> Result<Expr, AstError> {
        if id.arena != self.arena {
            return Err(AstError::ForeignExpr(id));
        }
        self.expressions
            .get(id.index)
            .map(|n| n.expression)
            .ok_or(AstError::UnknownExpr(id))
    }

    pub fn expression_count(&self) -> usize {
        self.expressions.len()
    }
}
