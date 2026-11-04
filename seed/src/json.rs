//! JSON, core and written by hand (ADR 0072): `JSON.parse`, `JSON.generate`,
//! and `JSON.pretty_generate` answering what Ruby's json gem (3.1.0 on Ruby
//! 4.0.7) answers, byte for byte — its sentences, its positions, its float
//! spellings — read from the gem's `parser.c`, `generator.c`, and vendored
//! `fpconv.c` rather than recalled, and pinned by the probes recorded in
//! #182. The compiler carries a Portland twin (`compiler/json.pdx`); the two
//! are compared by the differential suite, so this is the oracle, not the
//! implementation that lasts.
//!
//! What a bad document gets is a *failure* (ADR 0027): parsing is a fallible
//! operation on data. What a wrong program gets is a refusal: a non-string
//! handed to `parse`, a struct handed to `generate`.

use crate::value::Value;

/// The json gem's `max_nesting`: a 101st open container is too deep.
const MAX_NESTING: usize = 100;

/// How much of the document an error quotes, in bytes
/// (`PARSE_ERROR_FRAGMENT_LEN`).
const FRAGMENT_LENGTH: usize = 32;

/// `JSON.parse(text)`: the document's value, or a failure carrying the json
/// gem's sentence. `symbolize_names` makes the keys symbols (ADR 0071).
pub fn parse(text: &str, symbolize_names: bool) -> Value {
    let mut parser = Parser {
        bytes: text.as_bytes(),
        cursor: 0,
        nesting: 0,
        symbolize_names,
    };
    match parser.document() {
        Ok(value) => value,
        Err(sentence) => failure(sentence),
    }
}

/// `JSON.generate(value)`: the json gem's compact text, or a failure for
/// `NaN`, `Infinity`, and a document nested past 100.
pub fn generate(value: &Value) -> Value {
    let mut out = String::new();
    match write(value, &mut out, false, 0) {
        Ok(()) => Value::String(out),
        Err(sentence) => failure(sentence),
    }
}

/// `JSON.pretty_generate(value)`: two-space indentation, `"key": value`,
/// empties kept as `[]` and `{}`.
pub fn pretty_generate(value: &Value) -> Value {
    let mut out = String::new();
    match write(value, &mut out, true, 0) {
        Ok(()) => Value::String(out),
        Err(sentence) => failure(sentence),
    }
}

fn failure(sentence: String) -> Value {
    Value::Failure(Box::new(Value::String(sentence)))
}

type Parsed<T> = Result<T, String>;

struct Parser<'a> {
    bytes: &'a [u8],
    cursor: usize,
    nesting: usize,
    symbolize_names: bool,
}

