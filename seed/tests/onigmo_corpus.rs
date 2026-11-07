//! The seed's regex engine against Onigmo's own test suite, with Ruby
//! 4.0.9's answer for every case as the ground truth (ADR 0086: tie goes to
//! Ruby). See `fixtures/onigmo/README.md` for where the files come from.
//!
//! The count of agreeing cases is a ratchet: it may go up, never down. When
//! a build makes more cases agree, raise `AGREEING_CASES` to the new count.

use portland_seed::regex::Regex;
use std::path::Path;

const AGREEING_CASES: usize = 1075;
const STEP_BUDGET: u64 = 5_000_000;

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

/// Ruby's answer line without its id: `error <sentence>`, `nomatch`, or
/// `match <spans>`; an error sentence keeps only the part before `: /`,
/// which is where Ruby appends the source.
fn read_ruby_answers() -> std::collections::HashMap<String, String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/onigmo/ruby_4_0_9.tsv");
    let content = std::fs::read_to_string(path).expect("ruby_4_0_9.tsv");
    content
        .lines()
        .map(|line| {
            let (id, rest) = line.split_once('\t').expect("id");
            let answer = match rest.strip_prefix("error\t") {
                Some(sentence) => {
                    let bare = sentence.split(": /").next().unwrap_or(sentence);
                    format!("error\t{bare}")
                }
                None => rest.to_string(),
            };
            (id.to_string(), answer)
        })
        .collect()
}

fn seed_answer(case: &Case) -> String {
    match Regex::new(&case.pattern) {
        Err(error) => format!("error\t{}", error.message),
        Ok(regex) => match regex.search_bounded(&case.text, 0, STEP_BUDGET) {
            Err(error) => format!("budget\t{}", error.message),
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
                format!("match\t{}", spans.join("\t"))
            }
        },
    }
}

#[test]
fn the_seed_agrees_with_ruby_on_the_onigmo_corpus() {
    let cases = read_cases();
    let ruby = read_ruby_answers();
    let mut agreeing = 0;
    let mut disagreements = Vec::new();
    for case in &cases {
        let expected = &ruby[&case.id];
        let actual = seed_answer(case);
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
    let shown = disagreements.len().min(60);
    eprintln!(
        "onigmo corpus: {agreeing} of {} agree with Ruby 4.0.9 ({} disagree; first {shown} below)",
        cases.len(),
        disagreements.len()
    );
    for line in &disagreements[..shown] {
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
