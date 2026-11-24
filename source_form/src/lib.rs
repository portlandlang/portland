//! How a value is written back out as source (#507): what `inspect` and
//! `p` show for a String and a Symbol.
//!
//! One copy, on purpose. The seed prints with it, and so does the runtime
//! every native program is linked with (ADR 0095). Each once had its own,
//! and an escape added to one and not the other would have had a native
//! program and an interpreted one write a String differently.

/// A string as source: the closed escape set (docs/ruby/string-escapes.md)
/// written back out — `\\`, `\"`, `\n`, `\r`, `\t`, and `\#` where a `#`
/// would otherwise open an interpolation (before `{`, `$`, or `@`, as
/// Ruby's `inspect` escapes it) — and every other character raw, as Ruby's
/// `inspect` leaves a zero-width joiner or a flag's variation selector
/// alone. Rust's `{:?}` would write those as `\u{200d}`, which no Portland
/// literal can spell.
pub fn quoted(text: &str) -> String {
    let mut result = String::from("\"");
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            // Ruby's named control escapes, then every other character
            // that does not print — the controls and the two Unicode line
            // separators — as `\uXXXX`, so an inspected string is readable
            // whatever it holds (#345).
            '\u{7}' => result.push_str("\\a"),
            '\u{8}' => result.push_str("\\b"),
            '\u{b}' => result.push_str("\\v"),
            '\u{c}' => result.push_str("\\f"),
            '\u{1b}' => result.push_str("\\e"),
            '#' if matches!(characters.peek(), Some('{' | '$' | '@')) => result.push_str("\\#"),
            other if other.is_control() || matches!(other, '\u{2028}' | '\u{2029}') => {
                result.push_str(&format!("\\u{:04X}", other as u32));
            }
            other => result.push(other),
        }
    }
    result.push('"');
    result
}

/// How a symbol is written back out: `:paid`, or `:"odd key"` when the
/// name is not identifier-shaped.
/// A symbol as source: bare when its name is identifier-shaped — a
/// letter or underscore, then letters, digits, and underscores, with a
/// `?` or `!` allowed last — and quoted otherwise, so `:"9"` keeps its
/// quotes as Ruby's does.
pub fn symbol_source(name: &str) -> String {
    let mut characters = name.chars().enumerate().peekable();
    // A bare name is ASCII (#139), so a symbol with any other character
    // keeps its quotes — the spelling that lexes here, which Ruby's
    // bare `:ê` would not.
    let starts_like_a_name = matches!(
        characters.peek(),
        Some((_, character)) if character.is_ascii_alphabetic() || *character == '_'
    );
    let plain = starts_like_a_name
        && characters.all(|(index, character)| {
            character.is_ascii_alphanumeric()
                || character == '_'
                || (index + 1 == name.len() && matches!(character, '?' | '!'))
        });

    if plain {
        format!(":{name}")
    } else {
        format!(":{}", quoted(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_string_is_written_between_quotes_with_the_closed_escape_set() {
        let cases = [
            ("ada", "\"ada\""),
            ("say \"hi\"", "\"say \\\"hi\\\"\""),
            ("back\\slash", "\"back\\\\slash\""),
            ("line\nbreak\r\ttab", "\"line\\nbreak\\r\\ttab\""),
            (
                "caf\u{e9} \u{1F1FA}\u{1F1F8}",
                "\"caf\u{e9} \u{1F1FA}\u{1F1F8}\"",
            ),
            ("joined\u{200d}", "\"joined\u{200d}\""),
        ];
        for (text, source) in cases {
            assert_eq!(quoted(text), source, "{text:?}");
        }
    }

    /// The arms no Portland literal can reach, since it cannot spell the
    /// character (#304): the named controls, and `\uXXXX` for every other
    /// character that does not print.
    #[test]
    fn a_character_that_does_not_print_is_written_as_its_escape() {
        let cases = [
            ("\u{7}", "\"\\a\""),
            ("\u{8}", "\"\\b\""),
            ("\u{b}", "\"\\v\""),
            ("\u{c}", "\"\\f\""),
            ("\u{1b}", "\"\\e\""),
            ("\u{0}", "\"\\u0000\""),
            ("\u{7f}", "\"\\u007F\""),
            ("\u{85}", "\"\\u0085\""),
            ("\u{2028}\u{2029}", "\"\\u2028\\u2029\""),
        ];
        for (text, source) in cases {
            assert_eq!(quoted(text), source, "{text:?}");
        }
    }

    #[test]
    fn a_hash_is_escaped_only_where_it_would_open_an_interpolation() {
        assert_eq!(quoted("#{x} #$y #@z"), "\"\\#{x} \\#$y \\#@z\"");
        assert_eq!(quoted("# plain #1 #"), "\"# plain #1 #\"");
    }

    #[test]
    fn a_symbol_is_bare_when_its_name_is_identifier_shaped() {
        let cases = [
            ("paid", ":paid"),
            ("_private", ":_private"),
            ("ready?", ":ready?"),
            ("save!", ":save!"),
            ("A1", ":A1"),
            ("odd key", ":\"odd key\""),
            ("9lives", ":\"9lives\""),
            ("re?ady", ":\"re?ady\""),
            ("caf\u{e9}", ":\"caf\u{e9}\""),
            ("", ":\"\""),
        ];
        for (name, source) in cases {
            assert_eq!(symbol_source(name), source, "{name:?}");
        }
    }
}
