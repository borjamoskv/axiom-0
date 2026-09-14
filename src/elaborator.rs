//! Iterative bidirectional typing for the quantitative, simply typed fragment.
//!
//! O(N) time and space per call, N = allocated expression nodes: the visited
//! bitmap and type table cost O(N), each reachable node is entered once, each
//! entry emits at most five frames, and variables/types use direct arena indexing.
//! No recursive traversal, context copying, substitution or normalization.
//! Type interning happens beforehand in `Ast::function_type`, in O(log T).
//! This bound does NOT cover a future dependent type conversion procedure.

use crate::ast::{Ast, AstError, Expr, ExprId, Level, Quantity, Type, TypeId};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Ast(AstError),
    SharedExpression(ExprId),
    UnboundVariable {
        expr: ExprId,
        level: Level,
    },
    CannotInferLambda(ExprId),
    ExpectedFunction {
        expr: ExprId,
        found: TypeId,
    },
    TypeMismatch {
        expr: ExprId,
        expected: TypeId,
        found: TypeId,
    },
    QuantityMismatch {
        expr: ExprId,
        expected: Quantity,
        found: Quantity,
    },
    UsageMismatch {
        expr: ExprId,
        declared: Quantity,
        observed: Quantity,
    },
}

impl From<AstError> for Error {
    fn from(value: AstError) -> Self {
        Self::Ast(value)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ast(error) => error.fmt(f),
            Self::SharedExpression(expr) => write!(
                f,
                "expression {expr} is shared; allocate a separate node for each occurrence"
            ),
            Self::UnboundVariable { expr, level } => write!(
                f,
                "expression {expr}: variable at level {} is outside its scope",
                level.0
            ),
            Self::CannotInferLambda(expr) => write!(
                f,
                "expression {expr}: cannot infer a lambda; add a function type annotation or check against a function type"
            ),
            Self::ExpectedFunction { expr, found } => write!(
                f,
                "expression {expr}: expected a function type, found {found}"
            ),
            Self::TypeMismatch {
                expr,
                expected,
                found,
            } => write!(
                f,
                "expression {expr}: expected type {expected}, found {found}"
            ),
            Self::QuantityMismatch {
                expr,
                expected,
                found,
            } => write!(
                f,
                "expression {expr}: expected parameter quantity {expected}, found {found}"
            ),
            Self::UsageMismatch {
                expr,
                declared,
                observed,
            } => write!(
                f,
                "lambda {expr}: parameter quantity {declared} does not permit observed usage {observed}"
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Ast(error) => Some(error),
            _ => None,
        }
    }
}

impl Error {
    pub fn expression(&self) -> Option<ExprId> {
        match *self {
            Self::Ast(AstError::ForeignExpr(expr) | AstError::UnknownExpr(expr))
            | Self::SharedExpression(expr)
            | Self::CannotInferLambda(expr)
            | Self::UnboundVariable { expr, .. }
            | Self::ExpectedFunction { expr, .. }
            | Self::TypeMismatch { expr, .. }
            | Self::QuantityMismatch { expr, .. }
            | Self::UsageMismatch { expr, .. } => Some(expr),
            Self::Ast(_) => None,
        }
    }

    /// Add a source byte range when the offending node has one. Formatting is
    /// bounded: it does not recursively expand canonical type graphs.
    pub fn diagnostic<'a>(&'a self, ast: &'a Ast) -> Diagnostic<'a> {
        Diagnostic { error: self, ast }
    }
}

pub struct Diagnostic<'a> {
    error: &'a Error,
    ast: &'a Ast,
}

impl fmt::Display for Diagnostic<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(expr) = self.error.expression() {
            if let Ok(Some(span)) = self.ast.span(expr) {
                write!(f, "bytes {}..{}: ", span.start(), span.end())?;
            }
        }
        self.error.fmt(f)
    }
}

