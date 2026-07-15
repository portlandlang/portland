//! The Portland lexer.
//!
//! Grown incrementally; Prism's C lexer is the textbook for the hard parts
//! (heredocs, regex-vs-division, interpolation) when we get to them.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokenKind {
    Identifier,
    Integer,
    Newline,
    String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Token<'source> {
    pub kind: TokenKind,
    pub text: &'source str,
}

pub fn lex(source: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let mut chars = source.char_indices().peekable();

    while let Some(&(start, character)) = chars.peek() {
        match character {
            ' ' | '\t' => {
                chars.next();
            }
            '\n' => {
                chars.next();
                tokens.push(Token {
                    kind: TokenKind::Newline,
                    text: &source[start..start + 1],
                });
            }
            '0'..='9' => {
                let end = scan_while(&mut chars, |c| c.is_ascii_digit());
                tokens.push(Token {
                    kind: TokenKind::Integer,
                    text: &source[start..end],
                });
            }
            '"' => {
                chars.next();
                // No escapes or interpolation yet — Prism's lexer is the textbook for those.
                scan_while(&mut chars, |c| c != '"');
                let Some((closing, _)) = chars.next() else {
                    panic!("unterminated string starting at byte {start}");
                };
                tokens.push(Token {
                    kind: TokenKind::String,
                    text: &source[start..=closing],
                });
            }
            'a'..='z' | 'A'..='Z' | '_' => {
                let mut end = scan_while(&mut chars, |c| c.is_ascii_alphanumeric() || c == '_');
                // Ruby-surface joy, kept: a trailing `?` or `!` is part of the name.
                if let Some(&(index, suffix)) = chars.peek()
                    && (suffix == '?' || suffix == '!')
                {
                    end = index + suffix.len_utf8();
                    chars.next();
                }
                tokens.push(Token {
                    kind: TokenKind::Identifier,
                    text: &source[start..end],
                });
            }
            _ => panic!("unexpected character {character:?} at byte {start}"),
        }
    }

    tokens
}

/// Consume characters while `keep` holds; return the byte offset just past the last one.
fn scan_while(
    chars: &mut std::iter::Peekable<std::str::CharIndices>,
    keep: impl Fn(char) -> bool,
) -> usize {
    let mut end = 0;
    while let Some(&(index, character)) = chars.peek() {
        if !keep(character) {
            return index;
        }
        end = index + character.len_utf8();
        chars.next();
    }
    end
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(source: &str) -> Vec<TokenKind> {
        lex(source).iter().map(|token| token.kind).collect()
    }

    fn texts(source: &str) -> Vec<&str> {
        lex(source).iter().map(|token| token.text).collect()
    }

    #[test]
    fn lexes_an_integer_literal() {
        assert_eq!(kinds("42"), vec![TokenKind::Integer]);
        assert_eq!(texts("42"), vec!["42"]);
    }

    #[test]
    fn skips_spaces_and_tabs() {
        assert_eq!(texts("  1 \t 2  "), vec!["1", "2"]);
    }

    #[test]
    fn lexes_identifiers() {
        assert_eq!(kinds("photos"), vec![TokenKind::Identifier]);
        assert_eq!(texts("user_count2"), vec!["user_count2"]);
    }

    #[test]
    fn lexes_question_and_bang_identifier_suffixes() {
        assert_eq!(texts("empty? save!"), vec!["empty?", "save!"]);
        assert_eq!(
            kinds("empty? save!"),
            vec![TokenKind::Identifier, TokenKind::Identifier]
        );
    }

    #[test]
    fn lexes_a_double_quoted_string() {
        assert_eq!(kinds(r#""hello""#), vec![TokenKind::String]);
        assert_eq!(texts(r#""hello portland""#), vec![r#""hello portland""#]);
    }

    #[test]
    #[should_panic(expected = "unterminated string")]
    fn panics_on_an_unterminated_string() {
        lex(r#""oops"#);
    }

    #[test]
    fn lexes_newlines_as_tokens() {
        assert_eq!(
            kinds("1\n2"),
            vec![TokenKind::Integer, TokenKind::Newline, TokenKind::Integer]
        );
    }
}
