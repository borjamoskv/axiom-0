with open('src/eval.rs', 'r') as f:
    c = f.read()

eval_ind_code = """
pub fn apply(ast: &Ast, plicity: crate::ast::Plicity, function: Value, argument: Value, turbine: Option<&crate::turbine::TurbineEngine>) -> Value {
    match function {
        Value::Lam(_, _, closure) => closure.instantiate(ast, argument, turbine),
        Value::Neutral(neu) => Value::Neutral(Neutral::App(plicity, Box::new(neu), Box::new(argument))),
        Value::Meta(id, mut spine) => {
            spine.push(argument);
            Value::Meta(id, spine)
        }
        _ => panic!("Cannot apply to non-function"),
    }
}

fn eval_ind(ast: &Ast, mot: Value, z: Value, s: Value, target: Value, turbine: Option<&crate::turbine::TurbineEngine>) -> Value {
    match target {
        Value::Zero => z,
        Value::Succ(n) => {
            let ih = eval_ind(ast, mot.clone(), z, s.clone(), *n.clone(), turbine);
            let step_applied = apply(ast, crate::ast::Plicity::Explicit, s, *n, turbine);
            apply(ast, crate::ast::Plicity::Explicit, step_applied, ih, turbine)
        }
        Value::Neutral(n) => Value::Neutral(Neutral::Ind(Box::new(mot), Box::new(z), Box::new(s), Box::new(n))),
        _ => panic!("Invalid target for ind"),
    }
}
"""
if "fn eval_ind" not in c:
    c = c + eval_ind_code

with open('src/eval.rs', 'w') as f:
    f.write(c)

with open('src/elaborator.rs', 'r') as f:
    c = f.read()

match_elab = """            Expr::Pair { .. } => Err(Error::CannotInferLambda(expr)),
        }"""
repl_elab = """            Expr::Pair { .. } => Err(Error::CannotInferLambda(expr)),
            Expr::NatType => {
                let ty = Value::Universe(0);
                Ok(ElabExpr { expr, ty })
            }
            Expr::Zero => {
                let ty = Value::NatType;
                Ok(ElabExpr { expr, ty })
            }
            Expr::Succ(n) => {
                check(ast, n, Value::NatType, env, types)?;
                let ty = Value::NatType;
                Ok(ElabExpr { expr, ty })
            }
            Expr::Ind { mot, z, s, target } => {
                let target_elab = synthesize(ast, target, env, types)?;
                unify(ast, target_elab.ty, Value::NatType, env.len() as u32, turbine)?;
                
                let target_val = crate::eval::eval(ast, target, env, turbine);
                
                let mot_elab = synthesize(ast, mot, env, types)?;
                let mot_val = crate::eval::eval(ast, mot, env, turbine);
                
                // mot should be Nat -> Type
                match mot_elab.ty {
                    Value::Pi(_, _, ref domain, _) => {
                        unify(ast, (**domain).clone(), Value::NatType, env.len() as u32, turbine)?;
                    }
                    _ => panic!("mot must be a function Nat -> Type"),
                }
                
                let z_ty = crate::eval::apply(ast, crate::ast::Plicity::Explicit, mot_val.clone(), Value::Zero, turbine);
                check(ast, z, z_ty, env, types)?;
                
                // Synthesize `s` by constructing its type
                // s : (n : Nat) -> (ih : mot n) -> mot (Succ n)
                // We construct the type AST and evaluate it!
                let level = env.len();
                let var_n = ast.push(Expr::Var(crate::ast::Level(level))).unwrap();
                let mot_n = ast.push(Expr::App { plicity: crate::ast::Plicity::Explicit, function: mot, argument: var_n }).unwrap();
                let succ_n = ast.push(Expr::Succ(var_n)).unwrap();
                let mot_succ_n = ast.push(Expr::App { plicity: crate::ast::Plicity::Explicit, function: mot, argument: succ_n }).unwrap();
                
                let inner_pi = ast.push(Expr::Pi {
                    plicity: crate::ast::Plicity::Explicit,
                    quantity: crate::ast::Quantity::Omega,
                    domain: mot_n,
                    codomain: mot_succ_n,
                }).unwrap();
                
                let nat_type = ast.push(Expr::NatType).unwrap();
                let s_ty_ast = ast.push(Expr::Pi {
                    plicity: crate::ast::Plicity::Explicit,
                    quantity: crate::ast::Quantity::Omega,
                    domain: nat_type,
                    codomain: inner_pi,
                }).unwrap();
                
                let s_ty_val = crate::eval::eval(ast, s_ty_ast, env, turbine);
                check(ast, s, s_ty_val, env, types)?;
                
                let ret_ty = crate::eval::apply(ast, crate::ast::Plicity::Explicit, mot_val, target_val, turbine);
                Ok(ElabExpr { expr, ty: ret_ty })
            }
        }"""
c = c.replace(match_elab, repl_elab)

with open('src/elaborator.rs', 'w') as f:
    f.write(c)

