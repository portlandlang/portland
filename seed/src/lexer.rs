//! The Portland lexer.
//!
//! Grown incrementally; Prism's C lexer is the textbook for the hard parts
//! (heredocs, regex-vs-division, interpolation) when we get to them.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokenKind {
    AmpersandAmpersand,
    AmpersandDot,
    Bang,
    Caret,
    Colon,
    /// `::` — reaches a name inside a namespace (ADR 0021).
    ColonColon,
    Comma,
    Dot,
    /// `..` inclusive and `...` exclusive (ADR 0019).
    DotDot,
    DotDotDot,
    Equal,
    EqualEqual,
    FatArrow,
    Float,
    Greater,
    GreaterEqual,
    Identifier,
    Integer,
    Keyword,
    LeftBrace,
    LeftBracket,
    LeftParen,
    Less,
    LessEqual,
    LessLess,
    Minus,
    MinusEqual,
    Newline,
    NotEqual,
    Percent,
    PercentEqual,
    Pipe,
    PipePipe,
    /// `||=` — or-equals, the compound family's last member (ADR 0043).
    PipePipeEqual,
    Plus,
    PlusEqual,
    /// `?` standing alone — the ternary's question mark (#83). A `?` glued
    /// to a name's tail stays part of the identifier (`ready?`).
    Question,
    RightBrace,
    RightBracket,
    RightParen,
    Slash,
    SlashEqual,
    Star,
    StarEqual,
    StarStar,
    String,
    /// `:name` — a symbol literal (ADR 0023). Its `text` includes the colon.
    Symbol,
    /// `%i[...]` — a symbol array (ADR 0051), `%w[]`'s content rules with
    /// symbols out. Its `text` is the raw source, delimiters included.
    SymbolArray,
    /// `~` — a task line inside `together` (ADRs 0002, 0029), and nothing
    /// else anywhere: ADR 0003 cut the bitwise readings that would have
    /// contested it.
    Tilde,
    WordArray,
}

