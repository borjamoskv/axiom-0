//! Proof Erasure and Runtime Transduction for Quantitative Type Theory (QTT).
//!
//! Implements strict computational extraction from verified AXIOM-0 terms:
//! 1. All quantities with $\mathcal{Q} = 0$ (types, equality proofs, erased arguments) are eradicated.
//! 2. Martin-Löf identity eliminators $J(P, d, a, p)$ collapse definitionally to $d$ ($O(0)$ overhead).
//! 3. $\Sigma$-types with erased second components (subsets/refinements) unbox directly into raw scalars.
//! 4. Peano natural numbers unbox into native 64-bit unsigned integers (`u64`).
//! 5. Emits pure C99 code for zero-cost foreign function interfacing (C-ABI).

use crate::ast::{Ast, AstError, Expr, ExprId, Level, Quantity};
use alloc::{boxed::Box, vec::Vec};

/// An untyped, erased runtime term stripped of all logical proofs and type annotations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeTerm {
    /// Erased proof, type, or zero-quantity computational marker.
    Erased,
    /// Computational unit value `()`.
    Unit,
    /// Boolean primitive.
    Bool(bool),
    /// Unboxed 64-bit unsigned machine integer.
    Nat(u64),
    /// Successor constructor for non-constant natural terms.
    Succ(Box<RuntimeTerm>),
    /// Runtime De Bruijn variable level.
    Var(usize),
    /// Runtime function abstraction (only retained for $\mathcal{Q} \neq 0$ parameters).
    Lam(Box<RuntimeTerm>),
    /// Runtime closure pairing an abstraction body with captured environment.
    Closure(Box<RuntimeTerm>, Vec<RuntimeTerm>),
    /// Runtime function application.
    App(Box<RuntimeTerm>, Box<RuntimeTerm>),
    /// Primitive branch instruction.
    If {
        cond: Box<RuntimeTerm>,
        conseq: Box<RuntimeTerm>,
        alt: Box<RuntimeTerm>,
    },
    /// Runtime pair (only retained if both components are computational).
    Pair(Box<RuntimeTerm>, Box<RuntimeTerm>),
    /// First projection $\pi_1$.
    Fst(Box<RuntimeTerm>),
    /// Second projection $\pi_2$.
    Snd(Box<RuntimeTerm>),
    /// Primitive recursion on Peano naturals.
    NatRec {
        base: Box<RuntimeTerm>,
        step: Box<RuntimeTerm>,
        target: Box<RuntimeTerm>,
    },
}

impl RuntimeTerm {
    /// Returns true if this term is an erased proof or type marker.
    pub fn is_erased(&self) -> bool {
        matches!(self, Self::Erased | Self::Unit)
    }

    /// Returns true if this term is strictly a non-computational erased proof/type (`RuntimeTerm::Erased`).
    pub fn is_proof_erased(&self) -> bool {
        matches!(self, Self::Erased)
    }

    /// Size in AST nodes of the erased runtime term.
    pub fn node_count(&self) -> usize {
        match self {
            Self::Erased | Self::Unit | Self::Bool(_) | Self::Nat(_) | Self::Var(_) => 1,
            Self::Succ(b) | Self::Lam(b) | Self::Fst(b) | Self::Snd(b) => 1 + b.node_count(),
            Self::Closure(b, _) => 1 + b.node_count(),
            Self::App(f, a) | Self::Pair(f, a) => 1 + f.node_count() + a.node_count(),
            Self::If { cond, conseq, alt } => {
                1 + cond.node_count() + conseq.node_count() + alt.node_count()
            }
            Self::NatRec { base, step, target } => {
                1 + base.node_count() + step.node_count() + target.node_count()
            }
        }
    }
}

/// Erases proofs, types, and zero-quantity parameters from an elaborated AST.
pub fn erase_expression(ast: &Ast, root: ExprId) -> Result<RuntimeTerm, AstError> {
    let mut level_map = Vec::new();
    let mut runtime_depth = 0;
    let mut ty_env = Vec::new();
    let mut val_env = Vec::new();
    let term = erase_expr_rec(
        ast,
        root,
        &mut level_map,
        &mut runtime_depth,
        &mut ty_env,
        &mut val_env,
    )?;
    Ok(normalize_erased_root(term))
}