impl Parser<'_> {
    fn document(&mut self) -> Parsed<Value> {
        self.eat_whitespace()?;
        if self.at_end() {
            return Err(self.error("unexpected end of input".to_string(), self.cursor));
        }
        let value = self.value()?;
        self.eat_whitespace()?;
        if !self.at_end() {
            return Err(self.error(
                format!(
                    "unexpected token at end of stream {}",
                    self.fragment(self.cursor)
                ),
                self.cursor,
            ));
        }
        Ok(value)
    }

    fn at_end(&self) -> bool {
        self.cursor >= self.bytes.len()
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.cursor).copied()
    }

    /// Space, tab, newline, and return. A `/` would start one of the gem's
    /// optional comments, which are off, so it is an unexpected token.
    fn eat_whitespace(&mut self) -> Parsed<()> {
        while let Some(byte) = self.peek() {
            match byte {
                b' ' | b'\t' | b'\n' | b'\r' => self.cursor += 1,
                b'/' => {
                    return Err(self.error(
                        format!("unexpected token {}", self.fragment(self.cursor)),
                        self.cursor,
                    ));
                }
                _ => break,
            }
        }
        Ok(())
    }

    fn value(&mut self) -> Parsed<Value> {
        self.eat_whitespace()?;
        let Some(byte) = self.peek() else {
            return Err(self.error("unexpected end of input".to_string(), self.cursor));
        };
        match byte {
            b'n' => self.keyword("null", Value::Nil),
            b't' => self.keyword("true", Value::Boolean(true)),
            b'f' => self.keyword("false", Value::Boolean(false)),
            b'-' | b'0'..=b'9' => self.number(),
            b'"' => {
                let text = self.string()?;
                Ok(Value::String(text))
            }
            b'[' => self.array(),
            b'{' => self.object(),
            // The gem's NaN and Infinity are off, so their first letters are
            // unexpected tokens like any other word.
            b'N' | b'I' => Err(self.error(
                format!("unexpected token {}", self.fragment(self.cursor)),
                self.cursor,
            )),
            0 => Err(self.error(
                format!("unexpected NULL byte: {}", self.fragment(self.cursor)),
                self.cursor,
            )),
            _ => Err(self.error(
                format!("unexpected character: {}", self.fragment(self.cursor)),
                self.cursor,
            )),
        }
    }

    fn keyword(&mut self, word: &str, value: Value) -> Parsed<Value> {
        if self.bytes[self.cursor..].starts_with(word.as_bytes()) {
            self.cursor += word.len();
            return Ok(value);
        }
        Err(self.error(
            format!("unexpected token {}", self.fragment(self.cursor)),
            self.cursor,
        ))
    }

    fn digits(&mut self) -> usize {
        let start = self.cursor;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.cursor += 1;
        }
        self.cursor - start
    }

    /// The gem's grammar: an optional minus, digits with no leading zero, an
    /// optional fraction with at least one digit, an optional exponent with
    /// at least one digit. A fraction or exponent makes a Float; otherwise
    /// an Integer, and one past 64 bits is a failure (ADR 0070) where Ruby
    /// widens to a bignum.
    fn number(&mut self) -> Parsed<Value> {
        let start = self.cursor;
        if self.peek() == Some(b'-') {
            self.cursor += 1;
        }
        let first_digit = self.peek();
        let mut integer = true;
        let digits = self.digits();
        if (first_digit == Some(b'0') && digits > 1) || digits == 0 {
            return Err(self.invalid_number(start));
        }
        if self.peek() == Some(b'.') {
            integer = false;
            self.cursor += 1;
            if self.digits() == 0 {
                return Err(self.invalid_number(start));
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            integer = false;
            self.cursor += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.cursor += 1;
            }
            if self.digits() == 0 {
                return Err(self.invalid_number(start));
            }
        }
        let token = std::str::from_utf8(&self.bytes[start..self.cursor]).unwrap();
        if integer {
            return match token.parse::<i64>() {
                Ok(number) => Ok(Value::Integer(number)),
                Err(_) => Err(self.error(format!("{token} overflows the 64-bit integers"), start)),
            };
        }
        Ok(Value::Float(token.parse::<f64>().unwrap()))
    }

    fn invalid_number(&self, start: usize) -> String {
        self.error(format!("invalid number: {}", self.fragment(start)), start)
    }

    /// A string's text, cursor on the opening quote. The scan finds the
    /// closing quote first — a control character or the end of input refuse
    /// before any escape is read, as the gem's does — then the escapes are
    /// decoded.
    fn string(&mut self) -> Parsed<String> {
        self.cursor += 1;
        let start = self.cursor;
        loop {
            let Some(byte) = self.peek() else {
                return Err(self.error(
                    "unexpected end of input, expected closing \"".to_string(),
                    self.cursor,
                ));
            };
            match byte {
                b'"' => break,
                b'\\' => self.cursor += 2,
                0..=0x1f => {
                    return Err(self.error(
                        format!(
                            "invalid ASCII control character in string: {}",
                            self.fragment(self.cursor)
                        ),
                        self.cursor,
                    ));
                }
                _ => self.cursor += 1,
            }
        }
        let end = self.cursor;
        self.cursor += 1;
        self.unescape(start, end)
    }

    fn unescape(&self, start: usize, end: usize) -> Parsed<String> {
        let mut out: Vec<u8> = Vec::with_capacity(end - start);
        let mut position = start;
        while position < end {
            let byte = self.bytes[position];
            if byte != b'\\' {
                out.push(byte);
                position += 1;
                continue;
            }
            let escaped = self.bytes[position + 1];
            match escaped {
                b'n' => out.push(b'\n'),
                b'r' => out.push(b'\r'),
                b't' => out.push(b'\t'),
                b'b' => out.push(8),
                b'f' => out.push(12),
                b'"' | b'\\' | b'/' => out.push(escaped),
                b'u' => {
                    let mut code = self.hex_escape(position + 2, end, position)?;
                    let mut after = position + 6;
                    if (0xD800..=0xDBFF).contains(&code) {
                        if after + 6 <= end && &self.bytes[after..after + 2] == b"\\u" {
                            let low = self.hex_escape(after + 2, end, after)?;
                            if !(0xDC00..=0xDFFF).contains(&low) {
                                return Err(self.error(
                                    format!(
                                        "invalid surrogate pair at {}",
                                        self.fragment(position)
                                    ),
                                    position,
                                ));
                            }
                            code = 0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                            after += 6;
                        } else {
                            return Err(self.error(
                                format!("incomplete surrogate pair at {}", self.fragment(position)),
                                position,
                            ));
                        }
                    } else if (0xDC00..=0xDFFF).contains(&code) {
                        return Err(self.error(
                            format!("unpaired trailing surrogate at {}", self.fragment(position)),
                            position,
                        ));
                    }
                    let character = char::from_u32(code).unwrap();
                    let mut encoded = [0u8; 4];
                    out.extend_from_slice(character.encode_utf8(&mut encoded).as_bytes());
                    position = after;
                    continue;
                }
                b'\n' => {
                    return Err(self.error(
                        format!(
                            "Invalid unescaped newline character (\\n) in string: {}",
                            self.fragment(position)
                        ),
                        position,
                    ));
                }
                0 => {
                    return Err(self.error(
                        format!(
                            "Invalid unescaped NULL character (\\0) in string: {}",
                            self.fragment(position)
                        ),
                        position,
                    ));
                }
                1..=0x1f => {
                    return Err(self.error(
                        format!(
                            "invalid ASCII control character in string: {}",
                            self.fragment(position)
                        ),
                        position,
                    ));
                }
                _ => {
                    return Err(self.error(
                        format!(
                            "invalid escape character in string: {}",
                            self.fragment(position)
                        ),
                        position,
                    ));
                }
            }
            position += 2;
        }
        Ok(String::from_utf8(out).unwrap())
    }

    /// Four hex digits at `at`, inside a string ending at `end`; `backslash`
    /// is where the escape began, which is what the sentence points at.
    fn hex_escape(&self, at: usize, end: usize, backslash: usize) -> Parsed<u32> {
        if at + 4 > end {
            return Err(self.incomplete_unicode(backslash));
        }
        let mut code = 0u32;
        for &byte in &self.bytes[at..at + 4] {
            let digit = match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                b'A'..=b'F' => byte - b'A' + 10,
                _ => return Err(self.incomplete_unicode(backslash)),
            };
            code = (code << 4) | u32::from(digit);
        }
        Ok(code)
    }

    fn incomplete_unicode(&self, backslash: usize) -> String {
        self.error(
            format!(
                "incomplete unicode character escape sequence at {}",
                self.fragment(backslash)
            ),
            backslash,
        )
    }

    fn open(&mut self) -> Parsed<()> {
        self.nesting += 1;
        if self.nesting > MAX_NESTING {
            return Err(format!("nesting of {} is too deep", self.nesting));
        }
        Ok(())
    }

    fn array(&mut self) -> Parsed<Value> {
        self.cursor += 1;
        self.eat_whitespace()?;
        if self.peek() == Some(b']') {
            self.cursor += 1;
            return Ok(Value::array(Vec::new()));
        }
        self.open()?;
        let mut elements = Vec::new();
        loop {
            elements.push(self.value()?);
            self.eat_whitespace()?;
            match self.peek() {
                Some(b',') => self.cursor += 1,
                Some(b']') => {
                    self.cursor += 1;
                    break;
                }
                _ => {
                    return Err(self.error(
                        "expected ',' or ']' after array value".to_string(),
                        self.cursor,
                    ));
                }
            }
        }
        self.nesting -= 1;
        Ok(Value::array(elements))
    }

    fn object(&mut self) -> Parsed<Value> {
        let start = self.cursor;
        self.cursor += 1;
        self.eat_whitespace()?;
        if self.peek() == Some(b'}') {
            self.cursor += 1;
            return Ok(Value::hash(Vec::new()));
        }
        self.open()?;
        let mut pairs: Vec<(Value, Value)> = Vec::new();
        loop {
            self.eat_whitespace()?;
            if self.peek() != Some(b'"') {
                // The gem's two sentences: the first key, and a key after a
                // comma.
                let message = if pairs.is_empty() {
                    format!("expected object key, got {}", self.fragment(self.cursor))
                } else {
                    format!("expected object key, got: {}", self.fragment(self.cursor))
                };
                return Err(self.error(message, self.cursor));
            }
            let key = self.string()?;
            self.eat_whitespace()?;
            if self.peek() != Some(b':') {
                let message = if pairs.is_empty() {
                    "expected ':' after object key".to_string()
                } else {
                    format!(
                        "expected ':' after object key, got: {}",
                        self.fragment(self.cursor)
                    )
                };
                return Err(self.error(message, self.cursor));
            }
            self.cursor += 1;
            let value = self.value()?;
            let key = if self.symbolize_names {
                Value::Symbol(key)
            } else {
                Value::String(key)
            };
            pairs.push((key, value));
            self.eat_whitespace()?;
            match self.peek() {
                Some(b',') => self.cursor += 1,
                Some(b'}') => {
                    self.cursor += 1;
                    break;
                }
                _ => {
                    return Err(self.error(
                        format!(
                            "expected ',' or '}}' after object value, got: {}",
                            self.fragment(self.cursor)
                        ),
                        self.cursor,
                    ));
                }
            }
        }
        // json 3.1 refuses a duplicate key, and points at the object's own
        // opening brace.
        for (index, (key, _)) in pairs.iter().enumerate() {
            if pairs[..index].iter().any(|(earlier, _)| earlier == key) {
                let name = match key {
                    Value::Symbol(name) => Value::String(name.clone()).inspect(),
                    other => other.inspect(),
                };
                return Err(self.error(format!("duplicate key {name}"), start));
            }
        }
        self.nesting -= 1;
        Ok(Value::hash(pairs))
    }

    /// The sentence with the gem's position: the line, and the column in
    /// characters from the line's start, both counted from 1.
    fn error(&self, message: String, at: usize) -> String {
        let at = at.min(self.bytes.len());
        let line = 1 + self.bytes[..at]
            .iter()
            .filter(|&&byte| byte == b'\n')
            .count();
        let line_start = self.bytes[..at]
            .iter()
            .rposition(|&byte| byte == b'\n')
            .map_or(0, |index| index + 1);
        let column = 1 + self.bytes[line_start..at]
            .iter()
            .filter(|&&byte| !(0x80..0xC0).contains(&byte))
            .count();
        format!("{message} at line {line} column {column}")
    }

    /// What the gem quotes after a sentence: `EOF` at the end, else the
    /// text from `at` up to a NUL, newline, space, tab, or return, at most
    /// 32 bytes without splitting a character, in quotes. When the text
    /// *starts* with one of those stops the gem prints the raw C string
    /// instead — nothing for a NUL, everything up to the first NUL for the
    /// others, unquoted — and so does this.
    fn fragment(&self, at: usize) -> String {
        if at >= self.bytes.len() {
            return "EOF".to_string();
        }
        let rest = &self.bytes[at..];
        let stop = |byte: u8| matches!(byte, 0 | b'\n' | b' ' | b'\t' | b'\r');
        if stop(rest[0]) {
            let until_nul = rest
                .iter()
                .position(|&byte| byte == 0)
                .unwrap_or(rest.len());
            return String::from_utf8_lossy(&rest[..until_nul]).into_owned();
        }
        let mut length = 0;
        while length < FRAGMENT_LENGTH && length < rest.len() && !stop(rest[length]) {
            length += 1;
        }
        if length == FRAGMENT_LENGTH {
            while length > 0
                && rest
                    .get(length)
                    .is_some_and(|byte| (0x80..0xC0).contains(byte))
            {
                length -= 1;
            }
        }
        format!("'{}'", String::from_utf8_lossy(&rest[..length]))
    }
}