/// The Stage 0 keyword set — grows as the subset does.
#[rustfmt::skip]
const KEYWORDS: [&str; 34] = [
    "alias",
    "and",
    "break",
    "case",
    "def",
    "do",
    "else",
    "elsif",
    "end",
    "enum",
    "false",
    "if",
    "in",
    "include",
    "loop",
    "meanwhile",
    "module",
    "mutable",
    "next",
    "nil",
    "not",
    "or",
    "return",
    "self",
    "struct",
    "then",
    "together",
    "trait",
    "true",
    "unless",
    "until",
    "when",
    "while",
    "yield",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Token<'source> {
    /// Whether a space or tab immediately precedes this token (set after the
    /// scan; the never-guess ambiguity errors need it).
    pub leading_space: bool,
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
                scan_while(&mut chars, |character| character != '\n');
            }
            '\n' => {
                chars.next();
                tokens.push(Token {
                    leading_space: false,
                    kind: TokenKind::Newline,
                    text: &source[start..start + 1],
                });
            }
            // A base prefix, either case (#72, ADR 0050): `0x` hexadecimal,
            // `0b` binary, `0o` octal. The token keeps the spelling; the
            // parser folds it. Ruby's `0d` is not taken, and its bare
            // leading-zero octal refuses below.
            '0' if source[start + 1..].starts_with(['x', 'X', 'b', 'B', 'o', 'O']) => {
                chars.next(); // the `0`
                chars.next(); // the base letter
                let mut end = start + 2;
                if chars.peek().is_some_and(|&(_, character)| {
                    character.is_ascii_alphanumeric() || character == '_'
                }) {
                    end = scan_while(&mut chars, |character| {
                        character.is_ascii_alphanumeric() || character == '_'
                    });
                }
                let text = &source[start..end];
                check_prefixed_literal(text);
                tokens.push(Token {
                    leading_space: false,
                    kind: TokenKind::Integer,
                    text,
                });
            }
            '0'..='9' => {
                // Underscores group digits, Ruby's separators (#71) — legal
                // only between digits, refused loose below.
                let mut end = scan_while(&mut chars, |character| {
                    character.is_ascii_digit() || character == '_'
                });
                // A `.` makes this a float only when a digit follows it
                // (ADR 0018). That keeps `1.upto` a method call and leaves
                // `1..5` for ranges (ADR 0019) — neither has a digit next.
                let mut kind = TokenKind::Integer;
                if source[end..].starts_with('.')
                    && source[end + 1..].starts_with(|character: char| character.is_ascii_digit())
                {
                    chars.next(); // the `.`
                    end = scan_while(&mut chars, |character| {
                        character.is_ascii_digit() || character == '_'
                    });
                    kind = TokenKind::Float;
                }
                let text = &source[start..end];
                if text.contains("__") || text.ends_with('_') || text.contains("_.") {
                    panic!("an underscore in a number sits between digits — {text} has one loose");
                }
                check_leading_zero(text, kind);
                tokens.push(Token {
                    leading_space: false,
                    kind,
                    text,
                });
            }
            // `..` and `...` before the single `.` (ADR 0019).
            '.' if source[start..].starts_with("..") => {
                let exclusive = source[start..].starts_with("...");
                let length = if exclusive { 3 } else { 2 };
                for _ in 0..length {
                    chars.next();
                }
                tokens.push(Token {
                    leading_space: false,
                    kind: if exclusive {
                        TokenKind::DotDotDot
                    } else {
                        TokenKind::DotDot
                    },
                    text: &source[start..start + length],
                });
            }
            // `:name` — a symbol literal (ADR 0023), before the single `:`.
            //
            // The colon binds **left** when it directly follows a name, and
            // **right** otherwise. That one rule separates a keyword-argument
            // label from a symbol without a never-guess error, and it gives
            // `name:"pdx"` Ruby's own reading — a label and a string, not a
            // name and a quoted symbol.
            ':' if !source[start..].starts_with("::")
                && !attached_to_a_name_on_the_left(source, start)
                && starts_a_symbol(&source[start + 1..]) =>
            {
                chars.next(); // the colon
                let length = 1 + symbol_length(&source[start + 1..]);
                for _ in 1..length {
                    chars.next();
                }
                tokens.push(Token {
                    leading_space: start > 0
                        && matches!(source.as_bytes()[start - 1], b' ' | b'\t'),
                    kind: TokenKind::Symbol,
                    text: &source[start..start + length],
                });
            }
            // `::` before the single `:` (ADR 0021).
            ':' if source[start..].starts_with("::") => {
                chars.next();
                chars.next();
                tokens.push(Token {
                    leading_space: false,
                    kind: TokenKind::ColonColon,
                    text: &source[start..start + 2],
                });
            }
            '(' | ')' | '{' | '}' | '[' | ']' | ',' | '.' | ':' => {
                let kind = match character {
                    ':' => TokenKind::Colon,
                    ',' => TokenKind::Comma,
                    '.' => TokenKind::Dot,
                    '{' => TokenKind::LeftBrace,
                    '[' => TokenKind::LeftBracket,
                    '(' => TokenKind::LeftParen,
                    '}' => TokenKind::RightBrace,
                    ']' => TokenKind::RightBracket,
                    ')' => TokenKind::RightParen,
                    _ => unreachable!(),
                };
                chars.next();
                tokens.push(Token {
                    leading_space: false,
                    kind,
                    text: &source[start..start + character.len_utf8()],
                });
            }
            // The `%` literal family (ADR 0051): `%w` and `%i`, with `[]`,
            // `()`, or `{}`. Every other member refuses by name below, and
            // a bare `%(` after a space is the string form, refused too.
            '%' if matches!(source.as_bytes().get(start + 1), Some(b'w' | b'i')) => {
                chars.next(); // the `%`
                let (_, letter) = chars.next().unwrap();
                let opener = match chars.next() {
                    Some((_, opener @ ('[' | '(' | '{'))) => opener,
                    Some((_, other)) => panic!(
                        "'%{letter}{other}' is not a Portland delimiter — write %{letter}[], %{letter}(), or %{letter}{{}}"
                    ),
                    None => panic!("unterminated %{letter} literal starting at byte {start}"),
                };
                let closer = closer_of(opener);
                // ADR 0030: a backslash escapes the next character, and
                // unescaped delimiters balance, so only an unescaped closer
                // at depth zero closes. Escapes stay raw here — the token
                // borrows the source, so unescaping is the parser's job.
                let mut depth = 0usize;
                let closing = loop {
                    match chars.next() {
                        None => panic!(
                            "unterminated %{letter}{opener}{closer} starting at byte {start}"
                        ),
                        Some((_, '\\')) => {
                            if chars.next().is_none() {
                                panic!(
                                    "unterminated %{letter}{opener}{closer} starting at byte {start}"
                                );
                            }
                        }
                        Some((_, character)) if character == opener => depth += 1,
                        Some((position, character)) if character == closer => {
                            if depth == 0 {
                                break position;
                            }
                            depth -= 1;
                        }
                        Some(_) => {}
                    }
                };
                tokens.push(Token {
                    leading_space: false,
                    kind: if letter == 'w' {
                        TokenKind::WordArray
                    } else {
                        TokenKind::SymbolArray
                    },
                    text: &source[start..=closing],
                });
            }
            '%' if declined_percent_literal(source, start) => {
                let spelling = &source[start..start + declined_percent_length(source, start)];
                panic!("{}", declined_percent_sentence(spelling));
            }
            '=' | '<' | '>' | '!' | '&' | '|' | '+' | '-' | '*' | '/' | '%' | '^' | '~' | '?' => {
                chars.next();
                let next = chars.peek().map(|&(_, following)| following);
                let (kind, length) = match (character, next) {
                    ('~', _) => (TokenKind::Tilde, 1),
                    // A `?` that is not a name's suffix (#83): the ternary's
                    // question mark. The identifier arm below claims `ready?`
                    // before this arm ever sees the character.
                    ('?', _) => (TokenKind::Question, 1),
                    ('&', Some('&')) => (TokenKind::AmpersandAmpersand, 2),
                    ('&', Some('.')) => (TokenKind::AmpersandDot, 2),
                    ('&', _) => panic!("unexpected character '&' at byte {start}"),
                    ('|', Some('|')) => {
                        if source[start..].starts_with("||=") {
                            (TokenKind::PipePipeEqual, 3)
                        } else {
                            (TokenKind::PipePipe, 2)
                        }
                    }
                    ('|', _) => (TokenKind::Pipe, 1),
                    ('=', Some('=')) => (TokenKind::EqualEqual, 2),
                    ('=', Some('>')) => (TokenKind::FatArrow, 2),
                    ('=', _) => (TokenKind::Equal, 1),
                    ('>', Some('=')) => (TokenKind::GreaterEqual, 2),
                    ('>', _) => (TokenKind::Greater, 1),
                    ('<', Some('=')) => (TokenKind::LessEqual, 2),
                    // `<<` exists only as the rebinding append (ADR 0015);
                    // bit-shift is out (ADR 0003), heredocs are future.
                    ('<', Some('<')) => (TokenKind::LessLess, 2),
                    ('<', _) => (TokenKind::Less, 1),
                    ('!', Some('=')) => (TokenKind::NotEqual, 2),
                    ('!', _) => (TokenKind::Bang, 1),
                    ('-', Some('=')) => (TokenKind::MinusEqual, 2),
                    ('-', _) => (TokenKind::Minus, 1),
                    ('%', Some('=')) => (TokenKind::PercentEqual, 2),
                    ('%', _) => (TokenKind::Percent, 1),
                    ('+', Some('=')) => (TokenKind::PlusEqual, 2),
                    ('+', _) => (TokenKind::Plus, 1),
                    ('/', Some('=')) => (TokenKind::SlashEqual, 2),
                    ('/', _) => (TokenKind::Slash, 1),
                    ('*', Some('=')) => (TokenKind::StarEqual, 2),
                    ('*', Some('*')) => (TokenKind::StarStar, 2),
                    ('*', _) => (TokenKind::Star, 1),
                    // `^` exists only as the pattern pin (ADR 0013 §4);
                    // bitwise xor is out (ADR 0003).
                    ('^', _) => (TokenKind::Caret, 1),
                    _ => unreachable!(),
                };
                for _ in 1..length {
                    chars.next();
                }
                tokens.push(Token {
                    leading_space: false,
                    kind,
                    text: &source[start..start + length],
                });
            }
            '"' => {
                chars.next();
                // Escapes and `#{...}` pass through raw here; the parser decodes
                // them. The lexer only has to keep the token boundary honest:
                // quotes inside an interpolation don't end the string.
                loop {
                    match chars.next() {
                        None => panic!("unterminated string starting at byte {start}"),
                        Some((_, '\\')) => {
                            if chars.next().is_none() {
                                panic!("unterminated string starting at byte {start}");
                            }
                        }
                        Some((_, '#')) if matches!(chars.peek(), Some(&(_, '{'))) => {
                            chars.next(); // the `{`
                            skip_interpolation(&mut chars, start);
                        }
                        Some((closing, '"')) => {
                            tokens.push(Token {
                                leading_space: false,
                                kind: TokenKind::String,
                                text: &source[start..=closing],
                            });
                            break;
                        }
                        Some(_) => {}
                    }
                }
            }
            '\'' => {
                chars.next();
                // Single-quoted: no interpolation; only \' and \\ mean anything,
                // and the parser handles that — the lexer just finds the end.
                loop {
                    match chars.next() {
                        None => panic!("unterminated string starting at byte {start}"),
                        Some((_, '\\')) => {
                            if chars.next().is_none() {
                                panic!("unterminated string starting at byte {start}");
                            }
                        }
                        Some((closing, '\'')) => {
                            tokens.push(Token {
                                leading_space: false,
                                kind: TokenKind::String,
                                text: &source[start..=closing],
                            });
                            break;
                        }
                        Some(_) => {}
                    }
                }
            }
            'a'..='z' | 'A'..='Z' | '_' => {
                let mut end = scan_while(&mut chars, |character| {
                    character.is_ascii_alphanumeric() || character == '_'
                });
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
                tokens.push(Token {
                    leading_space: false,
                    kind,
                    text,
                });
            }
            _ => panic!("unexpected character {character:?} at byte {start}"),
        }
    }

    // Every token's text is a slice of `source`, so its offset tells us what
    // came just before it.
    let source_start = source.as_ptr() as usize;
    for token in &mut tokens {
        let offset = token.text.as_ptr() as usize - source_start;
        token.leading_space = offset > 0 && matches!(source.as_bytes()[offset - 1], b' ' | b'\t');
    }

    tokens
}