fn normalize_erased_root(term: RuntimeTerm) -> RuntimeTerm {
    match term {
        RuntimeTerm::Erased => RuntimeTerm::Unit,
        other => other,
    }
}

/// Whether inhabitants of this type are erased (types themselves or equality proofs).
/// Values of Nat, Bool, Unit, Pi and Sigma types remain computational.
fn is_type_or_proof_value(val: &crate::eval::Value) -> bool {
    use crate::eval::Value;
    matches!(val, Value::Universe(_) | Value::IdType(..))
}

/// Extracts the domain type (`Value`) of a lambda binder if known from an enclosing `Pi` type.
fn extract_pi_domain_and_codomain(
    ast: &Ast,
    expected_ty: Option<&crate::eval::Value>,
    depth: usize,
) -> (Option<crate::eval::Value>, Option<crate::eval::Value>) {
    if let Some(crate::eval::Value::Pi(_plic, _q, dom, cod)) = expected_ty {
        let var = crate::eval::Value::Neutral(crate::eval::Neutral::Var(Level(depth)));
        let cod_val = cod.clone().instantiate(ast, var, None);
        (Some((**dom).clone()), Some(cod_val))
    } else {
        (None, None)
    }
}

/// Infers the semantic type (`Value`) of `expr_id` if possible in the current erasure context.
fn infer_expr_type_in_context(
    ast: &Ast,
    expr_id: ExprId,
    ty_env: &[Option<crate::eval::Value>],
    val_env: &[crate::eval::Value],
) -> Result<Option<crate::eval::Value>, AstError> {
    match ast.expr(expr_id)? {
        Expr::Var(Level(lvl)) => Ok(ty_env.get(lvl).cloned().flatten()),
        Expr::Ann { ty, .. } => Ok(Some(crate::eval::eval(ast, ty, val_env, None))),
        Expr::Fst(pair) => {
            if let Some(crate::eval::Value::Sigma(_q, dom, _cod)) =
                infer_expr_type_in_context(ast, pair, ty_env, val_env)?
            {
                Ok(Some(*dom))
            } else {
                Ok(None)
            }
        }
        Expr::Snd(pair) => {
            if let Some(crate::eval::Value::Sigma(_q, _dom, cod)) =
                infer_expr_type_in_context(ast, pair, ty_env, val_env)?
            {
                let pair_val = crate::eval::eval(ast, pair, val_env, None);
                let fst_val = match pair_val {
                    crate::eval::Value::Pair(f, _) => *f,
                    crate::eval::Value::Neutral(n) => {
                        crate::eval::Value::Neutral(crate::eval::Neutral::Fst(Box::new(n)))
                    }
                    other => other,
                };
                Ok(Some(cod.instantiate(ast, fst_val, None)))
            } else {
                Ok(None)
            }
        }
        Expr::App {
            function, argument, ..
        } => {
            if let Some(crate::eval::Value::Pi(_plic, _q, _dom, cod)) =
                infer_expr_type_in_context(ast, function, ty_env, val_env)?
            {
                let arg_val = crate::eval::eval(ast, argument, val_env, None);
                Ok(Some(cod.instantiate(ast, arg_val, None)))
            } else {
                Ok(None)
            }
        }
        Expr::If { conseq, alt, .. } => {
            if let Some(t) = infer_expr_type_in_context(ast, conseq, ty_env, val_env)? {
                Ok(Some(t))
            } else {
                infer_expr_type_in_context(ast, alt, ty_env, val_env)
            }
        }
        Expr::Ind { mot, target, .. } => {
            let mot_val = crate::eval::eval(ast, mot, val_env, None);
            let tgt_val = crate::eval::eval(ast, target, val_env, None);
            Ok(Some(crate::eval::apply(
                ast,
                crate::ast::Plicity::Explicit,
                mot_val,
                tgt_val,
                None,
            )))
        }
        Expr::J { base, .. } => infer_expr_type_in_context(ast, base, ty_env, val_env),
        _ => {
            let types: Vec<crate::eval::Value> = ty_env
                .iter()
                .map(|t| t.clone().unwrap_or(crate::eval::Value::Universe(0)))
                .collect();
            if let Ok(elab) = crate::elaborator::synthesize_in_env(ast, expr_id, val_env, &types) {
                Ok(Some(elab.ty))
            } else {
                Ok(None)
            }
        }
    }
}