// — generation —

fn write(value: &Value, out: &mut String, pretty: bool, depth: usize) -> Result<(), String> {
    match value {
        Value::Nil => out.push_str("null"),
        Value::Boolean(flag) => out.push_str(if *flag { "true" } else { "false" }),
        Value::Integer(number) => out.push_str(&number.to_string()),
        Value::Float(number) => {
            if number.is_nan() || number.is_infinite() {
                return Err(format!("{} not allowed in JSON", value.shown()));
            }
            out.push_str(&float_text(*number));
        }
        Value::String(text) => write_string(text, out),
        Value::Symbol(name) => write_string(name, out),
        Value::Array(elements) => {
            let depth = deeper(depth)?;
            if elements.is_empty() {
                out.push_str("[]");
                return Ok(());
            }
            out.push('[');
            for (index, element) in elements.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                if pretty {
                    out.push('\n');
                    indent(out, depth);
                }
                write(element, out, pretty, depth)?;
            }
            if pretty {
                out.push('\n');
                indent(out, depth - 1);
            }
            out.push(']');
        }
        Value::Hash(pairs) => {
            let depth = deeper(depth)?;
            if pairs.is_empty() {
                out.push_str("{}");
                return Ok(());
            }
            out.push('{');
            for (index, (key, element)) in pairs.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                if pretty {
                    out.push('\n');
                    indent(out, depth);
                }
                write_string(&key_text(key), out);
                out.push(':');
                if pretty {
                    out.push(' ');
                }
                write(element, out, pretty, depth)?;
            }
            if pretty {
                out.push('\n');
                indent(out, depth - 1);
            }
            out.push('}');
        }
        other => not_json(other),
    }
    Ok(())
}

