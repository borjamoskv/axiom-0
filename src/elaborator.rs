use crate::ast::{Ast, Expr, ExprId, Level, Quantity};
use crate::eval::{Value, equiv, eval};
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    SharedExpression(ExprId),
    TypeMismatch {
        expr: ExprId,
        expected: String,
        found: String,
    },
    ExpectedFunction {
        expr: ExprId,
        found: String,
    },
    CannotInferLambda(ExprId),
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
    UnboundVariable {
        expr: ExprId,
        level: Level,
    },
    UniverseOverflow {
        expr: ExprId,
        level: u32,
    },
    ContextLengthMismatch {
        values: usize,
        types: usize,
    },
    UnknownExpr(crate::ast::AstError),
}

impl From<crate::ast::AstError> for Error {
    fn from(err: crate::ast::AstError) -> Self {
        Self::UnknownExpr(err)
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for Error {}

#[derive(Debug)]
pub struct Elaboration {
    pub ty: Value,
    pub usages: Vec<Quantity>, // Usages indexed by De Bruijn level.
}

/// Synthesize in an open context, interpreting each context entry as a neutral variable.
pub fn synthesize(ast: &Ast, root: ExprId, types: &[Value]) -> Result<Elaboration, Error> {
    let env = neutral_environment(types.len());
    synthesize_in_env(ast, root, &env, types)
}

/// Synthesize with semantic values and types at corresponding De Bruijn levels.
pub fn synthesize_in_env(
    ast: &Ast,
    root: ExprId,
    env: &[Value],
    types: &[Value],
) -> Result<Elaboration, Error> {
    validate_context(env, types)?;
    let mut checker = Checker::new(ast);
    checker.synth(root, types.len(), env, types)
}

/// Check in an open context, interpreting each context entry as a neutral variable.
pub fn check(
    ast: &Ast,
    root: ExprId,
    expected: Value,
    types: &[Value],
) -> Result<Elaboration, Error> {
    let env = neutral_environment(types.len());
    check_in_env(ast, root, expected, &env, types)
}

/// Check with semantic values and types at corresponding De Bruijn levels.
pub fn check_in_env(
    ast: &Ast,
    root: ExprId,
    expected: Value,
    env: &[Value],
    types: &[Value],
) -> Result<Elaboration, Error> {
    validate_context(env, types)?;
    let mut checker = Checker::new(ast);
    checker.check(root, expected, types.len(), env, types)
}

fn neutral_environment(depth: usize) -> Vec<Value> {
    (0..depth)
        .map(|level| Value::Neutral(crate::eval::Neutral::Var(Level(level))))
        .collect()
}

fn validate_context(env: &[Value], types: &[Value]) -> Result<(), Error> {
    if env.len() != types.len() {
        return Err(Error::ContextLengthMismatch {
            values: env.len(),
            types: types.len(),
        });
    }
    Ok(())
}

pub fn synthesize_with_turbine(
    ast: &Ast,
    root: ExprId,
    types: &[Value],
    turbine: &crate::turbine::TurbineEngine,
) -> Result<Elaboration, Error> {
    let env = neutral_environment(types.len());
    let mut checker = Checker::with_turbine(ast, turbine);
    checker.synth(root, types.len(), &env, types)
}

pub fn check_with_turbine(
    ast: &Ast,
    root: ExprId,
    expected: Value,
    types: &[Value],
    turbine: &crate::turbine::TurbineEngine,
) -> Result<Elaboration, Error> {
    let env = neutral_environment(types.len());
    let mut checker = Checker::with_turbine(ast, turbine);
    checker.check(root, expected, types.len(), &env, types)
}

pub struct Checker<'a> {
    ast: &'a Ast,
    seen: Vec<bool>,
    turbine: Option<&'a crate::turbine::TurbineEngine>,
}

impl<'a> Checker<'a> {
    pub fn new(ast: &'a Ast) -> Self {
        Self {
            ast,
            seen: vec![false; ast.expression_count()],
            turbine: None,
        }
    }

    pub fn with_turbine(ast: &'a Ast, turbine: &'a crate::turbine::TurbineEngine) -> Self {
        Self {
            ast,
            seen: vec![false; ast.expression_count()],
            turbine: Some(turbine),
        }
    }

