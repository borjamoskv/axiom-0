use crate::ast::{Ast, Expr, ExprId, Quantity, Level};
use crate::eval::{eval, equiv, Value};
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    SharedExpression(ExprId),
    TypeMismatch { expr: ExprId, expected: String, found: String },
    ExpectedFunction { expr: ExprId, found: String },
    CannotInferLambda(ExprId),
    QuantityMismatch { expr: ExprId, expected: Quantity, found: Quantity },
    UsageMismatch { expr: ExprId, declared: Quantity, observed: Quantity },
    UnboundVariable { expr: ExprId, level: Level },
    UnknownExpr(crate::ast::AstError),
}

impl From<crate::ast::AstError> for Error {
    fn from(err: crate::ast::AstError) -> Self { Self::UnknownExpr(err) }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{:?}", self) }
}
impl std::error::Error for Error {}

pub struct Elaboration {
    pub ty: Value,
}

pub fn synthesize(ast: &Ast, root: ExprId) -> Result<Elaboration, Error> {
    let mut checker = Checker::new(ast);
    let ty = checker.synth(root, 0, &[], &[])?;
    Ok(Elaboration { ty })
}

pub fn check(ast: &Ast, root: ExprId, expected: Value) -> Result<Elaboration, Error> {
    let mut checker = Checker::new(ast);
    checker.check(root, expected.clone(), 0, &[], &[])?;
    Ok(Elaboration { ty: expected })
}

struct Checker<'a> {
    ast: &'a Ast,
    seen: Vec<bool>,
}

impl<'a> Checker<'a> {
    fn new(ast: &'a Ast) -> Self {
        Self { ast, seen: vec![false; ast.expression_count()] }
    }

    fn check(&mut self, expr: ExprId, expected: Value, depth: usize, env: &[Value], ctx: &[Value]) -> Result<(), Error> {
        let term = self.ast.expr(expr)?;
        if std::mem::replace(&mut self.seen[expr.index], true) {
            return Err(Error::SharedExpression(expr));
        }

        if let Expr::Lambda { quantity, body } = term {
            if let Value::Pi(decl_q, dom, cod_closure) = expected {
                if quantity != decl_q {
                    return Err(Error::QuantityMismatch { expr, expected: decl_q, found: quantity });
                }
                let var = Value::Neutral(crate::eval::Neutral::Var(Level(depth)));
                let mut new_env = env.to_vec();
                new_env.push(var.clone());
                let mut new_ctx = ctx.to_vec();
                new_ctx.push((*dom).clone());
                let expected_body_ty = cod_closure.instantiate(self.ast, var);
                self.check(body, expected_body_ty, depth + 1, &new_env, &new_ctx)?;
                return Ok(());
            } else {
                return Err(Error::ExpectedFunction { expr, found: format!("{:?}", expected) });
            }
        }

        let found = self.synth_term(expr, term, depth, env, ctx)?;
        if !equiv(self.ast, &found, &expected, depth) {
            return Err(Error::TypeMismatch {
                expr,
                expected: format!("{:?}", expected),
                found: format!("{:?}", found),
            });
        }
        Ok(())
    }

    fn synth(&mut self, expr: ExprId, depth: usize, env: &[Value], ctx: &[Value]) -> Result<Value, Error> {
        let term = self.ast.expr(expr)?;
        if std::mem::replace(&mut self.seen[expr.index], true) {
            return Err(Error::SharedExpression(expr));
        }
        self.synth_term(expr, term, depth, env, ctx)
    }

    fn synth_term(&mut self, expr: ExprId, term: Expr, depth: usize, env: &[Value], ctx: &[Value]) -> Result<Value, Error> {
        match term {
            Expr::Var(level) => {
                ctx.get(level.0)
                    .cloned()
                    .ok_or(Error::UnboundVariable { expr, level })
            }
            Expr::Universe | Expr::UnitType => Ok(Value::Universe),
            Expr::Unit => Ok(Value::UnitType),
            Expr::Pi { quantity: _, domain, codomain } => {
                self.check(domain, Value::Universe, depth, env, ctx)?;
                let dom_val = eval(self.ast, domain, env);
                let var = Value::Neutral(crate::eval::Neutral::Var(Level(depth)));
                let mut new_env = env.to_vec();
                new_env.push(var);
                let mut new_ctx = ctx.to_vec();
                new_ctx.push(dom_val);
                self.check(codomain, Value::Universe, depth + 1, &new_env, &new_ctx)?;
                Ok(Value::Universe)
            }
            Expr::Ann { term, ty } => {
                self.check(ty, Value::Universe, depth, env, ctx)?;
                let ty_val = eval(self.ast, ty, env);
                self.check(term, ty_val.clone(), depth, env, ctx)?;
                Ok(ty_val)
            }
            Expr::App { function, argument } => {
                let f_ty = self.synth(function, depth, env, ctx)?;
                if let Value::Pi(_q, dom, cod) = f_ty {
                    self.check(argument, *dom, depth, env, ctx)?;
                    let arg_val = eval(self.ast, argument, env);
                    Ok(cod.instantiate(self.ast, arg_val))
                } else {
                    Err(Error::ExpectedFunction { expr: function, found: format!("{:?}", f_ty) })
                }
            }
            Expr::Lambda { .. } => Err(Error::CannotInferLambda(expr)),
        }
    }
}
