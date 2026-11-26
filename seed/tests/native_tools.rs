//! The compiler's tools built natively and held to the seed (#600): the
//! linter, the parser, and the report, each compiled from scratch and run
//! beside the seed running the same file.
//!
//! Each build is minutes of the seed working out the front end's types,
//! so these are a step of their own: `script/test --native`, which CI
//! runs and the default `script/test` does not. A commit may be made
//! without them; a pull request does not merge with one failing.

use std::process::Command;

/// The linter builds natively (#572, #511): `lint.pdx`, which is the
/// compiler's lexer, its parser, the tree walk, and the rules, compiled
/// and run, says what the seed says running the same file. Over the
/// compiler's own source and the core, which lint clean, and over files
/// that break each kind of rule, where the two must find the same things
/// in the same words and leave with the same status.
///
/// It lives here because the build is the slow part: pinning the whole
/// front end's types takes the seed minutes.
#[test]
fn the_linter_built_natively_lints_as_the_seed_does() {
    let root = format!("{}/..", env!("CARGO_MANIFEST_DIR"));
    let program = format!("{root}/compiler/lint.pdx");
    let built = Command::new(format!("{root}/script/build"))
        .arg(&program)
        .env("PDX", env!("CARGO_BIN_EXE_pdx"))
        .output()
        .expect("failed to run script/build");
    assert!(
        built.status.success(),
        "lint.pdx should build: {}",
        String::from_utf8_lossy(&built.stderr)
    );
    let binary = String::from_utf8(built.stdout).unwrap().trim().to_string();

    let mut clean: Vec<_> = [format!("{root}/compiler"), format!("{root}/core")]
        .iter()
        .flat_map(|directory| std::fs::read_dir(directory).unwrap())
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "pdx"))
        .collect();
    clean.sort();

    let directory = std::env::temp_dir().join("native_lint_probe");
    std::fs::create_dir_all(&directory).unwrap();
    let broken = [
        (
            "order.pdx",
            "def zebra = 1\n\nALPHA = 2\n\nstruct Point\n  x\n\n  def wide = x\n  def area = x * x\nend\n\ndef apple = ALPHA\nrequire_relative \"nothing\"\n",
        ),
        (
            "margin.pdx",
            "def total(numbers)\n  mutable sum = 0\nnumbers.each do |number|\n    sum += number\n  end\n  sum\nend\n",
        ),
        ("parse.pdx", "def broken(\n  x = \nend\nputs [1, 2\n"),
        ("constants.pdx", "B = 1\nA = B + 1\nC = 3\n"),
    ];
    let mut runs: Vec<Vec<std::path::PathBuf>> = vec![clean];
    for (name, source) in broken {
        let path = directory.join(name);
        std::fs::write(&path, source).unwrap();
        runs.push(vec![path]);
    }

    for files in runs {
        let seed = Command::new(env!("CARGO_BIN_EXE_pdx"))
            .arg(&program)
            .args(&files)
            .output()
            .expect("failed to run pdx");
        let native = Command::new(&binary)
            .args(&files)
            .output()
            .expect("failed to run the native linter");
        assert_eq!(
            String::from_utf8_lossy(&native.stdout),
            String::from_utf8_lossy(&seed.stdout),
            "the native linter finds differently over {files:?}"
        );
        assert_eq!(
            native.status.code(),
            seed.status.code(),
            "the native linter leaves differently over {files:?}"
        );
    }
}

/// The parser builds natively and writes the trees the seed does (#572):
/// `parse.pdx`, which parses a file and writes each statement's tree out
/// by its `sexp`, compiled and run over the compiler's own source and the
/// core, writes what the seed writes running the same file. The whole
/// tree is compared, and writing it calls a method on a value that is
/// one of several structs at every node.
#[test]
fn the_parser_built_natively_writes_the_trees_the_seed_does() {
    let root = format!("{}/..", env!("CARGO_MANIFEST_DIR"));
    let program = format!("{root}/compiler/parse.pdx");
    let built = Command::new(format!("{root}/script/build"))
        .arg(&program)
        .env("PDX", env!("CARGO_BIN_EXE_pdx"))
        .output()
        .expect("failed to run script/build");
    assert!(
        built.status.success(),
        "parse.pdx should build: {}",
        String::from_utf8_lossy(&built.stderr)
    );
    let binary = String::from_utf8(built.stdout).unwrap().trim().to_string();

    let mut files: Vec<_> = [format!("{root}/compiler"), format!("{root}/core")]
        .iter()
        .flat_map(|directory| std::fs::read_dir(directory).unwrap())
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "pdx"))
        .collect();
    files.sort();
    assert!(files.len() > 5, "the compiler directory should hold files");

    for file in files {
        let seed = Command::new(env!("CARGO_BIN_EXE_pdx"))
            .arg(&program)
            .arg(&file)
            .output()
            .expect("failed to run pdx");
        let native = Command::new(&binary)
            .arg(&file)
            .output()
            .expect("failed to run the native parser");
        assert!(
            native.status.success(),
            "the native parser should read {}",
            file.display()
        );
        assert!(
            native.stdout == seed.stdout,
            "the native parser's tree differs from the seed's for {}",
            file.display()
        );
    }
}

/// The report builds natively (#30): `report.pdx`, compiled and run over
/// the compiler's own source and the core, reads back what the seed
/// reads back running the same file, with every concern and with each
/// one named.
#[test]
fn the_report_built_natively_reads_back_what_the_seed_does() {
    let root = format!("{}/..", env!("CARGO_MANIFEST_DIR"));
    let program = format!("{root}/compiler/report.pdx");
    let built = Command::new(format!("{root}/script/build"))
        .arg(&program)
        .env("PDX", env!("CARGO_BIN_EXE_pdx"))
        .output()
        .expect("failed to run script/build");
    assert!(
        built.status.success(),
        "report.pdx should build: {}",
        String::from_utf8_lossy(&built.stderr)
    );
    let binary = String::from_utf8(built.stdout).unwrap().trim().to_string();

    let mut files: Vec<String> = [format!("{root}/compiler"), format!("{root}/core")]
        .iter()
        .flat_map(|directory| std::fs::read_dir(directory).unwrap())
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "pdx"))
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    files.sort();

    for concern in [None, Some("panic"), Some("mutable"), Some("together")] {
        let arguments: Vec<&str> = concern
            .into_iter()
            .chain(files.iter().map(String::as_str))
            .collect();
        let seed = Command::new(env!("CARGO_BIN_EXE_pdx"))
            .arg(&program)
            .args(&arguments)
            .output()
            .expect("failed to run pdx");
        let native = Command::new(&binary)
            .args(&arguments)
            .output()
            .expect("failed to run the native report");
        assert!(native.status.success(), "the native report should run");
        assert!(
            native.stdout == seed.stdout,
            "the native report differs from the seed's for {concern:?}"
        );
    }
}
