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

#[test]
fn runs_blocks_pdx() {
    let output = run_fixture("blocks.pdx");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "PORTLAND\nSALEM\nEUGENE\n8, 5, 6\nGO! BLAZERS!\n"
    );
}

#[test]
fn runs_tour_pdx() {
    let output = run_fixture("tour.pdx");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "hello, portland!\nhello, stranger\nodd sum: 25\nPDX is portland\n1 + 4 + 9 = 14\n"
    );
}

#[test]
fn runs_mini_lexer_pdx() {
    let output = run_fixture("mini_lexer.pdx");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "identifier: total\npunctuation: =\nidentifier: compute\npunctuation: (\ninteger: 40\npunctuation: ,\ninteger: 2\npunctuation: )\npunctuation: +\ninteger: 1\n"
    );
}

#[test]
fn runs_word_count_pdx_with_argv() {
    let output = Command::new(env!("CARGO_BIN_EXE_pdx"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .arg("tests/fixtures/word_count.pdx")
        .arg("tests/fixtures/haiku.txt")
        .output()
        .expect("failed to run pdx");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "3 lines, 12 words: tests/fixtures/haiku.txt\n"
    );
}

#[test]
fn survives_deep_nesting_and_recursion() {
    // Regression guard for the deep-stack interpreter thread: these depths
    // hang-on-overflow with a default 8 MB main stack.
    let cases = [
        (
            "deep_parens.pdx",
            format!("puts({}1{})\n", "(".repeat(5_000), ")".repeat(5_000)),
        ),
        (
            "deep_recursion.pdx",
            "def f(n)\n  return 0 if n == 0\n  f(n - 1)\nend\nputs(f(5000))\n".to_string(),
        ),
    ];
    for (name, source) in cases {
        let path = std::env::temp_dir().join(name);
        std::fs::write(&path, source).expect("failed to write probe file");
        let output = Command::new(env!("CARGO_BIN_EXE_pdx"))
            .arg(&path)
            .output()
            .expect("failed to run pdx");
        assert!(output.status.success(), "{name} did not succeed");
        let stdout = String::from_utf8(output.stdout).unwrap();
        let expected = if name == "deep_parens.pdx" {
            "1\n"
        } else {
            "0\n"
        };
        assert_eq!(stdout, expected, "{name} output mismatch");
    }
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
fn repl_inspects_string_results() {
    let output = run_repl("\"port\" + \"land\"\n");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "=> \"portland\"\n"
    );
}

#[test]
fn repl_buffers_multiline_definitions() {
    let output = run_repl("def double(n)\n  n * 2\nend\ndouble(21)\n");
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "=> 42\n");
}

#[test]
fn repl_buffers_multiline_strings() {
    let output = run_repl("x = \"port\nland\"\nx.length\n");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "=> \"port\\nland\"\n=> 9\n"
    );
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
