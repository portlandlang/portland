//! End-to-end: the `pdx` binary runs real `.pdx` fixture files.

use std::process::Command;

fn run_fixture(name: &str) -> std::process::Output {
    let fixture = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    Command::new(env!("CARGO_BIN_EXE_pdx"))
        .arg(fixture)
        .output()
        .expect("failed to run pdx")
}

#[test]
fn runs_hello_pdx() {
    let output = run_fixture("hello.pdx");
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "hello world\n");
}

#[test]
fn runs_arithmetic_pdx() {
    let output = run_fixture("arithmetic.pdx");
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "42\n");
}

#[test]
fn runs_showcase_pdx() {
    let output = run_fixture("showcase.pdx");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "negative\nzero\npositive\n42\ntrue\n"
    );
}

#[test]
fn runs_fizzbuzz_pdx() {
    let output = run_fixture("fizzbuzz.pdx");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "1\n2\nFizz\n4\nBuzz\nFizz\n7\n8\nFizz\nBuzz\n11\nFizz\n13\n14\nFizzBuzz\n"
    );
}

fn run_repl(input: &str) -> std::process::Output {
    use std::io::Write;
    use std::process::Stdio;

    let mut child = Command::new(env!("CARGO_BIN_EXE_pdx"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to start pdx repl");
    child
        .stdin
        .as_mut()
        .expect("no stdin")
        .write_all(input.as_bytes())
        .expect("failed to write to repl");
    child.wait_with_output().expect("failed to run pdx repl")
}

#[test]
fn repl_evaluates_lines() {
    let output = run_repl("1 + 1\nx = 20\nx * 2 + 2\n");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "=> 2\n=> 20\n=> 42\n"
    );
}

#[test]
fn repl_buffers_multiline_definitions() {
    let output = run_repl("def double(n)\n  n * 2\nend\ndouble(21)\n");
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "=> 42\n");
}

#[test]
fn repl_reports_errors_and_continues() {
    let output = run_repl("nope\n1 + 1\n");
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "=> 2\n");
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("undefined variable nope")
    );
}

#[test]
fn fails_on_a_missing_file() {
    let output = Command::new(env!("CARGO_BIN_EXE_pdx"))
        .arg("no_such_file.pdx")
        .output()
        .expect("failed to run pdx");
    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("cannot read")
    );
}
