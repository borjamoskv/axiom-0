with open('src/elaborator.rs', 'r') as f:
    c = f.read()

match_elab = """            Expr::Pair { .. } => Err(Error::CannotInferLambda(expr)),
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

repl_elab = """            Expr::Pair { .. } => Err(Error::CannotInferLambda(expr)),
            Expr::NatType => {
                let ty = Value::Universe(0);
                Ok(Elaboration { ty, usages: vec![] })
            }
            Expr::Zero => {
                let ty = Value::NatType;
                Ok(Elaboration { ty, usages: vec![] })
            }
            Expr::Succ(n) => {
                self.check(n, Value::NatType, depth, env, types)?;
                let ty = Value::NatType;
                Ok(Elaboration { ty, usages: vec![] })
            }
            Expr::Ind { mot, z, s, target } => {
                let target_elab = self.synthesize(target, depth, env, types)?;
                self.unify(target_elab.ty, Value::NatType)?;
                
                let target_val = crate::eval::eval(self.ast, target, env, self.turbine);
                
                let mot_elab = self.synthesize(mot, depth, env, types)?;
                let mot_val = crate::eval::eval(self.ast, mot, env, self.turbine);
                
                // mot should be Nat -> Type
                match mot_elab.ty {
                    Value::Pi(_, _, ref domain, _) => {
                        self.unify((**domain).clone(), Value::NatType)?;
                    }
                    _ => panic!("mot must be a function Nat -> Type"),
                }
                
                let z_ty = crate::eval::apply(self.ast, crate::ast::Plicity::Explicit, mot_val.clone(), Value::Zero, self.turbine);
                self.check(z, z_ty, depth, env, types)?;
                
                // Synthesize `s` by constructing its type
                // s : (n : Nat) -> (ih : mot n) -> mot (Succ n)
                let level = env.len();
                let var_n = self.ast.push(Expr::Var(crate::ast::Level(level))).unwrap();
                let mot_n = self.ast.push(Expr::App { plicity: crate::ast::Plicity::Explicit, function: mot, argument: var_n }).unwrap();
                let succ_n = self.ast.push(Expr::Succ(var_n)).unwrap();
                let mot_succ_n = self.ast.push(Expr::App { plicity: crate::ast::Plicity::Explicit, function: mot, argument: succ_n }).unwrap();
                
                let inner_pi = self.ast.push(Expr::Pi {
                    plicity: crate::ast::Plicity::Explicit,
                    quantity: crate::ast::Quantity::Omega,
                    domain: mot_n,
                    codomain: mot_succ_n,
                }).unwrap();
                
                let nat_type = self.ast.push(Expr::NatType).unwrap();
                let s_ty_ast = self.ast.push(Expr::Pi {
                    plicity: crate::ast::Plicity::Explicit,
                    quantity: crate::ast::Quantity::Omega,
                    domain: nat_type,
                    codomain: inner_pi,
                }).unwrap();
                
                let s_ty_val = crate::eval::eval(self.ast, s_ty_ast, env, self.turbine);
                self.check(s, s_ty_val, depth, env, types)?;
                
                let ret_ty = crate::eval::apply(self.ast, crate::ast::Plicity::Explicit, mot_val, target_val, self.turbine);
                Ok(Elaboration { ty: ret_ty, usages: vec![] })
            }
        }"""
c = c.replace(match_elab, repl_elab)

with open('src/elaborator.rs', 'w') as f:
    f.write(c)

