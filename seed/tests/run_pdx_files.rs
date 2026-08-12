//! End-to-end: the `pdx` binary runs real `.pdx` fixture files.

use std::process::Command;

/// A coarse wall-clock tripwire (#32), not a benchmark (#25 is that).
///
/// `parses_the_whole_compiler_including_itself` once grew to 31.5s of a 33s
/// suite without anything noticing: `parser.pdx` got longer, `<<` was
/// quadratic, and no single commit looked slow. Nothing in the suite carried
/// a performance signal, so a test could get 10× slower and stay green.
///
/// The ceiling is deliberately loose. It exists to catch the *next* accidental
/// quadratic — which announces itself in multiples, not percentages — and a
/// tripwire that flakes gets deleted, at which point there is no signal at all.
///
/// Calibration, measured rather than guessed: this case runs in ~6s locally
/// and took 32.7s on the `macos-26` runner *before* the RC-exact append fix
/// (#34), when it cost 29.7s locally. So CI is within ~10% of a dev machine
/// here, and a 20s ceiling is roughly 3× headroom that still would have fired
/// on the regression that prompted this.
fn within_seconds<T>(limit: u64, label: &str, work: impl FnOnce() -> T) -> T {
    let started = std::time::Instant::now();
    let result = work();
    let elapsed = started.elapsed();

    assert!(
        elapsed.as_secs() < limit,
        "{label} took {elapsed:.1?}, over the {limit}s tripwire.\n\
         This is a coarse ceiling, so being near it means something got much \
         slower — look for an accidental quadratic before raising the number."
    );
    result
}

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
        "PORTLAND\nSALEM\nEUGENE\n8, 5, 6\nGO! BLAZERS!\n\
         a string argument\nthe block ran\n\
         an argument\nstill the outer call's block\n\
         brace on a paren-less call\nbrace on a paren-less call\n\
         brace on a parenthesized call\nbrace on a parenthesized call\n\
         hi pdx\nhi pdx\n"
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
fn runs_optionals_pdx() {
    // Living documentation for ADRs 0005–0010. Direct-run only: the hosted
    // evaluator can't index guest hashes yet (pair-list gap, #10), so the
    // optionals differential lives in its own hash-free test below.
    let output = run_fixture("optionals.pdx");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "teal\n30\nsome(nil)\nnil\nPORTLAND!\n...\n0\n9\nFRIEND\ntrue\ntrue\n"
    );
}

#[test]
fn runs_patterns_pdx() {
    let output = run_fixture("patterns.pdx");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "int 42\nplus\nabsent\n1 then 2 more\nover by 40\n"
    );
}

