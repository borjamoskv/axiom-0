#![no_std]
#![no_main]

extern crate alloc;

use alloc::{boxed::Box, vec::Vec};
use core::fmt::Write;
use core::panic::PanicInfo;
use micro_axiom_0::ast::{Ast, Command, ExprId, Quantity};
use micro_axiom_0::elaborator::{self, Error};
use micro_axiom_0::erasure::{self, RuntimeTerm};
use micro_axiom_0::eval::{self, Value};
use micro_axiom_0::lexer::Lexer;
use micro_axiom_0::parser::Parser;
use micro_axiom_0::seqlock::{ReadError, SeqlockCell};
use micro_axiom_0::turbine::{AtomicElabSnapshot, ElabStatus, TurbineEngine};

mod allocator;
mod platform;

// SAFETY: this assembly owns the ELF entry point and calls Rust only after
// establishing an aligned stack, zeroed BSS, enabled SIMD and masked interrupts.
core::arch::global_asm!(include_str!("boot.S"));

#[global_allocator]
static HEAP: allocator::BumpAllocator = allocator::BumpAllocator::new();

type Check = Result<(), &'static str>;
type NamedCheck = (&'static str, fn() -> Check);

fn require(condition: bool, message: &'static str) -> Check {
    condition.then_some(()).ok_or(message)
}

fn parse(source: &str) -> Result<(Ast, ExprId), &'static str> {
    let mut ast = Ast::new();
    let tokens = Lexer::new(source)
        .tokenize_all()
        .map_err(|_| "lexer rejected source")?;
    let command = Parser::new(&tokens, &mut ast, &[])
        .parse_command()
        .map_err(|_| "parser rejected source")?;
    match command {
        Command::Eval(root) => Ok((ast, root)),
        _ => Err("expected an expression"),
    }
}

fn heap_check() -> Check {
    #[repr(align(4096))]
    struct Aligned([u8; 128]);
    let aligned = Box::new(Aligned([0x5a; 128]));
    require(
        (&*aligned as *const Aligned as usize) % 4096 == 0,
        "heap alignment",
    )?;
    require(aligned.0[127] == 0x5a, "heap contents")?;
    let mut impossible = Vec::<u8>::new();
    require(
        impossible
            .try_reserve_exact(allocator::HEAP_BYTES + 1)
            .is_err(),
        "heap exhaustion",
    )?;
    let mut vector = Vec::new();
    for value in 0..64u64 {
        vector.push(value);
    }
    require(
        vector.iter().sum::<u64>() == 2016,
        "heap growth after failed reservation",
    )
}

fn polymorphism_check() -> Check {
    let (ast, root) =
        parse("((fn {A} -> fn :^1 x -> x) : fn {A : type} -> fn :^1 (x : A) -> A) {Nat} S(Z)")?;
    let ty = elaborator::synthesize(&ast, root, &[]).map_err(|_| "polymorphic type checking")?;
    require(ty.ty == Value::NatType, "polymorphic result type")?;
    require(
        eval::eval(&ast, root, &[], None) == Value::Succ(Box::new(Value::Zero)),
        "NbE result",
    )?;
    let erased = erasure::erase_expression(&ast, root).map_err(|_| "polymorphic erasure")?;
    require(
        erasure::eval_runtime(&erased, &[]) == RuntimeTerm::Nat(1),
        "runtime reindexing",
    )
}

fn linearity_check() -> Check {
    for body in ["True", "if x then x else x", "if True then x else False"] {
        let source = alloc::format!("(fn :^1 x -> {body}) : fn :^1 (x : Bool) -> Bool");
        let (ast, root) = parse(&source)?;
        require(
            matches!(
                elaborator::synthesize(&ast, root, &[]),
                Err(Error::UsageMismatch {
                    declared: Quantity::One,
                    ..
                })
            ),
            "linear misuse was accepted",
        )?;
    }
    Ok(())
}

fn erased_argument_check() -> Check {
    let (ast, root) = parse("((fn :^0 x -> True) : fn :^0 (x : Nat) -> Bool) Z")?;
    elaborator::synthesize(&ast, root, &[]).map_err(|_| "erased argument typing")?;
    let term = erasure::erase_expression(&ast, root).map_err(|_| "argument erasure")?;
    require(
        erasure::eval_runtime(&term, &[]) == RuntimeTerm::Bool(true),
        "erased application got stuck",
    )
}

