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
            "def f(number)\n  return 0 if number == 0\n  f(number - 1)\nend\nputs(f(5000))\n"
                .to_string(),
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

#[test]
fn fails_cleanly_when_too_deep() {
    // The depth guards must fire as clean Portland errors — on macOS 26 an
    // actual stack overflow hangs the process instead of crashing it.
    let cases = [
        (
            "too_deep_parens.pdx",
            format!("puts({}1{})\n", "(".repeat(15_000), ")".repeat(15_000)),
            "expression nesting deeper",
        ),
        (
            "runaway_recursion.pdx",
            "def f\n  f()\nend\nf()\n".to_string(),
            "call stack deeper",
        ),
    ];
    for (name, source, expected) in cases {
        let path = std::env::temp_dir().join(name);
        std::fs::write(&path, source).expect("failed to write probe file");
        let output = Command::new(env!("CARGO_BIN_EXE_pdx"))
            .arg(&path)
            .output()
            .expect("failed to run pdx");
        assert!(!output.status.success(), "{name} unexpectedly succeeded");
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains(expected), "{name} stderr: {stderr}");
    }
}

fn portland_lexer() -> String {
    format!("{}/../compiler/lexer.pdx", env!("CARGO_MANIFEST_DIR"))
}

fn portland_tokenize() -> String {
    format!("{}/../compiler/tokenize.pdx", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn require_relative_loads_once() {
    let output = run_fixture("requires_library.pdx");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "hello from the library\nfalse\n"
    );
}

#[test]
fn portland_lexer_lexes_a_sample() {
    let sample = std::env::temp_dir().join("lexer_sample.pdx");
    std::fs::write(&sample, "value = 40 + 2\nputs(\"answer #{value}!\")\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pdx"))
        .arg(portland_tokenize())
        .arg(&sample)
        .output()
        .expect("failed to run pdx");
    assert!(output.status.success());
    let expected = "identifier value\noperator =\ninteger 40\noperator +\ninteger 2\nnewline \nidentifier puts\noperator (\nstring \"answer #{value}!\"\noperator )\nnewline \n";
    assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
}

#[test]
fn portland_lexer_lexes_itself() {
    // The Stage 1 milestone in miniature: Portland tokenizing Portland.
    let output = Command::new(env!("CARGO_BIN_EXE_pdx"))
        .arg(portland_tokenize())
        .arg(portland_lexer())
        .output()
        .expect("failed to run pdx");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("keyword struct"));
    assert!(stdout.contains("identifier read_interpolation"));
    assert!(stdout.contains("keyword def"));
    let errors: Vec<&str> = stdout
        .lines()
        .filter(|line| line.starts_with("error "))
        .collect();
    assert!(errors.is_empty(), "error tokens: {errors:?}");
}

fn portland_parse() -> String {
    format!("{}/../compiler/parse.pdx", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn portland_parser_parses_integers() {
    let sample = std::env::temp_dir().join("parse_sample.pdx");
    std::fs::write(&sample, "42\n7\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pdx"))
        .arg(portland_parse())
        .arg(&sample)
        .output()
        .expect("failed to run pdx");
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "42\n7\n");
}

