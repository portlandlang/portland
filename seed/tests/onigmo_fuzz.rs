//! The seed's regex engine against Ruby 4.0.9 on recorded fuzz batches,
//! and against its own past on matcher steps (ADR 0086: tie goes to Ruby).
//!
//! Each file in `fixtures/onigmo/fuzz/` holds generated cases with the
//! answer Ruby 4.0.9 gave, recorded by `script/regex_fuzz`, and the number
//! of matcher steps the seed took when the baseline was recorded. A case
//! fails when the seed's answer differs from Ruby's, or when its steps grow
//! past the baseline by more than a quarter; the whole run fails when the
//! total grows by more than a twentieth. Steps depend only on the engine,
//! the pattern, and the text, so the guard is deterministic where a timer
//! would flake.
//!
//! After a change that moves the step counts on purpose, re-record the
//! baselines (no Ruby needed):
//!
//! ```text
//! REGEX_FUZZ_RECORD=1 cargo test --package portland-seed --test onigmo_fuzz
//! ```
//!
//! One case per line, tab-separated, `#` lines are comments:
//!
//! ```text
//! id  start  steps  pattern  text  answer
//! ```
//!
//! `start` is the character position the search begins at; `steps` the
//! baseline, `-` for an error or a case not yet recorded; `pattern` and
//! `text` escape `\` as `\\`, a tab, newline, and carriage return as `\t`,
//! `\n`, `\r`, and other invisible characters as `\u{hex}`; a text of
//! `@file:name` is the contents of that file in the same directory.
//! `answer` is `nomatch`, `match` and each group's byte span (`-` for a
//! group that took no part), or `error` and Ruby's sentence.

use portland_seed::regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The budget a case gets while its baseline is being recorded; a case
/// that reaches it is dropped from the batch, which keeps the replay quick
/// enough for every `script/test`. The benchmark batch, whose cases are
/// heavy by design, gets the larger one.
const RECORD_BUDGET: u64 = 250_000;
const BENCH_RECORD_BUDGET: u64 = 20_000_000;

fn record_budget(path: &Path) -> u64 {
    if path.file_name().is_some_and(|name| name == "bench.tsv") {
        BENCH_RECORD_BUDGET
    } else {
        RECORD_BUDGET
    }
}

/// How far past its baseline a case's steps may grow before it fails.
fn allowed_steps(baseline: u64) -> u64 {
    baseline + baseline / 4 + 32
}

fn fuzz_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/onigmo/fuzz")
}

fn unescape(field: &str) -> String {
    let mut result = String::with_capacity(field.len());
    let mut characters = field.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            result.push(character);
            continue;
        }
        match characters.next() {
            Some('\\') => result.push('\\'),
            Some('t') => result.push('\t'),
            Some('n') => result.push('\n'),
            Some('r') => result.push('\r'),
            Some('u') => {
                assert_eq!(characters.next(), Some('{'), "an escape in {field:?}");
                let hex: String = characters
                    .by_ref()
                    .take_while(|digit| *digit != '}')
                    .collect();
                let code = u32::from_str_radix(&hex, 16).expect("a hex code");
                result.push(char::from_u32(code).expect("a code point"));
            }
            other => panic!("unknown escape {other:?} in {field:?}"),
        }
    }
    result
}

struct Case {
    id: String,
    start_chars: usize,
    baseline: Option<u64>,
    pattern: String,
    text: String,
    answer: String,
}

/// A line kept as written, so recording rewrites only the steps column.
enum Line {
    Comment(String),
    Case { fields: Vec<String>, case: Case },
}

fn read_batch(path: &Path, texts: &mut HashMap<String, String>) -> Vec<Line> {
    let content = std::fs::read_to_string(path).expect("a fuzz batch");
    content
        .lines()
        .map(|line| {
            if line.starts_with('#') || line.is_empty() {
                return Line::Comment(line.to_string());
            }
            let fields: Vec<String> = line.split('\t').map(str::to_string).collect();
            assert_eq!(fields.len(), 6, "six fields in {}: {line}", path.display());
            let text = match fields[4].strip_prefix("@file:") {
                Some(name) => texts
                    .entry(name.to_string())
                    .or_insert_with(|| {
                        std::fs::read_to_string(fuzz_directory().join(name)).expect("a text file")
                    })
                    .clone(),
                None => unescape(&fields[4]),
            };
            let case = Case {
                id: fields[0].clone(),
                start_chars: fields[1].parse().expect("a start"),
                baseline: fields[2].parse().ok(),
                pattern: unescape(&fields[3]),
                text,
                answer: fields[5].clone(),
            };
            Line::Case { fields, case }
        })
        .collect()
}

fn batch_paths() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(fuzz_directory())
        .expect("the fuzz directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "tsv"))
        .collect();
    paths.sort();
    paths
}

/// The seed's answer in the fixtures' spelling, and its steps; `budget`
/// when the search ran out of steps.
fn seed_answer(case: &Case, budget: u64) -> (String, u64) {
    let regex = match Regex::new(&case.pattern) {
        Err(error) => return (format!("error {}", error.message), 0),
        Ok(regex) => regex,
    };
    let start = case
        .text
        .char_indices()
        .nth(case.start_chars)
        .map(|(offset, _)| offset)
        .unwrap_or(case.text.len());
    let (result, steps) = regex.search_counted(&case.text, start, budget);
    let answer = match result {
        Err(_) => "budget".to_string(),
        Ok(None) => "nomatch".to_string(),
        Ok(Some(captures)) => {
            let spans: Vec<String> = captures
                .groups
                .iter()
                .map(|group| match group {
                    Some((start, end)) => format!("{start}-{end}"),
                    None => "-".to_string(),
                })
                .collect();
            format!("match {}", spans.join(" "))
        }
    };
    (answer, steps)
}

