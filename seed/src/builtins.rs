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
//! block-taking method: `include?/1`, `first/0-1`, `each/0&`.

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
            // `  in ["array", *] then %w[all?/0& compact/0 …]`
            let trimmed = line.trim_start();
            let Some(rest) = trimmed.strip_prefix("in [\"") else {
                continue;
            };
            let Some((type_tag, rest)) = rest.split_once('"') else {
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
                let (low, high) = match arity.split_once('-') {
                    Some((low, high)) => (low.parse().unwrap_or(0), high.parse().unwrap_or(0)),
                    None => {
                        let count = arity.parse().unwrap_or(0);
                        (count, count)
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
}
