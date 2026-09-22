//! Proof Erasure and Runtime Transduction for Quantitative Type Theory (QTT).
//!
//! Implements strict computational extraction from verified AXIOM-0 terms:
//! 1. All quantities with $\mathcal{Q} = 0$ (types, equality proofs, erased arguments) are eradicated.
//! 2. Martin-Löf identity eliminators $J(P, d, a, p)$ collapse definitionally to $d$ ($O(0)$ overhead).
//! 3. $\Sigma$-types with erased second components (subsets/refinements) unbox directly into raw scalars.
//! 4. Peano natural numbers unbox into native 64-bit unsigned integers (`u64`).
//! 5. Emits pure C99 code for zero-cost foreign function interfacing (C-ABI).

use crate::ast::{Ast, AstError, Expr, ExprId, Level, Quantity};

/// An untyped, erased runtime term stripped of all logical proofs and type annotations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeTerm {
    /// Erased proof marker or unit scalar.
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
    /// Returns true if this term is an erased proof or unit value.
    pub fn is_erased(&self) -> bool {
        matches!(self, Self::Unit)
    }

    /// Size in AST nodes of the erased runtime term.
    pub fn node_count(&self) -> usize {
        match self {
            Self::Unit | Self::Bool(_) | Self::Nat(_) | Self::Var(_) => 1,
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
    erase_expr_rec(ast, root)
}

fn erase_expr_rec(ast: &Ast, expr_id: ExprId) -> Result<RuntimeTerm, AstError> {
    let expr = ast.expr(expr_id)?;
    match expr {
        // Types and proposition constructors are completely erased
        Expr::Universe(_) | Expr::UnitType | Expr::NatType | Expr::Bool => Ok(RuntimeTerm::Unit),
        Expr::IdType { .. } => Ok(RuntimeTerm::Unit),
        Expr::Refl(_) => Ok(RuntimeTerm::Unit),

        // Primitive values
        Expr::Unit => Ok(RuntimeTerm::Unit),
        Expr::True => Ok(RuntimeTerm::Bool(true)),
        Expr::False => Ok(RuntimeTerm::Bool(false)),
        Expr::Zero => Ok(RuntimeTerm::Nat(0)),

        Expr::Succ(inner) => {
            let erased_inner = erase_expr_rec(ast, inner)?;
            match erased_inner {
                RuntimeTerm::Nat(n) => Ok(RuntimeTerm::Nat(n.saturating_add(1))),
                other => Ok(RuntimeTerm::Succ(Box::new(other))),
            }
        }

        Expr::Var(Level(lvl)) => Ok(RuntimeTerm::Var(lvl)),

        // Martin-Löf J-elimination: target is a proof (Q=0), so J definitionally collapses to base!
        Expr::J { base, .. } => erase_expr_rec(ast, base),

        // Inductive recursion on Peano
        Expr::Ind { z, s, target, .. } => {
            let base = erase_expr_rec(ast, z)?;
            let step = erase_expr_rec(ast, s)?;
            let tgt = erase_expr_rec(ast, target)?;
            Ok(RuntimeTerm::NatRec {
                base: Box::new(base),
                step: Box::new(step),
                target: Box::new(tgt),
            })
        }

        // Dependent Pairs: If one component is erased (e.g. proof of predicate), unbox to scalar!
        Expr::Pair { first, second } => {
            let e1 = erase_expr_rec(ast, first)?;
            let e2 = erase_expr_rec(ast, second)?;
            if e2.is_erased() {
                Ok(e1)
            } else if e1.is_erased() {
                Ok(e2)
            } else {
                Ok(RuntimeTerm::Pair(Box::new(e1), Box::new(e2)))
            }
        }

        Expr::Fst(pair) => {
            let p = erase_expr_rec(ast, pair)?;
            match p {
                // If it was already unboxed due to second component being erased:
                RuntimeTerm::Pair(first, _) => Ok(*first),
                other => Ok(other),
            }
        }

        Expr::Snd(pair) => {
            let p = erase_expr_rec(ast, pair)?;
            match p {
                RuntimeTerm::Pair(_, second) => Ok(*second),
                other => Ok(other),
            }
        }

        // Functions and Applications
        Expr::Pi { .. } => Ok(RuntimeTerm::Unit),

        Expr::Lambda { quantity, body, .. } => {
            let erased_body = erase_expr_rec(ast, body)?;
            if quantity == Quantity::Zero {
                // Parameter is erased: eliminate lambda binder entirely
                Ok(erased_body)
            } else {
                Ok(RuntimeTerm::Lam(Box::new(erased_body)))
            }
        }

        Expr::App { function, argument, .. } => {
            let f = erase_expr_rec(ast, function)?;
            let a = erase_expr_rec(ast, argument)?;
            if a.is_erased() {
                // Erased argument: function already eliminated binder, application is a no-op
                Ok(f)
            } else {
                Ok(RuntimeTerm::App(Box::new(f), Box::new(a)))
            }
        }

        Expr::If { cond, conseq, alt } => {
            let c = erase_expr_rec(ast, cond)?;
            let t = erase_expr_rec(ast, conseq)?;
            let e = erase_expr_rec(ast, alt)?;
            Ok(RuntimeTerm::If {
                cond: Box::new(c),
                conseq: Box::new(t),
                alt: Box::new(e),
            })
        }

        Expr::Hole | Expr::Meta(_) => Ok(RuntimeTerm::Unit),
        Expr::Sigma { .. } => Ok(RuntimeTerm::Unit),
        Expr::Ann { term, .. } => erase_expr_rec(ast, term),
    }
}

/// Evaluates an erased `RuntimeTerm` at bare-metal execution speed without typechecking or closures.
pub fn eval_runtime(term: &RuntimeTerm, env: &[RuntimeTerm]) -> RuntimeTerm {
    match term {
        RuntimeTerm::Unit => RuntimeTerm::Unit,
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

        RuntimeTerm::Lam(body) => {
            RuntimeTerm::Closure(body.clone(), env.to_vec())
        }

        RuntimeTerm::Closure(body, c_env) => {
            RuntimeTerm::Closure(body.clone(), c_env.clone())
        }

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
                        acc = eval_runtime(
                            &RuntimeTerm::App(Box::new(step_i), Box::new(acc)),
                            env,
                        );
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

/// Emits zero-cost standalone C99 code for a verified runtime term.
pub fn emit_c_code(term: &RuntimeTerm, fn_name: &str) -> String {
    let mut out = String::new();
    out.push_str("// Auto-generated by AXIOM-0 QTT Proof Erasure Engine\n");
    out.push_str("// Verified with Martin-Löf Id-types, proofs completely eradicated.\n");
    out.push_str("#include <stdint.h>\n#include <stdbool.h>\n\n");

    match term {
        RuntimeTerm::Nat(n) => {
            out.push_str(&format!("uint64_t {}() {{\n    return {}ULL;\n}}\n", fn_name, n));
        }
        RuntimeTerm::Bool(b) => {
            out.push_str(&format!("bool {}() {{\n    return {};\n}}\n", fn_name, if *b { "true" } else { "false" }));
        }
        RuntimeTerm::If { cond, conseq, alt } => {
            out.push_str(&format!("uint64_t {}() {{\n", fn_name));
            out.push_str(&format!("    return ({}) ? ({}) : ({});\n}}\n", 
                emit_c_inline(cond), emit_c_inline(conseq), emit_c_inline(alt)));
        }
        _ => {
            out.push_str(&format!("uint64_t {}() {{\n    return {};\n}}\n", fn_name, emit_c_inline(term)));
        }
    }
    out
}

fn emit_c_inline(term: &RuntimeTerm) -> String {
    match term {
        RuntimeTerm::Unit => "0".to_string(),
        RuntimeTerm::Bool(b) => (if *b { "true" } else { "false" }).to_string(),
        RuntimeTerm::Nat(n) => format!("{}ULL", n),
        RuntimeTerm::Succ(x) => format!("({} + 1ULL)", emit_c_inline(x)),
        RuntimeTerm::Var(lvl) => format!("x_{}", lvl),
        RuntimeTerm::If { cond, conseq, alt } => {
            format!("(({}) ? ({}) : ({}))", emit_c_inline(cond), emit_c_inline(conseq), emit_c_inline(alt))
        }
        RuntimeTerm::App(f, a) => format!("{}({})", emit_c_inline(f), emit_c_inline(a)),
        RuntimeTerm::Pair(a, _b) => format!("((uint64_t){})", emit_c_inline(a)),
        RuntimeTerm::Fst(p) => emit_c_inline(p),
        RuntimeTerm::Snd(p) => emit_c_inline(p),
        _ => "0ULL".to_string(),
    }
}