/// Infers the sequence of parameter quantities (`Quantity`) for a function expression `expr_id`
/// using context types, annotations, or syntactic binders.
fn infer_function_param_quantities(
    ast: &Ast,
    mut expr_id: ExprId,
    ty_env: &[Option<crate::eval::Value>],
    val_env: &[crate::eval::Value],
) -> Result<Vec<Quantity>, AstError> {
    if let Some(mut ty) = infer_expr_type_in_context(ast, expr_id, ty_env, val_env)? {
        let mut quantities = Vec::new();
        while let crate::eval::Value::Pi(_plic, q, _dom, cod) = ty {
            quantities.push(q);
            let var = crate::eval::Value::Neutral(crate::eval::Neutral::Var(Level(
                val_env.len() + quantities.len(),
            )));
            ty = cod.instantiate(ast, var, None);
        }
        if !quantities.is_empty() {
            return Ok(quantities);
        }
    }

    let mut quantities = Vec::new();
    loop {
        match ast.expr(expr_id)? {
            Expr::Ann { term, ty } => {
                let mut cur_ty = ty;
                while let Ok(Expr::Pi {
                    quantity, codomain, ..
                }) = ast.expr(cur_ty)
                {
                    quantities.push(quantity);
                    cur_ty = codomain;
                }
                if !quantities.is_empty() {
                    return Ok(quantities);
                }
                expr_id = term;
            }
            Expr::Lambda { quantity, body, .. } => {
                quantities.push(quantity);
                expr_id = body;
            }
            Expr::App { function, .. } => {
                let mut inner = infer_function_param_quantities(ast, function, ty_env, val_env)?;
                if !inner.is_empty() {
                    inner.remove(0);
                }
                return Ok(inner);
            }
            _ => return Ok(quantities),
        }
    }
}

/// Determines whether a pair expression has its first or second component erased
/// (`(first_erased, second_erased)`), using context types, annotations, or literal structure.
fn pair_component_erasure(
    ast: &Ast,
    mut expr_id: ExprId,
    level_map: &mut Vec<Option<usize>>,
    runtime_depth: &mut usize,
    ty_env: &mut Vec<Option<crate::eval::Value>>,
    val_env: &mut Vec<crate::eval::Value>,
) -> Result<(bool, bool), AstError> {
    if let Some(crate::eval::Value::Sigma(q, dom, cod)) =
        infer_expr_type_in_context(ast, expr_id, ty_env, val_env)?
    {
        let var = crate::eval::Value::Neutral(crate::eval::Neutral::Var(Level(val_env.len())));
        let cod_val = cod.instantiate(ast, var, None);
        let first_erased = q == Quantity::Zero || is_type_or_proof_value(&dom);
        let second_erased = is_type_or_proof_value(&cod_val);
        return Ok((first_erased, second_erased));
    }

    loop {
        match ast.expr(expr_id)? {
            Expr::Ann { term, ty } => {
                if let Ok(Expr::Sigma {
                    quantity,
                    domain,
                    codomain,
                }) = ast.expr(ty)
                {
                    let dom_erased = quantity == Quantity::Zero
                        || matches!(
                            ast.expr(domain)?,
                            Expr::Universe(_)
                                | Expr::IdType { .. }
                                | Expr::Pi { .. }
                                | Expr::Sigma { .. }
                        );
                    let cod_erased = matches!(
                        ast.expr(codomain)?,
                        Expr::Universe(_)
                            | Expr::IdType { .. }
                            | Expr::Pi { .. }
                            | Expr::Sigma { .. }
                    );
                    if dom_erased || cod_erased {
                        return Ok((dom_erased, cod_erased));
                    }
                }
                expr_id = term;
            }
            Expr::Pair { first, second } => {
                let e1 = erase_expr_rec(ast, first, level_map, runtime_depth, ty_env, val_env)?;
                let e2 = erase_expr_rec(ast, second, level_map, runtime_depth, ty_env, val_env)?;
                return Ok((e1.is_proof_erased(), e2.is_proof_erased()));
            }
            _ => return Ok((false, false)),
        }
    }
}

fn erase_expr_rec(
    ast: &Ast,
    expr_id: ExprId,
    level_map: &mut Vec<Option<usize>>,
    runtime_depth: &mut usize,
    ty_env: &mut Vec<Option<crate::eval::Value>>,
    val_env: &mut Vec<crate::eval::Value>,
) -> Result<RuntimeTerm, AstError> {
    erase_expr_rec_typed(
        ast,
        expr_id,
        None,
        level_map,
        runtime_depth,
        ty_env,
        val_env,
    )
}

