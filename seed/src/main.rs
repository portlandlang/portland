//! `pdx` — run a Portland file through the seed interpreter, or start a REPL.

use std::io::{BufRead, IsTerminal, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::process;

use portland_seed::interpreter::Interpreter;
use portland_seed::parser;

fn main() {
    match std::env::args().nth(1) {
        Some(path) => run_file(&path),
        None => repl(),
    }
}

fn run_file(path: &str) {
    let source = std::fs::read_to_string(path).unwrap_or_else(|error| {
        eprintln!("pdx: cannot read {path}: {error}");
        process::exit(66);
    });
    let program = parser::parse(&source);
    let mut interpreter = Interpreter::new();
    interpreter.program(&program);
}

fn repl() {
    let interactive = std::io::stdin().is_terminal();
    if interactive {
        println!("Portland seed REPL — Ctrl-D to exit");
    }
    // The seed reports errors by panicking; the REPL catches them and carries on.
    std::panic::set_hook(Box::new(|_| {}));

    let mut interpreter = Interpreter::new();
    let mut buffer = String::new();
    prompt(interactive, &buffer);
    for line in std::io::stdin().lock().lines() {
        let line = line.expect("failed to read stdin");
        buffer.push_str(&line);
        buffer.push('\n');
        let source = buffer.clone();
        match catch_unwind(|| parser::parse(&source)) {
            Ok(program) => {
                buffer.clear();
                match catch_unwind(AssertUnwindSafe(|| interpreter.program(&program))) {
                    Ok(Some(value)) => println!("=> {value}"),
                    Ok(None) => {}
                    Err(payload) => eprintln!("error: {}", panic_message(&*payload)),
                }
            }
            Err(payload) => {
                let message = panic_message(&*payload);
                let incomplete = message.contains("unexpected end of input")
                    || message.contains("expected end to close");
                if incomplete {
                    // Mid-entry (an open def, if, or while) — keep reading lines.
                } else {
                    buffer.clear();
                    eprintln!("error: {message}");
                }
            }
        }
        prompt(interactive, &buffer);
    }
}

fn prompt(interactive: bool, buffer: &str) {
    if !interactive {
        return;
    }
    let prompt = if buffer.is_empty() { "pdx> " } else { "...> " };
    print!("{prompt}");
    std::io::stdout().flush().expect("failed to flush stdout");
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "unknown error".to_string()
    }
}
