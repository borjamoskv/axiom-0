use micro_axiom_0::lexer::Lexer;
fn main() {
    let mut lexer = Lexer::new("let id : Type -> Type = fn x -> x;");
    println!("{:?}", lexer.tokenize_all());
}
