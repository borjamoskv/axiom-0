# AGENTS.md — Sovereign Governance & Validation Contract (`axiom-0`)

## 1. Binary Validation Commands (Zero-Exploration Gate)
- **Type & Borrow Check:** `cargo check --workspace -q`
- **Unit & Formal Tests:** `cargo test --workspace -q`
- **Linter (Fail-Closed):** `cargo clippy --workspace -- -D warnings`

## 2. Repository Invariants
- **Domain:** Quantitative Type Theory (QTT) & Dependent Type Kernel in Rust.
- **Fail-Closed Policy:** Zero `unsafe` blocks without explicit formal safety invariant comment; zero panics (`unwrap`/`expect`) in kernel evaluation paths.
- **Language Invariant:** All public documentation, comments, commit messages, and issue descriptions must be in technical English.

## 3. Exploration Restrictions (`INV_C5_ARGON_GOVERNANCE`)
- Do not scan `target/` or `.git/`.
- Restrict reads to `src/`, `crates/`, and `Cargo.toml` unless explicitly instructed otherwise.

## 4. Kernel & Soundness Invariants (QTT & Silicon Architecture)
- **QTT Branch Parity:** Conditional branches (`if`) must enforce uniform usage across branches ($u_t = u_e$); asymmetric usage escalates to $\omega$ and rejects $\mathcal{Q}=1$ linear variables.
- **Stack-Frame Bounding:** Keep `Checker::synth_term` stack frames minimal by delegating deep inductive and identity synthesis to helper functions (`synth_ind`, `synth_j`) to prevent debug build stack overflows.
- **Erasure Discrimination:** Distinguish computational `Unit` from `RuntimeTerm::Erased` witnesses; reindex De Bruijn levels via level mappings when eliminating $\mathcal{Q}=0$ binders.
- **Cache-Line Alignment:** Maintain `#[repr(align(64))]` on `SeqlockCell` to ensure zero false sharing under Apple Silicon M-Series hardware.