/// Consume a `#{...}` interpolation body (opening brace already eaten),
/// tracking brace depth and nested string literals so the enclosing string
/// token ends at the right quote.
/// Is this colon glued to a name on its left, making it a label rather than
/// the head of a symbol? `name:` is a label; `{`, `(`, `,` or a space before
/// the colon means the name that follows belongs to it (ADR 0023).
fn attached_to_a_name_on_the_left(source: &str, colon: usize) -> bool {
    colon > 0
        && matches!(source.as_bytes()[colon - 1],
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'?' | b'!')
}

/// `:name` and `:"quoted"` only. Operator symbols (`:+`, `:[]`) are out —
/// `send`, `define_method` and `&:` were their whole job, and all three are
/// gone (ADR 0023 §2).
fn starts_a_symbol(after_colon: &str) -> bool {
    matches!(
        after_colon.as_bytes().first(),
        Some(b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'"')
    )
}

/// The characters after the colon, including a closing quote when quoted.
fn symbol_length(after_colon: &str) -> usize {
    if let Some(quoted) = after_colon.strip_prefix('"') {
        // No interpolation inside a symbol (ADR 0023 §2), so the first
        // closing quote ends it.
        return match quoted.find('"') {
            Some(end) => end + 2,
            None => panic!("unterminated quoted symbol"),
        };
    }

    let name = after_colon
        .find(|character: char| !character.is_alphanumeric() && character != '_')
        .unwrap_or(after_colon.len());
    // `?` and `!` are part of a name, so they are part of the symbol.
    match after_colon.as_bytes().get(name) {
        Some(b'?' | b'!') => name + 1,
        _ => name,
    }
}

