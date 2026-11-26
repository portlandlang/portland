//! The builtin method table (#88), read off `compiler/inference.pdx` at
//! build time so the fact has one home. The table is derived from this
//! seed's own behavior by `script/probe_builtin_methods` — the seed's
//! dispatch arms decide what a builtin answers; this table only lets the
//! catch-all *say* which of three things happened: no such method, a real
//! method with the wrong number of arguments, or a real method with
//! arguments it cannot take. `script/check_builtin_table` keeps the table
//! honest against the arms.
//!
//! An entry reads `name/arity` with an optional trailing `&` for a
//! block-taking method: `include?/1`, `first/0-1`, `each/0&`. A trailing
//! `+` on the arity takes any number from there: `values_at/0+`.

use std::sync::OnceLock;

const INFERENCE_SOURCE: &str = include_str!("../../compiler/inference.pdx");

struct Signature {
    type_tag: String,
    name: String,
    low: usize,
    high: usize,
}

fn table() -> &'static Vec<Signature> {
    static TABLE: OnceLock<Vec<Signature>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut signatures = Vec::new();
        for line in INFERENCE_SOURCE.lines() {
            // `  in ["string"] then %w[all?/0& compact/0 …]`, or for a kind
            // of type that is a struct (#593), `  in ArrayType then %w[…]`,
            // whose tag is the struct's name less `Type`, lowercased.
            let trimmed = line.trim_start();
            let (type_tag, rest) = if let Some(rest) = trimmed.strip_prefix("in [\"") {
                let Some((type_tag, rest)) = rest.split_once('"') else {
                    continue;
                };
                (type_tag.to_string(), rest)
            } else if let Some(rest) = trimmed.strip_prefix("in ") {
                let Some((name, rest)) = rest.split_once("Type ") else {
                    continue;
                };
                if name.is_empty() || !name.chars().all(char::is_alphanumeric) {
                    continue;
                }
                (name.to_lowercase(), rest)
            } else {
                continue;
            };
            let Some((_, entries)) = rest.split_once("then %w[") else {
                continue;
            };
            let Some(entries) = entries.strip_suffix(']') else {
                continue;
            };
            for entry in entries.split_whitespace() {
                let Some((name, arity)) = entry.split_once('/') else {
                    continue;
                };
                let arity = arity.trim_end_matches('&');
                let (low, high) = if let Some(low) = arity.strip_suffix('+') {
                    (low.parse().unwrap_or(0), usize::MAX)
                } else {
                    match arity.split_once('-') {
                        Some((low, high)) => (low.parse().unwrap_or(0), high.parse().unwrap_or(0)),
                        None => {
                            let count = arity.parse().unwrap_or(0);
                            (count, count)
                        }
                    }
                };
                signatures.push(Signature {
                    type_tag: type_tag.to_string(),
                    name: name.to_string(),
                    low,
                    high,
                });
            }
        }
        signatures
    })
}

/// The positional arity a builtin receiver's method takes, as an
/// inclusive range — or `None` where the type has no such method.
pub fn signature(type_tag: &str, name: &str) -> Option<(usize, usize)> {
    table()
        .iter()
        .find(|entry| entry.type_tag == type_tag && entry.name == name)
        .map(|entry| (entry.low, entry.high))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_table_off_the_compiler_source() {
        assert_eq!(signature("string", "include?"), Some((1, 1)));
        assert_eq!(signature("array", "first"), Some((0, 1)));
        assert_eq!(signature("array", "each"), Some((0, 0)));
        assert_eq!(signature("string", "knd"), None);
        assert_eq!(signature("nil", "to_s"), None);
    }

    #[test]
    fn reads_a_trailing_plus_as_any_number_from_there() {
        assert_eq!(signature("array", "values_at"), Some((0, usize::MAX)));
    }
}
