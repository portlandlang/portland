//! The Portland lexer.
//!
//! Grown incrementally; Prism's C lexer is the textbook for the hard parts
//! (heredocs, regex-vs-division, interpolation) when we get to them.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokenKind {
    Comma,
    Dot,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Identifier,
    Integer,
    Keyword,
    LeftParen,
    Less,
    LessEqual,
    Minus,
    Newline,
    NotEqual,
    Percent,
    Plus,
    RightParen,
    Slash,
    Star,
    String,
}

/// The Stage 0 keyword set — grows as the subset does.
const KEYWORDS: [&str; 7] = ["def", "do", "else", "end", "false", "if", "true"];

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
            '#' => {
                // Comment runs to end of line; the newline itself still lexes.
                scan_while(&mut chars, |c| c != '\n');
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
            '(' | ')' | ',' | '.' | '+' | '-' | '*' | '/' | '%' => {
                let kind = match character {
                    ',' => TokenKind::Comma,
                    '.' => TokenKind::Dot,
                    '(' => TokenKind::LeftParen,
                    '-' => TokenKind::Minus,
                    '%' => TokenKind::Percent,
                    '+' => TokenKind::Plus,
                    ')' => TokenKind::RightParen,
                    '/' => TokenKind::Slash,
                    '*' => TokenKind::Star,
                    _ => unreachable!(),
                };
                chars.next();
                tokens.push(Token {
                    kind,
                    text: &source[start..start + character.len_utf8()],
                });
            }
            '=' | '<' | '>' | '!' => {
                chars.next();
                let followed_by_equal = matches!(chars.peek(), Some(&(_, '=')));
                let (kind, length) = match (character, followed_by_equal) {
                    ('=', true) => (TokenKind::EqualEqual, 2),
                    ('=', false) => (TokenKind::Equal, 1),
                    ('>', true) => (TokenKind::GreaterEqual, 2),
                    ('>', false) => (TokenKind::Greater, 1),
                    ('<', true) => (TokenKind::LessEqual, 2),
                    ('<', false) => (TokenKind::Less, 1),
                    ('!', true) => (TokenKind::NotEqual, 2),
                    ('!', false) => panic!("unexpected character '!' at byte {start}"),
                    _ => unreachable!(),
                };
                if length == 2 {
                    chars.next();
                }
                tokens.push(Token {
                    kind,
                    text: &source[start..start + length],
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
                // Ruby-surface joy, kept: a trailing `?` or `!` is part of the name —
                // unless `=` follows, so `x != 1` stays a comparison, not `x!` then `=`.
                if let Some(&(index, suffix)) = chars.peek()
                    && (suffix == '?' || suffix == '!')
                    && source.as_bytes().get(index + 1) != Some(&b'=')
                {
                    end = index + suffix.len_utf8();
                    chars.next();
                }
                let text = &source[start..end];
                let kind = if KEYWORDS.contains(&text) {
                    TokenKind::Keyword
                } else {
                    TokenKind::Identifier
                };
                tokens.push(Token { kind, text });
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
    fn lexes_keywords() {
        assert_eq!(
            kinds("def do end"),
            vec![TokenKind::Keyword, TokenKind::Keyword, TokenKind::Keyword]
        );
        assert_eq!(texts("def do end"), vec!["def", "do", "end"]);
    }

    #[test]
    fn keyword_lookalikes_stay_identifiers() {
        // `def?` and `ending` are plain identifiers, not keywords.
        assert_eq!(kinds("def?"), vec![TokenKind::Identifier]);
        assert_eq!(kinds("ending"), vec![TokenKind::Identifier]);
    }

    #[test]
    fn lexes_single_character_punctuation() {
        assert_eq!(
            kinds("( ) , . = +"),
            vec![
                TokenKind::LeftParen,
                TokenKind::RightParen,
                TokenKind::Comma,
                TokenKind::Dot,
                TokenKind::Equal,
                TokenKind::Plus,
            ]
        );
    }

    #[test]
    fn skips_comments_to_end_of_line() {
        assert_eq!(
            kinds("1 # the loneliest number\n2"),
            vec![TokenKind::Integer, TokenKind::Newline, TokenKind::Integer]
        );
        assert_eq!(texts("# only a comment"), Vec::<&str>::new());
    }

    #[test]
    fn lexes_arithmetic_operators() {
        assert_eq!(
            kinds("- * /"),
            vec![TokenKind::Minus, TokenKind::Star, TokenKind::Slash]
        );
    }

    #[test]
    fn lexes_comparison_operators() {
        assert_eq!(
            kinds("== != < <= > >="),
            vec![
                TokenKind::EqualEqual,
                TokenKind::NotEqual,
                TokenKind::Less,
                TokenKind::LessEqual,
                TokenKind::Greater,
                TokenKind::GreaterEqual,
            ]
        );
    }

    #[test]
    fn not_equal_wins_over_a_bang_identifier_suffix() {
        assert_eq!(
            kinds("x != 1"),
            vec![
                TokenKind::Identifier,
                TokenKind::NotEqual,
                TokenKind::Integer
            ]
        );
        assert_eq!(texts("x != 1"), vec!["x", "!=", "1"]);
    }

    #[test]
    fn lexes_condition_keywords() {
        assert_eq!(
            kinds("if true else false"),
            vec![
                TokenKind::Keyword,
                TokenKind::Keyword,
                TokenKind::Keyword,
                TokenKind::Keyword,
            ]
        );
    }

    #[test]
    fn lexes_a_method_call_line() {
        assert_eq!(
            kinds(r#"greet("world", 2)"#),
            vec![
                TokenKind::Identifier,
                TokenKind::LeftParen,
                TokenKind::String,
                TokenKind::Comma,
                TokenKind::Integer,
                TokenKind::RightParen,
            ]
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
