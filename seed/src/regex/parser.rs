//! Pattern to tree: Onigmo's `regparse.c` for `ONIG_SYNTAX_RUBY`, run over
//! the text CRuby's preprocessing pass leaves. The tokenizer, the class
//! parser's state machine, the name readers, and the quantifier rules follow
//! the C function by function so the refusals land on the same patterns
//! with the same sentences.

use super::ast::{
    AnchorKind, AnchorNode, BackRefNode, CallNode, CharSet, ClassNode, EncloseKind, Node,
    QuantifierNode,
};
use super::error::{self, RegexError};
use super::unicode::{self, CTYPE_ASCII, CTYPE_DIGIT, CTYPE_SPACE, CTYPE_WORD, CTYPE_XDIGIT};

const MAX_REPEAT_NUM: u32 = 100_000;
const MAX_BACKREF_NUM: u32 = 1000;
const PARSE_DEPTH_LIMIT: usize = 4096;
const POSIX_BRACKET_CHECK_LIMIT_LENGTH: usize = 20;
const POSIX_BRACKET_NAME_MIN_LEN: usize = 4;

/// The options in force at a point of the pattern: the literal's flags plus
/// the character-range family `(?a)`, `(?u)`, and `(?d)` toggle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseOptions {
    pub ignore_case: bool,
    pub extend: bool,
    pub multiline: bool,
    pub ascii_range: bool,
    pub posix_bracket_all_range: bool,
    pub word_bound_all_range: bool,
}

impl ParseOptions {
    /// Ruby's defaults: `\w`, `\d`, `\s` ASCII-only; POSIX brackets and
    /// `\b` over all of Unicode.
    pub fn ruby(ignore_case: bool, extend: bool, multiline: bool) -> ParseOptions {
        ParseOptions {
            ignore_case,
            extend,
            multiline,
            ascii_range: true,
            posix_bracket_all_range: true,
            word_bound_all_range: true,
        }
    }
}

