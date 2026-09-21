#![allow(unsafe_code)]

use micro_axiom_0::ast::{Ast, Expr, Level, Quantity};
use micro_axiom_0::elaborator::{Error as ElabError, check, synthesize};
use micro_axiom_0::eval::{Closure, Value, eval};

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

// ---------------------------------------------------------------------
// 1. INSTRUMENTACIÓN DE MEMORIA EN SILICIO (GlobalAlloc Tracking)
// ---------------------------------------------------------------------
struct TrackingAlloc;
static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static DEALLOCATED: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
        }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        DEALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) };
    }
}

#[global_allocator]
static GLOBAL: TrackingAlloc = TrackingAlloc;

fn allocated_bytes() -> usize {
    ALLOCATED.load(Ordering::SeqCst)
}

fn deallocated_bytes() -> usize {
    DEALLOCATED.load(Ordering::SeqCst)
}

// ---------------------------------------------------------------------
// 2. GENERADOR PSEUDO-ALEATORIO DETERMINISTA (XorShift64)
// ---------------------------------------------------------------------
struct Prng {
    state: u64,
}

impl Prng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0xdead_beef_c001_cafe
            } else {
                seed
            },
        }
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
    fn next_range(&mut self, min: usize, max: usize) -> usize {
        min + (self.next_u64() as usize % (max - min + 1))
    }
    #[allow(dead_code)]
    fn next_quantity(&mut self) -> Quantity {
        match self.next_u64() % 3 {
            0 => Quantity::Zero,
            1 => Quantity::One,
            _ => Quantity::Omega,
        }
    }
}