/// Successful typing information. Unreachable nodes and foreign handles have
/// no entry. Nodes allocated after this result was produced also have no entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Elaboration {
    pub ty: TypeId,
    pub visited_nodes: usize,
    root: ExprId,
    node_types: Vec<Option<TypeId>>,
    processed_frames: usize,
}

impl Elaboration {
    pub fn root(&self) -> ExprId {
        self.root
    }

    /// O(1) lookup of the checked type of a reachable expression.
    pub fn type_of(&self, expr: ExprId) -> Option<TypeId> {
        if expr.arena != self.root.arena {
            return None;
        }
        self.node_types.get(expr.index).copied().flatten()
    }

    /// Deterministic work counter, bounded by 5 * visited_nodes on success.
    pub fn processed_frames(&self) -> usize {
        self.processed_frames
    }
}

/// Synthesize a closed expression. Bare lambdas require a type annotation.
pub fn synthesize(ast: &Ast, root: ExprId) -> Result<Elaboration, Error> {
    ast.expr(root)?;
    Machine::new(ast).run(root, Mode::Synth)
}

/// Check a closed expression against a canonical type from the same arena.
pub fn check(ast: &Ast, root: ExprId, expected: TypeId) -> Result<Elaboration, Error> {
    ast.ty(expected)?;
    ast.expr(root)?;
    Machine::new(ast).run(root, Mode::Check(expected))
}

#[derive(Clone, Copy)]
enum Mode {
    Synth,
    Check(TypeId),
}

/// Prefix counts along the current path. Never divide in the usage semiring:
/// zero is not invertible. Differences recover the demand local to a binder.
#[derive(Clone, Copy, Default)]
struct Scale {
    zeros: usize,
    omegas: usize,
}

impl Scale {
    fn under(self, quantity: Quantity) -> Self {
        Self {
            zeros: self.zeros + usize::from(quantity == Quantity::Zero),
            omegas: self.omegas + usize::from(quantity == Quantity::Omega),
        }
    }

    fn relative_to(self, introduced: Self) -> Quantity {
        if self.zeros > introduced.zeros {
            Quantity::Zero
        } else if self.omegas > introduced.omegas {
            Quantity::Omega
        } else {
            Quantity::One
        }
    }
}

struct Binding {
    ty: TypeId,
    declared: Quantity,
    observed: Quantity,
    introduced: Scale,
}

enum Frame {
    Visit {
        expr: ExprId,
        mode: Mode,
        scale: Scale,
    },
    Compare {
        expr: ExprId,
        expected: TypeId,
    },
    Record {
        expr: ExprId,
    },
    FinishLambda {
        expr: ExprId,
        ty: TypeId,
    },
    Apply {
        expr: ExprId,
        argument: ExprId,
        scale: Scale,
    },
    FinishApp {
        codomain: TypeId,
    },
}

struct Machine<'a> {
    ast: &'a Ast,
    seen: Vec<bool>,
    context: Vec<Binding>,
    frames: Vec<Frame>,
    value: Option<TypeId>,
    node_types: Vec<Option<TypeId>>,
    visited_nodes: usize,
    processed_frames: usize,
}

impl<'a> Machine<'a> {
    fn new(ast: &'a Ast) -> Self {
        Self {
            ast,
            seen: vec![false; ast.expression_count()],
            context: Vec::new(),
            frames: Vec::new(),
            value: None,
            node_types: vec![None; ast.expression_count()],
            visited_nodes: 0,
            processed_frames: 0,
        }
    }