fn skip_interpolation(chars: &mut std::iter::Peekable<std::str::CharIndices>, start: usize) {
    let mut depth = 1;
    while depth > 0 {
        match chars.next() {
            None => panic!("unterminated string starting at byte {start}"),
            Some((_, '{')) => depth += 1,
            Some((_, '}')) => depth -= 1,
            Some((_, '"')) => loop {
                match chars.next() {
                    None => panic!("unterminated string starting at byte {start}"),
                    Some((_, '\\')) => {
                        chars.next();
                    }
                    Some((_, '"')) => break,
                    Some(_) => {}
                }
            },
            Some(_) => {}
        }
    }
}

/// Consume characters while `keep` holds; return the byte offset just past the last one.
fn closer_of(opener: char) -> char {
    match opener {
        '[' => ']',
        '(' => ')',
        _ => '}',
    }
}

/// Whether a `%` at `start` begins one of the family's declined members
/// (ADR 0051): a letter in `qQsWIrx` followed by a delimiter character, or
/// a bare `%` before `[`, `(`, or `{` where a space (or the line's start)
/// precedes it — Ruby's own reading of `a %(b)` — so `a % (b)` and `a%(b)`
/// stay modulo.
fn declined_percent_literal(source: &str, start: usize) -> bool {
    let bytes = source.as_bytes();
    match bytes.get(start + 1) {
        Some(b'q' | b'Q' | b's' | b'W' | b'I' | b'r' | b'x') => {
            bytes.get(start + 2).is_some_and(|byte| {
                !byte.is_ascii_alphanumeric() && !byte.is_ascii_whitespace() && *byte != b'_'
            })
        }
        Some(b'[' | b'(' | b'{') => start == 0 || bytes[start - 1].is_ascii_whitespace(),
        _ => false,
    }
}

