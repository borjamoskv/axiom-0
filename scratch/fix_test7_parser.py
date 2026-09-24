with open('tests/stress.rs', 'r') as f:
    c = f.read()

import re

new_test7 = """/// 7. Deep NbE Reduction: Peano Multiplication via Nested Inductive Double Recursion (4 * 5 = 20).
#[test]
fn test_stress_peano_multiplication_double_induction() {
    println!("\\n=== [STRESS 7/12] Nested Inductive Double Recursion (Peano Multiplication) ===");
    let start = Instant::now();
    // 4 * 5 = 20 using nested ind expressions parsed directly
    // mul(4, 5) = ind(fn n -> Nat, Z, fn n -> fn ih -> ind(fn m -> Nat, ih, fn k -> fn acc -> S(acc), S(S(S(S(S(Z)))))), S(S(S(S(Z)))))
    let src = "ind(fn n -> Nat, Z, fn n -> fn ih -> ind(fn m -> Nat, ih, fn k -> fn acc -> S(acc), S(S(S(S(S(Z)))))), S(S(S(S(Z)))))";
    
    let mut ast = Ast::new();
    let expr = {
        let mut lexer = micro_axiom_0::lexer::Lexer::new(src);
        let tokens = lexer.tokenize_all().unwrap();
        let mut parser = micro_axiom_0::parser::Parser::new(&tokens, &mut ast, &[]);
        parser.parse_expression().unwrap()
    };

    // 1. Synthesize & Type Check
    let elab = elaborator::synthesize(&ast, expr, &[]).expect("Type check failed for Peano multiplication");
    assert_eq!(elab.ty, Value::NatType);

    // 2. Evaluate via NbE
    let val = eval(&ast, expr, &[], None);

    // Count successors
    let mut current = &val;
    let mut count = 0;
    while let Value::Succ(inner) = current {
        count += 1;
        current = inner;
    }
    assert_eq!(*current, Value::Zero);
    assert_eq!(count, 20, "Peano multiplication 4 * 5 must evaluate to 20");

    let elapsed = start.elapsed();
    println!("-> Elaborated and normalized Peano 4 * 5 = 20 in {:?}", elapsed);
}"""

# Replace the existing test_stress_peano_multiplication_double_induction function
idx = c.find("/// 7. Deep NbE Reduction:")
if idx != -1:
    c = c[:idx] + new_test7 + "\n"

with open('tests/stress.rs', 'w') as f:
    f.write(c)

