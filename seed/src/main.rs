//! `pdx` — run a Portland file through the seed interpreter, or start a REPL.

use std::io::{BufRead, IsTerminal, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::process;

use portland_seed::interpreter::Interpreter;
use portland_seed::parser;

fn main() {
    // Parser and interpreter both recurse on the stack, and the 8 MB main
    // stack hangs (rather than crashes) on overflow under macOS. A spawned
    // thread gets a deep stack and a clean "has overflowed its stack" abort.
    let seed = std::thread::Builder::new()
        .name("portland".to_string())
        .stack_size(512 * 1024 * 1024)
        .spawn(run)
        .expect("failed to spawn the interpreter thread");
    if seed.join().is_err() {
        process::exit(1);
    }
}

fn run() {
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
    interpreter.set_arguments(std::env::args().skip(2).collect());
    interpreter.set_current_file(std::path::PathBuf::from(path));
    interpreter.program(&program);
}

/// Where the interactive REPL remembers previous entries between sessions.
fn history_path() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".pdx_history"))
}

/// Lines come from rustyline when a human is typing (history, editing,
/// Ctrl-C) and from plain stdin when piped, so scripts and tests keep
/// working unchanged.
enum Source {
    Piped(std::io::Lines<std::io::StdinLock<'static>>),
    Typed(Box<rustyline::DefaultEditor>),
}

/// One line, or why there isn't one.
enum Input {
    Line(String),
    /// Ctrl-C — abandon the entry in progress, stay in the REPL.
    Interrupt,
    /// Ctrl-D or end of piped input.
    Done,
}

impl Source {
    fn read(&mut self, buffer: &str) -> Input {
        match self {
            Source::Piped(lines) => match lines.next() {
                Some(line) => Input::Line(line.expect("failed to read stdin")),
                None => Input::Done,
            },
            Source::Typed(editor) => {
                let prompt = if buffer.is_empty() { "pdx> " } else { "...> " };
                match editor.readline(prompt) {
                    Ok(line) => {
                        // Only whole entries are worth recalling.
                        if buffer.is_empty() && !line.trim().is_empty() {
                            let _ = editor.add_history_entry(line.as_str());
                        }
                        Input::Line(line)
                    }
                    Err(rustyline::error::ReadlineError::Interrupted) => Input::Interrupt,
                    Err(_) => Input::Done,
                }
            }
        }
    }
}

fn repl() {
    let interactive = std::io::stdin().is_terminal();
    let mut source = if interactive {
        println!("Portland seed REPL — :help for commands, :quit or Ctrl-D to exit");
        match rustyline::DefaultEditor::new() {
            Ok(mut editor) => {
                if let Some(path) = history_path() {
                    let _ = editor.load_history(&path);
                }
                Source::Typed(Box::new(editor))
            }
            // No terminal to drive: fall back rather than refuse to start.
            Err(_) => Source::Piped(std::io::stdin().lock().lines()),
        }
    } else {
        Source::Piped(std::io::stdin().lock().lines())
    };
    // The seed reports errors by panicking; the REPL catches them and carries on.
    std::panic::set_hook(Box::new(|_| {}));

    // rustyline draws its own prompt; only the piped path needs ours.
    let piped = matches!(source, Source::Piped(_));
    let mut interpreter = Interpreter::new();
    let mut buffer = String::new();
    prompt(piped && interactive, &buffer);
    loop {
        let line = match source.read(&buffer) {
            Input::Line(line) => line,
            Input::Interrupt => {
                if !buffer.is_empty() {
                    println!("cancelled {} line(s)", buffer.lines().count());
                    buffer.clear();
                }
                continue;
            }
            Input::Done => break,
        };
        match repl_command(line.trim()) {
            Some(Command::Quit) => break,
            Some(Command::Help) => {
                println!("{HELP}");
                prompt(piped && interactive, &buffer);
                continue;
            }
            // An unfinished entry is otherwise inescapable: every further
            // line just extends it, and only `end` or exiting gets out.
            Some(Command::Cancel) => {
                if buffer.is_empty() {
                    println!("nothing to cancel");
                } else {
                    println!("cancelled {} line(s)", buffer.lines().count());
                    buffer.clear();
                }
                prompt(piped && interactive, &buffer);
                continue;
            }
            Some(Command::Show) => {
                if buffer.is_empty() {
                    println!("nothing buffered");
                } else {
                    print!("{buffer}");
                }
                prompt(piped && interactive, &buffer);
                continue;
            }
            None => {}
        }
        buffer.push_str(&line);
        buffer.push('\n');
        let entry = buffer.clone();
        match catch_unwind(|| parser::parse(&entry)) {
            Ok(program) => {
                buffer.clear();
                match catch_unwind(AssertUnwindSafe(|| interpreter.program(&program))) {
                    Ok(Some(value)) => {
                        println!("=> {}", value.inspect());
                        interpreter.set_last_value(value);
                    }
                    Ok(None) => {}
                    Err(payload) => eprintln!("error: {}", panic_message(&*payload)),
                }
            }
            Err(payload) => {
                let message = panic_message(&*payload);
                let incomplete = message.contains("unexpected end of input")
                    || message.contains("expected end to close")
                    || message.contains("unterminated string")
                    || message.contains("unterminated %w")
                    || message.contains("unterminated heredoc")
                    || message.contains("unterminated interpolation")
                    || message.contains("expected } to close {");
                if incomplete {
                    // Mid-entry (an open def, if, or while) — keep reading lines.
                } else {
                    buffer.clear();
                    eprintln!("error: {message}");
                }
            }
        }
        prompt(piped && interactive, &buffer);
    }
    if let Source::Typed(editor) = &mut source
        && let Some(path) = history_path()
    {
        let _ = editor.save_history(&path);
    }
}

/// REPL-only commands, kept in a `:` namespace so they can never collide
/// with Portland code — the seed has no `:` prefix syntax.
enum Command {
    Cancel,
    Help,
    Quit,
    Show,
}

const HELP: &str = "  :cancel        discard the entry in progress
  :show          print the entry in progress
  :help          this
  quit / exit    leave (:quit and Ctrl-D do too)

  Ctrl-C abandons the entry in progress; it never leaves.

  `_` holds the last value.";

fn repl_command(line: &str) -> Option<Command> {
    match line {
        ":cancel" => Some(Command::Cancel),
        ":help" => Some(Command::Help),
        ":show" => Some(Command::Show),
        // Bare `quit`/`exit` too: they are what hands actually type, and
        // neither is a Portland builtin, so nothing is shadowed.
        ":quit" | ":exit" | "quit" | "exit" | "quit()" | "exit()" => Some(Command::Quit),
        _ => None,
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
