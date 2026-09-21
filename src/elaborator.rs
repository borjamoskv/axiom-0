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

    pub fn unify(
        ast: &Ast,
        a: &Value,
        b: &Value,
        depth: usize,
        turbine: &crate::turbine::TurbineEngine,
    ) -> bool {
        let a = turbine.force(ast, a.clone());
        let b = turbine.force(ast, b.clone());

        match (a.clone(), b.clone()) {
            (Value::Meta(m1, sp1), Value::Meta(m2, sp2)) if m1 == m2 && sp1.len() == sp2.len() => {
                sp1.iter().zip(sp2.iter()).all(|(x, y)| Self::unify(ast, x, y, depth, turbine))
            }
            (Value::Meta(m, sp), val) | (val, Value::Meta(m, sp)) => {
                // Occurs check omitted for MVP
                turbine.solve_meta(m, val);
                true
            }
            (Value::Unit, Value::Unit) => true,
            (Value::UnitType, Value::UnitType) => true,
            (Value::Bool, Value::Bool) => true,
            (Value::True, Value::True) => true,
            (Value::False, Value::False) => true,
            (Value::Pair(f1, s1), Value::Pair(f2, s2)) => {
                Self::unify(ast, &f1, &f2, depth, turbine) && Self::unify(ast, &s1, &s2, depth, turbine)
            }
            (Value::Sigma(q1, d1, c1), Value::Sigma(q2, d2, c2)) => {
                q1 == q2 && Self::unify(ast, &d1, &d2, depth, turbine) && {
                    let var = Value::Neutral(crate::eval::Neutral::Var(Level(depth)));
                    let v1 = c1.instantiate(ast, var.clone(), Some(turbine));
                    let v2 = c2.instantiate(ast, var, Some(turbine));
                    Self::unify(ast, &v1, &v2, depth + 1, turbine)
                }
            }
            (Value::Pi(_, q1, d1, c1), Value::Pi(_, q2, d2, c2)) => {
                q1 == q2 && Self::unify(ast, &d1, &d2, depth, turbine) && {
                    let var = Value::Neutral(crate::eval::Neutral::Var(Level(depth)));
                    let v1 = c1.instantiate(ast, var.clone(), Some(turbine));
                    let v2 = c2.instantiate(ast, var, Some(turbine));
                    Self::unify(ast, &v1, &v2, depth + 1, turbine)
                }
            }
            (Value::Universe(u1), Value::Universe(u2)) => u1 == u2,
            (Value::Neutral(n1), Value::Neutral(n2)) => crate::eval::equiv_neu(ast, &n1, &n2, depth), // Might need turbine for neu
            _ => false,
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

        if let Expr::Lambda { plicity, quantity, body } = term {
            if let Value::Pi(plic, decl_q, dom, cod_closure) = expected {
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

                let expected_body_ty = cod_closure.clone().instantiate(self.ast, var, self.turbine);
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

                let out_ty = Value::Pi(plic, decl_q, dom, cod_closure);
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

        if let Expr::Pair { first, second } = term {
            if let Value::Sigma(decl_q, dom, cod_closure) = expected.clone() {
                let mut first_elab = self.check(first, (*dom).clone(), depth, env, types)?;
                let first_val = eval(self.ast, first, env, self.turbine);
                let expected_second = cod_closure.clone().instantiate(self.ast, first_val, self.turbine);
                let second_elab = self.check(second, expected_second, depth, env, types)?;

                let max_len = first_elab.usages.len().max(second_elab.usages.len());
                first_elab.usages.resize(max_len, Quantity::Zero);
                for (i, u) in second_elab.usages.into_iter().enumerate() {
                    first_elab.usages[i] = first_elab.usages[i].plus(u);
                }

                let out_ty = Value::Sigma(decl_q, dom, cod_closure);
                if let Some(turbine) = self.turbine {
                    let snap =
                        crate::turbine::AtomicElabSnapshot::from_value(&out_ty, Quantity::Zero);
                    let _ = turbine.publish(expr, snap);
                }
                return Ok(Elaboration {
                    ty: out_ty,
                    usages: first_elab.usages,
                });
            } else {
                return Err(Error::TypeMismatch {
                    expr,
                    expected: "Sigma".into(),
                    found: format!("{:?}", expected),
                });
            }
        }

        if let Expr::True | Expr::False = term {
            if let Value::Bool = expected {
                if let Some(turbine) = self.turbine {
                    let snap = crate::turbine::AtomicElabSnapshot::from_value(&Value::Bool, Quantity::Zero);
                    let _ = turbine.publish(expr, snap);
                }
                return Ok(Elaboration { ty: Value::Bool, usages: vec![] });
            }
        }

        let elab = self.synth_term(expr, term, depth, env, types)?;
        
        let unified = if let Some(t) = self.turbine {
            Self::unify(self.ast, &elab.ty, &expected, depth, t)
        } else {
            equiv(self.ast, &elab.ty, &expected, depth)
        };

        if !unified {
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
            Expr::Bool => Ok(Elaboration {
                ty: Value::Universe(0),
                usages: vec![],
            }),
            Expr::True | Expr::False => Ok(Elaboration {
                ty: Value::Bool,
                usages: vec![],
            }),
            Expr::Hole => {
                if let Some(turbine) = self.turbine {
                    let meta_ty_id = turbine.new_meta();
                    let meta_val_id = turbine.new_meta();
                    turbine.expr_to_meta.write().unwrap().insert(expr, meta_val_id);
                    let meta_ty = Value::Meta(meta_ty_id, Vec::new());
                    Ok(Elaboration {
                        ty: meta_ty,
                        usages: vec![],
                    })
                } else {
                    Err(Error::TypeMismatch {
                        expr,
                        expected: "no holes allowed without turbine".into(),
                        found: "?".into(),
                    })
                }
            }
            Expr::Meta(id) => {
                // If it's explicitly written in AST, it should have a type, but we can't easily know it here.
                // Normally holes are checked, or their type is inferred.
                // We'll panic if Meta is found directly in synth without being resolved.
                panic!("Expr::Meta synthesized directly");
            }
            Expr::If { cond, conseq, alt } => {
                let cond_elab = self.check(cond, Value::Bool, depth, env, types)?;
                let conseq_elab = self.synth(conseq, depth, env, types)?;
                let alt_elab = self.check(alt, conseq_elab.ty.clone(), depth, env, types)?;

                let mut usages = cond_elab.usages;
                let max_len = usages.len().max(conseq_elab.usages.len()).max(alt_elab.usages.len());
                usages.resize(max_len, Quantity::Zero);
                for (i, u) in conseq_elab.usages.into_iter().enumerate() {
                    usages[i] = usages[i].plus(u);
                }
                for (i, u) in alt_elab.usages.into_iter().enumerate() {
                    usages[i] = usages[i].plus(u);
                }

                Ok(Elaboration {
                    ty: conseq_elab.ty,
                    usages,
                })
            }
            Expr::Fst(body) => {
                let body_elab = self.synth(body, depth, env, types)?;
                if let Value::Sigma(_q, dom, _) = body_elab.ty {
                    Ok(Elaboration {
                        ty: *dom,
                        usages: body_elab.usages,
                    })
                } else {
                    Err(Error::TypeMismatch {
                        expr: body,
                        expected: "Sigma".into(),
                        found: format!("{:?}", body_elab.ty),
                    })
                }
            }
            Expr::Snd(body) => {
                let body_elab = self.synth(body, depth, env, types)?;
                if let Value::Sigma(_q, _, cod_closure) = body_elab.ty {
                    let body_val = eval(self.ast, body, env, self.turbine);
                    let fst_val = match body_val {
                        Value::Pair(first, _) => *first,
                        Value::Neutral(n) => Value::Neutral(crate::eval::Neutral::Fst(Box::new(n))),
                        _ => panic!("eval error in elaborator for snd"),
                    };
                    Ok(Elaboration {
                        ty: cod_closure.instantiate(self.ast, fst_val, self.turbine),
                        usages: body_elab.usages,
                    })
                } else {
                    Err(Error::TypeMismatch {
                        expr: body,
                        expected: "Sigma".into(),
                        found: format!("{:?}", body_elab.ty),
                    })
                }
            }
            Expr::Sigma {
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
                let dom_val = eval(self.ast, domain, env, self.turbine);
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
            Expr::Pi {
                plicity: _,
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
                let dom_val = eval(self.ast, domain, env, self.turbine);
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
                let ty_val = eval(self.ast, ty, env, self.turbine);
                let elab = self.check(term, ty_val.clone(), depth, env, types)?;
                Ok(elab)
            }
            Expr::App { plicity, function, argument } => {
                let mut f_elab = self.synth(function, depth, env, types)?;
                
                let mut implicits_to_insert = Vec::new();
                while let Value::Pi(plic, _decl_q, _dom, cod) = f_elab.ty.clone() {
                    if plic == crate::ast::Plicity::Implicit && plicity == crate::ast::Plicity::Explicit {
                        if let Some(turbine) = self.turbine {
                            let meta_val_id = turbine.new_meta();
                            let meta_val = Value::Meta(meta_val_id, env.to_vec());
                            f_elab.ty = cod.instantiate(self.ast, meta_val, self.turbine);
                            implicits_to_insert.push(meta_val_id);
                        } else {
                            return Err(Error::TypeMismatch {
                                expr: function,
                                expected: "Explicit function".to_string(),
                                found: "Implicit function without Turbine".to_string(),
                            });
                        }
                    } else {
                        break;
                    }
                }

                if !implicits_to_insert.is_empty() {
                    if let Some(turbine) = self.turbine {
                        turbine.inserted_implicits.write().unwrap().insert(expr, implicits_to_insert);
                    }
                }

                if let Value::Pi(plic, decl_q, dom, cod) = f_elab.ty {
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

                    let arg_val = eval(self.ast, argument, env, self.turbine);
                    Ok(Elaboration {
                        ty: cod.instantiate(self.ast, arg_val, self.turbine),
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
            Expr::Pair { .. } => Err(Error::CannotInferLambda(expr)),
        }
    }
}
