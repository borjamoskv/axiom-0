with open('src/elaborator.rs', 'r') as f:
    c = f.read()

match_mot = """                let mot_elab = self.synth(mot, depth, env, types)?;
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
                }"""

repl_mot = """                let mot_term = self.ast.expr(mot)?;
                if let Expr::Lambda { body: mot_body1, .. } = mot_term {
                    let mut env_mot = env.to_vec();
                    let mut types_mot = types.to_vec();
                    types_mot.push(Value::NatType);
                    let var_n = Value::Neutral(crate::eval::Neutral::Var(crate::ast::Level(depth)));
                    env_mot.push(var_n.clone());
                    
                    let mot_body_elab = self.synth(*mot_body1, depth + 1, &env_mot, &types_mot)?;
                    if let Value::Universe(_) = mot_body_elab.ty {
                        // valid motive body
                    } else {
                        panic!("MOT BODY MUST BE A TYPE");
                    }
                } else {
                    let mot_elab = self.synth(mot, depth, env, types)?;
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
                        _ => panic!("MOT MUST BE A FUNCTION NAT -> TYPE"),
                    }
                }
                let mot_val = crate::eval::eval(self.ast, mot, env, self.turbine);"""

c = c.replace(match_mot, repl_mot)

with open('src/elaborator.rs', 'w') as f:
    f.write(c)