/// One container deeper; the gem counts every array and hash, empties
/// included, and the 101st is too deep.
fn deeper(depth: usize) -> Result<usize, String> {
    let depth = depth + 1;
    if depth > MAX_NESTING {
        return Err(format!(
            "nesting of {depth} is too deep. Did you try to serialize objects with circular references?"
        ));
    }
    Ok(depth)
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

/// A hash key as the gem writes it: a string as is, a symbol by its name,
/// anything else by its `to_s` — `1`, `1.5`, `true`, `[1, "a"]`, and the
/// empty string for nil.
fn key_text(key: &Value) -> String {
    match key {
        Value::String(text) => text.clone(),
        Value::Symbol(name) => name.clone(),
        Value::Nil => String::new(),
        Value::Boolean(_)
        | Value::Integer(_)
        | Value::Float(_)
        | Value::Array(_)
        | Value::Hash(_) => key.shown(),
        other => not_json(other),
    }
}

/// Ruby writes `"#<struct a=1>"` for a struct and `"1..3"` for a range —
/// silent garbage — where this refuses (ADR 0072 §5).
fn not_json(value: &Value) -> ! {
    match value {
        Value::Range { .. } => panic!("a Range is not JSON — generate its 'to_a'"),
        Value::Struct { name, .. } => panic!("a {name} is not JSON — generate its 'to_h'"),
        Value::EnumCase { name, .. } => {
            panic!(":{name} carries a payload, which is not JSON — generate its 'to_h'")
        }
        other => panic!("{} is not JSON", other.inspect()),
    }
}

/// The gem's escapes: the quote, the backslash, the five named controls,
/// `\u00xx` for the other controls, and everything else raw — `/`, DEL,
/// U+2028, every non-ASCII character.
fn write_string(text: &str, out: &mut String) {
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0'..='\u{1f}' => out.push_str(&format!("\\u{:04x}", character as u32)),
            _ => out.push(character),
        }
    }
    out.push('"');
}

/// The gem's float writer, `fpconv.c`'s `emit_digits` over the shortest
/// round-trip digits: `1e+20` where `Float#to_s` says `1.0e+20`,
/// `0.00000015` where it says `1.5e-07`, `100.0` with its point kept.
///
/// The gem takes its digits from Grisu2, which is not always the shortest
/// spelling; this takes the shortest, so a rare float may print a digit
/// shorter here than there (recorded in docs/ruby/json.md).
pub fn float_text(value: f64) -> String {
    if value == 0.0 {
        return if value.is_sign_negative() {
            "-0.0"
        } else {
            "0.0"
        }
        .to_string();
    }
    let negative = value < 0.0;
    let scientific = format!("{:e}", value.abs());
    let (mantissa, exponent) = scientific.split_once('e').unwrap();
    let exponent: i32 = exponent.parse().unwrap();
    let digits: String = mantissa
        .chars()
        .filter(|character| *character != '.')
        .collect();
    let count = digits.len() as i32;
    let point = exponent - (count - 1);
    let mut out = String::new();
    if negative {
        out.push('-');
    }
    out.push_str(&emit_digits(&digits, point));
    out
}