#[test]
fn portland_parser_climbs_the_precedence_ladder() {
    let sample = std::env::temp_dir().join("parse_ladder.pdx");
    let source = "1 + 2 * 3\n(1 + 2) * 3\n1 + 2 - 3\na && b || !c\nx == 1 + 2\n-5\n\"hi\" + name\ntrue && false\n10 % 3 == 1\n";
    std::fs::write(&sample, source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pdx"))
        .arg(portland_parse())
        .arg(&sample)
        .output()
        .expect("failed to run pdx");
    assert!(output.status.success());
    let expected = "(+ 1 (* 2 3))\n(* (+ 1 2) 3)\n(- (+ 1 2) 3)\n(|| (&& a b) (! c))\n(== x (+ 1 2))\n-5\n(+ \"hi\" name)\n(&& true false)\n(== (% 10 3) 1)\n";
    assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
}

#[test]
fn portland_parser_handles_postfix_chains() {
    let sample = std::env::temp_dir().join("parse_postfix.pdx");
    let source = "name.upcase\nlist.push(1, 2)\ngreet()\ngreet(\"pdx\", 2)\nitems[0]\nmatrix[1][2]\n\"pdx\".upcase.reverse\n-5.abs\na.b + c.d\nshout(name).length\n\"x\"\n  .upcase\n";
    std::fs::write(&sample, source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pdx"))
        .arg(portland_parse())
        .arg(&sample)
        .output()
        .expect("failed to run pdx");
    assert!(output.status.success());
    let expected = "(. name upcase)\n(. list push 1 2)\n(call greet)\n(call greet \"pdx\" 2)\n([] items 0)\n([] ([] matrix 1) 2)\n(. (. \"pdx\" upcase) reverse)\n(. -5 abs)\n(+ (. a b) (. c d))\n(. (call shout name) length)\n(. \"x\" upcase)\n";
    assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
}

#[test]
fn portland_parser_handles_statements() {
    let sample = std::env::temp_dir().join("parse_statements.pdx");
    let source = "x = 1 + 2\ntotal += 5\ncount -= 1\nreturn 42\nreturn\nbreak\nnext\nname = shout(\"hi\").length\n1 2\n";
    std::fs::write(&sample, source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pdx"))
        .arg(portland_parse())
        .arg(&sample)
        .output()
        .expect("failed to run pdx");
    assert!(output.status.success());
    let expected = "(= x (+ 1 2))\n(= total (+ total 5))\n(= count (- count 1))\n(return 42)\n(return)\n(break)\n(next)\n(= name (. (call shout \"hi\") length))\n1\n(error expected newline after statement, got 2)\n";
    assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
}

#[test]
fn portland_parser_handles_control_flow() {
    let sample = std::env::temp_dir().join("parse_control.pdx");
    let source = "if ready\n  go\nend\nif a\n  1\nelsif b\n  2\nelse\n  3\nend\nunless quiet\n  shout(\"hi\")\nend\nputs(\"hi\") if ready\nreturn if done\nwhile n > 0\n  n -= 1\nend\ncase n\nwhen 0 then \"none\"\nwhen 1, 2 then \"few\"\nelse\n  \"many\"\nend\n";
    std::fs::write(&sample, source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pdx"))
        .arg(portland_parse())
        .arg(&sample)
        .output()
        .expect("failed to run pdx");
    assert!(output.status.success());
    let expected = "(if ready (then go))\n(if a (then 1) (else (if b (then 2) (else 3))))\n(if quiet (then) (else (call shout \"hi\")))\n(if ready (then (call puts \"hi\")))\n(if done (then (return)))\n(while (> n 0) (= n (- n 1)))\n(case n (when 0 \"none\") (when 1 2 \"few\") (else \"many\"))\n";
    assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
}

#[test]
fn portland_parser_handles_definitions() {
    let sample = std::env::temp_dir().join("parse_defs.pdx");
    let source = "def pair(base, twice = base * 2)\n  base + twice\nend\ndef ready?\n  true\nend\nstruct Token\n  kind\n  text\nend\nToken.new(kind: \"integer\", text: \"42\")\ntoken.with(text: \"43\")\nlist.map do |item|\n  item * 2\nend\ncities.each do |code, city|\n  puts(city)\nend\n5.times do\n  beep\nend\n";
    std::fs::write(&sample, source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pdx"))
        .arg(portland_parse())
        .arg(&sample)
        .output()
        .expect("failed to run pdx");
    assert!(output.status.success());
    let expected = "(def pair (params base (= twice (* base 2))) (+ base twice))\n(def ready? (params) true)\n(struct Token kind text)\n(. Token new (: kind \"integer\") (: text \"42\"))\n(. token with (: text \"43\"))\n(. list map (do |item| (* item 2)))\n(. cities each (do |code city| (call puts city)))\n(. 5 times (do beep))\n";
    assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
}

#[test]
fn portland_parser_handles_commands_and_literals() {
    let sample = std::env::temp_dir().join("parse_commands.pdx");
    let source = "puts \"hello\"\nshout word\nputs 1 + 2\nrequire_relative \"lexer\"\nfoo - 1\ntokens = []\npairs = {\"a\" => 1}\nwords = %w[rose city]\nputs -1\nputs [1]\nputs (1)\n";
    std::fs::write(&sample, source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pdx"))
        .arg(portland_parse())
        .arg(&sample)
        .output()
        .expect("failed to run pdx");
    assert!(output.status.success());
    let expected = "(call puts \"hello\")\n(call shout word)\n(call puts (+ 1 2))\n(call require_relative \"lexer\")\n(- foo 1)\n(= tokens (array))\n(= pairs (hash (=> \"a\" 1)))\n(= words %w[rose city])\n(error ambiguous without parens — write puts(-1) or puts - 1)\n(error ambiguous without parens — write puts([...]) to pass an array or puts[...] to index)\n(error ambiguous without parens — write puts(...) with no space to call)\n";
    assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
}

#[test]
fn portland_parser_parses_the_whole_compiler_including_itself() {
    // The summit of #18: Portland parsing Portland, all of it.
    for file in ["lexer.pdx", "tokenize.pdx", "parse.pdx", "parser.pdx"] {
        let target = format!("{}/../compiler/{file}", env!("CARGO_MANIFEST_DIR"));
        let output = Command::new(env!("CARGO_BIN_EXE_pdx"))
            .arg(portland_parse())
            .arg(&target)
            .output()
            .expect("failed to run pdx");
        assert!(output.status.success(), "{file} did not parse");
        let stdout = String::from_utf8(output.stdout).unwrap();
        // `"(error` is the sexp printer's own string literal; a real error
        // node prints unquoted.
        let real_errors = stdout.matches("(error ").count() - stdout.matches("\"(error ").count();
        assert_eq!(real_errors, 0, "{file} produced error nodes");
        assert!(!stdout.is_empty(), "{file} produced no output");
    }
}

#[test]
fn portland_parser_reports_error_nodes() {
    let sample = std::env::temp_dir().join("parse_error_sample.pdx");
    std::fs::write(&sample, "]\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pdx"))
        .arg(portland_parse())
        .arg(&sample)
        .output()
        .expect("failed to run pdx");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "(error unexpected operator ])\n"
    );
}

fn portland_run() -> String {
    format!("{}/../compiler/run.pdx", env!("CARGO_MANIFEST_DIR"))
}

// Differential harness: the Portland-on-Portland evaluator must produce
// byte-identical output to the seed running the same source directly.
fn assert_evaluator_matches_seed(name: &str, source: &str) {
    let sample = std::env::temp_dir().join(name);
    std::fs::write(&sample, source).unwrap();
    let direct = Command::new(env!("CARGO_BIN_EXE_pdx"))
        .arg(&sample)
        .output()
        .expect("failed to run pdx");
    let hosted = Command::new(env!("CARGO_BIN_EXE_pdx"))
        .arg(portland_run())
        .arg(&sample)
        .output()
        .expect("failed to run pdx");
    assert!(
        direct.status.success() && hosted.status.success(),
        "{name} failed to run"
    );
    assert_eq!(
        String::from_utf8(direct.stdout).unwrap(),
        String::from_utf8(hosted.stdout).unwrap(),
        "{name} diverged from the seed"
    );
}

#[test]
fn portland_evaluator_matches_the_seed_on_literals() {
    assert_evaluator_matches_seed(
        "eval_rung0.pdx",
        "puts 42\nputs \"rose city\"\nputs \"line\\nbreak\"\nputs true\nputs\n",
    );
}

#[test]
fn portland_evaluator_matches_the_seed_on_operators() {
    assert_evaluator_matches_seed(
        "eval_rung1.pdx",
        "puts 1 + 2 * 3\nputs((1 + 2) * 3)\nputs 10 % 3\nputs 7 / 2\nputs 1 + 1 == 2\nputs 3 > 2 && 2 > 1\nputs false || true\nputs !false\nputs(-5)\nputs \"port\" + \"land\"\nputs 10 - 2 - 3\n",
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
    let output = run_repl("1 + 1\nvalue = 20\nvalue * 2 + 2\n");
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
    let output = run_repl("def double(number)\n  number * 2\nend\ndouble(21)\n");
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "=> 42\n");
}

#[test]
fn repl_buffers_multiline_strings() {
    let output = run_repl("value = \"port\nland\"\nvalue.length\n");
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
            .contains("undefined variable or method nope")
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
