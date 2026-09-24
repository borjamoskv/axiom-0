with open('tests/stress.rs', 'r') as f:
    c = f.read()

# Replace the mot reuse in test 7
old_block = """    // Motive: fn _ -> Nat
    let nat_ty = ast.push(Expr::NatType).unwrap();
    let mot = ast
        .push(Expr::Lambda {
            plicity: Plicity::Explicit,
            quantity: Quantity::Omega,
            body: nat_ty,
        })
        .unwrap();

    // step_add: fn _ -> fn ih -> Succ(ih)
    let var_ih_add = ast.push(Expr::Var(Level(1))).unwrap();
    let succ_ih_add = ast.push(Expr::Succ(var_ih_add)).unwrap();
    let inner_add = ast
        .push(Expr::Lambda {
            plicity: Plicity::Explicit,
            quantity: Quantity::Omega,
            body: succ_ih_add,
        })
        .unwrap();
    let step_add = ast
        .push(Expr::Lambda {
            plicity: Plicity::Explicit,
            quantity: Quantity::Omega,
            body: inner_add,
        })
        .unwrap();

    // step_mul: fn n -> fn ih -> add(n12_b, ih)
    // where add(n12_b, ih) = ind(mot, ih, step_add, n12_b)
    let var_ih_mul = ast.push(Expr::Var(Level(1))).unwrap();
    let add_call = ast
        .push(Expr::Ind {
            mot,
            z: var_ih_mul,
            s: step_add,
            target: n12_b,
        })
        .unwrap();
    let inner_mul = ast
        .push(Expr::Lambda {
            plicity: Plicity::Explicit,
            quantity: Quantity::Omega,
            body: add_call,
        })
        .unwrap();
    let step_mul = ast
        .push(Expr::Lambda {
            plicity: Plicity::Explicit,
            quantity: Quantity::Omega,
            body: inner_mul,
        })
        .unwrap();

    // mul(n12_a, n12_b) = ind(mot, 0, step_mul, n12_a)
    let zero = ast.push(Expr::Zero).unwrap();
    let mul_expr = ast
        .push(Expr::Ind {
            mot,
            z: zero,
            s: step_mul,
            target: n12_a,
        })
        .unwrap();"""

new_block = """    // Motive for add: fn _ -> Nat
    let nat_ty_add = ast.push(Expr::NatType).unwrap();
    let mot_add = ast
        .push(Expr::Lambda {
            plicity: Plicity::Explicit,
            quantity: Quantity::Omega,
            body: nat_ty_add,
        })
        .unwrap();

    // step_add: fn _ -> fn ih -> Succ(ih)
    let var_ih_add = ast.push(Expr::Var(Level(1))).unwrap();
    let succ_ih_add = ast.push(Expr::Succ(var_ih_add)).unwrap();
    let inner_add = ast
        .push(Expr::Lambda {
            plicity: Plicity::Explicit,
            quantity: Quantity::Omega,
            body: succ_ih_add,
        })
        .unwrap();
    let step_add = ast
        .push(Expr::Lambda {
            plicity: Plicity::Explicit,
            quantity: Quantity::Omega,
            body: inner_add,
        })
        .unwrap();

    // Motive for mul: fn _ -> Nat
    let nat_ty_mul = ast.push(Expr::NatType).unwrap();
    let mot_mul = ast
        .push(Expr::Lambda {
            plicity: Plicity::Explicit,
            quantity: Quantity::Omega,
            body: nat_ty_mul,
        })
        .unwrap();

    // step_mul: fn n -> fn ih -> add(n12_b, ih)
    // where add(n12_b, ih) = ind(mot_add, ih, step_add, n12_b)
    let var_ih_mul = ast.push(Expr::Var(Level(1))).unwrap();
    let add_call = ast
        .push(Expr::Ind {
            mot: mot_add,
            z: var_ih_mul,
            s: step_add,
            target: n12_b,
        })
        .unwrap();
    let inner_mul = ast
        .push(Expr::Lambda {
            plicity: Plicity::Explicit,
            quantity: Quantity::Omega,
            body: add_call,
        })
        .unwrap();
    let step_mul = ast
        .push(Expr::Lambda {
            plicity: Plicity::Explicit,
            quantity: Quantity::Omega,
            body: inner_mul,
        })
        .unwrap();

    // mul(n12_a, n12_b) = ind(mot_mul, 0, step_mul, n12_a)
    let zero = ast.push(Expr::Zero).unwrap();
    let mul_expr = ast
        .push(Expr::Ind {
            mot: mot_mul,
            z: zero,
            s: step_mul,
            target: n12_a,
        })
        .unwrap();"""

c = c.replace(old_block, new_block)
with open('tests/stress.rs', 'w') as f:
    f.write(c)

