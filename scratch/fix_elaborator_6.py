with open('src/elaborator.rs', 'r') as f:
    c = f.read()

match_elab = """            Expr::Ind { mot, z, s, target } => {
                let target_elab = self.synth(target, depth, env, types)?;
                if !Checker::unify(self.ast, &target_elab.ty, &Value::NatType, depth, self.turbine) {
                    return Err(Error::TypeMismatch {
                        expr: target,
                        expected: Value::NatType,
                        actual: target_elab.ty,
                    });
                }
                
                let target_val = crate::eval::eval(self.ast, target, env, Some(self.turbine));
                
                let mot_elab = self.synth(mot, depth, env, types)?;
                let mot_val = crate::eval::eval(self.ast, mot, env, Some(self.turbine));
                
                match mot_elab.ty {
                    Value::Pi(_, _, ref domain, _) => {
                        if !Checker::unify(self.ast, &**domain, &Value::NatType, depth, self.turbine) {
                            return Err(Error::TypeMismatch {
                                expr: mot,
                                expected: Value::NatType,
                                actual: (**domain).clone(),
                            });
                        }
                    }
                    _ => return Err(Error::CannotInferLambda(mot)),
                }"""

repl_elab = """            Expr::Ind { mot, z, s, target } => {
                let target_elab = self.synth(target, depth, env, types)?;
                let unified_target = if let Some(t) = self.turbine {
                    Checker::unify(self.ast, &target_elab.ty, &Value::NatType, depth, t)
                } else {
                    target_elab.ty == Value::NatType
                };
                if !unified_target {
                    return Err(Error::TypeMismatch {
                        expr: target,
                        expected: Value::NatType,
                        found: target_elab.ty,
                    });
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
                            return Err(Error::TypeMismatch {
                                expr: mot,
                                expected: Value::NatType,
                                found: (**domain).clone(),
                            });
                        }
                    }
                    _ => return Err(Error::CannotInferLambda(mot)),
                }"""
c = c.replace(match_elab, repl_elab)

with open('src/elaborator.rs', 'w') as f:
    f.write(c)

