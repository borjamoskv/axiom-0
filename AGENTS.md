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
