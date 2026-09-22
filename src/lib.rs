#![forbid(unsafe_code)]
//! Micro-AXIOM-0: Quantitative Type Theory with Normalization by Evaluation
//!
//! (Docs in progress during architectural leap to NbE)
pub mod ast;
pub mod elaborator;
pub mod eval;
pub mod lexer;
pub mod parser;
pub mod repl;
pub mod seqlock;
pub mod turbine;
pub mod erasure;
