with open('src/elaborator.rs', 'r') as f:
    c = f.read()

match_elab = """                // Synthesize `s` by constructing its type
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
                
                let s_ty_val = crate::eval::eval(self.ast, s_ty_ast, env, Some(self.turbine));
                self.check(s, s_ty_val, depth, env, types)?;
                
                let ret_ty = crate::eval::apply(self.ast, crate::ast::Plicity::Explicit, mot_val, target_val, Some(self.turbine));
                Ok(Elaboration { ty: ret_ty, usages: vec![] })
            }"""

repl_elab = """                // Manually check `s` without synthesizing its type AST
                let s_term = self.ast.expr(s)?;
                if let Expr::Lambda { body: s_body1, .. } = s_term {
                    let mut env1 = env.to_vec();
                    let mut types1 = types.to_vec();
                    types1.push(Value::NatType);
                    let var_n = Value::Neutral(crate::eval::Neutral::Var(crate::ast::Level(depth)));
                    env1.push(var_n.clone());
                    
                    let s_body1_term = self.ast.expr(*s_body1)?;
                    if let Expr::Lambda { body: s_body2, .. } = s_body1_term {
                        let mut env2 = env1.clone();
                        let mut types2 = types1.clone();
                        
                        let mot_n = crate::eval::apply(self.ast, crate::ast::Plicity::Explicit, mot_val.clone(), var_n.clone(), self.turbine);
                        types2.push(mot_n);
                        let var_ih = Value::Neutral(crate::eval::Neutral::Var(crate::ast::Level(depth + 1)));
                        env2.push(var_ih.clone());
                        
                        let succ_n = Value::Succ(Box::new(var_n));
                        let mot_succ_n = crate::eval::apply(self.ast, crate::ast::Plicity::Explicit, mot_val.clone(), succ_n, self.turbine);
                        
                        self.check(*s_body2, mot_succ_n, depth + 2, &env2, &types2)?;
                    } else {
                        return Err(Error::ExpectedFunction { expr: *s_body1 });
                    }
                } else {
                    let s_elab = self.synth(s, depth, env, types)?;
                    let mut unified = false;
                    if let Value::Pi(_, _, ref d1, ref c1) = s_elab.ty {
                        let unif_d1 = if let Some(t) = self.turbine {
                            Checker::unify(self.ast, &**d1, &Value::NatType, depth, t)
                        } else {
                            **d1 == Value::NatType
                        };
                        if unif_d1 {
                            let var_n = Value::Neutral(crate::eval::Neutral::Var(crate::ast::Level(depth)));
                            let c1_val = c1.clone().instantiate(self.ast, var_n.clone(), self.turbine);
                            if let Value::Pi(_, _, ref d2, ref c2) = c1_val {
                                let mot_n = crate::eval::apply(self.ast, crate::ast::Plicity::Explicit, mot_val.clone(), var_n.clone(), self.turbine);
                                let unif_d2 = if let Some(t) = self.turbine {
                                    Checker::unify(self.ast, &**d2, &mot_n, depth + 1, t)
                                } else {
                                    **d2 == mot_n
                                };
                                if unif_d2 {
                                    let var_ih = Value::Neutral(crate::eval::Neutral::Var(crate::ast::Level(depth + 1)));
                                    let c2_val = c2.clone().instantiate(self.ast, var_ih, self.turbine);
                                    let succ_n = Value::Succ(Box::new(var_n));
                                    let mot_succ_n = crate::eval::apply(self.ast, crate::ast::Plicity::Explicit, mot_val.clone(), succ_n, self.turbine);
                                    let unif_c2 = if let Some(t) = self.turbine {
                                        Checker::unify(self.ast, &c2_val, &mot_succ_n, depth + 2, t)
                                    } else {
                                        c2_val == mot_succ_n
                                    };
                                    unified = unif_c2;
                                }
                            }
                        }
                    }
                    if !unified {
                        return Err(Error::TypeMismatch {
                            expr: s,
                            expected: "Step function for Nat induction".into(),
                            found: format!("{:?}", s_elab.ty),
                        });
                    }
                }
                
                let ret_ty = crate::eval::apply(self.ast, crate::ast::Plicity::Explicit, mot_val, target_val, self.turbine);
                Ok(Elaboration { ty: ret_ty, usages: vec![] })
            }"""

c = c.replace(match_elab, repl_elab)
with open('src/elaborator.rs', 'w') as f:
    f.write(c)

