import re

with open('src/elaborator.rs', 'r') as f:
    c = f.read()

c = c.replace("""                    if let Some(turbine) = self.turbine {
                        turbine.inserted_implicits.write().unwrap().insert(expr, implicits_to_insert);
                    }""", """                    if let Some(turbine) = self.turbine {
                        println!("Inserting {} implicits for expr {:?}", implicits_to_insert.len(), expr);
                        turbine.inserted_implicits.write().unwrap().insert(expr, implicits_to_insert);
                    }""")

with open('src/elaborator.rs', 'w') as f:
    f.write(c)