fn declined_percent_length(source: &str, start: usize) -> usize {
    if source.as_bytes()[start + 1].is_ascii_alphabetic() {
        3
    } else {
        2
    }
}

/// One sentence per declined member, in ADR 0047's voice: the spelling as
/// written, a dash, the Portland form.
pub fn declined_percent_sentence(spelling: &str) -> String {
    let next = match spelling.as_bytes()[1] {
        b'q' | b'Q' => "write a quoted string or a heredoc",
        b's' => "write :name, or :\"odd name\" for a name with spaces",
        b'W' => "write %w[] when no word interpolates, or [\"#{a}\", \"b\"] when one does",
        b'I' => "write %i[]; a symbol does not interpolate",
        b'r' => "there is no regex yet",
        b'x' => "there is no shell execution",
        _ => "write a quoted string or a heredoc",
    };
    format!("'{spelling}' is not a Portland literal — {next}")
}

/// A prefixed integer literal's digits must suit its base, sit at least one
/// deep, and keep their underscores between digits (#72, ADR 0050).
fn check_prefixed_literal(text: &str) {
    let (base, base_name, allowed) = match &text[1..2] {
        "x" | "X" => (16, "hexadecimal", "0-9 and a-f"),
        "b" | "B" => (2, "binary", "0 and 1"),
        _ => (8, "octal", "0-7"),
    };
    let digits = &text[2..];
    if digits.is_empty() {
        panic!("'{text}' has no digits after its prefix");
    }
    if digits.starts_with('_') || digits.contains("__") || digits.ends_with('_') {
        panic!("an underscore in a number sits between digits — {text} has one loose");
    }
    if digits
        .chars()
        .any(|character| character != '_' && !character.is_digit(base))
    {
        panic!("'{text}' has a digit outside its base — {base_name} digits are {allowed}");
    }
}

