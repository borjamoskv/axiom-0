# W-Types Implementation Plan

## 1. Lexer (`src/lexer.rs`)
Add keywords: `W` (or `wtype`), `sup`, `wrec`.
```rust
"wtype" => TokenKind::WType,
"sup" => TokenKind::Sup,
"wrec" => TokenKind::WRec,
```

## 2. AST (`src/ast.rs`)
```rust
    WType {
        domain: ExprId,
        codomain: ExprId, // closure taking domain
    },
    Sup {
        tag: ExprId, // a : A
        payload: ExprId, // f : B(a) -> W(A, B)
    },
    WRec {
        mot: ExprId,
        step: ExprId,
        target: ExprId,
    },
```

## 3. Parser (`src/parser.rs`)
- `wtype (x : A) * B` (Syntax similar to Sigma)
- `sup(a, f)`
- `wrec(mot, step, target)`

## 4. Eval (`src/eval.rs`)
```rust
pub enum Value {
    // ...
    WType(Box<Value>, Closure),
    Sup(Box<Value>, Box<Value>),
}
pub enum Neutral {
    // ...
    WRec(Box<Value>, Box<Value>, Box<Neutral>),
}
```
Evaluation of `WRec`:
```rust
        Expr::WRec { mot, step, target } => {
            let mot_val = eval(ast, *mot, env, turbine);
            let step_val = eval(ast, *step, env, turbine);
            let target_val = eval(ast, *target, env, turbine);
            eval_wrec(ast, mot_val, step_val, target_val, turbine)
        }
```
```rust
fn eval_wrec(ast: &Ast, mot: Value, step: Value, target: Value, turbine: Option<&TurbineEngine>) -> Value {
    match target {
        Value::Sup(tag, payload) => {
            // we need to compute IH: \b -> wrec(mot, step, payload b)
            // But we don't have a closure that closes over `mot`, `step`, `payload` in our simple NbE unless we construct one, OR we just use a helper Value variant if we supported closures.
            // Wait, in standard NbE without environment-based closures for everything, how do we represent IH?
        }
        Value::Neutral(n) => Value::Neutral(Neutral::WRec(Box::new(mot), Box::new(step), Box::new(n))),
        _ => panic!("eval_wrec: invalid target"),
    }
}
```
Wait! To compute the IH, we need a semantic function. Our NbE `Value` only has syntactic `Closure` (which pairs an `ExprId` with an `Env`). It does not have semantic closures (like Rust closures `Box<dyn Fn(Value) -> Value>`).
If we don't have semantic closures, we cannot construct the IH as a `Value` on the fly easily, unless we add a new AST node specifically for the IH, or we extend `Value` with semantic closures or a special `IH(mot, step, payload)` variant that behaves like a function.

Let's check how `Value` and `eval` handle function application.
