use super::*;
use crate::ast::{Ast, Expr, Level, Plicity, Quantity};
#[cfg(feature = "std")]
use crate::elaborator::Checker;
use crate::elaborator::synthesize;
use crate::eval::{Value, eval};
#[cfg(feature = "std")]
use crate::turbine::TurbineEngine;

#[test]
#[cfg(feature = "std")]
fn reg_p1_1_unifier_rejects_cyclic_occurs_check() {
    let ast = Ast::new();
    let turbine = TurbineEngine::new(4);
    let m = turbine.new_meta();
    let meta_val = Value::Meta(m, Vec::new());
    let cyclic_val = Value::Succ(Box::new(meta_val.clone()));

    assert!(
        !Checker::unify(&ast, &meta_val, &cyclic_val, 0, &turbine),
        "Cyclic unification ?m = Succ(?m) must be rejected by occurs check"
    );
    // Subsequent comparison of ?m with itself must succeed without stack overflow.
    assert!(Checker::unify(&ast, &meta_val, &meta_val, 0, &turbine));
}

#[test]
#[cfg(feature = "std")]
fn reg_p1_2_concurrent_meta_unification_rejects_incompatible_solutions() {
    let ast = Ast::new();
    let mut saw_double_accept = false;
    for _ in 0..4096 {
        let turbine = TurbineEngine::new(4);
        let m = turbine.new_meta();
        let meta_val = Value::Meta(m, Vec::new());
        let barrier = std::sync::Barrier::new(2);

        let (r1, r2) = std::thread::scope(|s| {
            let h1 = s.spawn(|| {
                barrier.wait();
                Checker::unify(&ast, &meta_val, &Value::Bool, 0, &turbine)
            });
            let h2 = s.spawn(|| {
                barrier.wait();
                Checker::unify(&ast, &meta_val, &Value::NatType, 0, &turbine)
            });
            (h1.join().unwrap(), h2.join().unwrap())
        });

        if r1 && r2 {
            saw_double_accept = true;
            break;
        }
        assert!(
            r1 || r2,
            "At least one thread must succeed solving the fresh metavariable"
        );
    }
    assert!(
        !saw_double_accept,
        "Two threads must never both succeed unifying the same metavariable with Bool and NatType"
    );
}

