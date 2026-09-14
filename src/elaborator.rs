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

#[derive(Debug)]
pub struct Elaboration {
    pub ty: Value,
    pub usages: Vec<Quantity>, // De Bruijn index usages
}

pub fn synthesize(ast: &Ast, root: ExprId, types: &[Value]) -> Result<Elaboration, Error> {
    let mut checker = Checker::new(ast);
    checker.synth(root, types.len(), &[], types)
}

pub fn check(ast: &Ast, root: ExprId, expected: Value, types: &[Value]) -> Result<Elaboration, Error> {
    let mut checker = Checker::new(ast);
    checker.check(root, expected, types.len(), &[], types)
}

struct Checker<'a> {
    ast: &'a Ast,
    seen: Vec<bool>,
}

impl<'a> Checker<'a> {
    fn new(ast: &'a Ast) -> Self {
        Self { ast, seen: vec![false; ast.expression_count()] }
    }

    fn check(&mut self, expr: ExprId, expected: Value, depth: usize, env: &[Value], types: &[Value]) -> Result<Elaboration, Error> {
        let term = self.ast.expr(expr)?;
        if std::mem::replace(&mut self.seen[expr.index], true) { return Err(Error::SharedExpression(expr)); }

        if let Expr::Lambda { quantity, body } = term {
            if let Value::Pi(decl_q, dom, cod_closure) = expected {
                if quantity != decl_q { return Err(Error::QuantityMismatch { expr, expected: decl_q, found: quantity }); }
                let var = Value::Neutral(crate::eval::Neutral::Var(Level(depth)));
                let mut new_env = env.to_vec();
                new_env.push(var.clone());
                
                let expected_body_ty = cod_closure.clone().instantiate(self.ast, var);
                let mut new_types = types.to_vec();
                new_types.push((*dom).clone());
                
                let mut body_elab = self.check(body, expected_body_ty, depth + 1, &new_env, &new_types)?;
                
                let observed = if body_elab.usages.len() > depth { body_elab.usages[depth] } else { Quantity::Zero };
                if !decl_q.permits(observed) {
                    return Err(Error::UsageMismatch { expr, declared: decl_q, observed });
                }
                
                // Truncate the usage of the variable we just bound
                if body_elab.usages.len() > depth {
                    body_elab.usages.truncate(depth);
                }
                
                return Ok(Elaboration { ty: Value::Pi(decl_q, dom, cod_closure), usages: body_elab.usages });
            } else {
                return Err(Error::ExpectedFunction { expr, found: format!("{:?}", expected) });
            }
        }

        let elab = self.synth_term(expr, term, depth, env, types)?;
        if !equiv(self.ast, &elab.ty, &expected, depth) {
            return Err(Error::TypeMismatch { expr, expected: format!("{:?}", expected), found: format!("{:?}", elab.ty) });
        }
        Ok(Elaboration { ty: expected, usages: elab.usages })
    }

    fn synth(&mut self, expr: ExprId, depth: usize, env: &[Value], types: &[Value]) -> Result<Elaboration, Error> {
        let term = self.ast.expr(expr)?;
        if std::mem::replace(&mut self.seen[expr.index], true) { return Err(Error::SharedExpression(expr)); }
        self.synth_term(expr, term, depth, env, types)
    }

    fn synth_term(&mut self, expr: ExprId, term: Expr, depth: usize, env: &[Value], types: &[Value]) -> Result<Elaboration, Error> {
        match term {
            Expr::Var(level) => {
                if let Some(ty) = types.get(level.0) {
                    let mut usages = vec![Quantity::Zero; depth.max(level.0 + 1)];
                    usages[level.0] = Quantity::One;
                    Ok(Elaboration { ty: ty.clone(), usages })
                } else {
                    Err(Error::UnboundVariable { expr, level })
                }
            }
            Expr::Universe(level) => Ok(Elaboration { ty: Value::Universe(level + 1), usages: vec![] }),
            Expr::UnitType => Ok(Elaboration { ty: Value::Universe(0), usages: vec![] }),
            Expr::Unit => Ok(Elaboration { ty: Value::UnitType, usages: vec![] }),
            Expr::Pi { quantity: _, domain, codomain } => {
                let dom_elab = self.synth(domain, depth, env, types)?;
                let u_dom = match dom_elab.ty {
                    Value::Universe(u) => u,
                    _ => return Err(Error::TypeMismatch { expr: domain, expected: "Universe".into(), found: format!("{:?}", dom_elab.ty) }),
                };
                let var = Value::Neutral(crate::eval::Neutral::Var(Level(depth)));
                let mut new_env = env.to_vec();
                new_env.push(var);
                let dom_val = eval(self.ast, domain, env);
                let mut new_types = types.to_vec();
                new_types.push(dom_val);
                let cod_elab = self.synth(codomain, depth + 1, &new_env, &new_types)?;
                let u_cod = match cod_elab.ty {
                    Value::Universe(u) => u,
                    _ => return Err(Error::TypeMismatch { expr: codomain, expected: "Universe".into(), found: format!("{:?}", cod_elab.ty) }),
                };
                Ok(Elaboration { ty: Value::Universe(u_dom.max(u_cod)), usages: vec![] })
            }
            Expr::Ann { term, ty } => {
                let ty_elab = self.synth(ty, depth, env, types)?;
                match ty_elab.ty {
                    Value::Universe(_) => {},
                    _ => return Err(Error::TypeMismatch { expr: ty, expected: "Universe".into(), found: format!("{:?}", ty_elab.ty) }),
                };
                let ty_val = eval(self.ast, ty, env);
                let elab = self.check(term, ty_val.clone(), depth, env, types)?;
                Ok(elab)
            }
            Expr::App { function, argument } => {
                let mut f_elab = self.synth(function, depth, env, types)?;
                if let Value::Pi(decl_q, dom, cod) = f_elab.ty {
                    let mut arg_elab = self.check(argument, *dom, depth, env, types)?;
                    
                    // Multiply argument usages by the Pi declared quantity
                    for u in arg_elab.usages.iter_mut() {
                        *u = u.times(decl_q);
                    }
                    
                    // Merge usages using `plus`
                    let max_len = f_elab.usages.len().max(arg_elab.usages.len());
                    f_elab.usages.resize(max_len, Quantity::Zero);
                    for (i, u) in arg_elab.usages.into_iter().enumerate() {
                        f_elab.usages[i] = f_elab.usages[i].plus(u);
                    }
                    
                    let arg_val = eval(self.ast, argument, env);
                    Ok(Elaboration { ty: cod.instantiate(self.ast, arg_val), usages: f_elab.usages })
                } else {
                    Err(Error::ExpectedFunction { expr: function, found: format!("{:?}", f_elab.ty) })
                }
            }
            Expr::Lambda { .. } => Err(Error::CannotInferLambda(expr)),
        }
    }
}