fn describe(case: &Case) -> String {
    let text: String = case.text.chars().take(80).collect();
    format!(
        "{}  /{}/ on {:?} from {}",
        case.id, case.pattern, text, case.start_chars
    )
}

#[test]
fn the_seed_agrees_with_ruby_on_the_fuzz_batches_within_its_recorded_steps() {
    if std::env::var_os("REGEX_FUZZ_RECORD").is_some() {
        record();
        return;
    }
    let mut texts = HashMap::new();
    let mut cases = 0usize;
    let mut disagreements = Vec::new();
    let mut slower = Vec::new();
    let mut faster = 0usize;
    let mut total_baseline = 0u64;
    let mut total_steps = 0u64;
    for path in batch_paths() {
        for line in read_batch(&path, &mut texts) {
            let Line::Case { case, .. } = line else {
                continue;
            };
            cases += 1;
            let budget = case
                .baseline
                .map(|baseline| allowed_steps(baseline) + 1)
                .unwrap_or(record_budget(&path));
            let (answer, steps) = seed_answer(&case, budget);
            if answer != case.answer && answer != "budget" {
                disagreements.push(format!(
                    "{}\n      ruby: {}\n      seed: {answer}",
                    describe(&case),
                    case.answer
                ));
                continue;
            }
            let Some(baseline) = case.baseline else {
                if answer != case.answer {
                    disagreements.push(format!("{}\n      ran out of steps", describe(&case)));
                }
                continue;
            };
            total_baseline += baseline;
            total_steps += steps;
            if steps > allowed_steps(baseline) || answer == "budget" {
                slower.push(format!(
                    "{}\n      steps: {baseline} recorded, now past {}",
                    describe(&case),
                    allowed_steps(baseline)
                ));
            } else if steps < baseline - baseline / 4 {
                faster += 1;
            }
        }
    }
    eprintln!(
        "onigmo fuzz: {cases} cases, {} disagree with Ruby 4.0.9, {} past their step margin, \
         {faster} a quarter faster; {total_steps} steps against {total_baseline} recorded",
        disagreements.len(),
        slower.len()
    );
    for line in disagreements.iter().take(40) {
        eprintln!("{line}");
    }
    for line in slower.iter().take(40) {
        eprintln!("{line}");
    }
    if faster > 0 {
        eprintln!(
            "{faster} cases now take a quarter fewer steps; re-record the baselines to keep \
             the gain: REGEX_FUZZ_RECORD=1 cargo test --package portland-seed --test onigmo_fuzz"
        );
    }
    assert!(disagreements.is_empty(), "the seed disagrees with Ruby");
    assert!(slower.is_empty(), "cases grew past their step margin");
    assert!(
        total_steps <= total_baseline + total_baseline / 20,
        "the batches took {total_steps} steps, more than a twentieth past the {total_baseline} recorded"
    );
}

/// Rewrites every batch's steps column from what the seed takes now. A
/// case the seed answers differently from Ruby keeps `-` and is reported;
/// one that runs out of the recording budget is dropped and reported.
fn record() {
    let mut texts = HashMap::new();
    for path in batch_paths() {
        let budget = record_budget(&path);
        let mut output = String::new();
        let mut recorded = 0usize;
        let mut dropped = Vec::new();
        let mut disagreeing = Vec::new();
        for line in read_batch(&path, &mut texts) {
            let (mut fields, case) = match line {
                Line::Comment(comment) => {
                    output.push_str(&comment);
                    output.push('\n');
                    continue;
                }
                Line::Case { fields, case } => (fields, case),
            };
            let (answer, steps) = seed_answer(&case, budget);
            if answer == "budget" {
                dropped.push(case.id.clone());
                continue;
            }
            fields[2] = if answer != case.answer {
                disagreeing.push(describe(&case));
                "-".to_string()
            } else if case.answer.starts_with("error") {
                "-".to_string()
            } else {
                recorded += 1;
                steps.to_string()
            };
            output.push_str(&fields.join("\t"));
            output.push('\n');
        }
        std::fs::write(&path, output).expect("a rewritten batch");
        eprintln!(
            "{}: {recorded} recorded, {} dropped past {budget} steps, {} disagree with Ruby",
            path.file_name().expect("a name").to_string_lossy(),
            dropped.len(),
            disagreeing.len()
        );
        for line in &disagreeing {
            eprintln!("  {line}");
        }
    }
}

/// Wall-clock microseconds for each case of the benchmark batch, twenty
/// searches each, for `script/regex_fuzz bench` to set beside Ruby's. Not
/// a test: timings vary from run to run and machine to machine.
#[test]
#[ignore = "timings for script/regex_fuzz bench; run with --release --ignored"]
fn bench_timings() {
    let mut texts = HashMap::new();
    let path = fuzz_directory().join("bench.tsv");
    for line in read_batch(&path, &mut texts) {
        let Line::Case { case, .. } = line else {
            continue;
        };
        let Ok(regex) = Regex::new(&case.pattern) else {
            continue;
        };
        let started = std::time::Instant::now();
        for _ in 0..20 {
            std::hint::black_box(regex.search(&case.text));
        }
        println!("{}\t{}", case.id, started.elapsed().as_micros());
    }
}