fn recursion_check() -> Check {
    let (ast, root) = parse("ind(fn n -> Nat, Z, fn :^0 n -> fn :^1 ih -> S(ih), S(S(S(Z))))")?;
    elaborator::synthesize(&ast, root, &[]).map_err(|_| "recursor typing")?;
    let mut expected = Value::Zero;
    for _ in 0..3 {
        expected = Value::Succ(Box::new(expected));
    }
    require(
        eval::eval(&ast, root, &[], None) == expected,
        "recursor NbE",
    )?;
    let term = erasure::erase_expression(&ast, root).map_err(|_| "recursor erasure")?;
    require(
        erasure::eval_runtime(&term, &[]) == RuntimeTerm::Nat(3),
        "erased recursor",
    )?;
    let c = erasure::try_emit_c_code(&term, "guest_result").map_err(|_| "scalar C generation")?;
    require(
        c.contains("__axiom0_natrec_0"),
        "recursion silently replaced by a scalar",
    )
}

fn identity_proof_check() -> Check {
    let (ast, root) = parse("J(fn b -> fn p -> Nat, S(S(Z)), refl(Z))")?;
    elaborator::synthesize(&ast, root, &[]).map_err(|_| "identity elimination typing")?;
    let term = erasure::erase_expression(&ast, root).map_err(|_| "identity proof erasure")?;
    require(
        term == RuntimeTerm::Nat(2),
        "identity elimination retained runtime proof",
    )
}

fn snapshot_check() -> Check {
    let cell = SeqlockCell::<u64, 4>::new([0, 0, 0, 0]);
    require(
        matches!(
            cell.try_read(0),
            Err(ReadError::RetryBudgetExhausted { attempts: 0 })
        ),
        "read budget",
    )?;
    require(core::mem::align_of_val(&cell) == 64, "snapshot alignment")?;
    for generation in 1..=64u64 {
        let words = [generation, generation * 2, generation * 3, generation * 4];
        let version = cell.try_write(words).map_err(|_| "snapshot write")?;
        let snap = cell.try_read(1).map_err(|_| "snapshot read")?;
        require(
            version == generation as usize * 2,
            "snapshot commit version",
        )?;
        require(
            snap.version == version && snap.value == words && snap.attempts == 1,
            "snapshot coherence",
        )?;
    }
    Ok(())
}

fn telemetry_check() -> Check {
    let (ast, root) = parse("S(Z)")?;
    let engine = TurbineEngine::for_ast(&ast);
    elaborator::synthesize_with_turbine(&ast, root, &[], &engine)
        .map_err(|_| "telemetry typing")?;
    let snapshot = engine.try_snapshot(root, 1).map_err(|_| "telemetry read")?;
    require(
        AtomicElabSnapshot::from_words(snapshot.value).status == ElabStatus::CertifiedValid,
        "missing completed elaboration telemetry",
    )?;
    let (hole_ast, hole) = parse("?")?;
    let hole_engine = TurbineEngine::for_ast(&hole_ast);
    require(
        elaborator::synthesize_with_turbine(&hole_ast, hole, &[], &hole_engine).is_err(),
        "unresolved hole accepted",
    )
}

// SAFETY: _start is the only caller and establishes the documented boot invariant;
// the unique exported symbol matches boot.S and never returns to an unknown caller.
#[unsafe(no_mangle)]
pub extern "C" fn firmware_main() -> ! {
    let _ = writeln!(
        platform::Console,
        "AXIOM-0 BOOT: aarch64 EL1, no_std + alloc"
    );
    let checks: [NamedCheck; 8] = [
        ("allocator alignment, exhaustion and growth", heap_check),
        ("QTT polymorphism and NbE", polymorphism_check),
        ("QTT linearity and branch parity", linearity_check),
        ("zero-quantity argument erasure", erased_argument_check),
        ("natural recursion and C99 generation", recursion_check),
        ("identity proof erasure", identity_proof_check),
        ("atomic snapshots: 64 publications", snapshot_check),
        ("Turbine telemetry and rejected holes", telemetry_check),
    ];
    for (name, check) in checks {
        match check() {
            Ok(()) => {
                let _ = writeln!(platform::Console, "[OK] {name}");
            }
            Err(reason) => {
                let _ = writeln!(platform::Console, "[FAIL] {name}: {reason}");
                platform::exit(1);
            }
        }
    }
    let _ = writeln!(
        platform::Console,
        "Heap used: {} / {} bytes (bump, no reclamation)",
        HEAP.used(),
        allocator::HEAP_BYTES
    );
    #[cfg(feature = "panic-test")]
    panic!("intentional panic self-test");
    #[cfg(not(feature = "panic-test"))]
    {
        let _ = writeln!(platform::Console, "FIRMWARE PASS: 8 checks");
        platform::exit(0)
    }
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    // Formatting and the semihosting character writer require no heap allocation.
    let _ = writeln!(platform::Console, "[PANIC] {info}");
    platform::exit(1)
}