/// `digits × 10^point`, laid out as `fpconv.c` lays it out.
fn emit_digits(digits: &str, point: i32) -> String {
    let count = digits.len() as i32;
    let exponent = (point + count - 1).abs();
    if point >= 0 && exponent < 15 {
        let mut out = digits.to_string();
        out.push_str(&"0".repeat(point as usize));
        out.push_str(".0");
        return out;
    }
    if point < 0 && (point > -7 || exponent < 10) {
        let offset = count + point;
        if offset <= 0 {
            return format!("0.{}{digits}", "0".repeat((-offset) as usize));
        }
        let offset = offset as usize;
        return format!("{}.{}", &digits[..offset], &digits[offset..]);
    }
    let mut out = String::new();
    out.push_str(&digits[..1]);
    if count > 1 {
        out.push('.');
        out.push_str(&digits[1..]);
    }
    out.push('e');
    out.push(if point + count - 1 < 0 { '-' } else { '+' });
    out.push_str(&exponent.to_string());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(text: &str) -> Value {
        parse(text, false)
    }

    fn failed(text: &str) -> String {
        match parse(text, false) {
            Value::Failure(inner) => match *inner {
                Value::String(sentence) => sentence,
                other => panic!("a failure of {other:?}"),
            },
            other => panic!("{text:?} parsed as {other:?}"),
        }
    }

    fn generated(value: Value) -> String {
        match generate(&value) {
            Value::String(text) => text,
            other => panic!("generate answered {other:?}"),
        }
    }

    fn sentence_of(value: Value) -> String {
        match value {
            Value::Failure(inner) => match *inner {
                Value::String(sentence) => sentence,
                other => panic!("a failure of {other:?}"),
            },
            other => panic!("not a failure: {other:?}"),
        }
    }

    fn string(text: &str) -> Value {
        Value::String(text.to_string())
    }

    #[test]
    fn scalars_and_null() {
        assert_eq!(parsed("2"), Value::Integer(2));
        assert_eq!(parsed(" \"x\" "), string("x"));
        assert_eq!(parsed("null"), Value::Nil);
        assert_eq!(
            parsed("[true,false,null]"),
            Value::array(vec![
                Value::Boolean(true),
                Value::Boolean(false),
                Value::Nil
            ])
        );
    }

    #[test]
    fn numbers_split_on_rubys_rule() {
        assert_eq!(
            parsed("[-0, -0.0, 1E2, 1e+2, 1e-2, 0.5, 1e5, 0.1e1]"),
            Value::array(vec![
                Value::Integer(0),
                Value::Float(-0.0),
                Value::Float(100.0),
                Value::Float(100.0),
                Value::Float(0.01),
                Value::Float(0.5),
                Value::Float(100000.0),
                Value::Float(1.0),
            ])
        );
        assert_eq!(parsed("1e400"), Value::Float(f64::INFINITY));
        assert_eq!(parsed("1e-400"), Value::Float(0.0));
        assert_eq!(
            failed("99999999999999999999"),
            "99999999999999999999 overflows the 64-bit integers at line 1 column 1"
        );
    }

    #[test]
    fn invalid_numbers_are_the_gems() {
        assert_eq!(failed("01"), "invalid number: '01' at line 1 column 1");
        assert_eq!(failed("[01]"), "invalid number: '01]' at line 1 column 2");
        assert_eq!(failed("-"), "invalid number: '-' at line 1 column 1");
        assert_eq!(failed("1."), "invalid number: '1.' at line 1 column 1");
        assert_eq!(failed("1e"), "invalid number: '1e' at line 1 column 1");
        assert_eq!(failed("2.x"), "invalid number: '2.x' at line 1 column 1");
        assert_eq!(
            failed("-Infinity"),
            "invalid number: '-Infinity' at line 1 column 1"
        );
        assert_eq!(
            failed(".5"),
            "unexpected character: '.5' at line 1 column 1"
        );
    }

    #[test]
    fn words_and_ends() {
        assert_eq!(failed(""), "unexpected end of input at line 1 column 1");
        assert_eq!(failed(" "), "unexpected end of input at line 1 column 2");
        assert_eq!(failed("\n"), "unexpected end of input at line 2 column 1");
        assert_eq!(failed("["), "unexpected end of input at line 1 column 2");
        assert_eq!(failed("[1,"), "unexpected end of input at line 1 column 4");
        assert_eq!(failed("NaN"), "unexpected token 'NaN' at line 1 column 1");
        assert_eq!(
            failed("Infinity"),
            "unexpected token 'Infinity' at line 1 column 1"
        );
        assert_eq!(failed("tru"), "unexpected token 'tru' at line 1 column 1");
        assert_eq!(
            failed("falsy"),
            "unexpected token 'falsy' at line 1 column 1"
        );
        assert_eq!(
            failed("nullx"),
            "unexpected token at end of stream 'x' at line 1 column 5"
        );
        assert_eq!(
            failed("/* c */ 1"),
            "unexpected token '/*' at line 1 column 1"
        );
        assert_eq!(failed("1 // c"), "unexpected token '//' at line 1 column 3");
        assert_eq!(
            failed("{\"a\":1} x"),
            "unexpected token at end of stream 'x' at line 1 column 9"
        );
        assert_eq!(
            failed("[1]\n\n x"),
            "unexpected token at end of stream 'x' at line 3 column 2"
        );
        assert_eq!(
            failed("{\"é\":1} x"),
            "unexpected token at end of stream 'x' at line 1 column 9"
        );
        assert_eq!(
            failed("[1]\0"),
            "unexpected token at end of stream  at line 1 column 4"
        );
        assert_eq!(
            failed("[\x001]"),
            "unexpected NULL byte:  at line 1 column 2"
        );
        assert_eq!(
            failed("\x0c[1]"),
            "unexpected character: '\x0c[1]' at line 1 column 1"
        );
        assert_eq!(
            failed("[é]"),
            "unexpected character: 'é]' at line 1 column 2"
        );
    }

    #[test]
    fn arrays_and_objects_refuse_in_the_gems_words() {
        assert_eq!(
            failed("[1,]"),
            "unexpected character: ']' at line 1 column 4"
        );
        assert_eq!(
            failed("[1,,2]"),
            "unexpected character: ',2]' at line 1 column 4"
        );
        assert_eq!(
            failed("[,]"),
            "unexpected character: ',]' at line 1 column 2"
        );
        assert_eq!(
            failed("[\n1,\n]"),
            "unexpected character: ']' at line 3 column 1"
        );
        assert_eq!(
            failed("[1 2]"),
            "expected ',' or ']' after array value at line 1 column 4"
        );
        assert_eq!(
            failed("[1,2"),
            "expected ',' or ']' after array value at line 1 column 5"
        );
        assert_eq!(
            failed("[1x]"),
            "expected ',' or ']' after array value at line 1 column 3"
        );
        assert_eq!(
            failed("{a:1}"),
            "expected object key, got 'a:1}' at line 1 column 2"
        );
        assert_eq!(
            failed("{"),
            "expected object key, got EOF at line 1 column 2"
        );
        assert_eq!(
            failed("{,}"),
            "expected object key, got ',}' at line 1 column 2"
        );
        assert_eq!(
            failed("{\"a\":1,}"),
            "expected object key, got: '}' at line 1 column 8"
        );
        assert_eq!(
            failed("{\"a\" 1}"),
            "expected ':' after object key at line 1 column 6"
        );
        assert_eq!(
            failed("{\"a\"}"),
            "expected ':' after object key at line 1 column 5"
        );
        assert_eq!(
            failed("{\"a\""),
            "expected ':' after object key at line 1 column 5"
        );
        assert_eq!(
            failed("{\"a\":1,\"b\""),
            "expected ':' after object key, got: EOF at line 1 column 11"
        );
        assert_eq!(
            failed("{\"a\":1,\"b\" 2}"),
            "expected ':' after object key, got: '2}' at line 1 column 12"
        );
        assert_eq!(
            failed("{\"a\":}"),
            "unexpected character: '}' at line 1 column 6"
        );
        assert_eq!(
            failed("{\"a\":1 \"b\":2}"),
            "expected ',' or '}' after object value, got: '\"b\":2}' at line 1 column 8"
        );
        assert_eq!(
            failed("{\"a\":1"),
            "expected ',' or '}' after object value, got: EOF at line 1 column 7"
        );
        assert_eq!(
            failed("{\"a\":1}}"),
            "unexpected token at end of stream '}' at line 1 column 8"
        );
        assert_eq!(
            failed("{\"a\":1,\"a\":2}"),
            "duplicate key \"a\" at line 1 column 1"
        );
        assert_eq!(
            failed("{\"x\":{\"a\":1,\"a\":2}}"),
            "duplicate key \"a\" at line 1 column 6"
        );
        assert_eq!(
            failed("{\"a\\\"b\":1,\"a\\\"b\":2}"),
            "duplicate key \"a\\\"b\" at line 1 column 1"
        );
        let long = format!("[1,{}]", "x".repeat(50));
        assert_eq!(
            failed(&long),
            format!(
                "unexpected character: '{}' at line 1 column 4",
                "x".repeat(32)
            )
        );
    }

    #[test]
    fn strings_decode_the_gems_escapes() {
        assert_eq!(parsed("\"\\u00e9\\n\""), string("é\n"));
        assert_eq!(parsed("\"\\u00E9\""), string("é"));
        assert_eq!(parsed("\"\\/\""), string("/"));
        assert_eq!(parsed("\"\\\"\""), string("\""));
        assert_eq!(parsed("\"\\uD83D\\uDE00\""), string("😀"));
        assert_eq!(parsed("\"😀\""), string("😀"));
        assert_eq!(parsed("\"\\udbff\\udfff\""), string("\u{10FFFF}"));
        assert_eq!(parsed("\"\\u0000\""), string("\0"));
        assert_eq!(parsed("\"a\u{7f}b\""), string("a\u{7f}b"));
        assert_eq!(parsed("\"\\b\\f\\r\\t\""), string("\u{8}\u{c}\r\t"));
    }

    #[test]
    fn bad_strings_are_the_gems() {
        assert_eq!(
            failed("\"abc"),
            "unexpected end of input, expected closing \" at line 1 column 5"
        );
        assert_eq!(
            failed("\"\\"),
            "unexpected end of input, expected closing \" at line 1 column 3"
        );
        assert_eq!(
            failed("\"\\u"),
            "unexpected end of input, expected closing \" at line 1 column 4"
        );
        assert_eq!(
            failed("\"\\q\""),
            "invalid escape character in string: '\\q\"' at line 1 column 2"
        );
        assert_eq!(
            failed("[1,\n2\n,\"\\q\"]"),
            "invalid escape character in string: '\\q\"]' at line 3 column 3"
        );
        assert_eq!(
            failed("\"a\tb\""),
            "invalid ASCII control character in string: \tb\" at line 1 column 3"
        );
        assert_eq!(
            failed("\"a\0b\""),
            "invalid ASCII control character in string:  at line 1 column 3"
        );
        assert_eq!(
            failed("\"\\q\t\""),
            "invalid ASCII control character in string: \t\" at line 1 column 4"
        );
        assert_eq!(
            failed("\"\\\t\""),
            "invalid ASCII control character in string: '\\' at line 1 column 2"
        );
        assert_eq!(
            failed("\"\\\n\""),
            "Invalid unescaped newline character (\\n) in string: '\\' at line 1 column 2"
        );
        assert_eq!(
            failed("\"\\q\n\""),
            "invalid ASCII control character in string: \n\" at line 1 column 4"
        );
        assert_eq!(
            failed("\"\\ud83d\""),
            "incomplete surrogate pair at '\\ud83d\"' at line 1 column 2"
        );
        assert_eq!(
            failed("\"\\ud83dx\""),
            "incomplete surrogate pair at '\\ud83dx\"' at line 1 column 2"
        );
        assert_eq!(
            failed("\"\\ud83d\\u\""),
            "incomplete surrogate pair at '\\ud83d\\u\"' at line 1 column 2"
        );
        assert_eq!(
            failed("[\"\\ud83d\\u12\"]"),
            "incomplete surrogate pair at '\\ud83d\\u12\"]' at line 1 column 3"
        );
        assert_eq!(
            failed("\"\\ud83d\\u0041\""),
            "invalid surrogate pair at '\\ud83d\\u0041\"' at line 1 column 2"
        );
        assert_eq!(
            failed("\"\\ud83d\\ud83d\""),
            "invalid surrogate pair at '\\ud83d\\ud83d\"' at line 1 column 2"
        );
        assert_eq!(
            failed("[\"\\ud83d\\uzzzz\"]"),
            "incomplete unicode character escape sequence at '\\uzzzz\"]' at line 1 column 9"
        );
        assert_eq!(
            failed("\"\\udc00\""),
            "unpaired trailing surrogate at '\\udc00\"' at line 1 column 2"
        );
        assert_eq!(
            failed("\"\\u12\""),
            "incomplete unicode character escape sequence at '\\u12\"' at line 1 column 2"
        );
        assert_eq!(
            failed("\"\\u00zz\""),
            "incomplete unicode character escape sequence at '\\u00zz\"' at line 1 column 2"
        );
        assert_eq!(
            failed("\"\\u\""),
            "incomplete unicode character escape sequence at '\\u\"' at line 1 column 2"
        );
    }

    #[test]
    fn nesting_counts_open_containers() {
        let arrays = |count: usize| format!("{}{}", "[".repeat(count), "]".repeat(count));
        assert!(!matches!(parsed(&arrays(101)), Value::Failure(_)));
        assert_eq!(failed(&arrays(102)), "nesting of 101 is too deep");
        let filled = |count: usize| format!("{}1{}", "[".repeat(count), "]".repeat(count));
        assert!(!matches!(parsed(&filled(100)), Value::Failure(_)));
        assert_eq!(failed(&filled(101)), "nesting of 101 is too deep");
        let objects = |count: usize| format!("{}1{}", "{\"a\":".repeat(count), "}".repeat(count));
        assert!(!matches!(parsed(&objects(100)), Value::Failure(_)));
        assert_eq!(failed(&objects(101)), "nesting of 101 is too deep");
    }

    #[test]
    fn keys_are_strings_or_symbols() {
        assert_eq!(
            parsed("{\"a\":null}"),
            Value::hash(vec![(string("a"), Value::Nil)])
        );
        assert_eq!(
            parse("{\"a b\":{\"c\":1}}", true),
            Value::hash(vec![(
                Value::Symbol("a b".to_string()),
                Value::hash(vec![(Value::Symbol("c".to_string()), Value::Integer(1))])
            )])
        );
        assert_eq!(
            sentence_of(parse("{\"a\":1,\"a\":2}", true)),
            "duplicate key \"a\" at line 1 column 1"
        );
    }

    #[test]
    fn generates_the_gems_text() {
        assert_eq!(
            generated(Value::array(vec![
                Value::Integer(1),
                Value::Float(1.0),
                Value::Float(1e20),
                Value::Float(0.1),
                Value::Float(-0.0)
            ])),
            "[1,1.0,1e+20,0.1,-0.0]"
        );
        assert_eq!(generated(string("é\n\t\"\\/")), "\"é\\n\\t\\\"\\\\/\"");
        assert_eq!(
            generated(string("\u{1}\u{1f}\u{7f}\u{2028}")),
            "\"\\u0001\\u001f\u{7f}\u{2028}\""
        );
        assert_eq!(
            generated(string("\0\u{1b}\u{8}\u{c}\r")),
            "\"\\u0000\\u001b\\b\\f\\r\""
        );
        assert_eq!(
            generated(Value::Symbol("odd key".to_string())),
            "\"odd key\""
        );
        assert_eq!(generated(Value::Nil), "null");
        assert_eq!(
            generated(Value::hash(vec![
                (Value::Symbol("a".to_string()), Value::Integer(1)),
                (Value::Integer(1), Value::Integer(2)),
                (Value::Float(1.5), Value::Integer(4)),
                (Value::Nil, Value::Integer(3)),
                (Value::Boolean(false), Value::Integer(5)),
                (
                    Value::array(vec![Value::Integer(1), string("a")]),
                    Value::Integer(6)
                ),
                (Value::Float(1e20), Value::Integer(7)),
            ])),
            "{\"a\":1,\"1\":2,\"1.5\":4,\"\":3,\"false\":5,\"[1, \\\"a\\\"]\":6,\"1.0e+20\":7}"
        );
        assert_eq!(
            generated(Value::array(vec![
                Value::array(vec![]),
                Value::hash(vec![]),
                Value::array(vec![Value::array(vec![])]),
                Value::hash(vec![(string("a"), Value::hash(vec![]))]),
            ])),
            "[[],{},[[]],{\"a\":{}}]"
        );
    }

    #[test]
    fn floats_are_fpconvs() {
        let cases = [
            (1.0, "1.0"),
            (0.1, "0.1"),
            (1e20, "1e+20"),
            (1e15, "1e+15"),
            (1e14, "100000000000000.0"),
            (1e16, "1e+16"),
            (123456789012345680.0, "1.2345678901234568e+17"),
            (1.5e-7, "0.00000015"),
            (1e-5, "0.00001"),
            (1e-4, "0.0001"),
            (1e-6, "0.000001"),
            (1e-7, "0.0000001"),
            (1e-10, "1e-10"),
            (1.23456e-8, "0.0000000123456"),
            (0.001, "0.001"),
            (100.0, "100.0"),
            (-0.0, "-0.0"),
            (2.5, "2.5"),
            (1e21, "1e+21"),
            (1e100, "1e+100"),
            (5e-324, "5e-324"),
            (1e-323, "1e-323"),
            (f64::MAX, "1.7976931348623157e+308"),
            (0.30000000000000004, "0.30000000000000004"),
            (1234567.0, "1234567.0"),
            (12345678901234567890.0, "1.2345678901234567e+19"),
            (9007199254740993.0, "9.007199254740992e+15"),
            (0.000123, "0.000123"),
            (1e300, "1e+300"),
            (4.35, "4.35"),
            (0.5e-6, "0.0000005"),
            (123456789.0, "123456789.0"),
            (-2.5e-7, "-0.00000025"),
        ];
        for (value, expected) in cases {
            assert_eq!(float_text(value), expected, "for {value:e}");
        }
    }

    #[test]
    fn pretty_is_two_spaces() {
        let document = Value::hash(vec![(
            string("a"),
            Value::array(vec![
                Value::Integer(1),
                Value::hash(vec![(string("b"), Value::Nil)]),
            ]),
        )]);
        assert_eq!(
            match pretty_generate(&document) {
                Value::String(text) => text,
                other => panic!("{other:?}"),
            },
            "{\n  \"a\": [\n    1,\n    {\n      \"b\": null\n    }\n  ]\n}"
        );
        let empties = Value::hash(vec![
            (string("a"), Value::hash(vec![])),
            (string("b"), Value::array(vec![])),
            (string("c"), Value::array(vec![Value::array(vec![])])),
        ]);
        assert_eq!(
            match pretty_generate(&empties) {
                Value::String(text) => text,
                other => panic!("{other:?}"),
            },
            "{\n  \"a\": {},\n  \"b\": [],\n  \"c\": [\n    []\n  ]\n}"
        );
        assert_eq!(pretty_generate(&Value::Integer(1)), string("1"));
        assert_eq!(pretty_generate(&Value::array(vec![])), string("[]"));
    }

    #[test]
    fn generate_fails_on_nan_and_depth() {
        let sentence = |value: Value| sentence_of(generate(&value));
        assert_eq!(sentence(Value::Float(f64::NAN)), "NaN not allowed in JSON");
        assert_eq!(
            sentence(Value::array(vec![
                Value::Integer(1),
                Value::Float(f64::INFINITY)
            ])),
            "Infinity not allowed in JSON"
        );
        assert_eq!(
            sentence(Value::Float(f64::NEG_INFINITY)),
            "-Infinity not allowed in JSON"
        );
        let mut deep = Value::array(vec![]);
        for _ in 0..99 {
            deep = Value::array(vec![deep]);
        }
        assert_eq!(generated(deep.clone()).len(), 200);
        deep = Value::array(vec![deep]);
        assert_eq!(
            sentence(deep),
            "nesting of 101 is too deep. Did you try to serialize objects with circular references?"
        );
    }

    #[test]
    #[should_panic(expected = "a Range is not JSON — generate its 'to_a'")]
    fn a_range_refuses() {
        generate(&Value::Range {
            end: Some(3),
            exclusive: false,
            start: Some(1),
        });
    }
}
