with open('src/elaborator.rs', 'r') as f:
    c = f.read()

match_elab = """            Expr::Ind { mot, z, s, target } => {
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
                }"""

repl_elab = """            Expr::Ind { mot, z, s, target } => {
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
c = c.replace(match_elab, repl_elab)

match_elab2 = """                let s_ty_val = crate::eval::eval(self.ast, s_ty_ast, env, self.turbine);
                self.check(s, s_ty_val, depth, env, types)?;
                
                let ret_ty = crate::eval::apply(self.ast, crate::ast::Plicity::Explicit, mot_val, target_val, self.turbine);
                Ok(Elaboration { ty: ret_ty, usages: vec![] })
            }"""

repl_elab2 = """                let s_ty_val = crate::eval::eval(self.ast, s_ty_ast, env, Some(self.turbine));
                self.check(s, s_ty_val, depth, env, types)?;
                
                let ret_ty = crate::eval::apply(self.ast, crate::ast::Plicity::Explicit, mot_val, target_val, Some(self.turbine));
                Ok(Elaboration { ty: ret_ty, usages: vec![] })
            }"""
c = c.replace(match_elab2, repl_elab2)
with open('src/elaborator.rs', 'w') as f:
    f.write(c)