#[test]
fn reg_p1_3_induction_rejects_duplicating_linear_base_via_step() {
    // fn 1 (x : Nat) -> Ind(fn _ -> Nat, x, fn _ -> fn w ih -> Ind(fn _ -> Nat, ih, fn _ -> fn 1 acc -> Succ(acc), ih), Succ(Z))
    // Here `x` is declared linear (Q=1) and passed as base case `z = x`, while the step duplicates `ih` (observed Omega).
    let mut ast = Ast::new();
    let nat_ty = ast.push(Expr::NatType).unwrap();
    let mot_body = ast.push(Expr::NatType).unwrap();
    let mot = ast
        .push(Expr::Lambda {
            plicity: Plicity::Explicit,
            quantity: Quantity::Omega,
            body: mot_body,
        })
        .unwrap();

    // Outer linear parameter `x` is at De Bruijn level 0.
    let var_x = ast.push(Expr::Var(Level(0))).unwrap();

    // Inside `s`: level 1 is `n`, level 2 is `ih`.
    // Duplicate `ih` by using it both as base and target of an inner `Ind` (or any expression using level 2 twice).
    let inner_mot_body = ast.push(Expr::NatType).unwrap();
    let inner_mot = ast
        .push(Expr::Lambda {
            plicity: Plicity::Explicit,
            quantity: Quantity::Omega,
            body: inner_mot_body,
        })
        .unwrap();
    let ih_use1 = ast.push(Expr::Var(Level(2))).unwrap();
    let ih_use2 = ast.push(Expr::Var(Level(2))).unwrap();
    let inner_acc = ast.push(Expr::Var(Level(4))).unwrap();
    let inner_succ_acc = ast.push(Expr::Succ(inner_acc)).unwrap();
    let inner_s2 = ast
        .push(Expr::Lambda {
            plicity: Plicity::Explicit,
            quantity: Quantity::One,
            body: inner_succ_acc,
        })
        .unwrap();
    let inner_s1 = ast
        .push(Expr::Lambda {
            plicity: Plicity::Explicit,
            quantity: Quantity::Zero,
            body: inner_s2,
        })
        .unwrap();
    let dup_ih = ast
        .push(Expr::Ind {
            mot: inner_mot,
            z: ih_use1,
            s: inner_s1,
            target: ih_use2,
        })
        .unwrap();

    let s_inner = ast
        .push(Expr::Lambda {
            plicity: Plicity::Explicit,
            quantity: Quantity::Omega,
            body: dup_ih,
        })
        .unwrap();
    let s = ast
        .push(Expr::Lambda {
            plicity: Plicity::Explicit,
            quantity: Quantity::Zero,
            body: s_inner,
        })
        .unwrap();

    let z_lit = ast.push(Expr::Zero).unwrap();
    let one_lit = ast.push(Expr::Succ(z_lit)).unwrap();
    let ind = ast
        .push(Expr::Ind {
            mot,
            z: var_x,
            s,
            target: one_lit,
        })
        .unwrap();

    let fn_term = ast
        .push(Expr::Lambda {
            plicity: Plicity::Explicit,
            quantity: Quantity::One,
            body: ind,
        })
        .unwrap();
    let pi_cod = ast.push(Expr::NatType).unwrap();
    let pi_ty = ast
        .push(Expr::Pi {
            plicity: Plicity::Explicit,
            quantity: Quantity::One,
            domain: nat_ty,
            codomain: pi_cod,
        })
        .unwrap();
    let ann = ast
        .push(Expr::Ann {
            term: fn_term,
            ty: pi_ty,
        })
        .unwrap();

    let res = synthesize(&ast, ann, &[]);
    assert!(
        res.is_err(),
        "Induction duplicating linear base resource `x` via `ih` must be rejected, got: {:?}",
        res
    );
}

#[test]
fn reg_p1_4_erasure_drops_zero_quantity_computational_arguments() {
    // ((fn 0 x -> True) : fn 0 (x : Nat) -> Bool) Z
    let mut ast = Ast::new();
    let nat_ty = ast.push(Expr::NatType).unwrap();
    let bool_ty = ast.push(Expr::Bool).unwrap();
    let pi_ty = ast
        .push(Expr::Pi {
            plicity: Plicity::Explicit,
            quantity: Quantity::Zero,
            domain: nat_ty,
            codomain: bool_ty,
        })
        .unwrap();
    let t_val = ast.push(Expr::True).unwrap();
    let lam = ast
        .push(Expr::Lambda {
            plicity: Plicity::Explicit,
            quantity: Quantity::Zero,
            body: t_val,
        })
        .unwrap();
    let ann_fn = ast
        .push(Expr::Ann {
            term: lam,
            ty: pi_ty,
        })
        .unwrap();
    let z_arg = ast.push(Expr::Zero).unwrap();
    let app = ast
        .push(Expr::App {
            plicity: Plicity::Explicit,
            function: ann_fn,
            argument: z_arg,
        })
        .unwrap();

    assert!(synthesize(&ast, app, &[]).is_ok());
    assert_eq!(eval(&ast, app, &[], None), Value::True);

    let erased = erase_expression(&ast, app).unwrap();
    let evaluated = eval_runtime(&erased, &[]);
    assert_eq!(
        evaluated,
        RuntimeTerm::Bool(true),
        "Erased application of Q=0 parameter to Z must evaluate to Bool(true), not get stuck as {:?}",
        evaluated
    );
}

