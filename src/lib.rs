//! Micro-AXIOM-0: a simply typed quantitative fragment, without dependent conversion.
//!
//! ```
//! use micro_axiom_0::{ast::{Ast, Expr, Level, Quantity}, elaborator};
//!
//! let mut ast = Ast::new();
//! let unit = ast.unit_type();
//! let ty = ast.function_type(Quantity::One, unit, unit)?;
//! let x = ast.push(Expr::Var(Level(0)))?;
//! let identity = ast.push(Expr::Lambda { quantity: Quantity::One, body: x })?;
//! let typed = elaborator::check(&ast, identity, ty)?;
//! assert_eq!(typed.type_of(x), Some(unit));
//! assert_eq!(typed.type_of(identity), Some(ty));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
pub mod ast;
pub mod elaborator;
pub mod lexer;
pub mod seqlock;
