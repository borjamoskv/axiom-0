# Micro-AXIOM-0

> **Zero-dependency QTT kernel with normalization by evaluation, cumulative universes, a `no_std` + `alloc` core, and bounded-attempt atomic snapshots.**

[![CI Status](https://img.shields.io/badge/CI_Workflow-Passing-brightgreen?style=flat-square)](https://github.com/borjamoskv/axiom-0/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue?style=flat-square)](./LICENSE)
[![Dependencies](https://img.shields.io/badge/Dependencies-0_(Pure_Rust)-orange?style=flat-square)](./Cargo.toml)

---

## ⚡ Quickstart

Run the complete validation suite from the repository root.

```sh
python3 validate.py
```

The suite requires Python 3.10 or newer, a stable Rust toolchain with `rustfmt` and `clippy`,
the `aarch64-unknown-none` target, `qemu-system-aarch64`, and a native C99 compiler
selected by `CC` or named `cc`. Install the Rust components with these commands.

```sh
rustup component add rustfmt clippy
rustup target add aarch64-unknown-none
```

[validate.py](validate.py) runs the three AGENTS.md controls, hosted formatting,
all-target checks and stress coverage, host tests and Clippy without default
features, the bare-metal library build, firmware formatting and Clippy, and
QEMU runs for one vCPU, two-vCPU snapshot contention and the separate panic
control. The CI workflow uses the same entrypoint. Every stage is required and
the first failure stops validation with a nonzero exit status.

An absolute path to `validate.py` works from any current directory.
`--qemu /absolute/path/qemu-system-aarch64` selects the emulator, and
`--timeout 30` sets the timeout for each guest run. The default is 30 seconds
and the value must be finite and positive.

Launch the interactive REPL separately.

```sh
cargo run --bin micro-axiom-0
```

### Hosted and bare-metal builds

The `std` feature is enabled by default. Disable it to build the explicit QTT
checker, evaluator, parser, erasure, scalar C99 generator and atomic snapshot
plane with `core` and `alloc`. REPL, metavariable/implicit inference and parallel
batches require `std`.

```sh
rustup target add aarch64-unknown-none
cargo build --lib --no-default-features --target aarch64-unknown-none
cargo test --workspace --no-default-features
```

The final bare-metal application supplies its allocator, panic handler and
device-specific startup/linker configuration. Building the library does not
verify firmware execution. See [features and progress guarantees](docs/NO_STD.md).

The full validation command boots and tests actual `no_std` ELFs in QEMU.
For individual firmware runs and platform setup, follow the
[minimal AArch64 firmware instructions](crates/qemu-firmware/README.md).

---

## 🧬 Architecture & Compilation Topology

```mermaid
flowchart TD
    classDef core fill:#1e293b,stroke:#38bdf8,stroke-width:2px,color:#f8fafc
    classDef verify fill:#064e3b,stroke:#34d399,stroke-width:2px,color:#f8fafc
    classDef mem fill:#450a0a,stroke:#f87171,stroke-width:2px,color:#f8fafc

    A["Zero-Copy QTT Lexer & Recursive Parser<br/>(src/lexer.rs · src/parser.rs)"]:::core --> B["Contiguous Arena AST & De Bruijn Levels<br/>(src/ast.rs)"]:::core
    B --> C["Bidirectional QTT Elaborator (0, 1, ω)<br/>(src/elaborator.rs)"]:::verify
    C --> D["Normalization by Evaluation (NbE)<br/>β-reduction & η-equality (src/eval.rs)"]:::verify
    D --> E["Bounded-attempt atomic SeqlockCell<T, N><br/>Axiom-MM (src/seqlock.rs)"]:::mem
```

```text
.
├── Cargo.toml
├── Cargo.lock
├── src/
│   ├── lib.rs              # Exportaciones públicas de Ring-0
│   ├── main.rs             # Punto de entrada interactivo REPL
│   ├── ast.rs              # Arena continua de expresiones y niveles De Bruijn
│   ├── lexer.rs            # Transductor léxico zero-copy con anotaciones QTT
│   ├── parser.rs           # Parser recursivo, comandos globales let y binders
│   ├── eval.rs             # Normalización por Evaluación (NbE), Valores y Cierres
│   ├── elaborator.rs       # Verificador bidireccional, subtipado cumulativo y álgebra QTT
│   ├── repl.rs             # Bucle interactivo de alta exergía con estado global
│   └── seqlock.rs          # Core-only atomic snapshots with bounded attempts
├── docs/
│   ├── MEMORY_MODEL.md     # Contrato formal y garantías de coherencia secuencial
│   └── snapshot-protocol.md
├── examples/
│   ├── snapshot.rs         # Demostración del protocolo de instantáneas
│   ├── stress_10k.rs       # Prueba de carga de 10.000 iteraciones
│   ├── stress_10k_rigorous.rs
│   └── stress_popperian_10k.rs # Falsación empírica masiva generativa
└── tests/
    ├── qtt_nbe.rs          # Verificación formal de NbE, QTT, universos y eta-igualdad
    ├── seqlock.rs          # Concurrencia, contención y exhaustión de versiones
    ├── seqlock_contract.rs
    ├── seqlock_handoff.rs
    ├── seqlock_model.rs
    └── seqlock_wide.rs
```

---

## 1. Fundamentos Teóricos y Semántica

### Semianillo Cuantitativo (QTT)
El cálculo modela recursos físicos mediante cantidades $\rho \in \{0, 1, \omega\}$:
- **`0` (Erased)**: Término puramente computacional/estático en tiempo de compilación. Borrado en tiempo de ejecución.
- **`1` (Linear)**: Consumo unívoco obligatorio. Prohibida la duplicación o el descarte (*drop*).
- **`\omega` (Unrestricted)**: Uso libre/afín (cero o más apariciones).

```text
    + | 0  1  ω           × | 0  1  ω
   ---+--------          ---+--------
    0 | 0  1  ω           0 | 0  0  0
    1 | 1  ω  ω           1 | 0  1  ω
    ω | ω  ω  ω           ω | 0  ω  ω
```

### Normalización por Evaluación (NbE)
La conversión no opera mediante sustituciones sintácticas ingenuas $O(2^n)$, sino proyectando la sintaxis muerta (`ExprId`) sobre el territorio semántico vivo (`Value`):
- Cierres semánticos de entorno diferido: `Closure { env, body }`.
- Valores neutrales para variables libres: `Neutral::Var(Level)` y aplicaciones neutrales encadenadas.
- Conversión semántica completa con **$\beta$-reducción** y **$\eta$-igualdad funcional** (`\x. f x == f`).

### Estratificación de Universos y Subtipado Cumulativo
Para erradicar la Paradoja de Girard (Burali-Forti en teoría de tipos), los universos están estratificados:
$$\mathrm{Type}_0 : \mathrm{Type}_1 : \mathrm{Type}_2 : \dots : \mathrm{Type}_n : \mathrm{Type}_{n+1}$$
- `type n` sintetiza estrictamente a `Value::Universe(n + 1)`.
- Se implementa **Subtipado Cumulativo**: $\mathrm{Type}_i \le \mathrm{Type}_j \iff i \le j$.
- El supremo de un producto dependiente satisface:
  $$\Pi (x : \mathrm{Type}_u) \to \mathrm{Type}_v : \mathrm{Type}_{\max(u, v)}$$

---

## 2. Sintaxis y REPL

El REPL (`cargo run`) expone un entorno global persistente:

```rust
=== AXIOM-0 REPL (Singularidad NbE) ===
Modo de Alta Exergía Activo. Presiona Ctrl+C para salir.

# Declaración lineal de identidad polimórfica:
> let id : fn :^1 (x: type 0) -> type 0 = fn :^1 x -> x;
id definido.
==> Val: Lam(One, Closure { ... })
    Typ: Pi(One, Universe(0), Closure { ... })

# Aplicación reducida por NbE:
> id UnitType
==> Val: UnitType
    Typ: Universe(0)

# Falsación de borrado: variable de tipo 0 usada computacionalmente falla
> let leak : fn :^0 (x: type 0) -> type 0 = fn :^0 x -> x;
Error de Tipado en Let: UsageMismatch { declared: Zero, observed: One }

# Falsación lineal: variable lineal descartada (drop) falla
> let drop : fn :^1 (x: type 0) -> type 1 = fn :^1 x -> type 0;
Error de Tipado en Let: UsageMismatch { declared: One, observed: Zero }
```

---

## 3. Modelo Atómico Axiom-MM (`seqlock.rs`)

`SeqlockCell<T, N>` provides coherent snapshots of native atomic words using
Acquire/Release ordering and an explicit read-attempt budget. `try_read(B)` uses
at most `B * (N + 2)` atomic loads; `try_write` uses one load, at most one CAS,
`N` payload stores and one commit. Both may fail under contention.

These are Rust-level operation bounds, not a general hardware wait-free or
real-time success guarantee. A paused writer can prevent successful reads and
writes. Hosted `TurbineEngine` inference also uses Mutex/RwLock and joins worker
threads, so the complete engine can block. See the [progress contract](docs/NO_STD.md#progress-and-wait-free-scope).

---

## 4. License & Security

Distributed under the **MIT License** (see [`LICENSE`](./LICENSE)). Formal memory-model contracts and security invariants are documented in [`docs/MEMORY_MODEL.md`](./docs/MEMORY_MODEL.md) and `SECURITY.md`.