/// What the parse answers besides the tree.
#[derive(Debug)]
pub struct Parsed {
    pub root: Node,
    /// The number of capturing groups as parsed, before unnamed groups are
    /// dropped when named ones exist.
    pub num_mem: usize,
    pub num_named: usize,
    pub num_call: usize,
    /// Group names in definition order, each with its group numbers.
    pub names: Vec<(String, Vec<usize>)>,
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Eot,
    Alt,
    SubexpOpen,
    SubexpClose,
    /// One literal character as written.
    Char(char),
    /// A character an escape produced, which still joins a string.
    CodePoint(u32),
    /// `\xHH` or an octal escape: one byte of a character.
    RawByte(u8),
    AnyChar,
    CharType {
        ctype: u16,
        not: bool,
    },
    CharProperty {
        not: bool,
    },
    BackRef {
        groups: Vec<usize>,
        by_name: bool,
        level: Option<i32>,
    },
    Call {
        name: String,
        gnum: i64,
        rel: bool,
    },
    Anchor {
        kind: AnchorKind,
        ascii_range: bool,
    },
    Repeat {
        lower: u32,
        upper: Option<u32>,
        greedy: bool,
        possessive: bool,
        interval: bool,
    },
    CcOpen,
    Keep,
    LineBreak,
    ExtendedGraphemeCluster,
    // Inside a class only.
    CcClose,
    CcRange,
    CcCcOpen,
    CcAnd,
    PosixBracketOpen,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ClassState {
    Start,
    Value,
    Range,
    Complete,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ValueType {
    SingleByte,
    CodePoint,
    Class,
}

pub struct Parser<'a> {
    chars: &'a [char],
    pos: usize,
    option: ParseOptions,
    num_mem: usize,
    num_named: usize,
    num_call: usize,
    names: Vec<(String, Vec<usize>)>,
    parse_depth: usize,
}

type ParseResult<T> = Result<T, RegexError>;

pub fn parse(pattern: &str, options: ParseOptions) -> ParseResult<Parsed> {
    let chars: Vec<char> = pattern.chars().collect();
    let mut parser = Parser {
        chars: &chars,
        pos: 0,
        option: options,
        num_mem: 0,
        num_named: 0,
        num_call: 0,
        names: Vec::new(),
        parse_depth: 0,
    };
    let root = parser.parse_regexp()?;
    Ok(Parsed {
        root,
        num_mem: parser.num_mem,
        num_named: parser.num_named,
        num_call: parser.num_call,
        names: parser.names,
    })
}

fn is_digit(character: char) -> bool {
    character.is_ascii_digit()
}

fn is_word(code: u32) -> bool {
    unicode::ctype_contains(CTYPE_WORD, code)
}

fn name_end_code(start: char) -> char {
    match start {
        '<' => '>',
        '\'' => '\'',
        '(' => ')',
        '{' => '}',
        _ => '\0',
    }
}

impl<'a> Parser<'a> {
    fn at_end(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn peek(&self) -> char {
        self.chars[self.pos]
    }

    fn peek_is(&self, character: char) -> bool {
        !self.at_end() && self.peek() == character
    }

    fn fetch(&mut self) -> char {
        let character = self.chars[self.pos];
        self.pos += 1;
        character
    }

    fn text(&self, start: usize, end: usize) -> String {
        self.chars[start..end.min(self.chars.len())]
            .iter()
            .collect()
    }

    /// `onig_scan_unsigned_number`: decimal digits; `None` on overflow.
    fn scan_unsigned_number(&mut self) -> Option<u32> {
        let mut value: u64 = 0;
        let mut overflow = false;
        while !self.at_end() && is_digit(self.peek()) {
            let digit = u64::from(self.fetch() as u32 - '0' as u32);
            value = value * 10 + digit;
            if value > i32::MAX as u64 {
                overflow = true;
            }
        }
        if overflow { None } else { Some(value as u32) }
    }

    /// Up to `max` octal digits; `None` on overflow of an int.
    fn scan_unsigned_octal(&mut self, max: usize) -> Option<u32> {
        let mut value: u64 = 0;
        let mut count = 0;
        while count < max && !self.at_end() && ('0'..='7').contains(&self.peek()) {
            value = value * 8 + u64::from(self.fetch() as u32 - '0' as u32);
            count += 1;
            if value > i32::MAX as u64 {
                return None;
            }
        }
        Some(value as u32)
    }

    /// Up to `max` hex digits; `None` on overflow of an int.
    fn scan_unsigned_hex(&mut self, max: usize) -> Option<u32> {
        let mut value: u64 = 0;
        let mut count = 0;
        while count < max && !self.at_end() && self.peek().is_ascii_hexdigit() {
            value = value * 16 + u64::from(self.fetch().to_digit(16).expect("hex digit"));
            count += 1;
            if value > i32::MAX as u64 {
                return None;
            }
        }
        Some(value as u32)
    }

    // --- names ---------------------------------------------------------

    /// `fetch_name` for a group definition (`ref == 0`): a name that is not
    /// a number, read up to `end_code`.
    fn fetch_definition_name(&mut self, start_code: char) -> ParseResult<String> {
        let end_code = name_end_code(start_code);
        let start = self.pos;
        if self.at_end() {
            return Err(RegexError::new(error::EMPTY_GROUP_NAME));
        }
        let first = self.fetch();
        if first == end_code {
            return Err(RegexError::new(error::EMPTY_GROUP_NAME));
        }
        let mut failure: Option<&str> = None;
        if is_digit(first) || first == '-' {
            failure = Some(error::INVALID_GROUP_NAME);
        }
        let mut name_end = self.chars.len();
        let mut last = first;
        if failure.is_none() {
            while !self.at_end() {
                name_end = self.pos;
                last = self.fetch();
                if last == end_code || last == ')' {
                    break;
                }
            }
            if last != end_code {
                return Err(RegexError::named(
                    error::INVALID_GROUP_NAME,
                    &self.text(start, self.chars.len()),
                ));
            }
            return Ok(self.text(start, name_end));
        }
        // Teardown: read on to the terminator so the sentence names the
        // whole attempted name.
        let mut teardown_end = self.chars.len();
        while !self.at_end() {
            teardown_end = self.pos;
            let character = self.fetch();
            if character == end_code || character == ')' {
                break;
            }
        }
        if self.at_end() {
            teardown_end = self.chars.len();
        }
        Err(RegexError::named(
            failure.expect("set above"),
            &self.text(start, teardown_end),
        ))
    }

    /// `fetch_name` for a reference (`ref == 1`): a number, a negative
    /// number, or a name, read up to `end_code`. Answers the name text and
    /// the number (0 for a name).
    fn fetch_reference_name(&mut self, start_code: char) -> ParseResult<(String, i64)> {
        let end_code = name_end_code(start_code);
        let start = self.pos;
        if self.at_end() {
            return Err(RegexError::new(error::EMPTY_GROUP_NAME));
        }
        let first = self.fetch();
        if first == end_code {
            return Err(RegexError::new(error::EMPTY_GROUP_NAME));
        }
        // is_num: 0 a name, 1 digits seen, 2 a sign seen and no digit yet.
        let mut is_num = 0;
        let mut sign: i64 = 1;
        let mut number_start = start;
        let mut failure: Option<&str> = None;
        if is_digit(first) {
            is_num = 1;
        } else if first == '-' {
            is_num = 2;
            sign = -1;
            number_start = self.pos;
        }
        let mut name_end = self.chars.len();
        let mut last = first;
        if failure.is_none() {
            while !self.at_end() {
                name_end = self.pos;
                last = self.fetch();
                if last == end_code || last == ')' {
                    if is_num == 2 {
                        failure = Some(error::INVALID_GROUP_NAME);
                    }
                    break;
                }
                if is_num != 0 {
                    if is_digit(last) {
                        is_num = 1;
                    } else {
                        failure = Some(if is_word(last as u32) {
                            error::INVALID_GROUP_NAME
                        } else {
                            error::INVALID_CHAR_IN_GROUP_NAME
                        });
                        break;
                    }
                }
            }
        }
        if failure.is_none() {
            if last != end_code {
                return Err(RegexError::named(
                    error::INVALID_GROUP_NAME,
                    &self.text(start, self.chars.len()),
                ));
            }
            if is_num != 0 {
                let digits: String = self.text(number_start, name_end);
                let value: i64 = match digits.parse::<u32>() {
                    Ok(value) if value <= i32::MAX as u32 => i64::from(value),
                    _ => return Err(RegexError::new(error::TOO_BIG_NUMBER)),
                };
                if value == 0 {
                    return Err(RegexError::named(
                        error::INVALID_GROUP_NAME,
                        &self.text(start, name_end),
                    ));
                }
                return Ok((self.text(start, name_end), value * sign));
            }
            return Ok((self.text(start, name_end), 0));
        }
        let mut teardown_end = self.chars.len();
        while !self.at_end() {
            teardown_end = self.pos;
            let character = self.fetch();
            if character == end_code || character == ')' {
                break;
            }
        }
        if self.at_end() {
            teardown_end = self.chars.len();
        }
        Err(RegexError::named(
            failure.expect("set above"),
            &self.text(start, teardown_end),
        ))
    }

    /// `fetch_name_with_level`: a backreference's name or number with an
    /// optional `+n` or `-n` recursion level.
    fn fetch_name_with_level(
        &mut self,
        start_code: char,
    ) -> ParseResult<(String, i64, Option<i32>)> {
        let end_code = name_end_code(start_code);
        let start = self.pos;
        if self.at_end() {
            return Err(RegexError::new(error::EMPTY_GROUP_NAME));
        }
        let first = self.fetch();
        if first == end_code {
            return Err(RegexError::new(error::EMPTY_GROUP_NAME));
        }
        let mut is_num = 0;
        let mut sign: i64 = 1;
        let mut number_start = start;
        let mut failure: Option<&str> = None;
        if is_digit(first) {
            is_num = 1;
        } else if first == '-' {
            is_num = 2;
            sign = -1;
            number_start = self.pos;
        }
        let mut name_end = self.chars.len();
        let mut last = first;
        while !self.at_end() {
            name_end = self.pos;
            last = self.fetch();
            if last == end_code || last == ')' || last == '+' || last == '-' {
                if is_num == 2 {
                    failure = Some(error::INVALID_GROUP_NAME);
                }
                break;
            }
            if is_num != 0 {
                if is_digit(last) {
                    is_num = 1;
                } else {
                    failure = Some(error::INVALID_GROUP_NAME);
                    is_num = 0;
                }
            }
        }
        let mut level: Option<i32> = None;
        if failure.is_none() && last != end_code {
            let mut ok = false;
            if last == '+' || last == '-' {
                let flag: i32 = if last == '-' { -1 } else { 1 };
                if self.at_end() {
                    failure = Some(error::INVALID_CHAR_IN_GROUP_NAME);
                } else {
                    let digit = self.fetch();
                    if is_digit(digit) {
                        self.pos -= 1;
                        let value = self
                            .scan_unsigned_number()
                            .ok_or_else(|| RegexError::new(error::TOO_BIG_NUMBER))?;
                        level = Some(value as i32 * flag);
                        if !self.at_end() {
                            let closer = self.fetch();
                            if closer == end_code {
                                ok = true;
                            }
                        }
                    }
                }
            }
            if !ok && failure.is_none() {
                failure = Some(error::INVALID_GROUP_NAME);
                name_end = self.chars.len();
            }
        }
        match failure {
            None => {
                if is_num != 0 {
                    let digits = self.text(number_start, name_end);
                    let value: i64 = match digits.parse::<u32>() {
                        Ok(value) if value <= i32::MAX as u32 => i64::from(value),
                        _ => return Err(RegexError::new(error::TOO_BIG_NUMBER)),
                    };
                    if value == 0 {
                        return Err(RegexError::named(
                            error::INVALID_GROUP_NAME,
                            &self.text(start, self.chars.len()),
                        ));
                    }
                    Ok((self.text(start, name_end), value * sign, level))
                } else {
                    Ok((self.text(start, name_end), 0, level))
                }
            }
            Some(sentence) => Err(RegexError::named(sentence, &self.text(start, name_end))),
        }
    }

    fn name_to_groups(&self, name: &str) -> Option<Vec<usize>> {
        self.names
            .iter()
            .find(|(candidate, _)| candidate == name)
            .map(|(_, groups)| groups.clone())
    }

    /// `BACKREF_REL_TO_ABS`.
    fn relative_to_absolute(&self, relative: i64) -> i64 {
        self.num_mem as i64 + 1 + relative
    }

    /// `fetch_named_backref_token`: after `\k<` or `\k'`.
    fn fetch_named_backref(&mut self, start_code: char) -> ParseResult<Token> {
        let (name, number, level) = self.fetch_name_with_level(start_code)?;
        if number != 0 {
            let mut absolute = number;
            if absolute < 0 {
                absolute = self.relative_to_absolute(absolute);
                if absolute <= 0 {
                    return Err(RegexError::new(error::INVALID_BACKREF));
                }
            }
            return Ok(Token::BackRef {
                groups: vec![absolute as usize],
                by_name: false,
                level,
            });
        }
        match self.name_to_groups(&name) {
            Some(groups) => Ok(Token::BackRef {
                groups,
                by_name: true,
                level,
            }),
            None => Err(RegexError::named(error::UNDEFINED_NAME_REFERENCE, &name)),
        }
    }

    // --- escapes -------------------------------------------------------

    /// `conv_backslash_value`.
    fn conv_backslash_value(character: char) -> u32 {
        match character {
            'n' => 10,
            't' => 9,
            'r' => 13,
            'f' => 12,
            'a' => 7,
            'b' => 8,
            'e' => 27,
            'v' => 11,
            _ => character as u32,
        }
    }

    /// `fetch_escaped_value`: `\M-x`, `\C-x`, `\cx`, or a named control.
    fn fetch_escaped_value(&mut self) -> ParseResult<u32> {
        if self.at_end() {
            return Err(RegexError::new(error::END_PATTERN_AT_ESCAPE));
        }
        let character = self.fetch();
        match character {
            'M' => {
                if self.at_end() {
                    return Err(RegexError::new(error::END_PATTERN_AT_META));
                }
                if self.fetch() != '-' {
                    return Err(RegexError::new(error::META_CODE_SYNTAX));
                }
                if self.at_end() {
                    return Err(RegexError::new(error::END_PATTERN_AT_META));
                }
                let mut code = self.fetch() as u32;
                if code == '\\' as u32 {
                    code = self.fetch_escaped_value()?;
                }
                Ok((code & 0xff) | 0x80)
            }
            'C' | 'c' => {
                if character == 'C' {
                    if self.at_end() {
                        return Err(RegexError::new(error::END_PATTERN_AT_CONTROL));
                    }
                    if self.fetch() != '-' {
                        return Err(RegexError::new(error::CONTROL_CODE_SYNTAX));
                    }
                }
                if self.at_end() {
                    return Err(RegexError::new(error::END_PATTERN_AT_CONTROL));
                }
                let mut code = self.fetch() as u32;
                if code == '?' as u32 {
                    return Ok(0o177);
                }
                if code == '\\' as u32 {
                    code = self.fetch_escaped_value()?;
                }
                Ok(code & 0x9f)
            }
            _ => Ok(Self::conv_backslash_value(character)),
        }
    }

    // --- quantifier reading -------------------------------------------

    /// `fetch_range_quantifier`: answers `Some((lower, upper, fixed))`, or
    /// `None` when the brace is a plain character.
    fn fetch_range_quantifier(&mut self) -> ParseResult<Option<(u32, Option<u32>, bool)>> {
        let start = self.pos;
        if self.at_end() {
            return Ok(None);
        }
        let low = match self.scan_unsigned_number() {
            Some(value) => value,
            None => return Err(RegexError::new(error::TOO_BIG_NUMBER_FOR_REPEAT_RANGE)),
        };
        if low > MAX_REPEAT_NUM {
            return Err(RegexError::new(error::TOO_BIG_NUMBER_FOR_REPEAT_RANGE));
        }
        let mut non_low = false;
        let mut low = low;
        if self.pos == start {
            low = 0;
            non_low = true;
        }
        if self.at_end() {
            self.pos = start;
            return Ok(None);
        }
        let mut fixed = false;
        let upper: Option<u32>;
        let separator = self.fetch();
        if separator == ',' {
            let before = self.pos;
            let up = match self.scan_unsigned_number() {
                Some(value) => value,
                None => return Err(RegexError::new(error::TOO_BIG_NUMBER_FOR_REPEAT_RANGE)),
            };
            if up > MAX_REPEAT_NUM {
                return Err(RegexError::new(error::TOO_BIG_NUMBER_FOR_REPEAT_RANGE));
            }
            if self.pos == before {
                if non_low {
                    self.pos = start;
                    return Ok(None);
                }
                upper = None;
            } else {
                upper = Some(up);
            }
        } else {
            if non_low {
                self.pos = start;
                return Ok(None);
            }
            self.pos -= 1;
            upper = Some(low);
            fixed = true;
        }
        if self.at_end() {
            self.pos = start;
            return Ok(None);
        }
        if self.fetch() != '}' {
            self.pos = start;
            return Ok(None);
        }
        if let Some(up) = upper
            && low > up
        {
            return Err(RegexError::new(
                error::UPPER_SMALLER_THAN_LOWER_IN_REPEAT_RANGE,
            ));
        }
        Ok(Some((low, upper, fixed)))
    }

    /// The `greedy_check` and `possessive_check` tails of `fetch_token`.
    fn finish_repeat(
        &mut self,
        lower: u32,
        upper: Option<u32>,
        interval: bool,
        fixed: bool,
    ) -> Token {
        let mut greedy = true;
        let mut possessive = false;
        if !fixed && self.peek_is('?') {
            self.pos += 1;
            greedy = false;
        } else if !interval && self.peek_is('+') {
            self.pos += 1;
            possessive = true;
        }
        Token::Repeat {
            lower,
            upper,
            greedy,
            possessive,
            interval,
        }
    }

    // --- the tokenizer -------------------------------------------------

    /// `fetch_token`.
    fn fetch_token(&mut self) -> ParseResult<Token> {
        loop {
            if self.at_end() {
                return Ok(Token::Eot);
            }
            let character = self.fetch();
            if character == '\\' {
                if self.at_end() {
                    return Err(RegexError::new(error::END_PATTERN_AT_ESCAPE));
                }
                let escaped = self.fetch();
                return self.escaped_token(escaped);
            }
            match character {
                '.' => return Ok(Token::AnyChar),
                '*' => return Ok(self.finish_repeat(0, None, false, false)),
                '+' => return Ok(self.finish_repeat(1, None, false, false)),
                '?' => return Ok(self.finish_repeat(0, Some(1), false, false)),
                '{' => {
                    return match self.fetch_range_quantifier()? {
                        Some((lower, upper, fixed)) => {
                            Ok(self.finish_repeat(lower, upper, true, fixed))
                        }
                        None => Ok(Token::Char('{')),
                    };
                }
                '|' => return Ok(Token::Alt),
                '(' => {
                    if self.peek_is('?')
                        && self.pos + 1 < self.chars.len()
                        && self.chars[self.pos + 1] == '#'
                    {
                        self.pos += 2;
                        loop {
                            if self.at_end() {
                                return Err(RegexError::new(error::END_PATTERN_IN_GROUP));
                            }
                            let comment = self.fetch();
                            if comment == '\\' {
                                if !self.at_end() {
                                    self.pos += 1;
                                }
                            } else if comment == ')' {
                                break;
                            }
                        }
                        continue;
                    }
                    return Ok(Token::SubexpOpen);
                }
                ')' => return Ok(Token::SubexpClose),
                '^' => {
                    return Ok(Token::Anchor {
                        kind: AnchorKind::BeginLine,
                        ascii_range: false,
                    });
                }
                '$' => {
                    return Ok(Token::Anchor {
                        kind: AnchorKind::EndLine,
                        ascii_range: false,
                    });
                }
                '[' => return Ok(Token::CcOpen),
                '#' if self.option.extend => {
                    while !self.at_end() {
                        if self.fetch() == '\n' {
                            break;
                        }
                    }
                    continue;
                }
                ' ' | '\t' | '\n' | '\r' | '\x0c' if self.option.extend => continue,
                _ => return Ok(Token::Char(character)),
            }
        }
    }

    /// The escape arm of `fetch_token`, after the backslash and `escaped`
    /// were read.
    fn escaped_token(&mut self, escaped: char) -> ParseResult<Token> {
        let token = match escaped {
            '*' | '+' | '?' | '{' | '|' | '(' | ')' => Token::Char(escaped),
            'w' => Token::CharType {
                ctype: CTYPE_WORD,
                not: false,
            },
            'W' => Token::CharType {
                ctype: CTYPE_WORD,
                not: true,
            },
            'b' => Token::Anchor {
                kind: AnchorKind::WordBound,
                ascii_range: self.option.ascii_range && !self.option.word_bound_all_range,
            },
            'B' => Token::Anchor {
                kind: AnchorKind::NotWordBound,
                ascii_range: self.option.ascii_range && !self.option.word_bound_all_range,
            },
            's' => Token::CharType {
                ctype: CTYPE_SPACE,
                not: false,
            },
            'S' => Token::CharType {
                ctype: CTYPE_SPACE,
                not: true,
            },
            'd' => Token::CharType {
                ctype: CTYPE_DIGIT,
                not: false,
            },
            'D' => Token::CharType {
                ctype: CTYPE_DIGIT,
                not: true,
            },
            'h' => Token::CharType {
                ctype: CTYPE_XDIGIT,
                not: false,
            },
            'H' => Token::CharType {
                ctype: CTYPE_XDIGIT,
                not: true,
            },
            'A' => Token::Anchor {
                kind: AnchorKind::BeginBuf,
                ascii_range: false,
            },
            'Z' => Token::Anchor {
                kind: AnchorKind::SemiEndBuf,
                ascii_range: false,
            },
            'z' => Token::Anchor {
                kind: AnchorKind::EndBuf,
                ascii_range: false,
            },
            'G' => Token::Anchor {
                kind: AnchorKind::BeginPosition,
                ascii_range: false,
            },
            'x' => {
                if self.at_end() {
                    Token::Char('x')
                } else if self.peek_is('{') {
                    // `\x{...}`: eight hex digits at most, a wide character.
                    let before = self.pos;
                    self.pos += 1;
                    let digits_start = self.pos;
                    let value = self
                        .scan_unsigned_hex(8)
                        .ok_or_else(|| RegexError::new(error::TOO_BIG_WIDE_CHAR_VALUE))?;
                    if !self.at_end() && self.peek().is_ascii_hexdigit() {
                        return Err(RegexError::new(error::TOO_LONG_WIDE_CHAR_VALUE));
                    }
                    if self.pos > digits_start && self.peek_is('}') {
                        self.pos += 1;
                        Token::CodePoint(value)
                    } else {
                        self.pos = before;
                        Token::Char('x')
                    }
                } else {
                    let value = self
                        .scan_unsigned_hex(2)
                        .ok_or_else(|| RegexError::new(error::TOO_BIG_NUMBER))?;
                    Token::RawByte(value as u8)
                }
            }
            '1'..='9' => {
                self.pos -= 1;
                let before = self.pos;
                let number = self.scan_unsigned_number();
                let backref = match number {
                    Some(value) if value <= MAX_BACKREF_NUM => {
                        value as usize <= self.num_mem || value <= 9
                    }
                    _ => false,
                };
                if backref {
                    let value = number.expect("checked") as usize;
                    Token::BackRef {
                        groups: vec![value],
                        by_name: false,
                        level: None,
                    }
                } else if escaped == '8' || escaped == '9' {
                    self.pos = before + 1;
                    Token::Char(escaped)
                } else {
                    self.pos = before;
                    self.octal_raw_byte(3)?
                }
            }
            '0' => self.octal_raw_byte(2)?,
            'k' => {
                if !self.at_end() {
                    let next = self.fetch();
                    if next == '<' || next == '\'' {
                        self.fetch_named_backref(next)?
                    } else {
                        self.pos -= 1;
                        Token::Char('k')
                    }
                } else {
                    Token::Char('k')
                }
            }
            'g' => {
                if !self.at_end() {
                    let next = self.fetch();
                    if next == '<' || next == '\'' {
                        let mut gnum: i64 = -1;
                        let mut rel = false;
                        let mut name = String::new();
                        if self.peek_is('0') {
                            self.pos += 1;
                            if self.peek_is(name_end_code(next)) {
                                self.pos += 1;
                                gnum = 0;
                            }
                        } else if self.peek_is('+') {
                            self.pos += 1;
                            rel = true;
                        }
                        if gnum < 0 {
                            let (read_name, number) = self.fetch_reference_name(next)?;
                            name = read_name;
                            gnum = number;
                        }
                        Token::Call { name, gnum, rel }
                    } else {
                        self.pos -= 1;
                        Token::Char('g')
                    }
                } else {
                    Token::Char('g')
                }
            }
            'p' | 'P' => {
                if self.peek_is('{') {
                    self.pos += 1;
                    let mut not = escaped == 'P';
                    if self.peek_is('^') {
                        self.pos += 1;
                        not = !not;
                    }
                    Token::CharProperty { not }
                } else {
                    Token::Char(escaped)
                }
            }
            'R' => Token::LineBreak,
            'X' => Token::ExtendedGraphemeCluster,
            'K' => Token::Keep,
            _ => {
                self.pos -= 1;
                let value = self.fetch_escaped_value()?;
                if value != escaped as u32 {
                    Token::CodePoint(value)
                } else {
                    Token::Char(escaped)
                }
            }
        };
        Ok(token)
    }

    /// The `\0` and `\ooo` arms: up to `max` more octal digits.
    fn octal_raw_byte(&mut self, max: usize) -> ParseResult<Token> {
        let value = self
            .scan_unsigned_octal(max)
            .ok_or_else(|| RegexError::new(error::TOO_BIG_NUMBER))?;
        if value > 0xff {
            return Err(RegexError::new(error::TOO_BIG_NUMBER));
        }
        Ok(Token::RawByte(value as u8))
    }

    /// `fetch_token_in_cc`.
    fn fetch_token_in_cc(&mut self) -> ParseResult<Token> {
        if self.at_end() {
            return Ok(Token::Eot);
        }
        let character = self.fetch();
        match character {
            ']' => Ok(Token::CcClose),
            '-' => Ok(Token::CcRange),
            '\\' => {
                if self.at_end() {
                    return Err(RegexError::new(error::END_PATTERN_AT_ESCAPE));
                }
                let escaped = self.fetch();
                let token = match escaped {
                    'w' => Token::CharType {
                        ctype: CTYPE_WORD,
                        not: false,
                    },
                    'W' => Token::CharType {
                        ctype: CTYPE_WORD,
                        not: true,
                    },
                    'd' => Token::CharType {
                        ctype: CTYPE_DIGIT,
                        not: false,
                    },
                    'D' => Token::CharType {
                        ctype: CTYPE_DIGIT,
                        not: true,
                    },
                    's' => Token::CharType {
                        ctype: CTYPE_SPACE,
                        not: false,
                    },
                    'S' => Token::CharType {
                        ctype: CTYPE_SPACE,
                        not: true,
                    },
                    'h' => Token::CharType {
                        ctype: CTYPE_XDIGIT,
                        not: false,
                    },
                    'H' => Token::CharType {
                        ctype: CTYPE_XDIGIT,
                        not: true,
                    },
                    'p' | 'P' => {
                        if self.peek_is('{') {
                            self.pos += 1;
                            let mut not = escaped == 'P';
                            if self.peek_is('^') {
                                self.pos += 1;
                                not = !not;
                            }
                            Token::CharProperty { not }
                        } else {
                            Token::Char(escaped)
                        }
                    }
                    'x' => {
                        if self.at_end() {
                            Token::Char('x')
                        } else if self.peek_is('{') {
                            let before = self.pos;
                            self.pos += 1;
                            let digits_start = self.pos;
                            let value = self
                                .scan_unsigned_hex(8)
                                .ok_or_else(|| RegexError::new(error::TOO_BIG_WIDE_CHAR_VALUE))?;
                            if !self.at_end() && self.peek().is_ascii_hexdigit() {
                                return Err(RegexError::new(error::TOO_LONG_WIDE_CHAR_VALUE));
                            }
                            if self.pos > digits_start && self.peek_is('}') {
                                self.pos += 1;
                                Token::CodePoint(value)
                            } else {
                                self.pos = before;
                                Token::Char('x')
                            }
                        } else {
                            let value = self
                                .scan_unsigned_hex(2)
                                .ok_or_else(|| RegexError::new(error::TOO_BIG_NUMBER))?;
                            Token::RawByte(value as u8)
                        }
                    }
                    '0'..='7' => {
                        self.pos -= 1;
                        self.octal_raw_byte(3)?
                    }
                    _ => {
                        // An escaped character is a value, never the `^`
                        // that negates or the `-` that ranges.
                        self.pos -= 1;
                        let value = self.fetch_escaped_value()?;
                        Token::CodePoint(value)
                    }
                };
                Ok(token)
            }
            '[' => {
                if self.peek_is(':') {
                    // A POSIX bracket only when `:]` comes before any `]`.
                    let after_colon = self.pos + 1;
                    if self.colon_bracket_follows(after_colon) {
                        self.pos += 1;
                        return Ok(Token::PosixBracketOpen);
                    }
                }
                Ok(Token::CcCcOpen)
            }
            '&' => {
                if self.peek_is('&') {
                    self.pos += 1;
                    Ok(Token::CcAnd)
                } else {
                    Ok(Token::Char('&'))
                }
            }
            _ => Ok(Token::Char(character)),
        }
    }

    /// `str_exist_check_with_esc(":]", ..., bad = ']')`: whether `:]` occurs
    /// from `from` before a bare `]`, honouring backslash escapes.
    fn colon_bracket_follows(&self, from: usize) -> bool {
        let mut index = from;
        let mut in_escape = false;
        while index < self.chars.len() {
            let character = self.chars[index];
            if in_escape {
                in_escape = false;
                index += 1;
                continue;
            }
            if character == ':' && index + 1 < self.chars.len() && self.chars[index + 1] == ']' {
                return true;
            }
            if character == ']' {
                return false;
            }
            if character == '\\' {
                in_escape = true;
            }
            index += 1;
        }
        false
    }

    // --- classes -------------------------------------------------------

    /// `add_ctype_to_cc`: a builtin type's ranges, negated if asked, and
    /// clipped to ASCII (or, negated, extended over all non-ASCII) when the
    /// ASCII-range option applies.
    fn add_ctype_to_set(set: &mut CharSet, ctype: u16, not: bool, ascii_range: bool) {
        let mut work = CharSet::from_ranges(unicode::ctype_ranges(ctype));
        if not {
            work = work.negated();
        }
        if ascii_range {
            if not {
                work.add_range(0x80, super::ast::LAST_CODE_POINT);
            } else {
                work = work.intersection(&CharSet::from_ranges(&[(0, 0x7f)]));
            }
        }
        *set = set.union(&work);
    }

    /// `fetch_char_property_to_ctype`: the name up to `}`.
    fn fetch_char_property_to_ctype(&mut self) -> ParseResult<u16> {
        let start = self.pos;
        let mut prev = self.pos;
        while !self.at_end() {
            prev = self.pos;
            let character = self.fetch();
            if character == '}' {
                let name = self.text(start, prev);
                return match unicode::property_ctype(&name) {
                    Some(ctype) => Ok(ctype),
                    None => Err(RegexError::named(error::INVALID_CHAR_PROPERTY_NAME, &name)),
                };
            }
            if character == '(' || character == ')' || character == '{' || character == '|' {
                break;
            }
        }
        Err(RegexError::named(
            error::INVALID_CHAR_PROPERTY_NAME,
            &self.text(start, prev),
        ))
    }

    /// `parse_posix_bracket`: after `[:`. Answers `Ok(true)` when it was
    /// one, `Ok(false)` when the text is not a POSIX bracket after all.
    fn parse_posix_bracket(&mut self, class: &mut ClassNode) -> ParseResult<bool> {
        const BRACKETS: [(&str, u16); 14] = [
            ("alnum", unicode::CTYPE_ALNUM),
            ("alpha", unicode::CTYPE_ALPHA),
            ("blank", unicode::CTYPE_BLANK),
            ("cntrl", unicode::CTYPE_CNTRL),
            ("digit", unicode::CTYPE_DIGIT),
            ("graph", unicode::CTYPE_GRAPH),
            ("lower", unicode::CTYPE_LOWER),
            ("print", unicode::CTYPE_PRINT),
            ("punct", unicode::CTYPE_PUNCT),
            ("space", unicode::CTYPE_SPACE),
            ("upper", unicode::CTYPE_UPPER),
            ("xdigit", unicode::CTYPE_XDIGIT),
            ("ascii", unicode::CTYPE_ASCII),
            ("word", unicode::CTYPE_WORD),
        ];
        let mut not = false;
        if self.peek_is('^') {
            self.pos += 1;
            not = true;
        }
        let remaining = self.chars.len() - self.pos;
        if remaining >= POSIX_BRACKET_NAME_MIN_LEN + 3 {
            let ascii_range = self.option.ascii_range && !self.option.posix_bracket_all_range;
            for (name, ctype) in BRACKETS {
                let name_chars: Vec<char> = name.chars().collect();
                if self.chars[self.pos..].starts_with(&name_chars) {
                    let after = self.pos + name_chars.len();
                    if !(self.chars[after..].starts_with(&[':', ']'])) {
                        return Err(RegexError::new(error::INVALID_POSIX_BRACKET_TYPE));
                    }
                    Self::add_ctype_to_set(&mut class.set, ctype, not, ascii_range);
                    if let Some(twin) = class.ascii_twin.as_mut()
                        && ctype != CTYPE_WORD
                        && ctype != CTYPE_ASCII
                        && !ascii_range
                    {
                        Self::add_ctype_to_set(&mut twin.set, ctype, not, ascii_range);
                    }
                    self.pos = after + 2;
                    return Ok(true);
                }
            }
        }
        // Not a POSIX bracket: look a little way for `:]`, which would make
        // it a misspelled one.
        let mut count = 0;
        let mut last = '\0';
        while !self.at_end() {
            last = self.peek();
            if last == ':' || last == ']' {
                break;
            }
            self.pos += 1;
            count += 1;
            if count > POSIX_BRACKET_CHECK_LIMIT_LENGTH {
                break;
            }
        }
        if last == ':' && !self.at_end() {
            self.pos += 1;
            if !self.at_end() && self.fetch() == ']' {
                return Err(RegexError::new(error::INVALID_POSIX_BRACKET_TYPE));
            }
        }
        Ok(false)
    }

    /// `parse_char_class`, after the opening `[`.
    fn parse_char_class(&mut self) -> ParseResult<ClassNode> {
        self.parse_depth += 1;
        if self.parse_depth > PARSE_DEPTH_LIMIT {
            return Err(RegexError::new(error::PARSE_DEPTH_LIMIT_OVER));
        }
        let result = self.parse_char_class_body();
        self.parse_depth -= 1;
        result
    }

    fn parse_char_class_body(&mut self) -> ParseResult<ClassNode> {
        let mut token = self.fetch_token_in_cc()?;
        let neg = if token == Token::Char('^') {
            token = self.fetch_token_in_cc()?;
            true
        } else {
            false
        };
        if token == Token::CcClose {
            // `[]` is empty unless a later `]` closes it, as in `[]a]`.
            if !self.code_exists_later(']') {
                return Err(RegexError::new(error::EMPTY_CHAR_CLASS));
            }
            token = Token::Char(']');
        }
        let mut class = ClassNode::new();
        if self.option.ignore_case {
            class.ascii_twin = Some(Box::new(ClassNode::new()));
        }
        let mut previous: Option<ClassNode> = None;
        let mut state = ClassState::Start;
        let mut value_type = ValueType::SingleByte;
        let mut pending: u32 = 0;
        let mut and_start = false;
        while token != Token::CcClose {
            let mut fetched: Option<Token> = None;
            match token.clone() {
                Token::Char(character) => {
                    let in_type = if (character as u32) < 0x80 {
                        ValueType::SingleByte
                    } else {
                        ValueType::CodePoint
                    };
                    Self::next_state_val(
                        &mut class,
                        &mut pending,
                        character as u32,
                        in_type,
                        &mut value_type,
                        &mut state,
                    )?;
                }
                Token::RawByte(byte) => {
                    // The preprocessing pass leaves only ASCII bytes.
                    if byte >= 0x80 {
                        return Err(RegexError::new(error::TOO_SHORT_MULTI_BYTE_STRING));
                    }
                    Self::next_state_val(
                        &mut class,
                        &mut pending,
                        u32::from(byte),
                        ValueType::SingleByte,
                        &mut value_type,
                        &mut state,
                    )?;
                }
                Token::CodePoint(code) => {
                    let in_type = if code < 0x80 {
                        ValueType::SingleByte
                    } else {
                        ValueType::CodePoint
                    };
                    Self::next_state_val(
                        &mut class,
                        &mut pending,
                        code,
                        in_type,
                        &mut value_type,
                        &mut state,
                    )?;
                }
                Token::PosixBracketOpen => {
                    let after_bracket = self.pos - 1;
                    if self.parse_posix_bracket(&mut class)? {
                        Self::next_state_class(&mut class, pending, &mut value_type, &mut state)?;
                    } else {
                        // Not a POSIX bracket after all: the `[` is itself,
                        // and reading goes on from just after it.
                        self.pos = after_bracket;
                        Self::next_state_val(
                            &mut class,
                            &mut pending,
                            '[' as u32,
                            ValueType::SingleByte,
                            &mut value_type,
                            &mut state,
                        )?;
                    }
                }
                Token::CharType { ctype, not } => {
                    Self::add_ctype_to_set(&mut class.set, ctype, not, self.option.ascii_range);
                    if let Some(twin) = class.ascii_twin.as_mut()
                        && ctype != CTYPE_WORD
                    {
                        Self::add_ctype_to_set(&mut twin.set, ctype, not, self.option.ascii_range);
                    }
                    Self::next_state_class(&mut class, pending, &mut value_type, &mut state)?;
                }
                Token::CharProperty { not } => {
                    let ctype = self.fetch_char_property_to_ctype()?;
                    Self::add_ctype_to_set(&mut class.set, ctype, not, false);
                    if let Some(twin) = class.ascii_twin.as_mut()
                        && ctype != CTYPE_ASCII
                    {
                        Self::add_ctype_to_set(&mut twin.set, ctype, not, false);
                    }
                    Self::next_state_class(&mut class, pending, &mut value_type, &mut state)?;
                }
                Token::CcRange => match state {
                    ClassState::Value => {
                        let next = self.fetch_token_in_cc()?;
                        if next == Token::CcClose || next == Token::CcAnd {
                            // `[x-]` and `[x-&&...]`: the dash is itself.
                            fetched = Some(next);
                            Self::next_state_val(
                                &mut class,
                                &mut pending,
                                '-' as u32,
                                ValueType::SingleByte,
                                &mut value_type,
                                &mut state,
                            )?;
                        } else {
                            if value_type == ValueType::Class {
                                return Err(RegexError::new(
                                    error::UNMATCHED_RANGE_SPECIFIER_IN_CHAR_CLASS,
                                ));
                            }
                            fetched = Some(next);
                            state = ClassState::Range;
                        }
                    }
                    ClassState::Start => {
                        // `[-xa]` is allowed.
                        let next = self.fetch_token_in_cc()?;
                        fetched = Some(next);
                        let _ = and_start;
                        Self::next_state_val(
                            &mut class,
                            &mut pending,
                            '-' as u32,
                            ValueType::SingleByte,
                            &mut value_type,
                            &mut state,
                        )?;
                    }
                    ClassState::Range => {
                        // `[!--x]` is allowed: the dash is the range's end.
                        Self::next_state_val(
                            &mut class,
                            &mut pending,
                            '-' as u32,
                            ValueType::SingleByte,
                            &mut value_type,
                            &mut state,
                        )?;
                    }
                    ClassState::Complete => {
                        let next = self.fetch_token_in_cc()?;
                        fetched = Some(next);
                        // `[a-b-]`, `[a-b-&&...]`, and `[0-9-a]` all read
                        // the dash as itself.
                        Self::next_state_val(
                            &mut class,
                            &mut pending,
                            '-' as u32,
                            ValueType::SingleByte,
                            &mut value_type,
                            &mut state,
                        )?;
                    }
                },
                Token::CcCcOpen => {
                    let nested = self.parse_char_class()?;
                    Self::merge_nested(&mut class, nested);
                }
                Token::CcAnd => {
                    if state == ClassState::Value {
                        Self::flush_value(&mut class, pending, value_type, &mut state);
                    }
                    and_start = true;
                    state = ClassState::Start;
                    match previous.take() {
                        Some(left) => {
                            previous = Some(Self::and_classes(left, std::mem::take(&mut class)));
                        }
                        None => {
                            previous = Some(std::mem::take(&mut class));
                        }
                    }
                    class = ClassNode::new();
                    if self.option.ignore_case {
                        class.ascii_twin = Some(Box::new(ClassNode::new()));
                    }
                }
                Token::Eot => return Err(RegexError::new(error::PREMATURE_END_OF_CHAR_CLASS)),
                _ => return Err(RegexError::new("internal parser error (bug)")),
            }
            token = match fetched {
                Some(next) => next,
                None => self.fetch_token_in_cc()?,
            };
        }
        if state == ClassState::Value {
            Self::flush_value(&mut class, pending, value_type, &mut state);
        }
        if let Some(left) = previous {
            class = Self::and_classes(left, class);
        }
        class.not = neg;
        if let Some(twin) = class.ascii_twin.as_mut() {
            twin.not = neg;
        }
        Ok(class)
    }

    /// `code_exist_check(']')`: whether an unescaped `]` comes later.
    fn code_exists_later(&self, wanted: char) -> bool {
        let mut index = self.pos;
        let mut in_escape = false;
        while index < self.chars.len() {
            let character = self.chars[index];
            if in_escape {
                in_escape = false;
            } else if character == '\\' {
                in_escape = true;
            } else if character == wanted {
                return true;
            }
            index += 1;
        }
        false
    }

    fn flush_value(
        class: &mut ClassNode,
        pending: u32,
        value_type: ValueType,
        state: &mut ClassState,
    ) {
        if value_type != ValueType::Class {
            class.set.add_code(pending);
            if let Some(twin) = class.ascii_twin.as_mut() {
                twin.set.add_code(pending);
            }
        }
        *state = ClassState::Value;
    }

    /// `next_state_val`.
    fn next_state_val(
        class: &mut ClassNode,
        pending: &mut u32,
        value: u32,
        in_type: ValueType,
        value_type: &mut ValueType,
        state: &mut ClassState,
    ) -> ParseResult<()> {
        match *state {
            ClassState::Value => {
                if *value_type != ValueType::Class {
                    class.set.add_code(*pending);
                    if let Some(twin) = class.ascii_twin.as_mut() {
                        twin.set.add_code(*pending);
                    }
                }
            }
            ClassState::Range => {
                let from = *pending;
                if from > value {
                    return Err(RegexError::new(error::EMPTY_RANGE_IN_CHAR_CLASS));
                }
                class.set.add_range(from, value);
                if let Some(twin) = class.ascii_twin.as_mut() {
                    twin.set.add_range(from, value);
                }
                *state = ClassState::Complete;
                *pending = value;
                *value_type = in_type;
                return Ok(());
            }
            ClassState::Complete | ClassState::Start => {}
        }
        *state = ClassState::Value;
        *pending = value;
        *value_type = in_type;
        Ok(())
    }

    /// `next_state_class`.
    fn next_state_class(
        class: &mut ClassNode,
        pending: u32,
        value_type: &mut ValueType,
        state: &mut ClassState,
    ) -> ParseResult<()> {
        if *state == ClassState::Range {
            return Err(RegexError::new(error::CHAR_CLASS_VALUE_AT_END_OF_RANGE));
        }
        if *state == ClassState::Value && *value_type != ValueType::Class {
            class.set.add_code(pending);
            if let Some(twin) = class.ascii_twin.as_mut() {
                twin.set.add_code(pending);
            }
        }
        *state = ClassState::Value;
        *value_type = ValueType::Class;
        Ok(())
    }

    /// `or_cclass` of a nested class into the one being built.
    fn merge_nested(class: &mut ClassNode, nested: ClassNode) {
        let nested_set = if nested.not {
            nested.set.negated()
        } else {
            nested.set.clone()
        };
        class.set = class.set.union(&nested_set);
        if let (Some(twin), Some(nested_twin)) =
            (class.ascii_twin.as_mut(), nested.ascii_twin.as_ref())
        {
            let nested_twin_set = if nested_twin.not {
                nested_twin.set.negated()
            } else {
                nested_twin.set.clone()
            };
            twin.set = twin.set.union(&nested_twin_set);
        }
    }

    /// `and_cclass`: the intersection of two classes being built.
    fn and_classes(left: ClassNode, right: ClassNode) -> ClassNode {
        let left_set = if left.not {
            left.set.negated()
        } else {
            left.set.clone()
        };
        let right_set = if right.not {
            right.set.negated()
        } else {
            right.set.clone()
        };
        let mut result = ClassNode::new();
        result.set = left_set.intersection(&right_set);
        if let (Some(left_twin), Some(right_twin)) =
            (left.ascii_twin.as_ref(), right.ascii_twin.as_ref())
        {
            let left_twin_set = if left_twin.not {
                left_twin.set.negated()
            } else {
                left_twin.set.clone()
            };
            let right_twin_set = if right_twin.not {
                right_twin.set.negated()
            } else {
                right_twin.set.clone()
            };
            let mut twin = ClassNode::new();
            twin.set = left_twin_set.intersection(&right_twin_set);
            result.ascii_twin = Some(Box::new(twin));
        }
        result
    }

    // --- groups --------------------------------------------------------

    fn add_mem_entry(&mut self) -> usize {
        self.num_mem += 1;
        self.num_mem
    }

    fn name_add(&mut self, name: String, regnum: usize) {
        match self
            .names
            .iter_mut()
            .find(|(candidate, _)| *candidate == name)
        {
            Some((_, groups)) => groups.push(regnum),
            None => self.names.push((name, vec![regnum])),
        }
    }

    /// `parse_enclose`, after `(`.
    fn parse_enclose(&mut self, term: &Token) -> ParseResult<EncloseResult> {
        if self.at_end() {
            return Err(RegexError::new(
                error::END_PATTERN_WITH_UNMATCHED_PARENTHESIS,
            ));
        }
        let mut option = self.option;
        let mut node: Node;
        if self.peek_is('?') {
            self.pos += 1;
            if self.at_end() {
                return Err(RegexError::new(error::END_PATTERN_IN_GROUP));
            }
            let character = self.fetch();
            match character {
                ':' => {
                    let token = self.fetch_token()?;
                    let body = self.parse_subexp(token, term)?;
                    return Ok(EncloseResult::Group(body));
                }
                '=' => node = Node::anchor(AnchorKind::PrecRead),
                '!' => node = Node::anchor(AnchorKind::PrecReadNot),
                '>' => node = Node::enclose(EncloseKind::StopBacktrack, Node::Empty),
                '~' => node = Node::enclose(EncloseKind::Absent, Node::Empty),
                '\'' => node = self.named_group('\'')?,
                '<' => {
                    if self.at_end() {
                        return Err(RegexError::new(
                            error::END_PATTERN_WITH_UNMATCHED_PARENTHESIS,
                        ));
                    }
                    let next = self.fetch();
                    if next == '=' {
                        node = Node::anchor(AnchorKind::LookBehind);
                    } else if next == '!' {
                        node = Node::anchor(AnchorKind::LookBehindNot);
                    } else {
                        self.pos -= 1;
                        node = self.named_group('<')?;
                    }
                }
                '(' => {
                    if self.at_end() {
                        return Err(RegexError::new(error::UNDEFINED_GROUP_OPTION));
                    }
                    let next = self.fetch();
                    let regnum: usize;
                    let by_name: bool;
                    if is_digit(next) {
                        self.pos -= 1;
                        let (_, number) = self.fetch_reference_name('(')?;
                        regnum = number.max(0) as usize;
                        by_name = false;
                    } else if next == '<' || next == '\'' {
                        let token = self.fetch_named_backref(next)?;
                        if !self.peek_is(')') {
                            return Err(RegexError::new(error::UNDEFINED_GROUP_OPTION));
                        }
                        self.pos += 1;
                        regnum = match token {
                            Token::BackRef { groups, .. } => groups[0],
                            _ => unreachable!("fetch_named_backref answers a backref"),
                        };
                        // Written with brackets, numbered or not, counts
                        // as a name reference (Onigmo's NST_NAME_REF).
                        by_name = true;
                    } else {
                        return Err(RegexError::new(error::INVALID_CONDITION_PATTERN));
                    }
                    node = Node::enclose(EncloseKind::Condition { regnum, by_name }, Node::Empty);
                }
                '-' | 'i' | 'm' | 'x' | 'a' | 'd' | 'u' | 's' | 'l' => {
                    let mut neg = false;
                    let mut current = character;
                    loop {
                        match current {
                            ':' | ')' => {}
                            '-' => neg = true,
                            'x' => option.extend = !neg,
                            'i' => option.ignore_case = !neg,
                            'm' => option.multiline = !neg,
                            'a' if !neg => {
                                option.ascii_range = true;
                                option.posix_bracket_all_range = false;
                                option.word_bound_all_range = false;
                            }
                            'u' if !neg => {
                                option.ascii_range = false;
                                option.posix_bracket_all_range = false;
                                option.word_bound_all_range = false;
                            }
                            'd' if !neg => {
                                option.ascii_range = true;
                                option.posix_bracket_all_range = true;
                                option.word_bound_all_range = true;
                            }
                            _ => return Err(RegexError::new(error::UNDEFINED_GROUP_OPTION)),
                        }
                        if current == ')' {
                            return Ok(EncloseResult::OptionOnly(option));
                        }
                        if current == ':' {
                            let previous = self.option;
                            self.option = option;
                            let token = match self.fetch_token() {
                                Ok(token) => token,
                                Err(failure) => {
                                    self.option = previous;
                                    return Err(failure);
                                }
                            };
                            let body = self.parse_subexp(token, term);
                            self.option = previous;
                            return Ok(EncloseResult::Node(body?));
                        }
                        if self.at_end() {
                            return Err(RegexError::new(error::END_PATTERN_IN_GROUP));
                        }
                        current = self.fetch();
                    }
                }
                _ => return Err(RegexError::new(error::UNDEFINED_GROUP_OPTION)),
            }
        } else {
            let regnum = self.add_mem_entry();
            node = Node::enclose(
                EncloseKind::Memory {
                    regnum,
                    named: false,
                },
                Node::Empty,
            );
        }
        let token = self.fetch_token()?;
        let target = self.parse_subexp(token, term)?;
        match &mut node {
            Node::Anchor(anchor) => anchor.target = Some(Box::new(target)),
            Node::Enclose(enclose) => {
                if let EncloseKind::Condition { .. } = enclose.kind {
                    // (?(cond)yes) becomes (?(cond)yes|empty).
                    *enclose.target = match target {
                        Node::Alt(branches) => Node::Alt(branches),
                        other => Node::Alt(vec![other, Node::Empty]),
                    };
                } else {
                    *enclose.target = target;
                }
            }
            _ => unreachable!("an enclose or anchor node"),
        }
        Ok(EncloseResult::Node(node))
    }

    /// `(?<name>...)` and `(?'name'...)`: the name read, the group numbered.
    fn named_group(&mut self, start_code: char) -> ParseResult<Node> {
        let name = self.fetch_definition_name(start_code)?;
        let regnum = self.add_mem_entry();
        self.name_add(name, regnum);
        self.num_named += 1;
        Ok(Node::enclose(
            EncloseKind::Memory {
                regnum,
                named: true,
            },
            Node::Empty,
        ))
    }

    // --- expressions ---------------------------------------------------

    /// `is_invalid_quantifier_target`.
    fn is_invalid_quantifier_target(node: &Node) -> bool {
        match node {
            Node::Anchor(_) => true,
            Node::Alt(branches) => branches.iter().any(Self::is_invalid_quantifier_target),
            _ => false,
        }
    }

    /// `popular_quantifier_num`: the index of `?`, `*`, `+`, `??`, `*?`,
    /// `+?`, or `None`.
    fn popular_quantifier_num(quantifier: &QuantifierNode) -> Option<usize> {
        match (quantifier.greedy, quantifier.lower, quantifier.upper) {
            (true, 0, Some(1)) => Some(0),
            (true, 0, None) => Some(1),
            (true, 1, None) => Some(2),
            (false, 0, Some(1)) => Some(3),
            (false, 0, None) => Some(4),
            (false, 1, None) => Some(5),
            _ => None,
        }
    }

    /// `set_quantifier` with `onig_reduce_nested_quantifier`: answers the
    /// node the quantifier becomes, or `None` when it is dropped (`{1,1}`).
    fn set_quantifier(mut quantifier: QuantifierNode, target: Node, group: bool) -> Option<Node> {
        if quantifier.lower == 1 && quantifier.upper == Some(1) {
            return Some(target);
        }
        match target {
            Node::Str { chars, ignore_case } if !group && chars.len() > 1 => {
                // `abc+` quantifies the last character only.
                let mut head = chars;
                let last = head.pop().expect("more than one");
                quantifier.target = Box::new(Node::str_from(vec![last], ignore_case));
                Some(Node::List(vec![
                    Node::str_from(head, ignore_case),
                    Node::Quantifier(quantifier),
                ]))
            }
            Node::Quantifier(inner) => {
                let nest_num = Self::popular_quantifier_num(&quantifier);
                let target_num = Self::popular_quantifier_num(&inner);
                match (target_num, nest_num) {
                    (Some(target_index), Some(nest_index)) => Some(Self::reduce_nested_quantifier(
                        quantifier,
                        inner,
                        target_index,
                        nest_index,
                    )),
                    (Some(1) | Some(2), None) => {
                        // (?:a*){n,m} and (?:a+){n,m} become {n,n}.
                        let mut outer = quantifier;
                        if let Some(upper) = outer.upper
                            && upper > 1
                            && outer.greedy
                        {
                            outer.upper = Some(if outer.lower == 0 { 1 } else { outer.lower });
                        }
                        outer.target = Box::new(Node::Quantifier(inner));
                        Some(Node::Quantifier(outer))
                    }
                    _ => {
                        quantifier.target = Box::new(Node::Quantifier(inner));
                        Some(Node::Quantifier(quantifier))
                    }
                }
            }
            other => {
                quantifier.target = Box::new(other);
                Some(Node::Quantifier(quantifier))
            }
        }
    }

    /// `ReduceTypeTable`, indexed by the inner (target) then the outer
    /// (nested) quantifier.
    fn reduce_nested_quantifier(
        mut outer: QuantifierNode,
        mut inner: QuantifierNode,
        target_index: usize,
        nest_index: usize,
    ) -> Node {
        #[derive(Clone, Copy)]
        enum Reduce {
            AsIs,
            Del,
            A,
            Aq,
            Qq,
            PQq,
        }
        use Reduce::*;
        const TABLE: [[Reduce; 6]; 6] = [
            [Del, A, A, Qq, Aq, AsIs],
            [Del, Del, Del, PQq, PQq, Del],
            [A, A, Del, AsIs, PQq, Del],
            [Del, Aq, Aq, Del, Aq, Aq],
            [Del, Del, Del, Del, Del, Del],
            // Ruby's copy of Onigmo keeps `(?:a+?)*` and `(?:a+?)+` as
            // written where upstream rewrites them.
            [AsIs, AsIs, AsIs, Aq, Aq, Del],
        ];
        match TABLE[target_index][nest_index] {
            Del => Node::Quantifier(inner),
            A => {
                outer.target = inner.target;
                outer.lower = 0;
                outer.upper = None;
                outer.greedy = true;
                Node::Quantifier(outer)
            }
            Aq => {
                outer.target = inner.target;
                outer.lower = 0;
                outer.upper = None;
                outer.greedy = false;
                Node::Quantifier(outer)
            }
            Qq => {
                outer.target = inner.target;
                outer.lower = 0;
                outer.upper = Some(1);
                outer.greedy = false;
                Node::Quantifier(outer)
            }
            PQq => {
                inner.lower = 1;
                inner.upper = None;
                inner.greedy = true;
                outer.target = Box::new(Node::Quantifier(inner));
                outer.lower = 0;
                outer.upper = Some(1);
                outer.greedy = false;
                Node::Quantifier(outer)
            }
            AsIs => {
                outer.target = Box::new(Node::Quantifier(inner));
                Node::Quantifier(outer)
            }
        }
    }

    /// `parse_exp`: one atom with its quantifiers. Answers the node and the
    /// token that follows it.
    fn parse_exp(&mut self, token: Token, term: &Token) -> ParseResult<(Node, Token)> {
        if &token == term {
            return Ok((Node::Empty, token));
        }
        let mut group = false;
        let mut node: Node;
        let mut next: Token;
        match token {
            Token::Alt | Token::Eot => return Ok((Node::Empty, token)),
            Token::SubexpOpen => match self.parse_enclose(&Token::SubexpClose)? {
                EncloseResult::Group(body) => {
                    group = true;
                    node = body;
                    next = self.fetch_token()?;
                }
                EncloseResult::OptionOnly(option) => {
                    // An isolated option reaches to the end of the enclosing
                    // group.
                    let previous = self.option;
                    self.option = option;
                    let first = match self.fetch_token() {
                        Ok(first) => first,
                        Err(failure) => {
                            self.option = previous;
                            return Err(failure);
                        }
                    };
                    let rest = self.parse_subexp(first, term);
                    self.option = previous;
                    let rest = rest?;
                    return Ok((rest, term.clone()));
                }
                EncloseResult::Node(enclosed) => {
                    node = enclosed;
                    next = self.fetch_token()?;
                }
            },
            Token::SubexpClose => return Err(RegexError::new(error::UNMATCHED_CLOSE_PARENTHESIS)),
            Token::LineBreak => {
                node = Self::node_linebreak();
                next = self.fetch_token()?;
            }
            Token::ExtendedGraphemeCluster => {
                node = Node::GraphemeCluster;
                next = self.fetch_token()?;
            }
            Token::Keep => {
                node = Node::anchor(AnchorKind::Keep);
                next = self.fetch_token()?;
            }
            Token::Char(character) => {
                let mut chars = vec![character];
                next = self.string_loop(&mut chars)?;
                node = Node::str_from(chars, self.option.ignore_case);
            }
            Token::RawByte(byte) => {
                if byte >= 0x80 {
                    return Err(RegexError::new(error::TOO_SHORT_MULTI_BYTE_STRING));
                }
                let mut chars = vec![char::from(byte)];
                next = self.string_loop(&mut chars)?;
                node = Node::str_from(chars, self.option.ignore_case);
            }
            Token::CodePoint(code) => {
                let character = char::from_u32(code)
                    .ok_or_else(|| RegexError::new(error::INVALID_CODE_POINT_VALUE))?;
                let mut chars = vec![character];
                next = self.string_loop(&mut chars)?;
                node = Node::str_from(chars, self.option.ignore_case);
            }
            Token::CharType { ctype, not } => {
                if ctype == CTYPE_WORD {
                    node = Node::Word {
                        not,
                        ascii_range: self.option.ascii_range,
                    };
                } else {
                    let mut class = ClassNode::new();
                    Self::add_ctype_to_set(&mut class.set, ctype, false, self.option.ascii_range);
                    class.not = not;
                    node = Node::Class(class);
                }
                next = self.fetch_token()?;
            }
            Token::CharProperty { not } => {
                let ctype = self.fetch_char_property_to_ctype()?;
                let mut class = ClassNode::new();
                Self::add_ctype_to_set(&mut class.set, ctype, false, false);
                class.not = not;
                if self.option.ignore_case {
                    let twin = if ctype == CTYPE_ASCII {
                        ClassNode::new()
                    } else {
                        class.clone()
                    };
                    class.ascii_twin = Some(Box::new(twin));
                    node = super::casefold::class_case_fold(class);
                } else {
                    node = Node::Class(class);
                }
                next = self.fetch_token()?;
            }
            Token::CcOpen => {
                let class = self.parse_char_class()?;
                if !class.not && class.set.single_code().is_some() {
                    let code = class.set.single_code().expect("checked");
                    let character = char::from_u32(code)
                        .ok_or_else(|| RegexError::new(error::INVALID_CODE_POINT_VALUE))?;
                    let mut chars = vec![character];
                    next = self.string_loop(&mut chars)?;
                    node = Node::str_from(chars, self.option.ignore_case);
                } else {
                    node = if self.option.ignore_case {
                        super::casefold::class_case_fold(class)
                    } else {
                        Node::Class(class)
                    };
                    next = self.fetch_token()?;
                }
            }
            Token::AnyChar => {
                node = Node::AnyChar {
                    multiline: self.option.multiline,
                };
                next = self.fetch_token()?;
            }
            Token::BackRef {
                groups,
                by_name,
                level,
            } => {
                node = Node::BackRef(BackRefNode {
                    groups,
                    by_name,
                    level,
                    ignore_case: self.option.ignore_case,
                });
                next = self.fetch_token()?;
            }
            Token::Call { name, gnum, rel } => {
                let mut group_number = gnum;
                if group_number < 0 || rel {
                    if group_number > 0 {
                        group_number -= 1;
                    }
                    group_number = self.relative_to_absolute(group_number);
                    if group_number <= 0 {
                        return Err(RegexError::new(error::INVALID_BACKREF));
                    }
                }
                let by_number = gnum != 0 || name.is_empty() || rel;
                node = Node::Call(CallNode {
                    name,
                    group: group_number as usize,
                    by_number,
                });
                self.num_call += 1;
                next = self.fetch_token()?;
            }
            Token::Anchor { kind, ascii_range } => {
                node = Node::Anchor(AnchorNode {
                    kind,
                    ascii_range,
                    target: None,
                    char_len: 0,
                });
                next = self.fetch_token()?;
            }
            Token::Repeat { .. } => {
                return Err(RegexError::new(
                    error::TARGET_OF_REPEAT_OPERATOR_NOT_SPECIFIED,
                ));
            }
            Token::CcClose
            | Token::CcRange
            | Token::CcCcOpen
            | Token::CcAnd
            | Token::PosixBracketOpen => {
                return Err(RegexError::new("internal parser error (bug)"));
            }
        }
        // Quantifiers, possibly stacked.
        let mut parse_depth = self.parse_depth;
        let mut quantified: Vec<Node> = Vec::new();
        while let Token::Repeat {
            lower,
            upper,
            greedy,
            possessive,
            interval,
        } = next
        {
            // Ruby's build defines USE_NO_INVALID_QUANTIFIER: an anchor may
            // be quantified, so `is_invalid_quantifier_target` never fires.
            let _ = Self::is_invalid_quantifier_target(&node);
            parse_depth += 1;
            if parse_depth > PARSE_DEPTH_LIMIT {
                return Err(RegexError::new(error::PARSE_DEPTH_LIMIT_OVER));
            }
            let quantifier = QuantifierNode::new(Node::Empty, lower, upper, greedy, interval);
            let applied = Self::set_quantifier(quantifier, node, group);
            let mut result = applied.expect("set_quantifier answers a node");
            if possessive {
                // Only the quantified tail is made atomic, so `abc++`
                // keeps `ab` as a plain prefix.
                result = match result {
                    Node::List(mut parts) if parts.len() == 2 => {
                        let tail = parts.pop().expect("two parts");
                        parts.push(Node::enclose(EncloseKind::StopBacktrack, tail));
                        Node::List(parts)
                    }
                    other => Node::enclose(EncloseKind::StopBacktrack, other),
                };
            }
            // A split (`abc+`) leaves a list whose last member takes the
            // next quantifier.
            match result {
                Node::List(mut parts)
                    if parts.len() == 2 && matches!(parts[0], Node::Str { .. }) =>
                {
                    let tail = parts.pop().expect("two parts");
                    quantified.push(parts.pop().expect("one part"));
                    node = tail;
                }
                other => node = other,
            }
            group = false;
            next = self.fetch_token()?;
        }
        if quantified.is_empty() {
            Ok((node, next))
        } else {
            quantified.push(node);
            Ok((Node::List(quantified), next))
        }
    }

    /// The `string_loop` of `parse_exp`: following characters join the run.
    fn string_loop(&mut self, chars: &mut Vec<char>) -> ParseResult<Token> {
        loop {
            let token = self.fetch_token()?;
            match token {
                Token::Char(character) => chars.push(character),
                Token::CodePoint(code) => {
                    let character = char::from_u32(code)
                        .ok_or_else(|| RegexError::new(error::INVALID_CODE_POINT_VALUE))?;
                    chars.push(character);
                }
                Token::RawByte(byte) if byte < 0x80 => chars.push(char::from(byte)),
                other => return Ok(other),
            }
        }
    }

    /// `\R`: `(?>\x0D\x0A|[\x0A-\x0D\x{85}\x{2028}\x{2029}])`.
    fn node_linebreak() -> Node {
        let mut class = ClassNode::new();
        class.set.add_range(0x0a, 0x0d);
        class.set.add_code(0x85);
        class.set.add_code(0x2028);
        class.set.add_code(0x2029);
        Node::enclose(
            EncloseKind::StopBacktrack,
            Node::Alt(vec![
                Node::str_from(vec!['\r', '\n'], false),
                Node::Class(class),
            ]),
        )
    }

    /// `parse_branch`: a sequence up to `|`, the terminator, or the end.
    fn parse_branch(&mut self, token: Token, term: &Token) -> ParseResult<(Node, Token)> {
        let (first, mut next) = self.parse_exp(token, term)?;
        if next == Token::Eot || &next == term || next == Token::Alt {
            return Ok((first, next));
        }
        let mut parts = Vec::new();
        Self::push_flat(&mut parts, first);
        while next != Token::Eot && &next != term && next != Token::Alt {
            let (node, following) = self.parse_exp(next, term)?;
            Self::push_flat(&mut parts, node);
            next = following;
        }
        Ok((Node::List(parts), next))
    }

    fn push_flat(parts: &mut Vec<Node>, node: Node) {
        match node {
            Node::List(inner) => parts.extend(inner),
            other => parts.push(other),
        }
    }

    /// `parse_subexp`: alternatives up to the terminator.
    fn parse_subexp(&mut self, token: Token, term: &Token) -> ParseResult<Node> {
        self.parse_depth += 1;
        if self.parse_depth > PARSE_DEPTH_LIMIT {
            return Err(RegexError::new(error::PARSE_DEPTH_LIMIT_OVER));
        }
        let (first, mut next) = self.parse_branch(token, term)?;
        let node = if &next == term {
            first
        } else if next == Token::Alt {
            let mut branches = vec![first];
            while next == Token::Alt {
                let token = self.fetch_token()?;
                let (branch, following) = self.parse_branch(token, term)?;
                branches.push(branch);
                next = following;
            }
            if &next != term {
                return Err(Self::subexp_end_failure(term));
            }
            Node::Alt(branches)
        } else {
            return Err(Self::subexp_end_failure(term));
        };
        self.parse_depth -= 1;
        Ok(node)
    }

    fn subexp_end_failure(term: &Token) -> RegexError {
        if *term == Token::SubexpClose {
            RegexError::new(error::END_PATTERN_WITH_UNMATCHED_PARENTHESIS)
        } else {
            RegexError::new("internal parser error (bug)")
        }
    }

    /// `parse_regexp`.
    fn parse_regexp(&mut self) -> ParseResult<Node> {
        let token = self.fetch_token()?;
        self.parse_subexp(token, &Token::Eot)
    }
}

/// `onig_reduce_nested_quantifier` for setup, which meets a quantifier on
/// a quantifier again once an unnamed group between them is removed.
pub fn reduce_quantifiers(outer: QuantifierNode, inner: QuantifierNode) -> Node {
    match (
        Parser::popular_quantifier_num(&inner),
        Parser::popular_quantifier_num(&outer),
    ) {
        (Some(target_index), Some(nest_index)) => {
            Parser::reduce_nested_quantifier(outer, inner, target_index, nest_index)
        }
        _ => {
            let mut outer = outer;
            outer.target = Box::new(Node::Quantifier(inner));
            Node::Quantifier(outer)
        }
    }
}

/// What `parse_enclose` answers: a plain `(?:...)` group's body, an
/// isolated option that reaches to the end of the enclosing group, or a node.
enum EncloseResult {
    Group(Node),
    OptionOnly(ParseOptions),
    Node(Node),
}
