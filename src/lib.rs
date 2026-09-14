#![forbid(unsafe_code)]
//! Micro-AXIOM-0: Quantitative Type Theory with Normalization by Evaluation
//!
//! (Docs in progress during architectural leap to NbE)
pub mod ast;
pub mod elaborator;
pub mod lexer;
pub mod seqlock;
pub mod parser;
pub mod eval;
pub mod repl;