#[test]
fn reg_p1_5_erasure_preserves_projections_on_variables() {
    // ((fn p -> p.2) : (Sigma (x : Nat) * Bool) -> Bool) (Z, True)
    let mut ast = Ast::new();
    let nat_ty = ast.push(Expr::NatType).unwrap();
    let bool_ty1 = ast.push(Expr::Bool).unwrap();
    let sigma_ty = ast
        .push(Expr::Sigma {
            quantity: Quantity::Omega,
            domain: nat_ty,
            codomain: bool_ty1,
        })
        .unwrap();
    let bool_ty2 = ast.push(Expr::Bool).unwrap();
    let pi_ty = ast
        .push(Expr::Pi {
            plicity: Plicity::Explicit,
            quantity: Quantity::Omega,
            domain: sigma_ty,
            codomain: bool_ty2,
        })
        .unwrap();

    let var_p = ast.push(Expr::Var(Level(0))).unwrap();
    let proj_snd = ast.push(Expr::Snd(var_p)).unwrap();
    let lam = ast
        .push(Expr::Lambda {
            plicity: Plicity::Explicit,
            quantity: Quantity::Omega,
            body: proj_snd,
        })
        .unwrap();
    let ann_fn = ast
        .push(Expr::Ann {
            term: lam,
            ty: pi_ty,
        })
        .unwrap();

    let z_val = ast.push(Expr::Zero).unwrap();
    let t_val = ast.push(Expr::True).unwrap();
    let pair_arg = ast
        .push(Expr::Pair {
            first: z_val,
            second: t_val,
        })
        .unwrap();
    let pair_sigma_nat = ast.push(Expr::NatType).unwrap();
    let pair_sigma_bool = ast.push(Expr::Bool).unwrap();
    let pair_sigma = ast
        .push(Expr::Sigma {
            quantity: Quantity::Omega,
            domain: pair_sigma_nat,
            codomain: pair_sigma_bool,
        })
        .unwrap();
    let ann_pair = ast
        .push(Expr::Ann {
            term: pair_arg,
            ty: pair_sigma,
        })
        .unwrap();

    let app = ast
        .push(Expr::App {
            plicity: Plicity::Explicit,
            function: ann_fn,
            argument: ann_pair,
        })
        .unwrap();

    assert!(synthesize(&ast, app, &[]).is_ok());
    assert_eq!(eval(&ast, app, &[], None), Value::True);

    let erased = erase_expression(&ast, app).unwrap();
    let evaluated = eval_runtime(&erased, &[]);
    assert_eq!(
        evaluated,
        RuntimeTerm::Bool(true),
        "Projection p.2 on runtime variable must evaluate to Bool(true), got {:?}",
        evaluated
    );
}

#[test]
fn reg_p1_6_c_codegen_preserves_nat_rec_semantics() {
    // Ind(fn _ -> Nat, Z, fn _ -> fn acc -> Succ(acc), Succ(Z)) => evaluates to 1
    let mut ast = Ast::new();
    let mot_body = ast.push(Expr::NatType).unwrap();
    let mot = ast
        .push(Expr::Lambda {
            plicity: Plicity::Explicit,
            quantity: Quantity::Omega,
            body: mot_body,
        })
        .unwrap();
    let z_base = ast.push(Expr::Zero).unwrap();
    let var_acc = ast.push(Expr::Var(Level(1))).unwrap();
    let succ_acc = ast.push(Expr::Succ(var_acc)).unwrap();
    let s2 = ast
        .push(Expr::Lambda {
            plicity: Plicity::Explicit,
            quantity: Quantity::One,
            body: succ_acc,
        })
        .unwrap();
    let s1 = ast
        .push(Expr::Lambda {
            plicity: Plicity::Explicit,
            quantity: Quantity::Omega,
            body: s2,
        })
        .unwrap();
    let z_arg = ast.push(Expr::Zero).unwrap();
    let target_one = ast.push(Expr::Succ(z_arg)).unwrap();
    let ind = ast
        .push(Expr::Ind {
            mot,
            z: z_base,
            s: s1,
            target: target_one,
        })
        .unwrap();

    assert!(synthesize(&ast, ind, &[]).is_ok());
    assert_eq!(
        eval(&ast, ind, &[], None),
        Value::Succ(Box::new(Value::Zero))
    );

    let erased = erase_expression(&ast, ind).unwrap();
    assert_eq!(eval_runtime(&erased, &[]), RuntimeTerm::Nat(1));

    let c_code = emit_c_code(&erased, "compute_one");
    assert!(
        !c_code.contains("return 0ULL;"),
        "Generated C99 code for NatRec returning 1 must not silently emit `return 0ULL;`: {}",
        c_code
    );
    assert!(
        c_code.contains("__axiom0_natrec_0(0ULL, 1ULL)"),
        "Generated C99 code must invoke the synthesized NatRec loop helper: {}",
        c_code
    );
}