fn erase_expr_rec_typed(
    ast: &Ast,
    expr_id: ExprId,
    expected_ty: Option<&crate::eval::Value>,
    level_map: &mut Vec<Option<usize>>,
    runtime_depth: &mut usize,
    ty_env: &mut Vec<Option<crate::eval::Value>>,
    val_env: &mut Vec<crate::eval::Value>,
) -> Result<RuntimeTerm, AstError> {
    let expr = ast.expr(expr_id)?;
    match expr {
        // Types and proposition constructors are completely erased
        Expr::Universe(_) | Expr::UnitType | Expr::NatType | Expr::Bool => Ok(RuntimeTerm::Erased),
        Expr::IdType { .. } => Ok(RuntimeTerm::Erased),
        Expr::Refl(_) => Ok(RuntimeTerm::Erased),

        // Primitive values
        Expr::Unit => Ok(RuntimeTerm::Unit),
        Expr::True => Ok(RuntimeTerm::Bool(true)),
        Expr::False => Ok(RuntimeTerm::Bool(false)),
        Expr::Zero => Ok(RuntimeTerm::Nat(0)),

        Expr::Succ(inner) => {
            let erased_inner =
                erase_expr_rec(ast, inner, level_map, runtime_depth, ty_env, val_env)?;
            match erased_inner {
                RuntimeTerm::Nat(n) => Ok(RuntimeTerm::Nat(n.saturating_add(1))),
                other => Ok(RuntimeTerm::Succ(Box::new(other))),
            }
        }

        Expr::Var(Level(lvl)) => {
            if let Some(Some(ty)) = ty_env.get(lvl) {
                if is_type_or_proof_value(ty) {
                    return Ok(RuntimeTerm::Erased);
                }
            }
            if let Some(mapped) = level_map.get(lvl) {
                match mapped {
                    Some(rt_lvl) => Ok(RuntimeTerm::Var(*rt_lvl)),
                    None => Ok(RuntimeTerm::Erased),
                }
            } else {
                // Free variable from outer context: adjust by number of erased binders currently in scope
                let erased_count = level_map.iter().filter(|m| m.is_none()).count();
                Ok(RuntimeTerm::Var(lvl.saturating_sub(erased_count)))
            }
        }

        // Martin-Löf J-elimination: target is a proof (Q=0), so J definitionally collapses to base!
        Expr::J { base, .. } => erase_expr_rec_typed(
            ast,
            base,
            expected_ty,
            level_map,
            runtime_depth,
            ty_env,
            val_env,
        ),

        // Inductive recursion on Peano
        Expr::Ind {
            mot, z, s, target, ..
        } => {
            let mot_val = crate::eval::eval(ast, mot, val_env, None);
            let z_ty = crate::eval::apply(
                ast,
                crate::ast::Plicity::Explicit,
                mot_val.clone(),
                crate::eval::Value::Zero,
                None,
            );
            let base = erase_expr_rec_typed(
                ast,
                z,
                Some(&z_ty),
                level_map,
                runtime_depth,
                ty_env,
                val_env,
            )?;
            // In `NatRec`, the runtime evaluator applies `step` to `(i, acc)`.
            // Even if the step binder `n` or `ih` was declared with `Q = 0` (unused in step body),
            // both runtime binders must be retained so `step(i)(acc)` has two lambdas to beta-reduce.
            let step = match ast.expr(s)? {
                Expr::Lambda { body: s_body1, .. } => {
                    let depth_n = val_env.len();
                    let var_n =
                        crate::eval::Value::Neutral(crate::eval::Neutral::Var(Level(depth_n)));
                    let rt_n = *runtime_depth;
                    *runtime_depth += 1;
                    level_map.push(Some(rt_n));
                    ty_env.push(Some(crate::eval::Value::NatType));
                    val_env.push(var_n.clone());

                    let inner = match ast.expr(s_body1)? {
                        Expr::Lambda { body: s_body2, .. } => {
                            let depth_ih = val_env.len();
                            let var_ih = crate::eval::Value::Neutral(crate::eval::Neutral::Var(
                                Level(depth_ih),
                            ));
                            let mot_n = crate::eval::apply(
                                ast,
                                crate::ast::Plicity::Explicit,
                                mot_val.clone(),
                                var_n.clone(),
                                None,
                            );
                            let mot_succ_n = crate::eval::apply(
                                ast,
                                crate::ast::Plicity::Explicit,
                                mot_val,
                                crate::eval::Value::Succ(Box::new(var_n)),
                                None,
                            );
                            let rt_ih = *runtime_depth;
                            *runtime_depth += 1;
                            level_map.push(Some(rt_ih));
                            ty_env.push(Some(mot_n));
                            val_env.push(var_ih);

                            let body_erased = erase_expr_rec_typed(
                                ast,
                                s_body2,
                                Some(&mot_succ_n),
                                level_map,
                                runtime_depth,
                                ty_env,
                                val_env,
                            )?;
                            val_env.pop();
                            ty_env.pop();
                            level_map.pop();
                            *runtime_depth -= 1;
                            RuntimeTerm::Lam(Box::new(normalize_erased_root(body_erased)))
                        }
                        _ => {
                            erase_expr_rec(ast, s_body1, level_map, runtime_depth, ty_env, val_env)?
                        }
                    };
                    val_env.pop();
                    ty_env.pop();
                    level_map.pop();
                    *runtime_depth -= 1;
                    RuntimeTerm::Lam(Box::new(normalize_erased_root(inner)))
                }
                _ => erase_expr_rec(ast, s, level_map, runtime_depth, ty_env, val_env)?,
            };
            let tgt = erase_expr_rec_typed(
                ast,
                target,
                Some(&crate::eval::Value::NatType),
                level_map,
                runtime_depth,
                ty_env,
                val_env,
            )?;
            Ok(RuntimeTerm::NatRec {
                base: Box::new(normalize_erased_root(base)),
                step: Box::new(normalize_erased_root(step)),
                target: Box::new(normalize_erased_root(tgt)),
            })
        }

        // Dependent Pairs: If one component is erased (e.g. proof of predicate or Q=0), unbox to scalar!
        Expr::Pair { first, second } => {
            let (sigma_q, dom_ty, cod_ty) =
                if let Some(crate::eval::Value::Sigma(q, dom, cod)) = expected_ty {
                    let fst_val = crate::eval::eval(ast, first, val_env, None);
                    let cod_val = cod.clone().instantiate(ast, fst_val, None);
                    (Some(*q), Some((**dom).clone()), Some(cod_val))
                } else {
                    (None, None, None)
                };

            let e1 = erase_expr_rec_typed(
                ast,
                first,
                dom_ty.as_ref(),
                level_map,
                runtime_depth,
                ty_env,
                val_env,
            )?;
            let e2 = erase_expr_rec_typed(
                ast,
                second,
                cod_ty.as_ref(),
                level_map,
                runtime_depth,
                ty_env,
                val_env,
            )?;
            let first_erased = sigma_q == Some(Quantity::Zero)
                || e1.is_proof_erased()
                || dom_ty.as_ref().is_some_and(is_type_or_proof_value);
            let second_erased =
                e2.is_proof_erased() || cod_ty.as_ref().is_some_and(is_type_or_proof_value);

            if first_erased && second_erased {
                Ok(RuntimeTerm::Erased)
            } else if second_erased {
                Ok(e1)
            } else if first_erased {
                Ok(e2)
            } else {
                Ok(RuntimeTerm::Pair(
                    Box::new(normalize_erased_root(e1)),
                    Box::new(normalize_erased_root(e2)),
                ))
            }
        }

        Expr::Fst(pair) => {
            let p = erase_expr_rec(ast, pair, level_map, runtime_depth, ty_env, val_env)?;
            match p {
                RuntimeTerm::Pair(first, _) => Ok(*first),
                other => {
                    let (first_erased, second_erased) = pair_component_erasure(
                        ast,
                        pair,
                        level_map,
                        runtime_depth,
                        ty_env,
                        val_env,
                    )?;
                    if first_erased {
                        Ok(RuntimeTerm::Erased)
                    } else if second_erased {
                        Ok(other)
                    } else {
                        Ok(RuntimeTerm::Fst(Box::new(normalize_erased_root(other))))
                    }
                }
            }
        }

        Expr::Snd(pair) => {
            let p = erase_expr_rec(ast, pair, level_map, runtime_depth, ty_env, val_env)?;
            match p {
                RuntimeTerm::Pair(_, second) => Ok(*second),
                other => {
                    let (first_erased, second_erased) = pair_component_erasure(
                        ast,
                        pair,
                        level_map,
                        runtime_depth,
                        ty_env,
                        val_env,
                    )?;
                    if second_erased {
                        Ok(RuntimeTerm::Erased)
                    } else if first_erased {
                        Ok(other)
                    } else {
                        Ok(RuntimeTerm::Snd(Box::new(normalize_erased_root(other))))
                    }
                }
            }
        }

        // Functions and Applications
        Expr::Pi { .. } => Ok(RuntimeTerm::Erased),

        Expr::Lambda { quantity, body, .. } => {
            let cur_depth = val_env.len();
            let (dom_ty, cod_ty) = extract_pi_domain_and_codomain(ast, expected_ty, cur_depth);
            let erase_binder =
                quantity == Quantity::Zero || dom_ty.as_ref().is_some_and(is_type_or_proof_value);
            let var = crate::eval::Value::Neutral(crate::eval::Neutral::Var(Level(cur_depth)));
            ty_env.push(dom_ty);
            val_env.push(var);

            if erase_binder {
                // Parameter is erased: eliminate lambda binder and map this De Bruijn level to None
                level_map.push(None);
                let erased_body = erase_expr_rec_typed(
                    ast,
                    body,
                    cod_ty.as_ref(),
                    level_map,
                    runtime_depth,
                    ty_env,
                    val_env,
                )?;
                level_map.pop();
                val_env.pop();
                ty_env.pop();
                Ok(erased_body)
            } else {
                let rt_lvl = *runtime_depth;
                *runtime_depth += 1;
                level_map.push(Some(rt_lvl));
                let erased_body = erase_expr_rec_typed(
                    ast,
                    body,
                    cod_ty.as_ref(),
                    level_map,
                    runtime_depth,
                    ty_env,
                    val_env,
                )?;
                level_map.pop();
                *runtime_depth -= 1;
                val_env.pop();
                ty_env.pop();
                Ok(RuntimeTerm::Lam(Box::new(normalize_erased_root(
                    erased_body,
                ))))
            }
        }

        Expr::App {
            function, argument, ..
        } => {
            let fn_ty = infer_expr_type_in_context(ast, function, ty_env, val_env)?;
            let arg_expected_ty = if let Some(crate::eval::Value::Pi(_plic, _q, ref dom, _)) = fn_ty
            {
                Some((**dom).clone())
            } else {
                None
            };
            let param_quantities = infer_function_param_quantities(ast, function, ty_env, val_env)?;
            let param_is_zero = param_quantities.first().copied() == Some(Quantity::Zero);
            let f = erase_expr_rec_typed(
                ast,
                function,
                fn_ty.as_ref(),
                level_map,
                runtime_depth,
                ty_env,
                val_env,
            )?;
            let a = erase_expr_rec_typed(
                ast,
                argument,
                arg_expected_ty.as_ref(),
                level_map,
                runtime_depth,
                ty_env,
                val_env,
            )?;
            if param_is_zero || arg_expected_ty.as_ref().is_some_and(is_type_or_proof_value) {
                // Erased argument: function already eliminated binder, application is a no-op
                Ok(f)
            } else {
                Ok(RuntimeTerm::App(
                    Box::new(normalize_erased_root(f)),
                    Box::new(normalize_erased_root(a)),
                ))
            }
        }

        Expr::If { cond, conseq, alt } => {
            let c = erase_expr_rec_typed(
                ast,
                cond,
                Some(&crate::eval::Value::Bool),
                level_map,
                runtime_depth,
                ty_env,
                val_env,
            )?;
            let t = erase_expr_rec_typed(
                ast,
                conseq,
                expected_ty,
                level_map,
                runtime_depth,
                ty_env,
                val_env,
            )?;
            let e = erase_expr_rec_typed(
                ast,
                alt,
                expected_ty,
                level_map,
                runtime_depth,
                ty_env,
                val_env,
            )?;
            Ok(RuntimeTerm::If {
                cond: Box::new(normalize_erased_root(c)),
                conseq: Box::new(normalize_erased_root(t)),
                alt: Box::new(normalize_erased_root(e)),
            })
        }

        Expr::Hole | Expr::Meta(_) => Ok(RuntimeTerm::Erased),
        Expr::Sigma { .. } => Ok(RuntimeTerm::Erased),
        Expr::Ann { term, ty } => {
            let ann_ty = crate::eval::eval(ast, ty, val_env, None);
            erase_expr_rec_typed(
                ast,
                term,
                Some(&ann_ty),
                level_map,
                runtime_depth,
                ty_env,
                val_env,
            )
        }
    }
}