fn main() {
    println!("===============================================================================");
    println!("=== AXIOM-0: CERTIFICACIÓN POPPERIANA DEFINITIVA (10.000 ITERACIONES) ===");
    println!("===============================================================================");

    let mem_start = allocated_bytes();

    let mut prng = Prng::new(0x2026_0914);
    let mut total_nodes = 0;
    let mut depth_histogram = [0usize; 11];
    let mut qtt_zero_verified = 0;
    let mut qtt_one_verified = 0;
    let mut qtt_omega_verified = 0;
    let mut qtt_multilinear_verified = 0;
    let mut qtt_mismatch_caught = 0;

    let t0 = Instant::now();

    for iter in 0..10_000 {
        let mut ast = Ast::new();

        if iter % 5 == 0 {
            // -----------------------------------------------------------------
            // BATERÍA DE MUTACIONES NEGATIVAS: Falsación estricta de oráculos QTT
            // -----------------------------------------------------------------
            let subcase = (iter / 5) % 4;
            match subcase {
                0 => {
                    // Declarado Zero (0), pero usado 1 vez en el cuerpo
                    let var = ast.push(Expr::Var(Level(0))).unwrap();
                    let lam = ast
                        .push(Expr::Lambda { plicity: micro_axiom_0::ast::Plicity::Explicit,
                            quantity: Quantity::Zero,
                            body: var,
                        })
                        .unwrap();
                    let pi_cod = ast.push(Expr::UnitType).unwrap();
                    let expected_ty = Value::Pi(
                        micro_axiom_0::ast::Plicity::Explicit,
                        Quantity::Zero,
                        Box::new(Value::UnitType),
                        Closure {
                            env: vec![],
                            body: pi_cod,
                        },
                    );

                    match check(&ast, lam, expected_ty, &[]) {
                        Err(ElabError::UsageMismatch {
                            declared, observed, ..
                        }) => {
                            assert_eq!(declared, Quantity::Zero);
                            assert_eq!(observed, Quantity::One);
                            qtt_mismatch_caught += 1;
                        }
                        other => panic!(
                            "Iter {iter}: Se esperaba UsageMismatch(0, 1) pero se obtuvo {:?}",
                            other
                        ),
                    }
                }
                1 => {
                    // Declarado One (1), pero usado 0 veces en el cuerpo
                    let unit = ast.push(Expr::Unit).unwrap();
                    let lam = ast
                        .push(Expr::Lambda { plicity: micro_axiom_0::ast::Plicity::Explicit,
                            quantity: Quantity::One,
                            body: unit,
                        })
                        .unwrap();
                    let pi_cod = ast.push(Expr::UnitType).unwrap();
                    let expected_ty = Value::Pi(
                        micro_axiom_0::ast::Plicity::Explicit,
                        Quantity::One,
                        Box::new(Value::UnitType),
                        Closure {
                            env: vec![],
                            body: pi_cod,
                        },
                    );

                    match check(&ast, lam, expected_ty, &[]) {
                        Err(ElabError::UsageMismatch {
                            declared, observed, ..
                        }) => {
                            assert_eq!(declared, Quantity::One);
                            assert_eq!(observed, Quantity::Zero);
                            qtt_mismatch_caught += 1;
                        }
                        other => panic!(
                            "Iter {iter}: Se esperaba UsageMismatch(1, 0) pero se obtuvo {:?}",
                            other
                        ),
                    }
                }
                2 => {
                    // Declarado One (1), pero usado 2 veces mediante aplicación
                    let u1 = ast.push(Expr::UnitType).unwrap();
                    let u2 = ast.push(Expr::UnitType).unwrap();
                    let u_u = ast
                        .push(Expr::Pi { plicity: micro_axiom_0::ast::Plicity::Explicit,
                            quantity: Quantity::Omega,
                            domain: u1,
                            codomain: u2,
                        })
                        .unwrap();
                    let u3 = ast.push(Expr::UnitType).unwrap();
                    let u_u_u = ast
                        .push(Expr::Pi { plicity: micro_axiom_0::ast::Plicity::Explicit,
                            quantity: Quantity::Omega,
                            domain: u3,
                            codomain: u_u,
                        })
                        .unwrap();

                    // Constante que toma dos argumentos y retorna ()
                    let ret_unit = ast.push(Expr::Unit).unwrap();
                    let lam_inner = ast
                        .push(Expr::Lambda { plicity: micro_axiom_0::ast::Plicity::Explicit,
                            quantity: Quantity::Omega,
                            body: ret_unit,
                        })
                        .unwrap();
                    let lam_f = ast
                        .push(Expr::Lambda { plicity: micro_axiom_0::ast::Plicity::Explicit,
                            quantity: Quantity::Omega,
                            body: lam_inner,
                        })
                        .unwrap();
                    let ann_f = ast
                        .push(Expr::Ann {
                            term: lam_f,
                            ty: u_u_u,
                        })
                        .unwrap();

                    // Aplicamos ann_f a x (Level 0), luego a x (Level 0) de nuevo
                    let var_x1 = ast.push(Expr::Var(Level(0))).unwrap();
                    let app1 = ast
                        .push(Expr::App { plicity: micro_axiom_0::ast::Plicity::Explicit,
                            function: ann_f,
                            argument: var_x1,
                        })
                        .unwrap();
                    let var_x2 = ast.push(Expr::Var(Level(0))).unwrap();
                    let app2 = ast
                        .push(Expr::App { plicity: micro_axiom_0::ast::Plicity::Explicit,
                            function: app1,
                            argument: var_x2,
                        })
                        .unwrap();

                    let lam_bad = ast
                        .push(Expr::Lambda { plicity: micro_axiom_0::ast::Plicity::Explicit,
                            quantity: Quantity::One,
                            body: app2,
                        })
                        .unwrap();
                    let pi_cod = ast.push(Expr::UnitType).unwrap();
                    let expected_ty = Value::Pi(
                        micro_axiom_0::ast::Plicity::Explicit,
                        Quantity::One,
                        Box::new(Value::UnitType),
                        Closure {
                            env: vec![],
                            body: pi_cod,
                        },
                    );

                    match check(&ast, lam_bad, expected_ty, &[]) {
                        Err(ElabError::UsageMismatch {
                            declared, observed, ..
                        }) => {
                            assert_eq!(declared, Quantity::One);
                            assert_eq!(observed, Quantity::Omega);
                            qtt_mismatch_caught += 1;
                        }
                        other => panic!(
                            "Iter {iter}: Se esperaba UsageMismatch(1, Omega) pero se obtuvo {:?}",
                            other
                        ),
                    }
                }
                _ => {
                    // Infracción de anotación de cantidad: Lambda(Zero) vs Pi(One)
                    let unit = ast.push(Expr::Unit).unwrap();
                    let lam = ast
                        .push(Expr::Lambda { plicity: micro_axiom_0::ast::Plicity::Explicit,
                            quantity: Quantity::Zero,
                            body: unit,
                        })
                        .unwrap();
                    let pi_cod = ast.push(Expr::UnitType).unwrap();
                    let expected_ty = Value::Pi(
                        micro_axiom_0::ast::Plicity::Explicit,
                        Quantity::One,
                        Box::new(Value::UnitType),
                        Closure {
                            env: vec![],
                            body: pi_cod,
                        },
                    );

                    match check(&ast, lam, expected_ty, &[]) {
                        Err(ElabError::QuantityMismatch {
                            expected, found, ..
                        }) => {
                            assert_eq!(expected, Quantity::One);
                            assert_eq!(found, Quantity::Zero);
                            qtt_mismatch_caught += 1;
                        }
                        other => panic!(
                            "Iter {iter}: Se esperaba QuantityMismatch pero se obtuvo {:?}",
                            other
                        ),
                    }
                }
            }
            total_nodes += ast.expression_count();
        } else if iter % 7 == 0 {
            // -----------------------------------------------------------------
            // CASO MULTI-LINEAL DE ORDEN SUPERIOR:
            // f :^1 (Unit -> Unit) -> x :^1 Unit -> f x
            // Ambos f y x son consumidos linealmente exactamente una vez
            // -----------------------------------------------------------------
            let u1 = ast.push(Expr::UnitType).unwrap();
            let u2 = ast.push(Expr::UnitType).unwrap();
            let fn_ty = ast
                .push(Expr::Pi { plicity: micro_axiom_0::ast::Plicity::Explicit,
                    quantity: Quantity::One,
                    domain: u1,
                    codomain: u2,
                })
                .unwrap();
            let u3 = ast.push(Expr::UnitType).unwrap();
            let u4 = ast.push(Expr::UnitType).unwrap();

            // Pi(f :^1 (Unit -> Unit)) -> Pi(x :^1 Unit) -> Unit
            let inner_pi = ast
                .push(Expr::Pi { plicity: micro_axiom_0::ast::Plicity::Explicit,
                    quantity: Quantity::One,
                    domain: u3,
                    codomain: u4,
                })
                .unwrap();
            let outer_pi = ast
                .push(Expr::Pi { plicity: micro_axiom_0::ast::Plicity::Explicit,
                    quantity: Quantity::One,
                    domain: fn_ty,
                    codomain: inner_pi,
                })
                .unwrap();

            let elab_pi = synthesize(&ast, outer_pi, &[]).expect("synth multi-linear Pi failed");
            assert!(matches!(elab_pi.ty, Value::Universe(_)));

            // f está en Level 0, x está en Level 1
            let var_f = ast.push(Expr::Var(Level(0))).unwrap();
            let var_x = ast.push(Expr::Var(Level(1))).unwrap();
            let body_app = ast
                .push(Expr::App { plicity: micro_axiom_0::ast::Plicity::Explicit,
                    function: var_f,
                    argument: var_x,
                })
                .unwrap();

            let inner_lam = ast
                .push(Expr::Lambda { plicity: micro_axiom_0::ast::Plicity::Explicit,
                    quantity: Quantity::One,
                    body: body_app,
                })
                .unwrap();
            let outer_lam = ast
                .push(Expr::Lambda { plicity: micro_axiom_0::ast::Plicity::Explicit,
                    quantity: Quantity::One,
                    body: inner_lam,
                })
                .unwrap();

            let expected_ty_val = eval(&ast, outer_pi, &[], None);
            let elab = check(&ast, outer_lam, expected_ty_val, &[])
                .expect("check multi-linear lambda failed");
            assert!(elab.usages.is_empty());

            // Reducción computacional real: aplicar a identidad (fn :^1 y -> y) y ()
            let u_id1 = ast.push(Expr::UnitType).unwrap();
            let u_id2 = ast.push(Expr::UnitType).unwrap();
            let fn_ty_id = ast
                .push(Expr::Pi { plicity: micro_axiom_0::ast::Plicity::Explicit,
                    quantity: Quantity::One,
                    domain: u_id1,
                    codomain: u_id2,
                })
                .unwrap();
            let var_y = ast.push(Expr::Var(Level(0))).unwrap();
            let id_lam = ast
                .push(Expr::Lambda { plicity: micro_axiom_0::ast::Plicity::Explicit,
                    quantity: Quantity::One,
                    body: var_y,
                })
                .unwrap();
            let ann_id = ast
                .push(Expr::Ann {
                    term: id_lam,
                    ty: fn_ty_id,
                })
                .unwrap();
            let unit_val = ast.push(Expr::Unit).unwrap();

            let app1 = ast
                .push(Expr::App { plicity: micro_axiom_0::ast::Plicity::Explicit,
                    function: outer_lam,
                    argument: ann_id,
                })
                .unwrap();
            let app2 = ast
                .push(Expr::App { plicity: micro_axiom_0::ast::Plicity::Explicit,
                    function: app1,
                    argument: unit_val,
                })
                .unwrap();

            let reduced = eval(&ast, app2, &[], None);
            assert!(matches!(reduced, Value::Unit));

            qtt_multilinear_verified += 1;
            total_nodes += ast.expression_count();
        } else {
            // -----------------------------------------------------------------
            // FUZZING GENERATIVO POSITIVO: Profundidades d in [1, 10]
            // Construcción y verificación de tipos Pi anidados arbitrarios y
            // torres de lambdas que satisfacen exactamente el semiring QTT
            // -----------------------------------------------------------------
            let depth = prng.next_range(1, 10);
            depth_histogram[depth] += 1;

            let target_mode = prng.next_range(0, 2); // 0: target=One, 1: target=Zero, 2: all Omega
            let target_var = prng.next_range(0, depth - 1);

            let mut quantities = Vec::with_capacity(depth);
            for i in 0..depth {
                let q = match target_mode {
                    0 => {
                        if i == target_var {
                            Quantity::One
                        } else if prng.next_u64() % 2 == 0 {
                            Quantity::Zero
                        } else {
                            Quantity::Omega
                        }
                    }
                    1 => {
                        if prng.next_u64() % 2 == 0 {
                            Quantity::Zero
                        } else {
                            Quantity::Omega
                        }
                    }
                    _ => Quantity::Omega,
                };
                quantities.push(q);
            }

            // Construcción del tipo Pi anidado:
            // Pi(x_0 : q_0 Unit) -> Pi(x_1 : q_1 Unit) -> ... -> Unit
            let mut current_pi = ast.push(Expr::UnitType).unwrap();
            for &q in quantities.iter().rev() {
                let dom = ast.push(Expr::UnitType).unwrap();
                current_pi = ast
                    .push(Expr::Pi { plicity: micro_axiom_0::ast::Plicity::Explicit,
                        quantity: q,
                        domain: dom,
                        codomain: current_pi,
                    })
                    .unwrap();
            }

            let elab_pi = synthesize(&ast, current_pi, &[]).expect("synth deep Pi failed");
            assert!(matches!(elab_pi.ty, Value::Universe(_)));

            // Construcción del cuerpo según el target
            let inner_body = match target_mode {
                0 => {
                    qtt_one_verified += 1;
                    ast.push(Expr::Var(Level(target_var))).unwrap()
                }
                1 => {
                    qtt_zero_verified += 1;
                    ast.push(Expr::Unit).unwrap()
                }
                _ => {
                    qtt_omega_verified += 1;
                    ast.push(Expr::Unit).unwrap()
                }
            };

            // Construcción de la torre de lambdas de adentro hacia afuera:
            // Lam(q_{depth-1}, ... Lam(q_0, inner_body))
            let mut current_lam = inner_body;
            for &q in quantities.iter().rev() {
                current_lam = ast
                    .push(Expr::Lambda { plicity: micro_axiom_0::ast::Plicity::Explicit,
                        quantity: q,
                        body: current_lam,
                    })
                    .unwrap();
            }

            let expected_ty_val = eval(&ast, current_pi, &[], None);
            let elab =
                check(&ast, current_lam, expected_ty_val, &[]).expect("check deep lambda failed");
            assert!(elab.usages.is_empty());

            total_nodes += ast.expression_count();
        }
    }

    let elapsed = t0.elapsed();
    let mem_end = allocated_bytes();
    let mem_dealloc = deallocated_bytes();
    let net_heap_residue = mem_end.saturating_sub(mem_dealloc);

    println!("\n[1] FUZZING GENERATIVO Y PROFUNDIDAD (d ∈ [1, 10]):");
    println!("  ├── Iteraciones totales procesadas: 10.000");
    println!("  ├── Nodos AST alocados: {} nodos", total_nodes);
    println!("  └── Histograma de profundidades:");
    for (d, &count) in depth_histogram.iter().enumerate().skip(1).take(10) {
        let bar = if count > 0 {
            "#".repeat((count / 100).max(1))
        } else {
            "".to_string()
        };
        println!("      d={:2}: {:4} casos | {}", d, count, bar);
    }

    println!("\n[2] VERIFICACIÓN CUANTITATIVA QTT COMPOSICIONAL (Semiring {{0, 1, ω}}):");
    println!(
        "  ├── Torres lineales unitarias (ρ=1) certificadas: {}",
        qtt_one_verified
    );
    println!(
        "  ├── Torres lineales de orden superior (f :^1 -> x :^1 -> f x) certificadas: {}",
        qtt_multilinear_verified
    );
    println!(
        "  ├── Torres de borrado estricto (ρ=0) certificadas: {}",
        qtt_zero_verified
    );
    println!(
        "  ├── Torres irrestrictas (ρ=ω) certificadas: {}",
        qtt_omega_verified
    );
    println!(
        "  └── Infracciones QTT detectadas y repelidas por oráculos: {}",
        qtt_mismatch_caught
    );
    println!("  └── Tasa de Panics / Colapsos: 0.000% (CERO colapsos)");

    println!("\n[3] INSTRUMENTACIÓN DE MEMORIA EN SILICIO (GlobalAlloc Tracking):");
    println!(
        "  ├── Bytes alocados totales: {} bytes ({:.2} MB)",
        mem_end - mem_start,
        (mem_end - mem_start) as f64 / 1_048_576.0
    );
    println!("  ├── Bytes liberados totales: {} bytes", mem_dealloc);
    println!(
        "  └── Residuo de memoria viva (allocated - deallocated): {} bytes",
        net_heap_residue
    );
    assert!(
        net_heap_residue < 100_000,
        "Fuga de memoria detectada en el heap!"
    );

    println!("\n[4] RENDIMIENTO Y EXERGÍA DE SILICIO:");
    println!("  ├── Tiempo total de ejecución: {:.2?}", elapsed);
    println!(
        "  └── Latencia media por término dependiente: {:.2} µs",
        elapsed.as_micros() as f64 / 10_000.0
    );

    println!("\n===============================================================================");
    println!(" RESULTADO: CERTIFICACIÓN POPPERIANA DEFINITIVA 100% CUMPLIDA (ALTA EXERGÍA)");
    println!("===============================================================================");
}
