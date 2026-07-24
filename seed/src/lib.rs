//! Stage 0 seed compiler for Portland.
//!
//! Crude and disposable on purpose — it never has to be good, it has to exist.
//! Retired at Stage 2, when the compiler written in Portland compiles itself.

pub mod ast;
pub mod heredoc;
pub mod interpreter;
pub mod lexer;
pub mod parser;
pub mod value;
