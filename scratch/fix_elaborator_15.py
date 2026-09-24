with open('src/elaborator.rs', 'r') as f:
    c = f.read()

match_elab = """            Expr::Pair { .. } => Err(Error::CannotInferLambda(expr)),
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
                let target_elab = self.synth(target, depth, env, types)?;
                let unified_target = if let Some(t) = self.turbine {
                    Checker::unify(self.ast, &target_elab.ty, &Value::NatType, depth, t)
                } else {
                    target_elab.ty == Value::NatType
                };
                if !unified_target {
                    panic!("TARGET UNIFY FAILED");
                }
                
                let target_val = crate::eval::eval(self.ast, target, env, self.turbine);
                
                let mot_elab = self.synth(mot, depth, env, types)?;
                let mot_val = crate::eval::eval(self.ast, mot, env, self.turbine);
                
                match mot_elab.ty {
                    Value::Pi(_, _, ref domain, _) => {
                        let unified_domain = if let Some(t) = self.turbine {
                            Checker::unify(self.ast, &**domain, &Value::NatType, depth, t)
                        } else {
                            **domain == Value::NatType
                        };
                        if !unified_domain {
                            panic!("DOMAIN UNIFY FAILED");
                        }
                    }
                    _ => return Err(Error::CannotInferLambda(mot)),
                }
                
                let z_ty = crate::eval::apply(self.ast, crate::ast::Plicity::Explicit, mot_val.clone(), Value::Zero, self.turbine);
                self.check(z, z_ty, depth, env, types)?;
                
                let s_term = self.ast.expr(s)?;
                if let Expr::Lambda { body: s_body1, .. } = s_term {
                    let mut env1 = env.to_vec();
                    let mut types1 = types.to_vec();
                    types1.push(Value::NatType);
                    let var_n = Value::Neutral(crate::eval::Neutral::Var(crate::ast::Level(depth)));
                    env1.push(var_n.clone());
                    
                    let s_body1_term = self.ast.expr(s_body1)?;
                    if let Expr::Lambda { body: s_body2, .. } = s_body1_term {
                        let mut env2 = env1.clone();
                        let mut types2 = types1.clone();
                        
                        let mot_n = crate::eval::apply(self.ast, crate::ast::Plicity::Explicit, mot_val.clone(), var_n.clone(), self.turbine);
                        types2.push(mot_n);
                        let var_ih = Value::Neutral(crate::eval::Neutral::Var(crate::ast::Level(depth + 1)));
                        env2.push(var_ih.clone());
                        
                        let succ_n = Value::Succ(Box::new(var_n));
                        let mot_succ_n = crate::eval::apply(self.ast, crate::ast::Plicity::Explicit, mot_val.clone(), succ_n, self.turbine);
                        
                        self.check(s_body2, mot_succ_n, depth + 2, &env2, &types2)?;
                    } else {
                        panic!("S EXPECTED FUNCTION");
                    }
                } else {
                    panic!("S WAS NOT A LAMBDA");
                }
                
                let ret_ty = crate::eval::apply(self.ast, crate::ast::Plicity::Explicit, mot_val, target_val, self.turbine);
                Ok(Elaboration { ty: ret_ty, usages: vec![] })
            }
        }
    }"""
c = c.replace(match_elab, repl_elab)

c = c.replace('if !unified {\n            return Err(Error::TypeMismatch {', 'if !unified {\n            panic!("CHECK UNIFY FAILED for expr {:?} expected {:?}", expr, expected);\n            return Err(Error::TypeMismatch {')

match_unify = """            (Value::Unit, Value::Unit) => true,
            (Value::UnitType, Value::UnitType) => true,"""

repl_unify = """            (Value::Unit, Value::Unit) => true,
            (Value::UnitType, Value::UnitType) => true,
            (Value::NatType, Value::NatType) => true,
            (Value::Zero, Value::Zero) => true,
            (Value::Succ(n1), Value::Succ(n2)) => Self::unify(ast, &n1, &n2, depth, turbine),"""

c = c.replace(match_unify, repl_unify)

with open('src/elaborator.rs', 'w') as f:
    f.write(c)

