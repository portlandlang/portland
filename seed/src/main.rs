//! `pdx` — run a Portland file through the seed interpreter.

use std::process;

use portland_seed::interpreter::Interpreter;
use portland_seed::parser;

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: pdx <file.pdx>");
        process::exit(64);
    });
    let source = std::fs::read_to_string(&path).unwrap_or_else(|error| {
        eprintln!("pdx: cannot read {path}: {error}");
        process::exit(66);
    });
    let program = parser::parse(&source);
    let mut interpreter = Interpreter::new();
    interpreter.program(&program);
}