/// Ruby reads `017` as octal fifteen, a trap inherited from C (#72, ADR
/// 0050). Portland refuses the leading zero and names both readings.
fn check_leading_zero(text: &str, kind: TokenKind) {
    if !text.starts_with('0')
        || !text[1..].starts_with(|character: char| character.is_ascii_digit() || character == '_')
    {
        return;
    }
    let rest = text.trim_start_matches(['0', '_']);
    if kind == TokenKind::Float {
        let written = if rest.starts_with('.') {
            format!("0{rest}")
        } else {
            rest.to_string()
        };
        panic!("'{text}' has a leading zero — write {written}");
    }
    let rest = if rest.is_empty() { "0" } else { rest };
    panic!("'{text}' has a leading zero — write 0o{rest} for octal or {rest} for decimal");
}

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
    fn underscores_group_digits_in_numbers() {
        assert_eq!(kinds("5_280"), vec![TokenKind::Integer]);
        assert_eq!(texts("5_280"), vec!["5_280"]);
        assert_eq!(kinds("1_000.5_5"), vec![TokenKind::Float]);
        // `1_000.to_s` stays a method call: the dot has no digit after it.
        assert_eq!(
            kinds("1_000.to_s"),
            vec![TokenKind::Integer, TokenKind::Dot, TokenKind::Identifier]
        );
    }

    #[test]
    #[should_panic(
        expected = "an underscore in a number sits between digits — 5__280 has one loose"
    )]
    fn a_doubled_underscore_refuses() {
        lex("5__280");
    }

    #[test]
    #[should_panic(expected = "an underscore in a number sits between digits — 5_ has one loose")]
    fn a_trailing_underscore_refuses() {
        lex("5_ + 1");
    }

    #[test]
    #[should_panic(expected = "an underscore in a number sits between digits — 5_.5 has one loose")]
    fn an_underscore_against_the_dot_refuses() {
        lex("5_.5");
    }

    #[test]
    fn a_base_prefix_lexes_as_one_integer() {
        assert_eq!(kinds("0xff"), vec![TokenKind::Integer]);
        assert_eq!(texts("0xff"), vec!["0xff"]);
        assert_eq!(
            texts("0XFF_FF 0b1010 0o17 0B1 0O7"),
            vec!["0XFF_FF", "0b1010", "0o17", "0B1", "0O7"]
        );
        // `0.5` and `0` are not prefixed, and `0.upto` stays a call.
        assert_eq!(kinds("0.5"), vec![TokenKind::Float]);
        assert_eq!(texts("0"), vec!["0"]);
        assert_eq!(
            kinds("0.upto"),
            vec![TokenKind::Integer, TokenKind::Dot, TokenKind::Identifier]
        );
    }

    #[test]
    #[should_panic(expected = "'017' has a leading zero — write 0o17 for octal or 17 for decimal")]
    fn a_leading_zero_refuses() {
        lex("017");
    }

    #[test]
    #[should_panic(expected = "'01.5' has a leading zero — write 1.5")]
    fn a_leading_zero_on_a_float_refuses() {
        lex("01.5");
    }

    #[test]
    #[should_panic(expected = "'0x' has no digits after its prefix")]
    fn a_bare_prefix_refuses() {
        lex("0x + 1");
    }

    #[test]
    #[should_panic(expected = "'0b102' has a digit outside its base — binary digits are 0 and 1")]
    fn a_digit_outside_the_base_refuses() {
        lex("0b102");
    }

    #[test]
    #[should_panic(
        expected = "an underscore in a number sits between digits — 0x_ff has one loose"
    )]
    fn an_underscore_against_the_prefix_refuses() {
        lex("0x_ff");
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
    fn lexes_word_arrays() {
        assert_eq!(kinds("%w[rose city]"), vec![TokenKind::WordArray]);
        assert_eq!(texts("%w[rose city]"), vec!["%w[rose city]"]);
        // `%` alone is still modulo.
        assert_eq!(
            kinds("1 % 2"),
            vec![TokenKind::Integer, TokenKind::Percent, TokenKind::Integer]
        );
        // ADR 0030: a backslash escapes the next character raw — the token
        // borrows the source, so unescaping waits for the parser — and
        // unescaped brackets balance instead of closing.
        assert_eq!(texts(r"%w[a \] b]"), vec![r"%w[a \] b]"]);
        assert_eq!(texts("%w[a [b] c]"), vec!["%w[a [b] c]"]);
        assert_eq!(
            kinds(r"%w[\]] * 2"),
            vec![TokenKind::WordArray, TokenKind::Star, TokenKind::Integer]
        );
    }

    #[test]
    fn lexes_the_percent_family_with_three_delimiters() {
        // ADR 0051: `%i` beside `%w`, each with `[]`, `()`, or `{}`, the
        // pair in use being the one that balances.
        assert_eq!(kinds("%i[rose city]"), vec![TokenKind::SymbolArray]);
        assert_eq!(texts("%w(a (b) c)"), vec!["%w(a (b) c)"]);
        assert_eq!(texts("%i{a {b} c}"), vec!["%i{a {b} c}"]);
        assert_eq!(texts("%w(a ] b)"), vec!["%w(a ] b)"]);
        // Modulo survives wherever a space or a name precedes the `%`
        // without a literal opener glued on.
        assert_eq!(
            kinds("10 % (3)"),
            vec![
                TokenKind::Integer,
                TokenKind::Percent,
                TokenKind::LeftParen,
                TokenKind::Integer,
                TokenKind::RightParen
            ]
        );
        assert_eq!(kinds("10%(3)").first(), Some(&TokenKind::Integer));
        assert_eq!(kinds("10%(3)").get(1), Some(&TokenKind::Percent));
    }

    #[test]
    #[should_panic(expected = "'%w|' is not a Portland delimiter — write %w[], %w(), or %w{}")]
    fn another_delimiter_refuses() {
        lex("%w|a b|");
    }

    #[test]
    #[should_panic(
        expected = "'%q(' is not a Portland literal — write a quoted string or a heredoc"
    )]
    fn the_q_string_refuses() {
        lex("x = %q(hi)");
    }

    #[test]
    #[should_panic(
        expected = "'%(' is not a Portland literal — write a quoted string or a heredoc"
    )]
    fn the_bare_string_refuses() {
        lex("x = %(hi)");
    }

    #[test]
    #[should_panic(
        expected = "'%s(' is not a Portland literal — write :name, or :\"odd name\" for a name with spaces"
    )]
    fn the_symbol_form_refuses() {
        lex("%s(name)");
    }

    #[test]
    #[should_panic(
        expected = "'%W[' is not a Portland literal — write %w[] when no word interpolates, or [\"#{a}\", \"b\"] when one does"
    )]
    fn the_interpolating_word_array_refuses() {
        lex("%W[a b]");
    }

    #[test]
    #[should_panic(
        expected = "'%I[' is not a Portland literal — write %i[]; a symbol does not interpolate"
    )]
    fn the_interpolating_symbol_array_refuses() {
        lex("%I[a b]");
    }

    #[test]
    #[should_panic(expected = "'%r{' is not a Portland literal — there is no regex yet")]
    fn the_regex_form_refuses() {
        lex("%r{a/b}");
    }

    #[test]
    #[should_panic(expected = "'%x(' is not a Portland literal — there is no shell execution")]
    fn the_shell_form_refuses() {
        lex("%x(ls)");
    }

    #[test]
    fn lexes_compound_assignment_operators() {
        assert_eq!(
            kinds("+= -= *= /= %="),
            vec![
                TokenKind::PlusEqual,
                TokenKind::MinusEqual,
                TokenKind::StarEqual,
                TokenKind::SlashEqual,
                TokenKind::PercentEqual,
            ]
        );
    }

    #[test]
    fn records_leading_whitespace() {
        let spaces: Vec<bool> = lex("foo -1")
            .iter()
            .map(|token| token.leading_space)
            .collect();
        // foo (line start), - (after a space), 1 (glued to the minus)
        assert_eq!(spaces, vec![false, true, false]);
        let spaced: Vec<bool> = lex("foo - 1")
            .iter()
            .map(|token| token.leading_space)
            .collect();
        assert_eq!(spaced, vec![false, true, true]);
    }

    #[test]
    fn lexes_struct_and_keyword_argument_labels() {
        assert_eq!(
            kinds("struct Token"),
            vec![TokenKind::Keyword, TokenKind::Identifier]
        );
        assert_eq!(
            kinds("kind: 1"),
            vec![TokenKind::Identifier, TokenKind::Colon, TokenKind::Integer]
        );
    }

    #[test]
    fn lexes_symbol_literals() {
        assert_eq!(kinds(":paid"), vec![TokenKind::Symbol]);
        // `?` and `!` are part of a name, so they are part of the symbol.
        assert_eq!(kinds(":paid? :ship!"), vec![TokenKind::Symbol; 2]);
        assert_eq!(kinds(":\"odd key\""), vec![TokenKind::Symbol]);
        assert_eq!(texts(":paid"), vec![":paid"]);
    }

    #[test]
    fn tells_a_symbol_from_a_keyword_argument_label() {
        // The colon binds left when it follows a name, right otherwise. That
        // one rule covers every real case without a never-guess error.
        assert_eq!(
            kinds("{name: :paid}"),
            vec![
                TokenKind::LeftBrace,
                TokenKind::Identifier,
                TokenKind::Colon,
                TokenKind::Symbol,
                TokenKind::RightBrace
            ]
        );
        // Ruby reads `name:"x"` as a label plus a string, and so do we:
        // the colon is attached to the name on its left.
        assert_eq!(
            kinds("{name:\"pdx\"}"),
            vec![
                TokenKind::LeftBrace,
                TokenKind::Identifier,
                TokenKind::Colon,
                TokenKind::String,
                TokenKind::RightBrace
            ]
        );
    }

    #[test]
    fn lexes_logical_operators() {
        assert_eq!(
            kinds("&& || !"),
            vec![
                TokenKind::AmpersandAmpersand,
                TokenKind::PipePipe,
                TokenKind::Bang,
            ]
        );
    }

    #[test]
    fn lexes_block_pipes() {
        assert_eq!(
            kinds("do |item|"),
            vec![
                TokenKind::Keyword,
                TokenKind::Pipe,
                TokenKind::Identifier,
                TokenKind::Pipe
            ]
        );
    }

    #[test]
    fn lexes_hash_punctuation() {
        assert_eq!(
            kinds("{ } =>"),
            vec![
                TokenKind::LeftBrace,
                TokenKind::RightBrace,
                TokenKind::FatArrow,
            ]
        );
        // `=>` must not lex as `=` then `>`.
        assert_eq!(texts("a => 1"), vec!["a", "=>", "1"]);
    }

    #[test]
    fn lexes_brackets() {
        assert_eq!(
            kinds("[1]"),
            vec![
                TokenKind::LeftBracket,
                TokenKind::Integer,
                TokenKind::RightBracket,
            ]
        );
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
    fn interpolation_keeps_the_string_token_whole() {
        // A quote inside `#{...}` must not end the string.
        let source = r##""#{"pdx".upcase} rules""##;
        assert_eq!(kinds(source), vec![TokenKind::String]);
        assert_eq!(texts(source), vec![source]);
    }

    #[test]
    fn escaped_quotes_stay_inside_the_string() {
        assert_eq!(texts(r#""a\"b""#), vec![r#""a\"b""#]);
        assert_eq!(kinds(r#""a\"b""#), vec![TokenKind::String]);
    }

    #[test]
    #[should_panic(expected = "unterminated string")]
    fn panics_on_a_string_ending_in_a_backslash() {
        lex(r#""oops\"#);
    }

    #[test]
    fn lexes_newlines_as_tokens() {
        assert_eq!(
            kinds("1\n2"),
            vec![TokenKind::Integer, TokenKind::Newline, TokenKind::Integer]
        );
    }
}