#[test]
fn runs_namespaces_pdx() {
    let output = run_fixture("namespaces.pdx");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "2\n10\n2\n4\n6\n107\nwidget 9\n42\n"
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

/// Once per file, not once per spelling: the three `false`s are a second
/// require of the same library written three ways, and a path that resolves
/// to a file already loaded answers false without running it again.
#[test]
fn require_relative_loads_once() {
    let output = run_fixture("requires_library.pdx");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "hello from the library\nfalse\nfalse\nfalse\n"
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

#[test]
fn portland_lexer_lexes_the_optionals_tokens() {
    let sample = std::env::temp_dir().join("lex_optionals.pdx");
    std::fs::write(
        &sample,
        "x = nil or fallback\nuser&.name\nnot done and ready?\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pdx"))
        .arg(portland_tokenize())
        .arg(&sample)
        .output()
        .expect("failed to run pdx");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("keyword nil"), "{stdout}");
    assert!(stdout.contains("keyword or"), "{stdout}");
    assert!(stdout.contains("keyword and"), "{stdout}");
    assert!(stdout.contains("keyword not"), "{stdout}");
    assert!(stdout.contains("operator &."), "{stdout}");
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
fn portland_parser_parses_the_optionals_surface() {
    let sample = std::env::temp_dir().join("parse_optionals.pdx");
    let source = "x = nil or 7\nuser&.upcase\nvalue = fetch() or return 0\ndone = fetch() or return\nflag = fetch() or break\ntotal = fetch() or next\nrow = fetch() or panic \"gone\"\nnot true\na and b\n";
    std::fs::write(&sample, source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pdx"))
        .arg(portland_parse())
        .arg(&sample)
        .output()
        .expect("failed to run pdx");
    assert!(output.status.success());
    // Word forms render as their sigils — dead-identical spellings collapse
    // in the tree itself (ADR 0007).
    let expected = "(= x (|| nil 7))\n(&. user upcase)\n(= value (|| (call fetch) (return 0)))\n(= done (|| (call fetch) (return)))\n(= flag (|| (call fetch) (break)))\n(= total (|| (call fetch) (next)))\n(= row (|| (call fetch) (call panic \"gone\")))\n(! true)\n(&& a b)\n";
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
    // The summit of #18: Portland parsing Portland, all of it. Also the
    // slowest thing in the suite by a wide margin, so it carries the tripwire.
    for file in ["lexer.pdx", "tokenize.pdx", "parse.pdx", "parser.pdx"] {
        let target = format!("{}/../compiler/{file}", env!("CARGO_MANIFEST_DIR"));
        let output = within_seconds(20, &format!("parsing {file}"), || {
            Command::new(env!("CARGO_BIN_EXE_pdx"))
                .arg(portland_parse())
                .arg(&target)
                .output()
                .expect("failed to run pdx")
        });
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
    // Hosted runs pay the trio's cost on top of the seed's, so this is where
    // a quadratic shows up first.
    let hosted = within_seconds(20, name, || {
        Command::new(env!("CARGO_BIN_EXE_pdx"))
            .arg(portland_run())
            .arg(&sample)
            .output()
            .expect("failed to run pdx")
    });
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
fn portland_evaluator_matches_the_seed_on_optionals() {
    assert_evaluator_matches_seed(
        "evaluator_optionals.pdx",
        "p nil\np nil == nil\np nil.nil?\np 5.some?\nx = nil or 7\nputs x\ndef bump(value)\n  found = value or return 0\n  found + 1\nend\nputs bump(41)\nputs bump(nil)\np([].first)\np([nil].first)\np([1, 2][9])\np([1, 2][-1])\np([].min)\ntheme = [].first or \"grey\"\nputs theme\nname = [].first&.upcase or \"FRIEND\"\nputs name\np some(nil)\np some(5)\nkept = 5 or panic \"unreachable\"\nputs kept\n",
    );
}

/// Where the trio can diagnose, it must say exactly what the seed says.
/// These four used to reach the evaluator as an unhelpful "cannot
/// evaluate error yet"; now the message survives to stderr verbatim.
#[test]
fn portland_evaluator_reports_the_seed_wording_on_errors() {
    let cases = [
        (
            "module S\n  def m(v)\n    1\n  end\nend\nputs S::m([1])\n",
            "`::` names, `.` invokes",
        ),
        (
            "struct Foo\n  bar\n  module H\n  end\nend\n",
            "modules don't nest inside structs",
        ),
        (
            "module stats\nend\n",
            "module names start with a capital letter",
        ),
        ("p([\"a\"].map { |w| it.upcase })\n", "use one or the other"),
        // The brace menu, both widths — the peek that drops the hash
        // reading has to agree too.
        (
            "def render(x)\n  x\nend\ndef config\n  1\nend\nrender config { \"a\" => 1 }\n",
            "could be three things",
        ),
        (
            "def render(x)\n  x\nend\ndef config\n  1\nend\nrender config { |item| item }\n",
            "is a block — but whose?",
        ),
        // A shorthand key is a hash too (ADR 0023), so the menu keeps all
        // three readings. Both oracles' peeks looked only for `=>` and dropped
        // one, which is picking rather than trimming.
        (
            "def render(x)\n  x\nend\ndef config\n  1\nend\nrender config {name: 1}\n",
            "could be three things",
        ),
        // A `{` directly after the name, where there is no argument for the
        // block to belong to instead. The trio used to crash on this rather
        // than diagnose it — `IdentifierNode has no field value`.
        //
        // ADR 0024 accepts the one-reading block forms, so the refusals that
        // remain are the genuinely two-reading ones — `{}` is one, an empty
        // hash or an empty block.
        (
            "def expecting(v)\n  v\nend\nexpecting {}\n",
            "could be two things",
        ),
        // And the one-reading *hash* form is told, not asked: a label cannot
        // start a statement, so there is no block body to weigh against it.
        (
            "def expecting(v)\n  v\nend\nexpecting {name: 1}\n",
            "is a hash, not a block",
        ),
        // ADR 0022's runtime refusals (#45): the trio used to construct all
        // three of these happily where the seed refuses.
        ("paid = :paid(on: 1)\n", "no enum declares a case :paid"),
        (
            "enum Status\n  :paid(on:)\nend\npaid = :paid(wrong: 1)\n",
            "`:paid` takes (on:)",
        ),
        (
            "enum A\n  :hit(x:)\nend\nenum B\n  :hit(y:)\nend\nv = :hit(x: 1)\n",
            "two enums declare :hit with different payloads",
        ),
        // ADR 0001's refusals (#45): the trio used to run all six of these.
        (
            "x = 1\nx = 2\n",
            "x is immutable — declare it `mutable x = ...`",
        ),
        (
            "mutable x = 1\nmutable x = 2\n",
            "x is already declared — mutable declares a new name once",
        ),
        (
            "def foo\n  1\nend\nfoo = 1\n",
            "local foo shadows method foo — rename one",
        ),
        ("puts = 1\n", "local puts shadows method puts — rename one"),
        (
            "hit = 5 in first\nagain = 6 in first\n",
            "first is immutable",
        ),
        (
            "it = 1\n[2].each { puts it }\n",
            "`it` is a local here and a block parameter there — rename one",
        ),
        // ADR 0027's refusals: `!` marks call sites only, and a failure
        // demands handling before use.
        (
            "def save!\n  1\nend\n",
            "`!` is unwrap-or-propagate and belongs to call sites — define save and write save! where its failure should propagate",
        ),
        (
            "content! = 1\n",
            "`!` is unwrap-or-propagate — a binding cannot take it; name it content",
        ),
        (
            "sad = failure(\"why\")\nputs sad.upcase\n",
            "a failure has no method upcase — handle the failure case first",
        ),
        (
            "sad = failure(\"why\")\nputs sad\n",
            "puts got a failure — handle the failure case first (p renders it for debugging)",
        ),
        // ADR 0028's refusals: every collision names both owners, and only
        // a trait can be included.
        (
            "trait A\n  def hit\n    1\n  end\nend\ntrait B\n  def hit\n    2\n  end\nend\nstruct S\n  x\n  include A\n  include B\nend\n",
            "hit is declared by both A and B — S must not include both, or one renames",
        ),
        (
            "trait A\n  def probe\n    1\n  end\nend\nstruct S\n  x\n  include A\n  def probe\n    2\n  end\nend\n",
            "S defines probe and includes it from A — one of them renames",
        ),
        (
            "trait A\n  def x\n    1\n  end\nend\nstruct S\n  x\n  include A\nend\n",
            "x is a field of S and a method of A — a name is a field or a method, never both",
        ),
        (
            "module M\n  def helper\n    1\n  end\nend\nstruct S\n  x\n  include M\nend\n",
            "M is a namespace, not a trait — namespaces are never injected",
        ),
        (
            "struct T\n  y\nend\nstruct S\n  x\n  include T\nend\n",
            "T is a struct, not a trait — a struct is a value; hold one in a field",
        ),
        (
            "struct S\n  x\n  include Missing\nend\n",
            "undefined trait Missing",
        ),
        // ADR 0029's refusals: independence is enforced, not hoped for.
        (
            "together do\n  ~ x = 1\n  ~ x = 2\nend\n",
            "two tasks bind x — every task line binds its own name",
        ),
        (
            "~ x = 1\n",
            "a task line belongs inside `together` — every task lives in the block that joins it",
        ),
        (
            "together do\n  ~ 1 + 1\nend\n",
            "a task line binds a name — write ~ name = ...",
        ),
        (
            "mutable total = 0\ntogether do\n  ~ x = [1, 2].map { total += it }\nend\n",
            "a task cannot rebind an outer mutable — bind a name and combine after the join",
        ),
        (
            "def risky\n  failure(\"down\")\nend\ndef f\n  together do\n    ~ x = risky!\n  end\nend\nf\n",
            "a task cannot unwind across the join — bind a name and handle it after end",
        ),
        // ADR 0032 (#68): the never-guess shapes after a dot call's name.
        (
            "p \"x\".slice -1\n",
            "ambiguous without parens — write slice(-1) or slice - 1",
        ),
        (
            "numbers = [1]\np numbers.slice [0]\n",
            "ambiguous without parens — write slice([...]) to pass an array or slice[...] to index",
        ),
        (
            "p \"abc\".slice (0)\n",
            "ambiguous without parens — write slice(...) with no space to call",
        ),
        // ADR 0033: the chained-literal corner and the two integer edges.
        (
            "x = -2.abs ** 2\n",
            "a chained negative literal under ** is ambiguous — write ((-2).abs) ** 2 or -(2.abs ** 2)",
        ),
        (
            "x = 2 ** -1\n",
            "2 ** -1 is a fraction, and integers have none — write 2.0 ** -1 for the float",
        ),
        ("x = 2 ** 100\n", "2 ** 100 overflows the 64-bit integers"),
        // ADR 0031: type functions, definable `new`, and `fields`.
        (
            "fields(x: 1)\n",
            "`fields` is the raw constructor — it only exists inside def self.new",
        ),
        (
            "def self.of\n  1\nend\n",
            "`def self.` belongs inside a struct body — write def of",
        ),
        (
            "trait Sexp\n  def self.of\n    1\n  end\nend\n",
            "a trait cannot define def self.of — a type function lives on the struct that includes it",
        ),
        (
            "struct Token\n  kind\n\n  def greet\n    kind\n  end\n\n  def self.greet\n    1\n  end\nend\n",
            "greet is defined on Token instances and on the type — rename one",
        ),
        (
            "def fields\n  1\nend\nstruct Token\n  kind\n\n  def self.new(raw)\n    fields(kind: raw)\n  end\nend\nToken.new(\"rose\")\n",
            "fields is the raw constructor inside def self.new — rename the fields function",
        ),
        (
            "struct Token\n  kind\n\n  def self.new(raw)\n    fields(raw)\n  end\nend\nToken.new(\"rose\")\n",
            "fields takes keyword arguments, not positional ones",
        ),
        (
            "def helper\n  fields(kind: \"word\")\nend\nstruct Token\n  kind\n\n  def self.new(raw)\n    helper\n  end\nend\nToken.new(\"rose\")\n",
            "`fields` is the raw constructor — it only exists inside def self.new",
        ),
        (
            "struct Token\n  kind\n\n  def self.with\n    1\n  end\nend\n",
            "with is reserved on structs",
        ),
    ];
    for (source, expected) in cases {
        let sample = std::env::temp_dir().join("trio_error_case.pdx");
        std::fs::write(&sample, source).expect("failed to write probe file");
        let hosted = Command::new(env!("CARGO_BIN_EXE_pdx"))
            .arg(portland_run())
            .arg(&sample)
            .output()
            .expect("failed to run pdx");
        let stderr = String::from_utf8(hosted.stderr).unwrap();
        assert!(
            stderr.contains(expected),
            "trio should report {expected:?}, got: {stderr}"
        );
    }
}

/// ADR 0021 threaded through the trio: namespaces, `::` paths, lexical
/// resolution outward, both declaration forms, and types nesting in types.
#[test]
fn portland_evaluator_matches_the_seed_on_namespaces() {
    assert_evaluator_matches_seed(
        "evaluator_namespaces.pdx",
        "module Statistics\n  LIMIT = 10\n\n  struct Summary\n    mean\n    median\n  end\n\n  def mean(values)\n    values.sum / values.length\n  end\n\n  def describe(values)\n    Summary.new(mean: mean(values), median: 0)\n  end\nend\n\nmodule Portland\n  SCALE = 2\n\n  module Compiler\n    struct Token\n      kind\n    end\n\n    def scaled\n      SCALE * 3\n    end\n  end\nend\n\nmodule Outer::Inner\n  struct Thing\n    value\n  end\n\n  def make\n    Thing.new(value: 7)\n  end\nend\n\nstruct Invoice\n  total\n\n  struct Line\n    amount\n  end\nend\n\np(Statistics.mean([1, 2, 3]))\np(Statistics::LIMIT)\np(Statistics::Summary.new(mean: 2, median: 1).mean)\np(Statistics.describe([2, 4, 6]).mean)\np(Portland::Compiler::Token.new(kind: \"integer\").kind)\np(Portland::Compiler.scaled)\np(Outer::Inner::Thing.new(value: 3).value)\np(Outer::Inner.make.value)\np(Invoice::Line.new(amount: 9).amount)\n",
    );
}

/// ADRs 0016 + 0017 threaded through the trio: brace blocks are
/// dead-identical to `do ... end`, and `it` is the implicit parameter.
#[test]
fn portland_evaluator_matches_the_seed_on_brace_blocks_and_it() {
    assert_evaluator_matches_seed(
        "evaluator_brace_blocks.pdx",
        "p([\"a\", \"b\"].map { \"x #{it}\" })\np([1].map { \"#{\"deep #{it}\"}\" })\np([1, 2].map { |number| number * 2 })\np([\"a\", \"b\"].map { |word| word.upcase }.join(\"-\"))\np([1, 2].map { it * 2 })\np([\"a\", \"b\"].map { it.upcase }.join(\"-\"))\np([1, 2, 3].select { it.odd? })\np([[1, 2]].map { |pair| pair.map { it * 2 } })\np([1, 2].map do |number|\n  number * 2\nend)\np([1, 2, 3].select do\n  it.odd?\nend)\np({\"a\" => 1}.length)\np([1, 2].map { |number|\n  doubled = number * 2\n  doubled + 1\n})\n",
    );
}

/// ADR 0020 threaded through the trio: every heredoc form, byte-identical.
#[test]
fn portland_evaluator_matches_the_seed_on_heredocs() {
    assert_evaluator_matches_seed(
        "evaluator_heredocs.pdx",
        "db = <<~SQL\n  select *\n    from orders\nSQL\np(db)\nname = \"pdx\"\np(<<~TEXT)\n  hello, #{name}\nTEXT\np(<<~'TEXT')\n  literal #{name}\nTEXT\np(<<~TEXT.upcase)\n  shout\nTEXT\ndef show\n  inner = <<~TEXT\n    indented\n      deeper\n  TEXT\n  inner\nend\np(show)\nmutable list = [1]\nlist << 2\np(list)\n",
    );
}

/// ADR 0018 + 0019 threaded through the trio: float and range literals
/// lex, parse, and evaluate to the same host values the seed produces.
#[test]
fn portland_evaluator_matches_the_seed_on_floats_and_ranges() {
    assert_evaluator_matches_seed(
        "evaluator_floats_ranges.pdx",
        "p 2.5\np 2.5.to_s\nputs 2.5.to_i\np(2.5)\np(1.0)\np(2.5 + 1)\np(7 / 2.0)\np(7 / 2)\np(1.0 == 1)\np(2.9.to_i)\np(\"2.5\".to_f)\np(1..3)\np(1...3)\np((1..3).to_a)\np((1...4).to_a)\np((1..5).include?(3))\np((1..4).sum)\np([1, 2, 3][1..])\np([1, 2, 3][4..])\np([1, 2, 3][..1])\np(\"hello\"[1..3])\np((1..3).map do |number|\n  number * 2\nend)\n",
    );
}

/// ADR 0030: `%w[]` takes Ruby's content rules — escaped and balanced
/// brackets, escaped whitespace joining a word, other escapes keeping
/// their backslash, whitespace runs splitting once — on both oracles.
#[test]
fn portland_evaluator_matches_the_seed_on_word_arrays() {
    assert_evaluator_matches_seed(
        "evaluator_word_arrays.pdx",
        "p %w[rose city]\np %w[]\np %w[a \\] b]\np %w[a [b] c]\np %w[[nested [deep]] x]\np %w[a\\ b c]\np %w[a\\nb a\\\\b]\np %w[a  b\tc]\np %w[one\ntwo]\nputs %w[\\] ) } ,].join(\"-\")\nputs %w[a \\] b].length\n",
    );
}

/// ADR 0031 threaded through the trio: type functions, sibling bare calls,
/// definable `new` with `fields` beneath, positional signatures, the
/// failure path, module-body `def self.` as a plain def, and raw `new` on
/// a plain struct untouched.
#[test]
fn portland_evaluator_matches_the_seed_on_type_functions() {
    assert_evaluator_matches_seed(
        "evaluator_type_functions.pdx",
        "struct Token\n  kind\n  text\n\n  def self.of(raw)\n    Token.new(kind: \"word\", text: raw)\n  end\n\n  def self.fallback\n    \"blank\"\n  end\n\n  def self.labeled(raw)\n    return Token.new(kind: fallback, text: \"\") if raw.empty?\n    Token.of(raw)\n  end\nend\n\np Token.of(\"rose\")\np Token.labeled(\"\")\np Token.labeled(\"city\")\n\nstruct Badge\n  label\n\n  def self.new(raw)\n    return failure(\"a badge needs text\") if raw.empty?\n    fields(label: raw)\n  end\nend\n\np Badge.new(\"rose\")\np Badge.new(\"\")\nbadge = Badge.new(\"\") or Badge.new(\"fallback\")\nputs badge.label\n\nmodule Statistics\n  def self.mean(values)\n    values.sum / values.length\n  end\nend\nputs Statistics.mean([2, 4, 6])\n\nstruct Point\n  x\nend\np Point.new(x: 41)\np Point.new(x: 41).with(x: 42)\n",
    );
}

/// ADR 0032 (#68): dot calls take paren-less arguments — the rspec chain
/// (`expect(x).to eq(y)`), multi-argument and keyword forms, recursive
/// nesting, a `do` block after the arguments, and subtraction staying
/// subtraction.
#[test]
fn portland_evaluator_matches_the_seed_on_dotted_commands() {
    assert_evaluator_matches_seed(
        "evaluator_dotted_commands.pdx",
        "def eq(value)\n  value * 10\nend\nstruct Wrap\n  actual\n\n  def to(matcher)\n    matcher + actual\n  end\nend\np Wrap.new(actual: 1).to eq(4)\np Wrap.new(actual: 5).to 7\np Wrap.new(actual: 2).to Wrap.new(actual: 3).to eq(1)\nputs %w[rose city].join \"-\"\nputs \"portland\".slice 0, 4\nstruct Token\n  kind\n  text\nend\np Token.new(kind: \"word\", text: \"42\").with text: \"43\"\nfolded = [1, 2, 3].reduce 10 do |total, number|\n  total + number\nend\np folded\nfive = 6 - 1\np five\np Wrap.new(actual: 1).to 5 - 1\n",
    );
}

/// ADR 0033: `**` and `pow` — right-associativity, precedence above `*` and
/// below a leading minus (`-2 ** 2` is `-4`, Ruby's answer), floats through
/// `powf`, the fused float literal, and the magnitude-one towers that must
/// not hit the width check.
#[test]
fn portland_evaluator_matches_the_seed_on_exponents() {
    assert_evaluator_matches_seed(
        "evaluator_exponents.pdx",
        "p 2 ** 8\np 2 ** 3 ** 2\np 4 * 2 ** 3\np 2 ** 0\np 0 ** 0\np 9 ** 0.5\np 2.0 ** -1\np(-2 ** 2)\nx = 3\np(-x ** 2)\np(-(2) ** 2)\np((-2) ** 2)\np(-2 ** 2 * 3)\ny = 2.0\np 2 ** -y\np(-2.5 ** 2)\np(-2.5.abs)\np((-1) ** 9)\np 0 ** 99999999999999\np 2.pow(8)\np 2.pow(0.5)\np 6.25.pow(0.5)\n",
    );
}

/// The checker (#9, ADR 0034): the first place the trio overtakes the seed.
/// Each case is a program the seed runs to completion — the offending line
/// is dead code — and the trio refuses at build. Nothing to compare, and
/// that is the point: these expectations are hand-written against the ADR,
/// the first tests in this suite that are, because no oracle has a wording
/// for a refusal the seed cannot make (ADR 0034 §3).
#[test]
fn the_checker_refuses_what_the_seed_cannot_see() {
    let cases = [
        (
            // A dead-branch construction with a wrong label: the seed prints
            // and exits zero; the checker refuses with the seed's own
            // *runtime* wording, moved to build time (ADR 0034 §2).
            "enum Status\n  :pending\n  :paid(on:)\nend\nif false\n  x = :paid(whenever: \"never\")\nend\nputs \"reached the end\"\n",
            "reached the end\n",
            "`:paid` takes (on:)",
        ),
        (
            "enum Status\n  :pending\nend\nif false\n  x = :shipped(on: \"never\")\nend\nputs \"reached the end\"\n",
            "reached the end\n",
            "no enum declares a case :shipped",
        ),
        (
            // The pattern typo the seed answers silently — the branch just
            // never matches. The trio's first original diagnostic.
            "enum Status\n  :pending\n  :paid(on:)\nend\ndef label(s)\n  case s\n  in :payed(on:) then \"typo\"\n  else\n    \"fine\"\n  end\nend\nputs label(:pending)\n",
            "fine\n",
            "in :payed can never match — no enum declares a case :payed",
        ),
        (
            "enum A\n  :hit(x:)\nend\nenum B\n  :hit(y:)\nend\nif false\n  z = :hit(x: 1)\nend\nputs \"reached the end\"\n",
            "reached the end\n",
            "two enums declare :hit with different payloads — the seed cannot tell them apart",
        ),
    ];
    for (source, seed_output, refusal) in cases {
        let sample = std::env::temp_dir().join("checker_case.pdx");
        std::fs::write(&sample, source).unwrap();
        let direct = Command::new(env!("CARGO_BIN_EXE_pdx"))
            .arg(&sample)
            .output()
            .expect("failed to run pdx");
        assert!(direct.status.success(), "the seed should run this program");
        assert_eq!(
            String::from_utf8(direct.stdout).unwrap(),
            seed_output,
            "the seed's run changed"
        );
        let hosted = Command::new(env!("CARGO_BIN_EXE_pdx"))
            .arg(portland_run())
            .arg(&sample)
            .output()
            .expect("failed to run pdx");
        assert!(
            !hosted.status.success(),
            "the checker should refuse this program"
        );
        let stderr = String::from_utf8(hosted.stderr).unwrap();
        assert!(
            stderr.contains(refusal),
            "the checker should say {refusal:?}, got: {stderr}"
        );
    }
}

/// `check.pdx` — the checker's own door: silence-then-ok on a clean
/// program, nothing evaluated.
#[test]
fn portland_check_passes_a_clean_program_without_running_it() {
    let sample = std::env::temp_dir().join("checker_clean.pdx");
    std::fs::write(
        &sample,
        "enum Status\n  :paid(on:)\nend\nputs \"side effect\"\n",
    )
    .unwrap();
    let checked = Command::new(env!("CARGO_BIN_EXE_pdx"))
        .arg(format!(
            "{}/../compiler/check.pdx",
            env!("CARGO_MANIFEST_DIR")
        ))
        .arg(&sample)
        .output()
        .expect("failed to run pdx");
    assert!(checked.status.success());
    assert_eq!(
        String::from_utf8(checked.stdout).unwrap(),
        "ok\n",
        "check must not evaluate the program"
    );
}

/// ADR 0018: the trio delegates `/` and `%` to the host, so Ruby's
/// floored semantics must reach hosted programs unchanged.
#[test]
fn portland_evaluator_matches_the_seed_on_floored_division() {
    assert_evaluator_matches_seed(
        "evaluator_floored_division.pdx",
        "p(-7 / 2)\np(7 / -2)\np(-7 / -2)\np(-6 / 2)\np(-7 % 2)\np(7 % -2)\np(-7 % -2)\np(-6 % 2)\np(7 / 2)\np(10 % 3)\n",
    );
}

#[test]
fn portland_evaluator_matches_the_seed_on_branchless_branches() {
    assert_evaluator_matches_seed(
        "evaluator_branchless.pdx",
        "x = if false\n  5\nend\np x\ny = if true\nelse\n  5\nend\np y\ngreeting = if false\n  \"gm\"\nend\nputs greeting or \"hello\"\n",
    );
}

#[test]
fn portland_evaluator_matches_the_seed_on_keyword_arguments() {
    assert_evaluator_matches_seed(
        "evaluator_kwargs.pdx",
        "def greet(name:, greeting: \"hi\")\n  \"#{greeting} #{name}\"\nend\nputs greet(name: \"pdx\")\nputs greet(greeting: \"yo\", name: \"pdx\")\ndef tag(word, separator: \"-\")\n  word + separator + word\nend\nputs tag(\"go\")\nputs tag(\"go\", separator: \"+\")\ndef shout(word:)\n  puts word.upcase\nend\nshout word: \"pdx\"\n",
    );
}

#[test]
fn portland_evaluator_matches_the_seed_on_case_in() {
    assert_evaluator_matches_seed(
        "evaluator_case_in.pdx",
        "struct ReturnNode\n  value\nend\nstruct BreakNode\n  label\nend\nnode = ReturnNode.new(value: nil)\ncase node\nin ReturnNode(value: nil) then puts \"(return)\"\nin ReturnNode(value:) then puts value\nend\nother = ReturnNode.new(value: 7)\ncase other\nin ReturnNode(value: nil) then puts \"(return)\"\nin ReturnNode(value:) then puts value\nend\ncase BreakNode.new(label: \"b\")\nin ReturnNode then puts \"return\"\nin BreakNode(label:) then puts label\nend\ncase 2\nin 1 | 2 then puts \"few\"\nelse\n  puts \"many\"\nend\nexpected = 5\ncase 5\nin ^expected then puts \"pinned\"\nelse\n  puts \"no\"\nend\ncase 50\nin score if score > 10 then puts \"big\"\nin score then puts \"small\"\nend\ncase [1, 2, 3]\nin [first, *rest] then puts first + rest.length\nend\ncase [].first\nin nil then puts \"empty\"\nin x then puts x\nend\n",
    );
}

/// Yield delegation (#49): a `yield` inside a yielded block reaches the
/// block of its *writer's* method, never itself — the seed used to loop
/// forever here while the trio delegated correctly. The accumulator runs
/// through both layers because the handed block is shared, not copied, so
/// a delegated yield's write-back lands where the writer will look.
#[test]
fn portland_evaluator_matches_the_seed_on_yield_delegation() {
    assert_evaluator_matches_seed(
        "evaluator_yield_delegation.pdx",
        "def inner_apply\n  yield\n  \"inner's answer\"\nend\ndef outer_apply\n  inner_apply do\n    yield\n  end\n  \"outer's answer\"\nend\ndef two_layers\n  outer_apply do\n    \"the block's value\"\n  end\nend\nputs two_layers\ndef accumulating\n  mutable count = 0\n  outer_apply do\n    count += 1\n  end\n  outer_apply do\n    count += 10\n  end\n  count\nend\nputs accumulating\ndef returning_through_two\n  outer_apply do\n    return \"through both layers\"\n  end\n  \"never\"\nend\nputs returning_through_two\ndef twice\n  yield\n  yield\n  \"twice done\"\nend\ndef delegate_twice\n  mutable log = []\n  twice do\n    inner_apply do\n      log << \"ran\"\n    end\n  end\n  log.length\nend\nputs delegate_twice\n",
    );
}

/// `together` (ADR 0029, #11): serial fork-join — task names bind at the
/// join, plain lines interleave, a failed task is a bound failure, and
/// plain-line locals die at end. Only promised semantics appear here, so
/// this differential survives the parallel build unchanged.
#[test]
fn portland_evaluator_matches_the_seed_on_together() {
    assert_evaluator_matches_seed(
        "evaluator_together.pdx",
        "def fetch_user(id)\n  \"user-#{id}\"\nend\ndef recent_orders(id)\n  return failure(\"orders down\") if id == 0\n  [\"order-a\", \"order-b\"]\nend\ntogether do\n  greeting = \"hello\"\n  ~ user = fetch_user(7)\n  meanwhile orders = recent_orders(7)\n  puts \"#{greeting} from a plain line\"\nend\nputs user\nputs orders.length\ntogether do\n  ~ sad_orders = recent_orders(0)\nend\nputs sad_orders.failure?\np(sad_orders or \"no orders today\")\n",
    );
}

/// Traits (ADR 0028, #60): stateless method bundles, included with Ruby's
/// verb, merged at registration; trait methods reach the carrier's fields
/// and its own methods, and a module-nested trait resolves outward.
#[test]
fn portland_evaluator_matches_the_seed_on_traits() {
    assert_evaluator_matches_seed(
        "evaluator_traits.pdx",
        "trait Sexpable\n  def sexp_list(nodes)\n    nodes.map { it.sexp }.join(\" \")\n  end\nend\ntrait Countable\n  def size_note\n    \"#{elements.length} elements\"\n  end\nend\nstruct ArrayNode\n  elements\n\n  include Sexpable\n  include Countable\n\n  def sexp\n    \"(array #{sexp_list(elements)})\"\n  end\nend\nstruct Leaf\n  value\n\n  def sexp\n    \"#{value}\"\n  end\nend\nnode = ArrayNode.new(elements: [Leaf.new(value: 1), Leaf.new(value: 2)])\nputs node.sexp\nputs node.size_note\nmodule Rendering\n  trait Framed\n    def framed\n      \"[#{label}]\"\n    end\n  end\n\n  struct Badge\n    label\n\n    include Framed\n  end\nend\nputs Rendering::Badge.new(label: \"pdx\").framed\n",
    );
}

/// Typed results (ADR 0027, #59): `failure` boxes a reason, `failure?` is
/// universal, `or` unwraps-or-else, patterns are transparent to the box,
/// `!` propagates to the write site — through blocks and yields like any
/// return — and `read_file` answers a failure for a missing path.
#[test]
fn portland_evaluator_matches_the_seed_on_failures() {
    assert_evaluator_matches_seed(
        "evaluator_failures.pdx",
        "sad = failure(\"out of roses\")\np sad\nputs sad.failure?\nputs 5.failure?\nputs nil.failure?\nputs sad.some?\nputs sad.nil?\nputs(sad or \"fallback\")\nputs(\"present\" or \"unused\")\nanswer = case sad\nin reason then \"reason was #{reason}\"\nend\nputs answer\nstruct ParseFailed\n  line\nend\ntyped = failure(ParseFailed.new(line: 7))\nshaped = case typed\nin ParseFailed(line:) then \"failed on line #{line}\"\nin content then \"parsed #{content}\"\nend\nputs shaped\np inspect(typed)\ndef risky(flag)\n  return failure(\"flagged\") if flag\n  \"fine\"\nend\ndef guarded(flag)\n  value = risky!(flag)\n  \"got #{value}\"\nend\nputs guarded(false)\np guarded(true)\ndef through_a_block(flag)\n  [1, 2].each do |n|\n    checked = risky!(flag)\n    puts checked\n  end\n  \"walked\"\nend\nputs through_a_block(false)\np through_a_block(true)\nmissing = read_file(\"/tmp/portland-definitely-missing\") or \"default config\"\nputs missing\nputs read_file(\"/tmp/portland-definitely-missing\").failure?\n",
    );
}

/// The legal side of ADR 0001's line, through the trio (#45): everything
/// here must RUN, because enforcement that over-refuses is worse than none.
/// A failed guard discards its captures so the next branch binds them fresh;
/// while-loop iterations are fresh scopes for their own locals; a block
/// parameter shadow-and-restores an outer local; a capture rebinds an outer
/// mutable; and a wrong-shaped subject misses an array or struct pattern
/// rather than crashing.
#[test]
fn portland_evaluator_matches_the_seed_on_binding_rules() {
    assert_evaluator_matches_seed(
        "evaluator_binding_rules.pdx",
        "answer = case 5\nin score if score > 10 then \"big\"\nin score then \"small #{score}\"\nend\nputs answer\nmutable i = 0\nwhile i < 3\n  doubled = i * 2\n  i += 1\nend\nputs i\nx = 1\n[2].each do |x|\n  puts x\nend\nputs x\nmutable found = 0\ncase 5\nin found then puts found\nend\nputs found\n[1].each do\n  temp = 2\nend\ntemp = 3\nputs temp\nstruct Node\n  kind\nend\nshape = case 7\nin [inner] then \"array\"\nin Node(kind:) then \"node\"\nin other then \"other #{other}\"\nend\nputs shape\n",
    );
}

/// `inspect(value)` (ADR 0026): p's rendering as a string, composable, nil
/// included.
#[test]
fn portland_evaluator_matches_the_seed_on_inspect() {
    assert_evaluator_matches_seed(
        "evaluator_inspect.pdx",
        "puts inspect(\"rose\")\nputs inspect(nil)\nputs inspect(:paid)\nputs inspect({name: \"pdx\"})\nputs inspect([1, \"two\"])\nputs inspect(some(nil))\nputs \"got #{inspect(nil)} and #{inspect(\"x\")}\"\nstruct Badge\n  label\nend\nputs inspect(Badge.new(label: \"pdx\"))\n",
    );
}

/// Rendering parity (#39): the trio's tagged guest shapes — symbols,
/// structs, enum cases — used to print raw (`{[__symbol__, name] => 1}`),
/// so any program that displayed a symbol-keyed hash diverged. The trio now
/// carries the seed's Display and inspect rules, so every printing path —
/// `p`, `puts`, `to_s`, `join`, interpolation — agrees byte for byte.
#[test]
fn portland_evaluator_matches_the_seed_on_rendering() {
    assert_evaluator_matches_seed(
        "evaluator_rendering.pdx",
        "enum Status\n  :pending\n  :paid(on:)\nend\nstruct Token\n  kind\n  text\nend\nconfig = {name: \"pdx\", port: 8080}\np config\nputs config\nodd = {:\"odd key\" => 1, \"plain\" => 2, 3 => :three}\np odd\nputs odd\np :paid\nputs :paid\nputs :paid.to_s\npaid = :paid(on: \"tuesday\")\np paid\nputs paid\ntoken = Token.new(kind: \"integer\", text: \"42\")\np token\nputs token\np([:paid, token, {a: 1}])\nputs([:paid, 1, \"two\"])\nputs([:paid, :pending].join(\"-\"))\nputs \"#{:paid} and #{config}\"\np some(nil)\nputs \"#{some(nil)}\"\nnested = {outer: {inner: :deep}}\np nested\np([1, [2, [3]]])\np \"quotes \\\" and \\\\ and\\nnewline\"\np({sym: nil})\np 1..5\nputs 1..5\n",
    );
}

/// Block interrupts, through the trio (#41, #42, ADR 0025): `break` stops
/// the iteration and the call answers nil (ADR 0012); `return` unwinds to
/// the method its block was *written* in — through a builtin's block, and
/// through any method that merely yielded to it, whose own body stops and
/// whose own result never happens. A valueless `map` pass refuses rather
/// than inventing a nil. None of these shapes appeared in any fixture,
/// which is how both promises broke silently.
#[test]
fn portland_evaluator_matches_the_seed_on_block_interrupts() {
    assert_evaluator_matches_seed(
        "evaluator_block_interrupts.pdx",
        "def broken_out\n  [1, 2].each do\n    break\n  end\nend\np broken_out\ndef through_blocks\n  [1, 2, 3].each do\n    return \"unwound\"\n  end\n  \"never reached\"\nend\nputs through_blocks\ndef stopped_map\n  [1, 2, 3].map do |n|\n    break if n == 2\n    n * 10\n  end\nend\np stopped_map\ndef early_from_select\n  [1, 2, 3, 4].select do |n|\n    return \"left early\" if n == 3\n    n.odd?\n  end\nend\np early_from_select\ndef counted\n  mutable ticks = 0\n  5.times do\n    break if ticks == 2\n    ticks += 1\n  end\n  ticks\nend\np counted\ndef apply\n  yield\n  puts \"apply continued\"\n  \"apply's own\"\nend\ndef outer\n  from_apply = apply do\n    return \"unwound through yield\"\n  end\n  puts from_apply\n  \"after\"\nend\nputs outer\ndef mixed\n  apply do\n    [1, 2].each do |n|\n      return \"from each inside a yielded block\" if n == 2\n    end\n  end\n  \"after helper\"\nend\nputs mixed\ndef or_guarded\n  apply do\n    found = [].first or return \"guarded through yield\"\n    found\n  end\n  \"after\"\nend\nputs or_guarded\nmutable seen = []\n1.upto(9) do |n|\n  break if n > 3\n  seen << n\nend\np seen\n",
    );
}

/// Range patterns, through the trio (#40): membership not equality, both
/// dot counts, beginless and endless ends, negative bounds, a non-integer
/// subject missing rather than erroring, and the one-line form. No fixture
/// matched on a range before this, which is how the trio shipped without
/// them and stayed green.
#[test]
fn portland_evaluator_matches_the_seed_on_range_patterns() {
    assert_evaluator_matches_seed(
        "evaluator_range_patterns.pdx",
        "def bucket(value)\n  case value\n  in ..-1  then \"low\"\n  in 0...10 then \"single\"\n  in 10..  then \"high\"\n  end\nend\nputs bucket(0 - 5)\nputs bucket(5)\nputs bucket(50)\ncase \"text\"\nin 1..9 then puts \"range hit\"\nelse\n  puts \"a string misses\"\nend\nputs 7 in 1..9 | 20..29\nputs 15 in 1..9 | 20..29\n",
    );
}

#[test]
fn portland_evaluator_matches_the_seed_on_one_line_patterns() {
    assert_evaluator_matches_seed(
        "evaluator_one_line_patterns.pdx",
        "hit = 5 in 1 | 5\nputs hit\nputs 5 in nil\npair = [1, 2]\npair => [a, b]\nputs a + b\nstruct Token\n  kind\n  text\nend\ntoken = Token.new(kind: \"plus\", text: \"+\")\nif token in Token(kind: \"plus\")\n  puts \"plus\"\nend\n",
    );
}

#[test]
fn portland_evaluator_matches_the_seed_on_append_and_index_assignment() {
    // Guest hashes are host hashes now, so whole-hash rendering matches too.
    assert_evaluator_matches_seed(
        "evaluator_rebinding_sugar.pdx",
        "mutable config = {\"theme\" => \"teal\"}\nconfig[\"digest\"] = nil\nconfig[\"theme\"] = \"grey\"\np config\np config[\"theme\"]\np config[\"digest\"]\np config[\"missing\"]\nmutable line = \"port\"\nline << \"land\"\nputs line\nmutable list = [1]\nlist << 2\np list.length\nconfig.each do |key, value|\n  puts key\nend\nmutable spots = [1, 2, 3]\naliased = spots\nspots[0] = 9\nspots[0 - 1] = 7\nspots[3] = 4\np spots\np aliased\n",
    );
}

#[test]
fn portland_evaluator_matches_the_seed_on_struct_methods_and_type_patterns() {
    assert_evaluator_matches_seed(
        "evaluator_struct_methods.pdx",
        "struct Token\n  kind\n  text\n\n  def integer?\n    kind == \"integer\"\n  end\n\n  def describe\n    \"#{kind}: #{text}\"\n  end\n\n  def loud\n    describe.upcase\n  end\n\n  def mirror\n    self\n  end\nend\ntoken = Token.new(kind: \"integer\", text: \"42\")\nputs token.integer?\nputs token.describe\nputs token.loud\nputs token.mirror == token\nrenamed = token.with(text: \"7\")\nputs renamed.describe\nputs token.describe\nputs renamed.with(kind: \"string\", text: \"go\").describe\ncase 5\nin String then puts \"text\"\nin Integer then puts \"number\"\nend\nputs([] in Array)\nputs(token in Array)\nputs(\"x\" in String)\n",
    );
}

/// Symbols, hash shorthand, and enums through the trio (ADRs 0022, 0023).
///
/// The fixture covers the same ground, but this pins the pieces separately
/// so a regression names which one broke.
#[test]
fn portland_evaluator_matches_the_seed_on_symbols_and_enums() {
    assert_evaluator_matches_seed(
        "evaluator_enums.pdx",
        "struct Purchase\n  status\n\n  enum Status\n    :pending\n    :paid(on:)\n  end\n\n  def paid?\n    status in :paid(on:)\n  end\nend\n\nputs(Purchase.new(status: :paid(on: \"tuesday\")).paid?)\nconfig = {name: \"pdx\", port: 8080}\nputs(config[:name])\nputs(config[:port].to_s)\nmixed = {\"s\" => 1, sym: 2}\nputs(mixed[:sym].to_s)\nsame = :paid == :paid\nputs(same.to_s)\ndiffer = :paid == :pending\nputs(differ.to_s)\ncase Purchase.new(status: :paid(on: \"tuesday\")).status\nin :pending then puts(\"no\")\nin :paid(on:) then puts(on)\nend\ncase :pending\nin :paid(on:) then puts(on)\nin :pending then puts(\"bare case\")\nend\n",
    );
}

/// A block rebinds the caller's `mutable`, and the rebinding outlives the
/// block — ADR 0001's accumulator pattern, through the trio.
///
/// Smallest case that pins the block-scope gap: a `yield`ed block that
/// counts. Nothing about `yield` is under test here beyond its scope
/// threading, which is why the count is the only thing printed.
#[test]
fn portland_evaluator_matches_the_seed_on_a_block_that_accumulates() {
    assert_evaluator_matches_seed(
        "evaluator_accumulator.pdx",
        "def twice\n  yield\n  yield\nend\n\nmutable count = 0\n\ntwice do\n  count += 1\nend\n\nputs(count)\n",
    );
}

/// The same accumulator, through a builtin's block rather than `yield` —
/// `times` here, standing in for the dozen arms of `evaluate_block_call`.
#[test]
fn portland_evaluator_matches_the_seed_on_a_builtin_block_that_accumulates() {
    assert_evaluator_matches_seed(
        "evaluator_accumulator_builtin.pdx",
        "mutable count = 0\n\n3.times do\n  count += 1\nend\n\nputs(count)\n",
    );
}

/// `yield`, through the trio.
#[test]
fn portland_evaluator_matches_the_seed_on_yield() {
    assert_evaluator_matches_seed(
        "evaluator_yield.pdx",
        "def twice\n  yield\n  yield\nend\n\ndef around\n  puts(\"before\")\n  yield\n  puts(\"after\")\nend\n\ntwice do\n  puts(\"body\")\nend\n\naround do\n  puts(\"inside\")\nend\n",
    );
}

#[test]
fn portland_evaluator_runs_the_fixture_suite() {
    // The summit of #19: Portland programs running on the Portland
    // evaluator, byte-identical to the seed.
    for fixture in [
        "hello",
        "arithmetic",
        "fizzbuzz",
        "showcase",
        "blocks",
        "tour",
        "patterns",
        "namespaces",
        "enums",
        "value_methods",
        "requires_library",
    ] {
        let path = format!(
            "{}/tests/fixtures/{fixture}.pdx",
            env!("CARGO_MANIFEST_DIR")
        );
        let direct = Command::new(env!("CARGO_BIN_EXE_pdx"))
            .arg(&path)
            .output()
            .expect("failed to run pdx");
        let hosted = Command::new(env!("CARGO_BIN_EXE_pdx"))
            .arg(portland_run())
            .arg(&path)
            .output()
            .expect("failed to run pdx");
        assert!(
            direct.status.success() && hosted.status.success(),
            "{fixture} failed"
        );
        assert_eq!(
            String::from_utf8(direct.stdout).unwrap(),
            String::from_utf8(hosted.stdout).unwrap(),
            "{fixture} diverged from the seed"
        );
    }
}

#[test]
fn portland_evaluator_matches_the_seed_on_literals() {
    assert_evaluator_matches_seed(
        "eval_rung0.pdx",
        "puts 42\nputs \"rose city\"\nputs \"line\\nbreak\"\nputs true\nputs\n",
    );
}

#[test]
fn portland_evaluator_matches_the_seed_on_variables() {
    assert_evaluator_matches_seed(
        "eval_rung2.pdx",
        "mutable x = 40 + 2\nputs x\nx = x + 1\nputs x\nmutable total = 0\ntotal += 5\ntotal *= 3\nputs total\nname = \"rose\"\nputs name + \" city\"\n",
    );
}

#[test]
fn portland_evaluator_matches_the_seed_on_interpolation() {
    assert_evaluator_matches_seed(
        "eval_rung3.pdx",
        "name = \"portland\"\nputs \"hello #{name}!\"\nputs \"sum: #{1 + 2 * 3}\"\nputs 'literal #{nope} and \\n stays'\nputs \"a#{1}b#{2}c\"\nanswer = 42\nputs \"#{answer}\"\n",
    );
}

#[test]
fn portland_evaluator_matches_the_seed_on_control_flow() {
    assert_evaluator_matches_seed(
        "eval_rung4.pdx",
        "mutable n = 3\nwhile n > 0\n  puts n\n  n -= 1\nend\nmutable count = 0\nwhile true\n  count += 1\n  next if count == 2\n  puts count\n  break if count > 3\nend\nif 1 < 2\n  puts \"yes\"\nelse\n  puts \"no\"\nend\nlabel = if 2 > 1\n  \"big\"\nelse\n  \"small\"\nend\nputs label\nputs \"guard\" if true\ncase 5\nwhen 1 then puts \"one\"\nwhen 5 then puts \"five\"\nelse\n  puts \"many\"\nend\n",
    );
}

#[test]
fn portland_evaluator_matches_the_seed_on_methods() {
    assert_evaluator_matches_seed(
        "eval_rung5.pdx",
        "def greet(name = \"stranger\")\n  \"hello, #{name}!\"\nend\nputs greet(\"pdx\")\nputs greet()\nputs greet\ndef factorial(n)\n  return 1 if n <= 1\n  n * factorial(n - 1)\nend\nputs factorial(10)\ndef pair(base, twice = base * 2)\n  base + twice\nend\nputs pair(5)\ndef shout(word)\n  puts word + \"!\"\nend\nshout \"go\"\n",
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

/// Every multi-line construct has to keep the REPL reading rather than
/// giving up after the first line.
#[test]
fn repl_keeps_reading_through_multi_line_constructs() {
    let heredoc = run_repl("x = <<~SQL\n  select 1\nSQL\n");
    assert_eq!(
        String::from_utf8(heredoc.stdout).unwrap(),
        "=> \"select 1\\n\"\n"
    );

    let braced = run_repl("[1, 2].map { |n|\n  n * 2\n}\n");
    assert_eq!(String::from_utf8(braced.stdout).unwrap(), "=> [2, 4]\n");

    let definition = run_repl("def greet(name)\n  \"hi #{name}\"\nend\ngreet(\"pdx\")\n");
    assert_eq!(
        String::from_utf8(definition.stdout).unwrap(),
        "=> \"hi pdx\"\n"
    );
}

/// `_` is the last printed value — a REPL affordance, not a language one.
#[test]
fn repl_binds_the_last_value_to_underscore() {
    let output = run_repl("40 + 2\n_ + 1\n_ * 2\n");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "=> 42\n=> 43\n=> 86\n"
    );
}

/// An unfinished entry is otherwise inescapable — every further line just
/// extends it.
#[test]
fn repl_cancels_an_entry_in_progress() {
    let output = run_repl("def broken\n:cancel\n1 + 1\n");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "cancelled 1 line(s)\n=> 2\n"
    );
}

/// Every spelling a hand might reach for, since none of them is a
/// Portland builtin and nothing is shadowed by claiming them.
#[test]
fn repl_quits_on_command() {
    for spelling in ["quit", "exit", ":quit", ":exit", "quit()", "exit()"] {
        let output = run_repl(&format!("{spelling}\n1 + 1\n"));
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            "",
            "{spelling} should quit before evaluating anything"
        );
    }
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

/// `pdx --parse <file>` — parse and exit, without evaluating (#35).
///
/// Exists so `script/docs/check` can verify that every code sample in the
/// docs is at least syntactically real. Parsing is the right depth: it
/// catches invented syntax without caring that a sample references
/// `lookup(id)` or `article` without defining them, which running would.
fn parse_only(source: &str, name: &str) -> std::process::Output {
    let sample = std::env::temp_dir().join(name);
    std::fs::write(&sample, source).unwrap();
    Command::new(env!("CARGO_BIN_EXE_pdx"))
        .arg("--parse")
        .arg(&sample)
        .output()
        .expect("failed to run pdx")
}

#[test]
fn parse_only_accepts_valid_portland() {
    let output = parse_only("x = 1 + 2\nputs x\n", "parse_only_valid.pdx");
    assert!(output.status.success());
}

#[test]
fn parse_only_does_not_evaluate() {
    // The whole point: a sample may reference names it never defines, and
    // may print. Neither should happen.
    let output = parse_only(
        "puts \"should not run\"\nundefined_call(missing_name)\n",
        "parse_only_no_eval.pdx",
    );
    assert!(
        output.status.success(),
        "parsing should not run the program"
    );
    assert!(
        String::from_utf8(output.stdout).unwrap().is_empty(),
        "--parse printed something, so it evaluated"
    );
}

#[test]
fn parse_only_rejects_invented_syntax() {
    // Each of these was written into docs/language.md from memory and does
    // not exist: an endless method, a one-line if/then/else, a ternary.
    let cases = [
        ("def integer? = kind == \"x\"\n", "endless method"),
        ("y = if true then 1 else 2 end\n", "one-line if/then/else"),
        ("y = true ? 1 : 2\n", "ternary"),
    ];
    for (source, what) in cases {
        let output = parse_only(source, "parse_only_invalid.pdx");
        assert!(
            !output.status.success(),
            "--parse accepted {what}, which is not Portland"
        );
    }
}

#[test]
fn parse_only_needs_a_file() {
    let output = Command::new(env!("CARGO_BIN_EXE_pdx"))
        .arg("--parse")
        .output()
        .expect("failed to run pdx");
    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("needs a file")
    );
}

/// The seed and the trio must implement the same builtin methods, and every
/// one of them must appear in a hosted fixture.
///
/// This exists because an audit found nine methods the trio simply did not
/// have — `positive?`, `keys`, `reduce`, `upto` and friends — and the
/// differential harness was green throughout, because no fixture used them.
/// Twenty-one of the forty-two documented methods had never run through the
/// trio at all.
///
/// Relying on someone remembering to add a fixture is what failed. These two
/// checks read the implementations instead, so a method added to the seed
/// tomorrow is picked up without anyone deciding to look.
///
/// Both directions are deliberately one-sided. Absence from the trio's
/// dispatch, or from every fixture, is conclusive; presence is only evidence.
/// A method could appear in a fixture inside a comment and count as covered —
/// a false negative, which is the safe direction. A check that cries wolf
/// gets deleted, and then there is no check at all.
fn seed_builtin_methods() -> Vec<String> {
    let source = include_str!("../src/interpreter.rs");
    let mut names: Vec<String> = source
        .match_indices(", \"")
        .filter_map(|(at, _)| {
            let rest = &source[at + 3..];
            let end = rest.find('"')?;
            let name = &rest[..end];
            // A dispatch arm is `(receiver, "name", [args])`.
            if !rest[end..].starts_with("\", [") {
                return None;
            }
            let plain = name
                .chars()
                .all(|character| character.is_ascii_lowercase() || "_?!".contains(character));
            (plain && !name.is_empty()).then(|| name.to_string())
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

#[test]
fn the_trio_implements_every_builtin_the_seed_does() {
    let evaluator = std::fs::read_to_string(format!(
        "{}/../compiler/evaluator.pdx",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("failed to read evaluator.pdx");

    let missing: Vec<String> = seed_builtin_methods()
        .into_iter()
        .filter(|name| !evaluator.contains(&format!("when \"{name}\"")))
        .collect();

    assert!(
        missing.is_empty(),
        "the seed implements these and compiler/evaluator.pdx does not: {}.\n\
         The trio's dispatch treats an unknown method as a struct-field read, so \
         these fail with an unrelated error rather than saying no such method.",
        missing.join(", ")
    );
}

#[test]
fn every_builtin_appears_in_a_hosted_fixture() {
    let fixtures: String =
        std::fs::read_dir(format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR")))
            .expect("failed to read the fixture directory")
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                (path.extension()? == "pdx").then(|| std::fs::read_to_string(path).ok())?
            })
            .collect();

    let untested: Vec<String> = seed_builtin_methods()
        .into_iter()
        .filter(|name| !fixtures.contains(name.as_str()))
        .collect();

    assert!(
        untested.is_empty(),
        "no fixture mentions these, so the differential harness never runs them: {}.\n\
         Add them to a fixture — green is not covered.",
        untested.join(", ")
    );
}

/// Portland's language spec runs, on both oracles (`spec/`).
///
/// The differential harness proves the seed and the trio agree with each
/// other. It cannot prove either agrees with what was *decided* — a shared
/// misreading of an ADR passes it. So the spec runs twice.
///
/// A failing example is a `  FAIL ` line, not a panic, and this test has to
/// look for it: the spec file reports every failure and keeps going, because a
/// Portland method cannot hold a tally, so counting lives in `script/spec`
/// instead. A zero exit status alone would therefore pass a spec that failed
/// every example — the same shape of hole as "green is not covered."
/// Every `*_spec.pdx` under a directory, recursively.
///
/// Recursive because specs nest: `spec/numbers/integers_spec.pdx` groups by
/// subject, and a spec that silently never runs is the worst way to be green.
/// Named rather than extension-matched so `spec_helper.pdx` is left alone —
/// running a library as a spec reports zero examples and passes, which is noise
/// dressed as coverage.
fn spec_files(directory: &std::path::Path, found: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(directory).expect("failed to read a spec directory") {
        let path = entry.expect("failed to read a spec directory entry").path();
        if path.is_dir() {
            spec_files(&path, found);
        } else if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with("_spec.pdx"))
        {
            found.push(path);
        }
    }
}

#[test]
fn the_language_spec_passes_on_both_oracles() {
    let mut specs = Vec::new();
    spec_files(
        std::path::Path::new(&format!("{}/../spec", env!("CARGO_MANIFEST_DIR"))),
        &mut specs,
    );
    // Directory order is not stable across filesystems, and a failure message
    // naming a different file each run is a worse failure message.
    specs.sort();
    assert!(!specs.is_empty(), "no *_spec.pdx files found under spec/");

    // The hosted half runs once for the whole suite. What a hosted spec costs
    // is not the trio — loading that is 0.02s — but `spec_helper.pdx`,
    // re-parsed into every spec's fresh scope at 0.40s a time; run_specs.pdx
    // parses the harness once and shares it (#69). The ceiling covers the
    // whole batch rather than one file, so it is scaled to match.
    let batch = within_seconds(120, "language spec, hosted", || {
        Command::new(env!("CARGO_BIN_EXE_pdx"))
            .arg(format!(
                "{}/../spec/run_specs.pdx",
                env!("CARGO_MANIFEST_DIR")
            ))
            .arg(format!(
                "{}/../spec/spec_helper.pdx",
                env!("CARGO_MANIFEST_DIR")
            ))
            .args(&specs)
            .output()
            .expect("failed to run pdx")
    });
    assert!(
        batch.status.success(),
        "the hosted run failed:\n{}{}",
        String::from_utf8_lossy(&batch.stdout),
        String::from_utf8_lossy(&batch.stderr)
    );

    // Split the batch back into one transcript per spec, on the `=== ` marker
    // the driver prints before each file.
    let batch_stdout = String::from_utf8(batch.stdout).unwrap();
    let mut hosted: Vec<String> = Vec::new();
    for line in batch_stdout.lines() {
        match line.strip_prefix("=== ") {
            Some(_) => hosted.push(String::new()),
            None => {
                let current = hosted
                    .last_mut()
                    .expect("the hosted run printed output before naming a spec");
                current.push_str(line);
                current.push('\n');
            }
        }
    }
    assert_eq!(
        hosted.len(),
        specs.len(),
        "the hosted run covered {} specs, not {}",
        hosted.len(),
        specs.len()
    );

    for (spec, hosted) in specs.iter().zip(hosted) {
        let direct = Command::new(env!("CARGO_BIN_EXE_pdx"))
            .arg(spec)
            .output()
            .expect("failed to run pdx");
        assert!(
            direct.status.success(),
            "{} failed direct:\n{}{}",
            spec.display(),
            String::from_utf8_lossy(&direct.stdout),
            String::from_utf8_lossy(&direct.stderr)
        );
        let direct = String::from_utf8(direct.stdout).unwrap();

        for (label, transcript) in [("direct", &direct), ("hosted", &hosted)] {
            let failures: Vec<&str> = transcript
                .lines()
                .filter(|line| line.starts_with("  FAIL "))
                .collect();
            assert!(
                failures.is_empty(),
                "{} reported failing examples {label}:\n{}",
                spec.display(),
                failures.join("\n")
            );
        }
        // Same spec, same oracles, same answers.
        assert_eq!(
            direct,
            hosted,
            "{} diverged between the seed and the trio",
            spec.display()
        );
    }
}
