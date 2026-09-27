//! The two whole-suite runs: the language spec on both implementations, and
//! the checker over the compiler's own source. Each takes over a minute,
//! which is most of the gate, so they live in their own test binary — cargo
//! reports them in a block of their own, apart from the hundred quick ones in
//! `run_pdx_files.rs`.

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

/// The checker's own source passes its own checks — every compiler file,
/// through `check.pdx`. The inference-backed refusals (ADR 0047) fire on
/// types read off real code, and this ~6,000-line corpus is the standing
/// proof that they refuse nothing that runs: a false positive here is a
/// checker bug by doctrine (ADR 0040), and the first one was caught by
/// exactly this run — `mutable result = nil` rebound inside an `each`.
#[test]
fn the_checker_passes_the_compilers_own_source() {
    let compiler = format!("{}/../compiler", env!("CARGO_MANIFEST_DIR"));
    // The fixtures ride along: every one is a program the seed runs, and a
    // `reduce(0) { |count, line| … }` among them was the first false
    // positive this run did not cover.
    let fixtures = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));
    let mut files: Vec<_> = [&compiler, &fixtures]
        .iter()
        .flat_map(|directory| std::fs::read_dir(directory).unwrap())
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "pdx"))
        .collect();
    files.sort();
    assert!(
        files.len() > 5,
        "the compiler directory should hold the trio"
    );
    // One process for the whole corpus: `check.pdx` follows each file's
    // requires (#94) and shares its memo across the files it is handed, so
    // the compiler is checked once rather than once per file that requires
    // it.
    let checked = Command::new(env!("CARGO_BIN_EXE_pdx"))
        .arg(format!("{compiler}/check.pdx"))
        .args(&files)
        .output()
        .expect("failed to run pdx");
    assert!(
        checked.status.success(),
        "the compiler and fixtures should pass the checker, got: {}",
        String::from_utf8_lossy(&checked.stderr)
    );
}

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
            // The ruby/spec stubs are parsed, not run — every example in them
            // is a pending, which cannot fail. See the_ruby_spec_stubs_parse.
            if path.ends_with("spec/ruby") {
                continue;
            }
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

/// Portland's language spec runs, on both oracles (`spec/`).
///
/// The differential harness proves the seed and the compiler agree with each
/// other. It cannot prove either agrees with what was *decided* — a shared
/// misreading of an ADR passes it. So the spec runs twice.
///
/// A failing example is a `  FAIL ` line, not a panic, and this test has to
/// look for it: the spec file reports every failure and keeps going, because a
/// Portland method cannot hold a tally, so counting lives in `script/spec`
/// instead. A zero exit status alone would therefore pass a spec that failed
/// every example — the same shape of hole as "green is not covered."
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
    // is not the compiler — loading that is 0.02s — but `spec_helper.pdx`,
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
            "{} diverged between the seed and the compiler",
            spec.display()
        );
    }
}