/// Evaluates an erased `RuntimeTerm` at bare-metal execution speed without typechecking or closures.
pub fn eval_runtime(term: &RuntimeTerm, env: &[RuntimeTerm]) -> RuntimeTerm {
    match term {
        RuntimeTerm::Erased | RuntimeTerm::Unit => RuntimeTerm::Unit,
        RuntimeTerm::Bool(b) => RuntimeTerm::Bool(*b),
        RuntimeTerm::Nat(n) => RuntimeTerm::Nat(*n),

        RuntimeTerm::Succ(inner) => {
            let v = eval_runtime(inner, env);
            match v {
                RuntimeTerm::Nat(n) => RuntimeTerm::Nat(n.saturating_add(1)),
                other => RuntimeTerm::Succ(Box::new(other)),
            }
        }

        RuntimeTerm::Var(lvl) => {
            if let Some(val) = env.get(*lvl) {
                val.clone()
            } else {
                RuntimeTerm::Var(*lvl)
            }
        }

        RuntimeTerm::Lam(body) => RuntimeTerm::Closure(body.clone(), env.to_vec()),

        RuntimeTerm::Closure(body, c_env) => RuntimeTerm::Closure(body.clone(), c_env.clone()),

        RuntimeTerm::App(func, arg) => {
            let eval_f = eval_runtime(func, env);
            let eval_a = eval_runtime(arg, env);
            match eval_f {
                RuntimeTerm::Closure(body, mut c_env) => {
                    c_env.push(eval_a);
                    eval_runtime(&body, &c_env)
                }
                RuntimeTerm::Lam(body) => {
                    let mut new_env = env.to_vec();
                    new_env.push(eval_a);
                    eval_runtime(&body, &new_env)
                }
                _ => RuntimeTerm::App(Box::new(eval_f), Box::new(eval_a)),
            }
        }

        RuntimeTerm::If { cond, conseq, alt } => {
            let c = eval_runtime(cond, env);
            match c {
                RuntimeTerm::Bool(true) => eval_runtime(conseq, env),
                RuntimeTerm::Bool(false) => eval_runtime(alt, env),
                _ => RuntimeTerm::If {
                    cond: Box::new(c),
                    conseq: conseq.clone(),
                    alt: alt.clone(),
                },
            }
        }

        RuntimeTerm::Pair(a, b) => {
            let ea = eval_runtime(a, env);
            let eb = eval_runtime(b, env);
            RuntimeTerm::Pair(Box::new(ea), Box::new(eb))
        }

        RuntimeTerm::Fst(p) => {
            let ep = eval_runtime(p, env);
            match ep {
                RuntimeTerm::Pair(first, _) => *first,
                other => other,
            }
        }

        RuntimeTerm::Snd(p) => {
            let ep = eval_runtime(p, env);
            match ep {
                RuntimeTerm::Pair(_, second) => *second,
                other => other,
            }
        }

        RuntimeTerm::NatRec { base, step, target } => {
            let tgt = eval_runtime(target, env);
            match tgt {
                RuntimeTerm::Nat(0) => eval_runtime(base, env),
                RuntimeTerm::Nat(n) => {
                    let mut acc = eval_runtime(base, env);
                    let step_val = eval_runtime(step, env);
                    for i in 0..n {
                        let step_i = eval_runtime(
                            &RuntimeTerm::App(
                                Box::new(step_val.clone()),
                                Box::new(RuntimeTerm::Nat(i)),
                            ),
                            env,
                        );
                        acc = eval_runtime(&RuntimeTerm::App(Box::new(step_i), Box::new(acc)), env);
                    }
                    acc
                }
                other => RuntimeTerm::NatRec {
                    base: base.clone(),
                    step: step.clone(),
                    target: Box::new(other),
                },
            }
        }
    }
}

mod codegen;
pub use codegen::{CodegenError, emit_c_code, try_emit_c_code};

#[cfg(test)]
mod tests;