#[test]
fn reg_p1_5_erasure_unboxes_variable_projection_with_erased_component() {
    // ((fn p -> p.1) : (Sigma (x : Nat) * Id(Nat, x, x)) -> Nat) (Succ(Z), refl(Succ(Z)))
    let mut ast = Ast::new();
    let nat_ty1 = ast.push(Expr::NatType).unwrap();
    let nat_ty2 = ast.push(Expr::NatType).unwrap();
    let var_x1 = ast.push(Expr::Var(Level(0))).unwrap();
    let var_x2 = ast.push(Expr::Var(Level(0))).unwrap();
    let id_ty = ast
        .push(Expr::IdType {
            ty: nat_ty2,
            lhs: var_x1,
            rhs: var_x2,
        })
        .unwrap();
    let sigma_ty = ast
        .push(Expr::Sigma {
            quantity: Quantity::Omega,
            domain: nat_ty1,
            codomain: id_ty,
        })
        .unwrap();
    let nat_ret = ast.push(Expr::NatType).unwrap();
    let pi_ty = ast
        .push(Expr::Pi {
            plicity: Plicity::Explicit,
            quantity: Quantity::Omega,
            domain: sigma_ty,
            codomain: nat_ret,
        })
        .unwrap();

    let var_p = ast.push(Expr::Var(Level(0))).unwrap();
    let proj_fst = ast.push(Expr::Fst(var_p)).unwrap();
    let lam = ast
        .push(Expr::Lambda {
            plicity: Plicity::Explicit,
            quantity: Quantity::Omega,
            body: proj_fst,
        })
        .unwrap();
    let ann_fn = ast
        .push(Expr::Ann {
            term: lam,
            ty: pi_ty,
        })
        .unwrap();

    let z1 = ast.push(Expr::Zero).unwrap();
    let s_z1 = ast.push(Expr::Succ(z1)).unwrap();
    let z2 = ast.push(Expr::Zero).unwrap();
    let s_z2 = ast.push(Expr::Succ(z2)).unwrap();
    let refl_sz = ast.push(Expr::Refl(s_z2)).unwrap();
    let pair_arg = ast
        .push(Expr::Pair {
            first: s_z1,
            second: refl_sz,
        })
        .unwrap();
    let nat_ty3 = ast.push(Expr::NatType).unwrap();
    let nat_ty4 = ast.push(Expr::NatType).unwrap();
    let var_x3 = ast.push(Expr::Var(Level(0))).unwrap();
    let var_x4 = ast.push(Expr::Var(Level(0))).unwrap();
    let id_ty2 = ast
        .push(Expr::IdType {
            ty: nat_ty4,
            lhs: var_x3,
            rhs: var_x4,
        })
        .unwrap();
    let sigma_ty2 = ast
        .push(Expr::Sigma {
            quantity: Quantity::Omega,
            domain: nat_ty3,
            codomain: id_ty2,
        })
        .unwrap();
    let ann_pair = ast
        .push(Expr::Ann {
            term: pair_arg,
            ty: sigma_ty2,
        })
        .unwrap();

    let app = ast
        .push(Expr::App {
            plicity: Plicity::Explicit,
            function: ann_fn,
            argument: ann_pair,
        })
        .unwrap();

    assert!(synthesize(&ast, app, &[]).is_ok());
    let erased = erase_expression(&ast, app).unwrap();
    assert_eq!(eval_runtime(&erased, &[]), RuntimeTerm::Nat(1));
}
