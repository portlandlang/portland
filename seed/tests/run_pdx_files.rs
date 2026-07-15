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
fn fails_without_a_file_argument() {
    let output = Command::new(env!("CARGO_BIN_EXE_pdx"))
        .output()
        .expect("failed to run pdx");
    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("usage: pdx")
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
