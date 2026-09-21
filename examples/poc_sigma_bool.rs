use micro_axiom_0::ast::{Ast, Command};
use micro_axiom_0::eval::eval;
use micro_axiom_0::lexer::Lexer;
use micro_axiom_0::parser::Parser;
use micro_axiom_0::elaborator::{synthesize_in_env, check_in_env};

fn main() {
    println!("============================================================");
    println!("[ABZU_KERNEL] Inicializando Turbina AXIOM-0 (Exergía: 21.000)");
    println!("============================================================\n");

    let scripts = vec![
        ("1. Función Lógica", "let not : fn (b : Bool) -> Bool = fn b -> if b { False } else { True }"),
        ("2. Definición Topológica (Tipo Sigma Dependiente)", "let DepType : type = sigma (b : Bool) * if b { UnitType } else { Bool }"),
        ("3. Colapso en Rama True (Exige Unit)", "let pair_t : DepType = (True, ())"),
        ("4. Colapso en Rama False (Exige Bool)", "let pair_f : DepType = (False, not False)"),
        ("5. Evaluación NbE (Proyección Fst)", "pair_f.1"),
        ("6. Evaluación NbE (Proyección Snd)", "pair_f.2"),
    ];

    let mut ast = Ast::new();
    let mut env = Vec::new();
    let mut types = Vec::new();
    let mut names = Vec::new();

    for (desc, script) in scripts {
        println!(">>> [{}]", desc);
        println!("  | {}", script);

        let mut lexer = Lexer::new(script);
        let tokens = lexer.tokenize_all().unwrap();
        let mut parser = Parser::new(&tokens, &mut ast, &names);
        let cmd = parser.parse_command().unwrap();

        match cmd {
            Command::Let { name, ty, term } => {
                let ty_val = if let Some(ty_expr) = ty {
                    synthesize_in_env(&ast, ty_expr, &env, &types).unwrap();
                    eval(&ast, ty_expr, &env, None)
                } else {
                    synthesize_in_env(&ast, term, &env, &types).unwrap().ty
                };

                if ty.is_some() {
                    check_in_env(&ast, term, ty_val.clone(), &env, &types).unwrap();
                }

                let term_val = eval(&ast, term, &env, None);

                println!("  * [STATE] LIGADO '{}'", name);
                println!("  * [TYPE]  {:?}", ty_val);
                println!("  * [VALUE] {:?}\n", term_val);

                names.push(name);
                env.push(term_val);
                types.push(ty_val);
            }
            Command::Eval(expr) => {
                let elab = synthesize_in_env(&ast, expr, &env, &types).unwrap();
                let val = eval(&ast, expr, &env, None);
                println!("  * [TYPE]  {:?}", elab.ty);
                println!("  * [VALUE] {:?}\n", val);
            }
        }
    }
    
    println!("============================================================");
    println!("[MUSHUSHU-0] Atestación de prueba dependiente completada con éxito.");
    println!("============================================================");
}
