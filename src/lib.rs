#![forbid(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]
//! Micro-AXIOM-0: Quantitative Type Theory with Normalization by Evaluation
//!
//! Disable the default `std` feature for the explicit QTT kernel, erasure,
//! C99 generation and atomic telemetry using `core` and `alloc`.
//! REPL, metavariable inference and parallel batches require `std`.
//! See `docs/NO_STD.md` for target and application requirements.
extern crate alloc;

#[cfg(all(test, not(feature = "std")))]
extern crate std;

#[cfg(not(target_has_atomic = "ptr"))]
compile_error!("AXIOM-0 requires native pointer-width atomics with compare-and-swap");

pub mod ast;
pub mod elaborator;
pub mod erasure;
pub mod eval;
pub mod lexer;
pub mod parser;
#[cfg(feature = "std")]
pub mod repl;
pub mod seqlock;
pub mod turbine;
