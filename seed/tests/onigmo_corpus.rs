//! The seed's regex engine against Onigmo's own test suite, with Ruby
//! 4.0.9's answer for every case as the ground truth (ADR 0086: tie goes to
//! Ruby). See `fixtures/onigmo/README.md` for where the files come from.
//!
//! The count of agreeing cases is a ratchet: it may go up, never down. When
//! a build makes more cases agree, raise `AGREEING_CASES` to the new count.
//! A bad pattern agrees when both refuse it — the sentences are the
//! engine's, not Ruby's, and are not compared. The cases the engine never
//! returns from are listed in `HANGS` and counted as disagreements without
//! being run (#212).

use portland_seed::regex::Pattern;
use std::path::Path;

const AGREEING_CASES: usize = 904;

/// The cases oniai does not return from within a second (#212).
const HANGS: &[&str] = &[
    "838", "1311", "1312", "1370", "1373", "1577", "1578", "1579", "1580", "1584", "1585", "1586",
    "1587", "1591", "1592", "1593", "1594",
];

struct Case {
    id: String,
    pattern: String,
    text: String,
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).expect("hex"))
        .collect()
}

fn read_cases() -> Vec<Case> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/onigmo/cases.tsv");
    let content = std::fs::read_to_string(path).expect("cases.tsv");
    content
        .lines()
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            Case {
                id: fields[0].to_string(),
                pattern: String::from_utf8(unhex(fields[2])).expect("utf-8 pattern"),
                text: String::from_utf8(unhex(fields[3])).expect("utf-8 text"),
            }
        })
        .collect()
}

/// Ruby's answer line without its id: `error`, `nomatch`, or `match <spans>`.
fn read_ruby_answers() -> std::collections::HashMap<String, String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/onigmo/ruby_4_0_9.tsv");
    let content = std::fs::read_to_string(path).expect("ruby_4_0_9.tsv");
    content
        .lines()
        .map(|line| {
            let (id, rest) = line.split_once('\t').expect("id");
            let answer = if rest.starts_with("error\t") {
                "error".to_string()
            } else {
                rest.to_string()
            };
            (id.to_string(), answer)
        })
        .collect()
}

fn seed_answer(case: &Case) -> String {
    let compiled = std::panic::catch_unwind(|| Pattern::new(&case.pattern, false, false, false));
    let Ok(pattern) = compiled else {
        return "error".to_string();
    };
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| pattern.search(&case.text))) {
        Err(_) => "panic".to_string(),
        Ok(None) => "nomatch".to_string(),
        Ok(Some(found)) => {
            let spans: Vec<String> = found
                .spans
                .iter()
                .map(|group| match group {
                    Some((start, end)) => format!("{start}-{end}"),
                    None => "-".to_string(),
                })
                .collect();
            format!("match\t{}", spans.join("\t"))
        }
    }
}

#[test]
fn the_seed_agrees_with_ruby_on_the_onigmo_corpus() {
    std::panic::set_hook(Box::new(|_| {}));
    let cases = read_cases();
    let ruby = read_ruby_answers();
    let mut agreeing = 0;
    let mut disagreements = Vec::new();
    for case in &cases {
        let expected = &ruby[&case.id];
        let actual = if HANGS.contains(&case.id.as_str()) {
            "hang".to_string()
        } else {
            seed_answer(case)
        };
        if &actual == expected {
            agreeing += 1;
        } else {
            disagreements.push(format!(
                "{}  /{}/ on {:?}\n      ruby: {}\n      seed: {}",
                case.id,
                case.pattern,
                case.text,
                expected.replace('\t', " "),
                actual.replace('\t', " ")
            ));
        }
    }
    let _ = std::panic::take_hook();
    eprintln!(
        "onigmo corpus: {agreeing} of {} agree with Ruby 4.0.9 ({} disagree, listed below)",
        cases.len(),
        disagreements.len()
    );
    for line in &disagreements {
        eprintln!("{line}");
    }
    assert!(
        agreeing >= AGREEING_CASES,
        "the ratchet slipped: {agreeing} cases agree with Ruby, the pin is {AGREEING_CASES}"
    );
    if agreeing > AGREEING_CASES {
        eprintln!("raise AGREEING_CASES to {agreeing}");
    }
}
