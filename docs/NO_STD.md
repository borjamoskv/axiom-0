# Hosted and bare-metal feature contract

`std` is the only Cargo feature and is enabled by default. There are no external
dependencies. Disabling default features applies `#![no_std]` to the library;
heap-backed data comes from `alloc`, and atomic operations come from `core`.
The crate continues to forbid unsafe code.

| Facility | `std` (default) | `--no-default-features` |
| --- | --- | --- |
| AST, UTF-8 lexer and parser | Available | Available with `alloc` |
| Explicit QTT checking, NbE, naturals and identity elimination | Available | Available with `alloc` |
| Proof erasure and scalar C99 generation | Available | Available with `alloc` |
| `SeqlockCell` | Available | `core` only; native atomics required |
| `TurbineEngine` construction and atomic snapshot plane | Available | Available with `alloc` |
| Synchronized metavariable solving and automatic implicit insertion | Available | Unavailable; holes and omitted implicit arguments are rejected |
| REPL module and `micro-axiom-0` executable | Available | Excluded |
| `elaborate_batch_parallel` and `check_batch_parallel` | Available | Excluded |

The optional engine argument on evaluator/checker APIs remains available in both
builds. In the allocation-only build it supplies telemetry; it does not enable
metavariable inference. Fully explicit typed terms use the same evaluator,
quantity checks and erasure implementation in both configurations. Hosted
metavariable methods, including `Checker::unify`, are compiled only with `std`.
The scalar C99 backend returns errors for higher-order or pair-valued results it
cannot represent; enabling `no_std` does not extend that backend's language.

## Bare-metal verification

The supported verification target is `aarch64-unknown-none`, an AArch64 bare-metal
target with `core`, `alloc` and native pointer-width compare-and-swap. The AST
arena allocator and snapshot protocol require pointer-width atomics; targets
without those operations are rejected rather than using an unsafe or blocking
software fallback. Other integer atomic widths are enabled according to
`target_has_atomic`.

```sh
rustup target add aarch64-unknown-none
cargo check --lib --no-default-features --target aarch64-unknown-none
cargo build --lib --no-default-features --target aarch64-unknown-none
cargo test --workspace --no-default-features
```

The build emits a Rust library for that target. It does not boot a board, run a
simulator or link a complete firmware image. The final application must provide
a global allocator, panic handler, startup code, stack/heap placement and a
linker script appropriate to its device. This crate supplies none of those
platform policies. ASTs, parser buffers, semantic closures, erasure and generated
C strings allocate; disabling `std` does not make those paths heap-free.

A separate [QEMU firmware smoke test](../crates/qemu-firmware/README.md) supplies
those platform policies and executes the real library as an AArch64 ELF.
`python3 validate.py` from the repository root includes the bare-metal release
library build and firmware checks, followed by single-vCPU, SMP and separate
intentional-panic runs in QEMU TCG. All stages are required. The firmware's
standalone workspace remains separate, and the validator invokes it explicitly.

Hosted tests can execute against the `no_std` library using the host test
harness and allocator. Tests/examples that require hosted inference or batches
are gated by `std`; the remaining QTT, erasure, memory-model and snapshot tests
still run without default features.

## Progress and wait-free scope

| Operation | Bound or blocking behavior |
| --- | --- |
| `SeqlockCell::try_read(B)` | At most `B * (N + 2)` atomic loads, no allocation or locks; may exhaust its budget |
| `SeqlockCell::try_write` | One atomic load, at most one strong CAS, `N` payload stores and one commit; may return contention/exhaustion |
| `TurbineEngine::try_snapshot` | Bounds/arena validation and at most six loads per attempt for its four-word cell |
| `TurbineEngine::publish` | Arena validation, at most one arena-binding CAS, then one cell write attempt |
| Engine construction and AST arena allocation | Allocation and/or a CAS retry loop; no wait-free claim |
| Hosted inference and unification | Mutex/RwLock acquisition can block; a complete unification transaction is serialized |
| Hosted batch elaboration | Allocates, starts threads and waits for joins; can block |
| Parsing, checking, evaluation and erasure | Allocation and input-dependent recursion; no bounded-latency claim |

Snapshot bounds count calls to Rust atomic operations. Rust guarantees that
available native atomic types are lock-free, but does not generally guarantee
that their operations are wait-free. A hardware/compiler implementation may
retry an atomic operation internally. Consequently the library makes no portable
hardware wait-free guarantee, successful-access deadline or fairness claim.
[Rust atomic portability](https://doc.rust-lang.org/core/sync/atomic/index.html#portability).

A writer paused after acquiring an odd version can make every bounded read fail
and cause other writes to return `Contended`. Callers that retry until success
introduce an unbounded loop outside the API. A successful snapshot may be old;
neither its width nor its attempt budget bounds its age. Sequence numbers never
wrap; exhaustion rejects a write while leaving the last version readable.

Cells retain 64-byte alignment. That is a layout guarantee, not a guarantee of
zero false sharing on every cache geometry. Neither switching features nor
cross-compiling strengthens the memory model or constitutes a formal proof of
progress. See [the snapshot protocol](snapshot-protocol.md) for coherence and
finite-model coverage.

## Unified validation contract

Use [validate.py](../validate.py) for the complete core and firmware suite.

```sh
python3 validate.py
```

The suite includes host tests and Clippy with default features disabled, while
retaining these exact AGENTS.md controls for the default hosted configuration.

```sh
cargo check --workspace -q
cargo test --workspace -q
cargo clippy --workspace -- -D warnings
```

Those hosted controls cover the V1 occurs-check, concurrency and native C99
regressions. The unified suite also retains formatting, all-target and stress
coverage, checks firmware with default and all features, and validates actual
QEMU transcripts and exit statuses. It stops at the first failure and offers no
option to skip a required stage. See the [quickstart](../README.md)
for prerequisites and the `--qemu` and `--timeout` options.