    fn run(mut self, root: ExprId, mode: Mode) -> Result<Elaboration, Error> {
        self.frames.push(Frame::Visit {
            expr: root,
            mode,
            scale: Scale::default(),
        });
        while let Some(frame) = self.frames.pop() {
            self.processed_frames += 1;
            match frame {
                Frame::Visit { expr, mode, scale } => self.visit(expr, mode, scale)?,
                Frame::Record { expr } => {
                    self.node_types[expr.index] = Some(self.value.expect("completed node"));
                }
                Frame::Compare { expr, expected } => {
                    let found = self.value.expect("completed synthesis");
                    if found != expected {
                        return Err(Error::TypeMismatch {
                            expr,
                            expected,
                            found,
                        });
                    }
                }
                Frame::FinishLambda { expr, ty } => {
                    self.value.take().expect("checked lambda body");
                    let binding = self.context.pop().expect("active lambda binder");
                    if !binding.declared.permits(binding.observed) {
                        return Err(Error::UsageMismatch {
                            expr,
                            declared: binding.declared,
                            observed: binding.observed,
                        });
                    }
                    self.value = Some(ty);
                }
                Frame::Apply {
                    expr,
                    argument,
                    scale,
                } => {
                    let found = self.value.take().expect("synthesized function");
                    let Type::Function {
                        quantity,
                        domain,
                        codomain,
                    } = self.ast.ty(found)?
                    else {
                        return Err(Error::ExpectedFunction { expr, found });
                    };
                    self.frames.push(Frame::FinishApp { codomain });
                    self.frames.push(Frame::Visit {
                        expr: argument,
                        mode: Mode::Check(domain),
                        scale: scale.under(quantity),
                    });
                }
                Frame::FinishApp { codomain } => {
                    self.value.take().expect("checked argument");
                    self.value = Some(codomain);
                }
            }
        }
        debug_assert!(self.context.is_empty());
        Ok(Elaboration {
            ty: self.value.expect("root result"),
            visited_nodes: self.visited_nodes,
            root,
            node_types: self.node_types,
            processed_frames: self.processed_frames,
        })
    }

    fn visit(&mut self, expr: ExprId, mode: Mode, scale: Scale) -> Result<(), Error> {
        let term = self.ast.expr(expr)?;
        if std::mem::replace(&mut self.seen[expr.index], true) {
            return Err(Error::SharedExpression(expr));
        }
        self.visited_nodes += 1;
        self.frames.push(Frame::Record { expr });

        if let Expr::Lambda { quantity, body } = term {
            let Mode::Check(expected) = mode else {
                return Err(Error::CannotInferLambda(expr));
            };
            let Type::Function {
                quantity: declared,
                domain,
                codomain,
            } = self.ast.ty(expected)?
            else {
                return Err(Error::ExpectedFunction {
                    expr,
                    found: expected,
                });
            };
            if quantity != declared {
                return Err(Error::QuantityMismatch {
                    expr,
                    expected: declared,
                    found: quantity,
                });
            }
            self.context.push(Binding {
                ty: domain,
                declared,
                observed: Quantity::Zero,
                introduced: scale,
            });
            self.frames.push(Frame::FinishLambda { expr, ty: expected });
            self.frames.push(Frame::Visit {
                expr: body,
                mode: Mode::Check(codomain),
                scale,
            });
            return Ok(());
        }

        // The synthesis-to-checking rule reuses the current node; it does not
        // schedule a second Visit or copy a usage vector.
        if let Mode::Check(expected) = mode {
            self.frames.push(Frame::Compare { expr, expected });
        }
        match term {
            Expr::Var(level) => {
                let binding = self
                    .context
                    .get_mut(level.0)
                    .ok_or(Error::UnboundVariable { expr, level })?;
                binding.observed = binding.observed.plus(scale.relative_to(binding.introduced));
                self.value = Some(binding.ty);
            }
            Expr::Unit => self.value = Some(self.ast.unit_type()),
            Expr::Ann { term, ty } => {
                self.frames.push(Frame::Visit {
                    expr: term,
                    mode: Mode::Check(ty),
                    scale,
                });
            }
            Expr::App { function, argument } => {
                self.frames.push(Frame::Apply {
                    expr,
                    argument,
                    scale,
                });
                self.frames.push(Frame::Visit {
                    expr: function,
                    mode: Mode::Synth,
                    scale,
                });
            }
            Expr::Lambda { .. } => unreachable!("lambda handled before synthesis"),
        }
        Ok(())
    }
}