    fn check(
        &mut self,
        expr: ExprId,
        expected: Value,
        depth: usize,
        env: &[Value],
        types: &[Value],
    ) -> Result<Elaboration, Error> {
        let term = self.ast.expr(expr)?;
        if std::mem::replace(&mut self.seen[expr.index], true) {
            return Err(Error::SharedExpression(expr));
        }

        if let Expr::Lambda { quantity, body } = term {
            if let Value::Pi(decl_q, dom, cod_closure) = expected {
                if quantity != decl_q {
                    return Err(Error::QuantityMismatch {
                        expr,
                        expected: decl_q,
                        found: quantity,
                    });
                }
                let var = Value::Neutral(crate::eval::Neutral::Var(Level(depth)));
                let mut new_env = env.to_vec();
                new_env.push(var.clone());

                let expected_body_ty = cod_closure.clone().instantiate(self.ast, var);
                let mut new_types = types.to_vec();
                new_types.push((*dom).clone());

                let mut body_elab =
                    self.check(body, expected_body_ty, depth + 1, &new_env, &new_types)?;

                let observed = if body_elab.usages.len() > depth {
                    body_elab.usages[depth]
                } else {
                    Quantity::Zero
                };
                if !decl_q.permits(observed) {
                    return Err(Error::UsageMismatch {
                        expr,
                        declared: decl_q,
                        observed,
                    });
                }

                // Truncate the usage of the variable we just bound
                if body_elab.usages.len() > depth {
                    body_elab.usages.truncate(depth);
                }

                let out_ty = Value::Pi(decl_q, dom, cod_closure);
                if let Some(turbine) = self.turbine {
                    let snap =
                        crate::turbine::AtomicElabSnapshot::from_value(&out_ty, Quantity::Zero);
                    let _ = turbine.publish(expr, snap);
                }
                return Ok(Elaboration {
                    ty: out_ty,
                    usages: body_elab.usages,
                });
            } else {
                return Err(Error::ExpectedFunction {
                    expr,
                    found: format!("{:?}", expected),
                });
            }
        }

        let elab = self.synth_term(expr, term, depth, env, types)?;
        if !equiv(self.ast, &elab.ty, &expected, depth) {
            if let (Value::Universe(l1), Value::Universe(l2)) = (&elab.ty, &expected) {
                if l1 <= l2 {
                    return Ok(Elaboration {
                        ty: expected,
                        usages: elab.usages,
                    });
                }
            }
            return Err(Error::TypeMismatch {
                expr,
                expected: format!("{:?}", expected),
                found: format!("{:?}", elab.ty),
            });
        }
        if let Some(turbine) = self.turbine {
            let snap = crate::turbine::AtomicElabSnapshot::from_value(&elab.ty, Quantity::Zero);
            let _ = turbine.publish(expr, snap);
        }
        Ok(Elaboration {
            ty: expected,
            usages: elab.usages,
        })
    }

    fn synth(
        &mut self,
        expr: ExprId,
        depth: usize,
        env: &[Value],
        types: &[Value],
    ) -> Result<Elaboration, Error> {
        let term = self.ast.expr(expr)?;
        if std::mem::replace(&mut self.seen[expr.index], true) {
            return Err(Error::SharedExpression(expr));
        }
        let elab = self.synth_term(expr, term, depth, env, types)?;
        if let Some(turbine) = self.turbine {
            let snap = crate::turbine::AtomicElabSnapshot::from_value(&elab.ty, Quantity::Zero);
            let _ = turbine.publish(expr, snap);
        }
        Ok(elab)
    }

    fn synth_term(
        &mut self,
        expr: ExprId,
        term: Expr,
        depth: usize,
        env: &[Value],
        types: &[Value],
    ) -> Result<Elaboration, Error> {
        match term {
            Expr::Var(level) => {
                if let Some(ty) = types.get(level.0) {
                    let mut usages = vec![Quantity::Zero; depth.max(level.0 + 1)];
                    usages[level.0] = Quantity::One;
                    Ok(Elaboration {
                        ty: ty.clone(),
                        usages,
                    })
                } else {
                    Err(Error::UnboundVariable { expr, level })
                }
            }
            Expr::Universe(level) => {
                let successor = level
                    .checked_add(1)
                    .ok_or(Error::UniverseOverflow { expr, level })?;
                Ok(Elaboration {
                    ty: Value::Universe(successor),
                    usages: vec![],
                })
            }
            Expr::UnitType => Ok(Elaboration {
                ty: Value::Universe(0),
                usages: vec![],
            }),
            Expr::Unit => Ok(Elaboration {
                ty: Value::UnitType,
                usages: vec![],
            }),
            Expr::Pi {
                quantity: _,
                domain,
                codomain,
            } => {
                let dom_elab = self.synth(domain, depth, env, types)?;
                let u_dom = match dom_elab.ty {
                    Value::Universe(u) => u,
                    _ => {
                        return Err(Error::TypeMismatch {
                            expr: domain,
                            expected: "Universe".into(),
                            found: format!("{:?}", dom_elab.ty),
                        });
                    }
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
                    _ => {
                        return Err(Error::TypeMismatch {
                            expr: codomain,
                            expected: "Universe".into(),
                            found: format!("{:?}", cod_elab.ty),
                        });
                    }
                };
                Ok(Elaboration {
                    ty: Value::Universe(u_dom.max(u_cod)),
                    usages: vec![],
                })
            }
            Expr::Ann { term, ty } => {
                let ty_elab = self.synth(ty, depth, env, types)?;
                match ty_elab.ty {
                    Value::Universe(_) => {}
                    _ => {
                        return Err(Error::TypeMismatch {
                            expr: ty,
                            expected: "Universe".into(),
                            found: format!("{:?}", ty_elab.ty),
                        });
                    }
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
                    Ok(Elaboration {
                        ty: cod.instantiate(self.ast, arg_val),
                        usages: f_elab.usages,
                    })
                } else {
                    Err(Error::ExpectedFunction {
                        expr: function,
                        found: format!("{:?}", f_elab.ty),
                    })
                }
            }
            Expr::Lambda { .. } => Err(Error::CannotInferLambda(expr)),
        }
    }
}
