use crate::ast::{Ast, Expr, ExprId, Level, Quantity};
use crate::eval::{Value, equiv, eval};
use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::fmt;

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
impl core::error::Error for Error {}

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

    /// Unifies under one transaction so concurrent occurs checks cannot commit a cycle.
    #[cfg(feature = "std")]
    pub fn unify(
        ast: &Ast,
        a: &Value,
        b: &Value,
        depth: usize,
        turbine: &crate::turbine::TurbineEngine,
    ) -> bool {
        let _transaction = turbine
            .unification
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        Self::unify_in_transaction(ast, a, b, depth, turbine)
    }

    #[cfg(feature = "std")]
    fn unify_in_transaction(
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
                sp1.iter()
                    .zip(sp2.iter())
                    .all(|(x, y)| Self::unify_in_transaction(ast, x, y, depth, turbine))
            }
            (Value::Meta(m, sp), val) | (val, Value::Meta(m, sp)) => {
                if !turbine.has_meta(m) || Self::occurs_in_value(ast, m, &val, depth, turbine) {
                    return false;
                }
                if turbine.solve_meta_in_transaction(m, val.clone()) {
                    true
                } else {
                    let forced = turbine.force(ast, Value::Meta(m, sp));
                    Self::unify_in_transaction(ast, &forced, &val, depth, turbine)
                }
            }
            (Value::Unit, Value::Unit) => true,
            (Value::UnitType, Value::UnitType) => true,
            (Value::NatType, Value::NatType) => true,
            (Value::Zero, Value::Zero) => true,
            (Value::Succ(n1), Value::Succ(n2)) => {
                Self::unify_in_transaction(ast, &n1, &n2, depth, turbine)
            }
            (Value::IdType(t1, l1, r1), Value::IdType(t2, l2, r2)) => {
                Self::unify_in_transaction(ast, &t1, &t2, depth, turbine)
                    && Self::unify_in_transaction(ast, &l1, &l2, depth, turbine)
                    && Self::unify_in_transaction(ast, &r1, &r2, depth, turbine)
            }
            (Value::Refl(x1), Value::Refl(x2)) => {
                Self::unify_in_transaction(ast, &x1, &x2, depth, turbine)
            }
            (Value::Bool, Value::Bool) => true,
            (Value::True, Value::True) => true,
            (Value::False, Value::False) => true,
            (Value::Pair(f1, s1), Value::Pair(f2, s2)) => {
                Self::unify_in_transaction(ast, &f1, &f2, depth, turbine)
                    && Self::unify_in_transaction(ast, &s1, &s2, depth, turbine)
            }
            (Value::Sigma(q1, d1, c1), Value::Sigma(q2, d2, c2)) => {
                q1 == q2 && Self::unify_in_transaction(ast, &d1, &d2, depth, turbine) && {
                    let var = Value::Neutral(crate::eval::Neutral::Var(Level(depth)));
                    let v1 = c1.instantiate(ast, var.clone(), Some(turbine));
                    let v2 = c2.instantiate(ast, var, Some(turbine));
                    Self::unify_in_transaction(ast, &v1, &v2, depth + 1, turbine)
                }
            }
            (Value::Pi(p1, q1, d1, c1), Value::Pi(p2, q2, d2, c2)) => {
                p1 == p2
                    && q1 == q2
                    && Self::unify_in_transaction(ast, &d1, &d2, depth, turbine)
                    && {
                        let var = Value::Neutral(crate::eval::Neutral::Var(Level(depth)));
                        let v1 = c1.instantiate(ast, var.clone(), Some(turbine));
                        let v2 = c2.instantiate(ast, var, Some(turbine));
                        Self::unify_in_transaction(ast, &v1, &v2, depth + 1, turbine)
                    }
            }
            (Value::Lam(p1, q1, c1), Value::Lam(p2, q2, c2)) => {
                p1 == p2 && q1 == q2 && {
                    let var = Value::Neutral(crate::eval::Neutral::Var(Level(depth)));
                    let v1 = c1.instantiate(ast, var.clone(), Some(turbine));
                    let v2 = c2.instantiate(ast, var, Some(turbine));
                    Self::unify_in_transaction(ast, &v1, &v2, depth + 1, turbine)
                }
            }
            (Value::Lam(_p, _q, c), Value::Neutral(n)) => {
                let var = Value::Neutral(crate::eval::Neutral::Var(Level(depth)));
                let v1 = c.instantiate(ast, var.clone(), Some(turbine));
                let v2 = Value::Neutral(crate::eval::Neutral::App(
                    crate::ast::Plicity::Explicit,
                    Box::new(n),
                    Box::new(var),
                ));
                Self::unify_in_transaction(ast, &v1, &v2, depth + 1, turbine)
            }
            (Value::Neutral(n), Value::Lam(_p, _q, c)) => {
                let var = Value::Neutral(crate::eval::Neutral::Var(Level(depth)));
                let v1 = Value::Neutral(crate::eval::Neutral::App(
                    crate::ast::Plicity::Explicit,
                    Box::new(n),
                    Box::new(var.clone()),
                ));
                let v2 = c.instantiate(ast, var, Some(turbine));
                Self::unify_in_transaction(ast, &v1, &v2, depth + 1, turbine)
            }
            (Value::Universe(u1), Value::Universe(u2)) => u1 == u2,
            (Value::Neutral(n1), Value::Neutral(n2)) => {
                crate::eval::equiv_neu(ast, &n1, &n2, depth)
            }
            _ => false,
        }
    }

    #[cfg(feature = "std")]
    fn occurs_in_value(
        ast: &Ast,
        target: crate::ast::MetaId,
        val: &Value,
        depth: usize,
        turbine: &crate::turbine::TurbineEngine,
    ) -> bool {
        let forced = turbine.force(ast, val.clone());
        match forced {
            Value::Meta(m, spine) => {
                m == target
                    || spine
                        .iter()
                        .any(|arg| Self::occurs_in_value(ast, target, arg, depth, turbine))
            }
            Value::Succ(n) | Value::Refl(n) => {
                Self::occurs_in_value(ast, target, &n, depth, turbine)
            }
            Value::IdType(ty, lhs, rhs) => {
                Self::occurs_in_value(ast, target, &ty, depth, turbine)
                    || Self::occurs_in_value(ast, target, &lhs, depth, turbine)
                    || Self::occurs_in_value(ast, target, &rhs, depth, turbine)
            }
            Value::Pair(f, s) => {
                Self::occurs_in_value(ast, target, &f, depth, turbine)
                    || Self::occurs_in_value(ast, target, &s, depth, turbine)
            }
            Value::Sigma(_q, dom, cod) | Value::Pi(_, _q, dom, cod) => {
                if Self::occurs_in_value(ast, target, &dom, depth, turbine) {
                    return true;
                }
                let var = Value::Neutral(crate::eval::Neutral::Var(Level(depth)));
                let body = cod.instantiate(ast, var, Some(turbine));
                Self::occurs_in_value(ast, target, &body, depth + 1, turbine)
            }
            Value::Lam(_p, _q, clos) => {
                let var = Value::Neutral(crate::eval::Neutral::Var(Level(depth)));
                let body = clos.instantiate(ast, var, Some(turbine));
                Self::occurs_in_value(ast, target, &body, depth + 1, turbine)
            }
            Value::Neutral(neu) => Self::occurs_in_neutral(ast, target, &neu, depth, turbine),
            Value::Unit
            | Value::UnitType
            | Value::NatType
            | Value::Zero
            | Value::Universe(_)
            | Value::Bool
            | Value::True
            | Value::False => false,
        }
    }

    #[cfg(feature = "std")]
    fn occurs_in_neutral(
        ast: &Ast,
        target: crate::ast::MetaId,
        neu: &crate::eval::Neutral,
        depth: usize,
        turbine: &crate::turbine::TurbineEngine,
    ) -> bool {
        match neu {
            crate::eval::Neutral::Var(_) => false,
            crate::eval::Neutral::If(cond, conseq, alt) => {
                Self::occurs_in_neutral(ast, target, cond, depth, turbine)
                    || Self::occurs_in_value(ast, target, conseq, depth, turbine)
                    || Self::occurs_in_value(ast, target, alt, depth, turbine)
            }
            crate::eval::Neutral::Ind(mot, z, s, tgt) => {
                Self::occurs_in_value(ast, target, mot, depth, turbine)
                    || Self::occurs_in_value(ast, target, z, depth, turbine)
                    || Self::occurs_in_value(ast, target, s, depth, turbine)
                    || Self::occurs_in_neutral(ast, target, tgt, depth, turbine)
            }
            crate::eval::Neutral::J(mot, base, tgt) => {
                Self::occurs_in_value(ast, target, mot, depth, turbine)
                    || Self::occurs_in_value(ast, target, base, depth, turbine)
                    || Self::occurs_in_neutral(ast, target, tgt, depth, turbine)
            }
            crate::eval::Neutral::App(_plic, func, arg) => {
                Self::occurs_in_neutral(ast, target, func, depth, turbine)
                    || Self::occurs_in_value(ast, target, arg, depth, turbine)
            }
            crate::eval::Neutral::Fst(inner) | crate::eval::Neutral::Snd(inner) => {
                Self::occurs_in_neutral(ast, target, inner, depth, turbine)
            }
        }
    }

    /// Explicit conversion is available without the hosted metavariable solver.
    fn convert(&self, a: &Value, b: &Value, depth: usize) -> bool {
        #[cfg(feature = "std")]
        if let Some(turbine) = self.turbine {
            return Self::unify(self.ast, a, b, depth, turbine);
        }
        equiv(self.ast, a, b, depth)
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
        if core::mem::replace(&mut self.seen[expr.index], true) {
            return Err(Error::SharedExpression(expr));
        }

        if let Expr::Lambda {
            plicity: _,
            quantity,
            body,
        } = term
        {
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
                for u in first_elab.usages.iter_mut() {
                    *u = u.times(decl_q);
                }
                let first_val = eval(self.ast, first, env, self.turbine);
                let expected_second =
                    cod_closure
                        .clone()
                        .instantiate(self.ast, first_val, self.turbine);
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
                    let snap = crate::turbine::AtomicElabSnapshot::from_value(
                        &Value::Bool,
                        Quantity::Zero,
                    );
                    let _ = turbine.publish(expr, snap);
                }
                return Ok(Elaboration {
                    ty: Value::Bool,
                    usages: vec![],
                });
            }
        }

        if let Expr::Refl(x) = term {
            if let Value::IdType(ref ty_a, ref lhs, ref rhs) = expected {
                let _x_elab = self.check(x, (**ty_a).clone(), depth, env, types)?;
                let x_val = eval(self.ast, x, env, self.turbine);

                let unified_lhs = self.convert(&x_val, lhs, depth);
                let unified_rhs = self.convert(&x_val, rhs, depth);

                if !unified_lhs || !unified_rhs {
                    return Err(Error::TypeMismatch {
                        expr,
                        expected: format!("Id({:?}, {:?}, {:?})", ty_a, lhs, rhs),
                        found: format!("refl({:?})", x_val),
                    });
                }

                if let Some(turbine) = self.turbine {
                    let snap =
                        crate::turbine::AtomicElabSnapshot::from_value(&expected, Quantity::Zero);
                    let _ = turbine.publish(expr, snap);
                }
                return Ok(Elaboration {
                    ty: expected,
                    usages: vec![],
                });
            } else {
                return Err(Error::TypeMismatch {
                    expr,
                    expected: "IdType".into(),
                    found: format!("{:?}", expected),
                });
            }
        }

        let elab = self.synth_term(expr, term, depth, env, types)?;

        let unified = self.convert(&elab.ty, &expected, depth);

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
        if core::mem::replace(&mut self.seen[expr.index], true) {
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
                #[cfg(feature = "std")]
                if let Some(turbine) = self.turbine {
                    let meta_ty_id = turbine.new_meta();
                    let meta_val_id = turbine.new_meta();
                    turbine
                        .expr_to_meta
                        .write()
                        .unwrap_or_else(|e| e.into_inner())
                        .insert(expr, meta_val_id);
                    return Ok(Elaboration {
                        ty: Value::Meta(meta_ty_id, Vec::new()),
                        usages: vec![],
                    });
                }
                Err(Error::TypeMismatch {
                    expr,
                    expected: "hole inference requires std and a turbine".into(),
                    found: "?".into(),
                })
            }
            Expr::Meta(id) => {
                #[cfg(feature = "std")]
                if let Some(turbine) = self.turbine {
                    return Ok(Elaboration {
                        ty: turbine.force(self.ast, Value::Meta(id, Vec::new())),
                        usages: vec![],
                    });
                }
                Err(Error::TypeMismatch {
                    expr,
                    expected: "resolved term".into(),
                    found: format!("Meta({:?})", id),
                })
            }
            Expr::If { cond, conseq, alt } => {
                let cond_elab = self.check(cond, Value::Bool, depth, env, types)?;
                let conseq_elab = self.synth(conseq, depth, env, types)?;
                let alt_elab = self.check(alt, conseq_elab.ty.clone(), depth, env, types)?;

                let branch_len = conseq_elab.usages.len().max(alt_elab.usages.len());
                let mut branch_usages = vec![Quantity::Zero; branch_len];
                for (i, slot) in branch_usages.iter_mut().enumerate() {
                    let u_c = conseq_elab.usages.get(i).copied().unwrap_or(Quantity::Zero);
                    let u_a = alt_elab.usages.get(i).copied().unwrap_or(Quantity::Zero);
                    *slot = if u_c == u_a { u_c } else { Quantity::Omega };
                }

                let mut usages = cond_elab.usages;
                let max_len = usages.len().max(branch_usages.len());
                usages.resize(max_len, Quantity::Zero);
                for (i, u) in branch_usages.into_iter().enumerate() {
                    usages[i] = usages[i].plus(u);
                }

                Ok(Elaboration {
                    ty: conseq_elab.ty,
                    usages,
                })
            }
            Expr::Fst(body) => {
                let body_elab = self.synth(body, depth, env, types)?;
                if let Value::Sigma(q, dom, _) = body_elab.ty {
                    if q == Quantity::Zero {
                        return Err(Error::UsageMismatch {
                            expr,
                            declared: Quantity::Zero,
                            observed: Quantity::One,
                        });
                    }
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
                        other => other,
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
                let elab = self.check(term, ty_val, depth, env, types)?;
                Ok(elab)
            }
            Expr::App {
                plicity,
                function,
                argument,
            } => {
                let mut f_elab = self.synth(function, depth, env, types)?;

                #[cfg(feature = "std")]
                let mut implicits_to_insert = Vec::new();
                #[cfg(feature = "std")]
                while let Value::Pi(plic, _decl_q, _dom, _cod) = f_elab.ty.clone() {
                    if plic == crate::ast::Plicity::Implicit
                        && plicity == crate::ast::Plicity::Explicit
                    {
                        if let Some(turbine) = self.turbine {
                            let meta_val_id = turbine.new_meta();
                            let meta_val = Value::Meta(meta_val_id, env.to_vec());
                            f_elab.ty = _cod.instantiate(self.ast, meta_val, self.turbine);
                            implicits_to_insert.push(meta_val_id);
                            continue;
                        }
                        return Err(Error::TypeMismatch {
                            expr: function,
                            expected: "explicit arguments".to_string(),
                            found: "implicit inference requires std and a turbine".to_string(),
                        });
                    } else {
                        break;
                    }
                }

                #[cfg(not(feature = "std"))]
                if matches!(f_elab.ty, Value::Pi(crate::ast::Plicity::Implicit, ..))
                    && plicity == crate::ast::Plicity::Explicit
                {
                    return Err(Error::TypeMismatch {
                        expr: function,
                        expected: "explicit arguments".to_string(),
                        found: "implicit inference requires std and a turbine".to_string(),
                    });
                }

                #[cfg(feature = "std")]
                if !implicits_to_insert.is_empty() {
                    if let Some(turbine) = self.turbine {
                        turbine
                            .inserted_implicits
                            .write()
                            .unwrap_or_else(|e| e.into_inner())
                            .insert(expr, implicits_to_insert);
                    }
                }

                if let Value::Pi(_plic, decl_q, dom, cod) = f_elab.ty {
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
            Expr::NatType => {
                let ty = Value::Universe(0);
                Ok(Elaboration { ty, usages: vec![] })
            }
            Expr::Zero => {
                let ty = Value::NatType;
                Ok(Elaboration { ty, usages: vec![] })
            }
            Expr::Succ(n) => {
                let n_elab = self.check(n, Value::NatType, depth, env, types)?;
                Ok(Elaboration {
                    ty: Value::NatType,
                    usages: n_elab.usages,
                })
            }
            Expr::Ind { mot, z, s, target } => {
                self.synth_ind((mot, z, s, target), depth, env, types)
            }
            Expr::IdType { ty, lhs, rhs } => {
                let ty_elab = self.synth(ty, depth, env, types)?;
                let u_level = match ty_elab.ty {
                    Value::Universe(u) => u,
                    _ => {
                        return Err(Error::TypeMismatch {
                            expr: ty,
                            expected: "Universe".into(),
                            found: format!("{:?}", ty_elab.ty),
                        });
                    }
                };
                let ty_val = eval(self.ast, ty, env, self.turbine);
                let _lhs_elab = self.check(lhs, ty_val.clone(), depth, env, types)?;
                let _rhs_elab = self.check(rhs, ty_val, depth, env, types)?;

                Ok(Elaboration {
                    ty: Value::Universe(u_level),
                    usages: vec![],
                })
            }
            Expr::Refl(x) => {
                let x_elab = self.synth(x, depth, env, types)?;
                let x_val = eval(self.ast, x, env, self.turbine);
                let ty = Value::IdType(
                    Box::new(x_elab.ty),
                    Box::new(x_val.clone()),
                    Box::new(x_val),
                );
                Ok(Elaboration { ty, usages: vec![] })
            }
            Expr::J { mot, base, target } => self.synth_j(mot, base, target, depth, env, types),
        }
    }

    fn synth_ind(
        &mut self,
        (mot, z, s, target): (ExprId, ExprId, ExprId, ExprId),
        depth: usize,
        env: &[Value],
        types: &[Value],
    ) -> Result<Elaboration, Error> {
        let target_elab = self.synth(target, depth, env, types)?;
        let unified_target = self.convert(&target_elab.ty, &Value::NatType, depth);
        if !unified_target {
            return Err(Error::TypeMismatch {
                expr: target,
                expected: "Nat".into(),
                found: format!("{:?}", target_elab.ty),
            });
        }

        let target_val = crate::eval::eval(self.ast, target, env, self.turbine);

        let mot_term = self.ast.expr(mot)?;
        if let Expr::Lambda {
            body: mot_body1, ..
        } = mot_term
        {
            let mut env_mot = env.to_vec();
            let mut types_mot = types.to_vec();
            types_mot.push(Value::NatType);
            let var_n = Value::Neutral(crate::eval::Neutral::Var(crate::ast::Level(depth)));
            env_mot.push(var_n);

            let mot_body_elab = self.synth(mot_body1, depth + 1, &env_mot, &types_mot)?;
            if !matches!(mot_body_elab.ty, Value::Universe(_)) {
                return Err(Error::TypeMismatch {
                    expr: mot_body1,
                    expected: "Universe".into(),
                    found: format!("{:?}", mot_body_elab.ty),
                });
            }
        } else {
            let mot_elab = self.synth(mot, depth, env, types)?;
            match mot_elab.ty {
                Value::Pi(_, _, ref domain, _) => {
                    let unified_domain = self.convert(domain, &Value::NatType, depth);
                    if !unified_domain {
                        return Err(Error::TypeMismatch {
                            expr: mot,
                            expected: "Pi(Nat, Universe)".into(),
                            found: format!("{:?}", mot_elab.ty),
                        });
                    }
                }
                other => {
                    return Err(Error::ExpectedFunction {
                        expr: mot,
                        found: format!("{:?}", other),
                    });
                }
            }
        }
        let mot_val = crate::eval::eval(self.ast, mot, env, self.turbine);

        let z_ty = crate::eval::apply(
            self.ast,
            crate::ast::Plicity::Explicit,
            mot_val.clone(),
            Value::Zero,
            self.turbine,
        );
        let mut z_elab = self.check(z, z_ty, depth, env, types)?;

        let s_term = self.ast.expr(s)?;
        let (mut s_usages, q_ih_scale) = if let Expr::Lambda {
            quantity: q_n,
            body: s_body1,
            ..
        } = s_term
        {
            let mut env1 = env.to_vec();
            let mut types1 = types.to_vec();
            types1.push(Value::NatType);
            let var_n = Value::Neutral(crate::eval::Neutral::Var(crate::ast::Level(depth)));
            env1.push(var_n.clone());

            let s_body1_term = self.ast.expr(s_body1)?;
            if let Expr::Lambda {
                quantity: q_ih,
                body: s_body2,
                ..
            } = s_body1_term
            {
                let mut env2 = env1;
                let mut types2 = types1;

                let mot_n = crate::eval::apply(
                    self.ast,
                    crate::ast::Plicity::Explicit,
                    mot_val.clone(),
                    var_n.clone(),
                    self.turbine,
                );
                types2.push(mot_n);
                let var_ih =
                    Value::Neutral(crate::eval::Neutral::Var(crate::ast::Level(depth + 1)));
                env2.push(var_ih);

                let succ_n = Value::Succ(Box::new(var_n));
                let mot_succ_n = crate::eval::apply(
                    self.ast,
                    crate::ast::Plicity::Explicit,
                    mot_val.clone(),
                    succ_n,
                    self.turbine,
                );

                let mut s_body_elab = self.check(s_body2, mot_succ_n, depth + 2, &env2, &types2)?;
                let obs_ih = s_body_elab
                    .usages
                    .get(depth + 1)
                    .copied()
                    .unwrap_or(Quantity::Zero);
                if !q_ih.permits(obs_ih) {
                    return Err(Error::UsageMismatch {
                        expr: s_body1,
                        declared: q_ih,
                        observed: obs_ih,
                    });
                }
                let obs_n = s_body_elab
                    .usages
                    .get(depth)
                    .copied()
                    .unwrap_or(Quantity::Zero);
                if !q_n.permits(obs_n) {
                    return Err(Error::UsageMismatch {
                        expr: s,
                        declared: q_n,
                        observed: obs_n,
                    });
                }
                if s_body_elab.usages.len() > depth {
                    s_body_elab.usages.truncate(depth);
                }
                // If `target` is dynamically zero or non-zero, base case `z` is used once at 0
                // and `q_ih^k` times at step k. Thus `q_ih == Zero` or `q_ih == Omega` (or `obs_ih != One`)
                // makes base case usage non-uniform across recursion depths, escalating to Omega.
                let base_scale = if q_ih == Quantity::One && obs_ih == Quantity::One {
                    Quantity::One
                } else {
                    Quantity::Omega
                };
                (s_body_elab.usages, base_scale)
            } else {
                return Err(Error::ExpectedFunction {
                    expr: s_body1,
                    found: format!("{:?}", s_body1_term),
                });
            }
        } else {
            return Err(Error::ExpectedFunction {
                expr: s,
                found: format!("{:?}", s_term),
            });
        };

        for u in z_elab.usages.iter_mut() {
            *u = u.times(q_ih_scale);
        }
        for u in s_usages.iter_mut() {
            *u = u.times(Quantity::Omega);
        }

        let mut usages = target_elab.usages;
        let max_len = usages.len().max(z_elab.usages.len()).max(s_usages.len());
        usages.resize(max_len, Quantity::Zero);
        for (i, u) in z_elab.usages.into_iter().enumerate() {
            usages[i] = usages[i].plus(u);
        }
        for (i, u) in s_usages.into_iter().enumerate() {
            usages[i] = usages[i].plus(u);
        }

        let ret_ty = crate::eval::apply(
            self.ast,
            crate::ast::Plicity::Explicit,
            mot_val,
            target_val,
            self.turbine,
        );
        Ok(Elaboration { ty: ret_ty, usages })
    }

    fn synth_j(
        &mut self,
        mot: ExprId,
        base: ExprId,
        target: ExprId,
        depth: usize,
        env: &[Value],
        types: &[Value],
    ) -> Result<Elaboration, Error> {
        let target_elab = self.synth(target, depth, env, types)?;
        let (ty_a, lhs, rhs) = match target_elab.ty {
            Value::IdType(ty_a, lhs, rhs) => (*ty_a, *lhs, *rhs),
            _ => {
                return Err(Error::TypeMismatch {
                    expr: target,
                    expected: "IdType".into(),
                    found: format!("{:?}", target_elab.ty),
                });
            }
        };
        let target_val = eval(self.ast, target, env, self.turbine);
        let mot_val = eval(self.ast, mot, env, self.turbine);

        let mot_term = self.ast.expr(mot)?;
        if let Expr::Lambda {
            body: mot_body1, ..
        } = mot_term
        {
            let mut env1 = env.to_vec();
            let mut types1 = types.to_vec();
            types1.push(ty_a.clone());
            let var_x = Value::Neutral(crate::eval::Neutral::Var(crate::ast::Level(depth)));
            env1.push(var_x.clone());

            let mot_body1_term = self.ast.expr(mot_body1)?;
            if let Expr::Lambda {
                body: mot_body2, ..
            } = mot_body1_term
            {
                let mut env2 = env1;
                let mut types2 = types1;
                let id_ty = Value::IdType(Box::new(ty_a), Box::new(lhs.clone()), Box::new(var_x));
                types2.push(id_ty);
                let var_p = Value::Neutral(crate::eval::Neutral::Var(crate::ast::Level(depth + 1)));
                env2.push(var_p);

                let mot_body_elab = self.synth(mot_body2, depth + 2, &env2, &types2)?;
                if !matches!(mot_body_elab.ty, Value::Universe(_)) {
                    return Err(Error::TypeMismatch {
                        expr: mot_body2,
                        expected: "Universe".into(),
                        found: format!("{:?}", mot_body_elab.ty),
                    });
                }
            }
        }

        let refl_lhs = Value::Refl(Box::new(lhs.clone()));
        let step1 = crate::eval::apply(
            self.ast,
            crate::ast::Plicity::Explicit,
            mot_val.clone(),
            lhs,
            self.turbine,
        );
        let expected_base_ty = crate::eval::apply(
            self.ast,
            crate::ast::Plicity::Explicit,
            step1,
            refl_lhs,
            self.turbine,
        );
        let base_elab = self.check(base, expected_base_ty, depth, env, types)?;

        let step_rhs = crate::eval::apply(
            self.ast,
            crate::ast::Plicity::Explicit,
            mot_val,
            rhs,
            self.turbine,
        );
        let ret_ty = crate::eval::apply(
            self.ast,
            crate::ast::Plicity::Explicit,
            step_rhs,
            target_val,
            self.turbine,
        );

        Ok(Elaboration {
            ty: ret_ty,
            usages: base_elab.usages,
        })
    }
}
