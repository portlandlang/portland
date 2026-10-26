//! `format(template, values...)` and its twin `sprintf` (ADR 0067): Ruby's
//! directives whole — `%d %i %u %b %B %o %x %X %e %E %f %g %G %a %A %c %p
//! %s %%`, the flags `-`, `+`, a space, `0`, and `#`, width and precision,
//! `*` and `n$` from the arguments, and `%<name>d` and `%{name}` from the
//! keyword arguments. The refusals are Ruby's words (4.0.7) where Ruby has
//! them; every one is a panic, since a refusal here is a program's mistake,
//! not a sad path a caller handles.
//!
//! Where Portland's strings differ from Ruby's, the difference is ADR 0038's:
//! width and precision on `%s`, `%p`, and `%c` count graphemes, and `%c` of a
//! string is its first grapheme, not its first codepoint.

use crate::value::Value;
use unicode_segmentation::UnicodeSegmentation;

/// The flags a directive may carry, each once.
#[derive(Clone, Copy, Default)]
struct Flags {
    alternate: bool,
    left: bool,
    plus: bool,
    space: bool,
    zero: bool,
}

/// Where a directive's value comes from: the next argument in order, an
/// absolute position, or a keyword. A template uses one style throughout.
#[derive(Clone, Copy, PartialEq)]
enum Sourcing {
    Named,
    Numbered,
    Unnumbered,
    Unset,
}

struct Formatter<'a> {
    values: &'a [Value],
    named: &'a [(String, Value)],
    next: usize,
    sourcing: Sourcing,
}

/// Format `template` with `values` and the keyword `named` as Ruby's
/// `format` would.
pub fn format_values(template: &str, values: &[Value], named: &[(String, Value)]) -> String {
    let mut formatter = Formatter {
        values,
        named,
        next: 0,
        sourcing: Sourcing::Unset,
    };
    formatter.run(template)
}

impl Formatter<'_> {
    fn run(&mut self, template: &str) -> String {
        let characters: Vec<char> = template.chars().collect();
        let mut output = String::new();
        let mut at = 0;
        while at < characters.len() {
            if characters[at] != '%' {
                output.push(characters[at]);
                at += 1;
                continue;
            }
            at += 1;
            at = self.directive(&characters, at, &mut output);
        }
        output
    }

    /// One directive, `at` just past its `%`; answers where the template
    /// continues. The grammar is Ruby's sprintf.c, refusals included.
    fn directive(&mut self, characters: &[char], mut at: usize, output: &mut String) -> usize {
        let mut flags = Flags::default();
        let mut width: Option<i64> = None;
        let mut precision: Option<i64> = None;
        let mut position: Option<usize> = None;
        let mut name: Option<(String, char)> = None;
        // Ruby's states: flags may not follow a width or a precision, and
        // neither may be given twice.
        let mut saw_width = false;
        let mut saw_precision = false;
        loop {
            let Some(&character) = characters.get(at) else {
                if at == characters.len() && characters.get(at - 1) == Some(&'%') {
                    panic!("incomplete format specifier; use %% (double %) instead");
                }
                panic!("malformed format string");
            };
            match character {
                ' ' | '#' | '+' | '-' | '0' => {
                    if saw_precision {
                        panic!("flag after precision");
                    }
                    if saw_width {
                        panic!("flag after width");
                    }
                    match character {
                        ' ' => flags.space = true,
                        '#' => flags.alternate = true,
                        '+' => flags.plus = true,
                        '-' => flags.left = true,
                        _ => flags.zero = true,
                    }
                    at += 1;
                }
                '1'..='9' => {
                    let (number, after) = read_digits(characters, at);
                    if characters.get(after) == Some(&'$') {
                        if position.is_some() {
                            panic!("value given twice - {number}$");
                        }
                        self.note_numbered(number);
                        position = Some(number as usize);
                        at = after + 1;
                    } else {
                        if saw_width {
                            panic!("width given twice");
                        }
                        if saw_precision {
                            panic!("width after precision");
                        }
                        saw_width = true;
                        width = Some(number);
                        at = after;
                    }
                }
                '*' => {
                    if saw_width {
                        panic!("width given twice");
                    }
                    if saw_precision {
                        panic!("width after precision");
                    }
                    saw_width = true;
                    let (count, after) = self.star_argument(characters, at + 1);
                    if count < 0 {
                        flags.left = true;
                        width = Some(-count);
                    } else {
                        width = Some(count);
                    }
                    at = after;
                }
                '.' => {
                    if saw_precision {
                        panic!("precision given twice");
                    }
                    saw_precision = true;
                    at += 1;
                    if characters.get(at) == Some(&'*') {
                        let (count, after) = self.star_argument(characters, at + 1);
                        precision = Some(count.max(0));
                        at = after;
                    } else {
                        let (number, after) = read_digits(characters, at);
                        precision = Some(number);
                        at = after;
                    }
                }
                '<' | '{' => {
                    let close = if character == '<' { '>' } else { '}' };
                    let start = at + 1;
                    let mut end = start;
                    while end < characters.len() && characters[end] != close {
                        end += 1;
                    }
                    if end == characters.len() {
                        panic!("malformed name - unmatched parenthesis");
                    }
                    let key: String = characters[start..end].iter().collect();
                    if let Some((earlier, _)) = &name {
                        panic!("named{character}{key}{close} after <{earlier}>");
                    }
                    self.note_named(&key, character, close);
                    name = Some((key, character));
                    at = end + 1;
                    if character == '{' {
                        // `%{name}` takes no type: it is `%s` of the value,
                        // and whatever follows is the template's own text.
                        let value = self.named_value(name.as_ref().unwrap(), close).clone();
                        let text = if matches!(value, Value::Nil) {
                            String::new()
                        } else {
                            value.to_string()
                        };
                        output.push_str(&pad_text(&text, flags, width, precision));
                        return at;
                    }
                }
                '%' => {
                    if saw_width || saw_precision || !is_default(flags) || position.is_some() {
                        panic!("invalid format character - %");
                    }
                    output.push('%');
                    return at + 1;
                }
                '\n' | '\0' => panic!("malformed format string"),
                _ => {
                    if !"bBdiuoxXeEfgGaAcps".contains(character) {
                        panic!("malformed format string - %{character}");
                    }
                    let value = match (&name, position) {
                        (Some(named), _) => self.named_value(named, '>').clone(),
                        (None, Some(index)) => self.numbered_value(index).clone(),
                        (None, None) => self.next_value().clone(),
                    };
                    output.push_str(&render(character, &value, flags, width, precision));
                    return at + 1;
                }
            }
        }
    }

    /// `*` or `*n$`: a width or precision read from the arguments.
    fn star_argument(&mut self, characters: &[char], at: usize) -> (i64, usize) {
        if matches!(characters.get(at), Some('1'..='9')) {
            let (number, after) = read_digits(characters, at);
            if characters.get(after) == Some(&'$') {
                self.note_numbered(number);
                let value = self.numbered_value(number as usize).clone();
                return (integer_for_star(&value), after + 1);
            }
        }
        let value = self.next_value().clone();
        (integer_for_star(&value), at)
    }

    fn note_numbered(&mut self, number: i64) {
        match self.sourcing {
            Sourcing::Unnumbered => panic!("numbered({number}) after unnumbered({})", self.next),
            Sourcing::Named => panic!("numbered({number}) after named"),
            _ => self.sourcing = Sourcing::Numbered,
        }
    }

    fn note_named(&mut self, key: &str, open: char, close: char) {
        match self.sourcing {
            Sourcing::Unnumbered => {
                panic!("named{open}{key}{close} after unnumbered({})", self.next)
            }
            Sourcing::Numbered => panic!("named{open}{key}{close} after numbered"),
            _ => self.sourcing = Sourcing::Named,
        }
    }

    fn next_value(&mut self) -> &Value {
        match self.sourcing {
            Sourcing::Numbered => {
                panic!("unnumbered({}) mixed with numbered", self.next + 1)
            }
            Sourcing::Named => panic!("unnumbered({}) mixed with named", self.next + 1),
            _ => self.sourcing = Sourcing::Unnumbered,
        }
        let value = self.values.get(self.next).expect_or_too_few();
        self.next += 1;
        value
    }

    fn numbered_value(&self, index: usize) -> &Value {
        self.values.get(index - 1).expect_or_too_few()
    }

    fn named_value(&self, named: &(String, char), close: char) -> &Value {
        let (key, open) = named;
        match self.named.iter().find(|(label, _)| label == key) {
            Some((_, value)) => value,
            None => panic!("key{open}{key}{close} not found"),
        }
    }
}

/// Ruby's one sentence for every missing positional argument.
fn too_few() -> ! {
    panic!("too few arguments")
}

trait OrTooFew<'a> {
    fn expect_or_too_few(self) -> &'a Value;
}

impl<'a> OrTooFew<'a> for Option<&'a Value> {
    fn expect_or_too_few(self) -> &'a Value {
        match self {
            Some(value) => value,
            None => too_few(),
        }
    }
}

fn is_default(flags: Flags) -> bool {
    !(flags.alternate || flags.left || flags.plus || flags.space || flags.zero)
}

/// A run of decimal digits as a number, and where it ended.
fn read_digits(characters: &[char], mut at: usize) -> (i64, usize) {
    let mut number: i64 = 0;
    while let Some(digit) = characters
        .get(at)
        .and_then(|character| character.to_digit(10))
    {
        number = number
            .checked_mul(10)
            .and_then(|number| number.checked_add(i64::from(digit)))
            .unwrap_or_else(|| panic!("width too big"));
        at += 1;
    }
    (number, at)
}

/// A `*` width or precision must be an Integer; Ruby's words otherwise.
fn integer_for_star(value: &Value) -> i64 {
    match value {
        Value::Integer(number) => *number,
        Value::Nil => panic!("no implicit conversion from nil to integer"),
        other => panic!(
            "no implicit conversion of {} into Integer",
            other.type_name()
        ),
    }
}

/// `nil`, `true`, and `false` are named as themselves in a conversion
/// refusal; everything else by its type, as Ruby does.
fn conversion_name(value: &Value) -> String {
    match value {
        Value::Nil => "nil".to_string(),
        Value::Boolean(true) => "true".to_string(),
        Value::Boolean(false) => "false".to_string(),
        other => other.type_name(),
    }
}

/// The integer a `%d` family directive formats: an Integer as is, a Float
/// cut toward zero, a String read as `Integer()` reads it (strictly, with
/// base prefixes), anything else refused.
fn integer_of(value: &Value) -> i64 {
    match value {
        Value::Integer(number) => *number,
        Value::Float(number) => {
            if number.is_nan() {
                panic!("NaN");
            }
            if number.is_infinite() {
                panic!(
                    "{}",
                    if *number < 0.0 {
                        "-Infinity"
                    } else {
                        "Infinity"
                    }
                );
            }
            let whole = number.trunc();
            if whole >= 9_223_372_036_854_775_808.0 || whole < -9_223_372_036_854_775_808.0 {
                panic!("integer {whole} is past 64 bits");
            }
            whole as i64
        }
        Value::String(text) => strict_integer(text),
        other => panic!("can't convert {} into Integer", conversion_name(other)),
    }
}

/// Ruby's `Integer()` on a string: optional whitespace, an optional sign,
/// an optional base prefix (`0b`, `0o`, `0x`, or a bare leading `0` for
/// octal), digits with single underscores between them, optional trailing
/// whitespace, and nothing else.
fn strict_integer(text: &str) -> i64 {
    let invalid = || -> ! { panic!("invalid value for Integer(): {text:?}") };
    let trimmed = text.trim();
    let (negative, rest) = match trimmed.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };
    let lower = rest.to_ascii_lowercase();
    let (base, digits) = if let Some(digits) = lower.strip_prefix("0b") {
        (2, digits)
    } else if let Some(digits) = lower.strip_prefix("0o") {
        (8, digits)
    } else if let Some(digits) = lower.strip_prefix("0x") {
        (16, digits)
    } else if lower.len() > 1 && lower.starts_with('0') {
        (8, &lower[1..])
    } else {
        (10, lower.as_str())
    };
    if digits.is_empty()
        || digits.starts_with('_')
        || digits.ends_with('_')
        || digits.contains("__")
    {
        invalid();
    }
    let mut magnitude: i64 = 0;
    for character in digits.chars() {
        if character == '_' {
            continue;
        }
        let digit = character.to_digit(base).unwrap_or_else(|| invalid());
        magnitude = magnitude
            .checked_mul(i64::from(base))
            .and_then(|magnitude| magnitude.checked_add(i64::from(digit)))
            .unwrap_or_else(|| panic!("integer {trimmed} is past 64 bits"));
    }
    if negative { -magnitude } else { magnitude }
}

/// The float a `%f` family directive formats: a Float as is, an Integer
/// widened, a String read as `Float()` reads it, anything else refused.
fn float_of(value: &Value) -> f64 {
    match value {
        Value::Float(number) => *number,
        Value::Integer(number) => *number as f64,
        Value::String(text) => strict_float(text),
        other => panic!("can't convert {} into Float", conversion_name(other)),
    }
}

/// Ruby's `Float()` on a string: a decimal number with optional sign,
/// fraction, and exponent, single underscores between digits, surrounding
/// whitespace, and nothing else.
fn strict_float(text: &str) -> f64 {
    let invalid = || -> ! { panic!("invalid value for Float(): {text:?}") };
    let trimmed = text.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('_')
        || trimmed.ends_with('_')
        || trimmed.contains("__")
    {
        invalid();
    }
    let mut cleaned = String::new();
    let characters: Vec<char> = trimmed.chars().collect();
    for (index, character) in characters.iter().enumerate() {
        if *character == '_' {
            let between_digits = index > 0
                && characters[index - 1].is_ascii_digit()
                && characters
                    .get(index + 1)
                    .is_some_and(|next| next.is_ascii_digit());
            if !between_digits {
                invalid();
            }
            continue;
        }
        cleaned.push(*character);
    }
    if cleaned.contains("inf") || cleaned.contains("nan") || cleaned.contains("0x") {
        invalid();
    }
    cleaned.parse::<f64>().unwrap_or_else(|_| invalid())
}

/// One directive's text, before its own width is applied where the flags
/// say so, and after.
fn render(
    kind: char,
    value: &Value,
    flags: Flags,
    width: Option<i64>,
    precision: Option<i64>,
) -> String {
    match kind {
        'b' | 'B' | 'd' | 'i' | 'u' | 'o' | 'x' | 'X' => {
            render_integer(kind, integer_of(value), flags, width, precision)
        }
        'e' | 'E' | 'f' | 'g' | 'G' | 'a' | 'A' => {
            render_float(kind, float_of(value), flags, width, precision)
        }
        'c' => {
            let text = match value {
                Value::Integer(code) => match u32::try_from(*code).ok().and_then(char::from_u32) {
                    Some(character) => character.to_string(),
                    None => panic!("invalid character"),
                },
                Value::String(text) => text.graphemes(true).next().unwrap_or("").to_string(),
                Value::Nil => panic!("no implicit conversion from nil to integer"),
                other => panic!(
                    "no implicit conversion of {} into Integer",
                    other.type_name()
                ),
            };
            pad_text(&text, flags, width, None)
        }
        'p' => pad_text(&value.inspect(), flags, width, precision),
        _ => {
            let text = if matches!(value, Value::Nil) {
                String::new()
            } else {
                value.to_string()
            };
            pad_text(&text, flags, width, precision)
        }
    }
}

/// `%s`, `%p`, `%c`, and `%{name}`: the text cut to `precision` graphemes
/// and padded to `width` with spaces — on the right under `-`, and never
/// with zeros, which Ruby ignores on text.
fn pad_text(text: &str, flags: Flags, width: Option<i64>, precision: Option<i64>) -> String {
    let graphemes: Vec<&str> = text.graphemes(true).collect();
    let kept: String = match precision {
        Some(limit) => graphemes
            .iter()
            .take(usize::try_from(limit).unwrap_or(0))
            .copied()
            .collect(),
        None => text.to_string(),
    };
    let length = kept.graphemes(true).count();
    let width = width.map_or(0, |width| usize::try_from(width).unwrap_or(0));
    if length >= width {
        return kept;
    }
    let padding = " ".repeat(width - length);
    if flags.left {
        format!("{kept}{padding}")
    } else {
        format!("{padding}{kept}")
    }
}

/// Digits of `magnitude` in `base`, lowercase.
fn digits_of(mut magnitude: u64, base: u64) -> String {
    if magnitude == 0 {
        return "0".to_string();
    }
    let mut digits = Vec::new();
    while magnitude > 0 {
        digits.push(std::char::from_digit((magnitude % base) as u32, base as u32).unwrap());
        magnitude /= base;
    }
    digits.iter().rev().collect()
}

/// Ruby's two's complement spelling of a negative number in a power-of-two
/// base: the digits that are not all the top digit, with the infinite run
/// of top digits collapsed to one behind `..` — `-10` is `..10110` in
/// binary, `-196` is `..f3c` in hex. Answers the digits and the top digit.
fn complement_digits(number: i64, base: i64) -> (String, char) {
    let top = std::char::from_digit((base - 1) as u32, base as u32).unwrap();
    let mut digits = Vec::new();
    let mut remaining = number;
    while remaining != -1 {
        let digit = remaining.rem_euclid(base);
        digits.push(std::char::from_digit(digit as u32, base as u32).unwrap());
        remaining = (remaining - digit) / base;
    }
    (digits.iter().rev().collect(), top)
}

fn render_integer(
    kind: char,
    number: i64,
    flags: Flags,
    width: Option<i64>,
    precision: Option<i64>,
) -> String {
    let base: i64 = match kind {
        'b' | 'B' => 2,
        'o' => 8,
        'x' | 'X' => 16,
        _ => 10,
    };
    let uppercase = matches!(kind, 'B' | 'X');
    let signed = base == 10 || flags.plus || flags.space || number >= 0;
    // The sign, or the space a space flag leaves in front of a positive.
    let mut sign = String::new();
    let mut prefix = String::new();
    let mut body;
    let mut complement_top = None;
    if signed {
        if number < 0 {
            sign.push('-');
        } else if flags.plus {
            sign.push('+');
        } else if flags.space {
            sign.push(' ');
        }
        body = digits_of(number.unsigned_abs(), base as u64);
        // Precision on an integer is a least count of digits; precision 0
        // with a zero leaves nothing at all.
        if let Some(precision) = precision {
            if precision == 0 && number == 0 {
                body.clear();
            } else if (body.len() as i64) < precision {
                body = format!(
                    "{}{body}",
                    "0".repeat((precision - body.len() as i64) as usize)
                );
            }
        }
        if flags.alternate && number != 0 {
            match kind {
                'b' => prefix.push_str("0b"),
                'B' => prefix.push_str("0B"),
                'x' => prefix.push_str("0x"),
                'X' => prefix.push_str("0X"),
                'o' if !body.starts_with('0') => prefix.push('0'),
                _ => {}
            }
        } else if flags.alternate && kind == 'o' && precision == Some(0) {
            body.push('0');
        }
    } else {
        let (digits, top) = complement_digits(number, base);
        body = digits;
        complement_top = Some(top);
        if flags.alternate {
            match kind {
                'b' => prefix.push_str("0b"),
                'B' => prefix.push_str("0B"),
                'x' => prefix.push_str("0x"),
                'X' => prefix.push_str("0X"),
                _ => {}
            }
        }
    }
    if uppercase {
        body = body.to_uppercase();
    }
    let width = width.map_or(0, |width| usize::try_from(width).unwrap_or(0));
    let zero_fill = flags.zero && !flags.left && precision.is_none();
    match complement_top {
        Some(top) => {
            let top = if uppercase {
                top.to_ascii_uppercase()
            } else {
                top
            };
            // `..` and at least one top digit; under the zero flag the top
            // digit fills the width instead of spaces.
            let fixed = prefix.len() + 2 + body.len();
            let fill = if zero_fill && width > fixed + 1 {
                width - fixed
            } else {
                1
            };
            let text = format!("{prefix}..{}{body}", top.to_string().repeat(fill));
            pad_text(&text, flags, Some(width as i64), None)
        }
        None => {
            let fixed = sign.len() + prefix.len() + body.len();
            if zero_fill && width > fixed {
                format!("{sign}{prefix}{}{body}", "0".repeat(width - fixed))
            } else {
                pad_text(
                    &format!("{sign}{prefix}{body}"),
                    flags,
                    Some(width as i64),
                    None,
                )
            }
        }
    }
}

fn render_float(
    kind: char,
    number: f64,
    flags: Flags,
    width: Option<i64>,
    precision: Option<i64>,
) -> String {
    let uppercase = matches!(kind, 'E' | 'G' | 'A');
    let mut sign = String::new();
    if number.is_sign_negative() && !(number.is_nan()) {
        sign.push('-');
    } else if flags.plus {
        sign.push('+');
    } else if flags.space {
        sign.push(' ');
    }
    let width = width.map_or(0, |width| usize::try_from(width).unwrap_or(0));
    if number.is_nan() || number.is_infinite() {
        // Inf and NaN take spaces, never zeros, whatever the flags.
        let text = if number.is_nan() { "NaN" } else { "Inf" };
        return pad_text(&format!("{sign}{text}"), flags, Some(width as i64), None);
    }
    let magnitude = number.abs();
    let mut body = match kind {
        'f' => fixed(magnitude, precision.unwrap_or(6), flags.alternate),
        'e' | 'E' => exponential(magnitude, precision.unwrap_or(6), flags.alternate),
        'g' | 'G' => general(magnitude, precision, flags.alternate),
        _ => hex_float(magnitude, precision, flags.alternate),
    };
    if uppercase {
        body = body.to_uppercase();
    }
    // `%a`'s `0x` stays in front of the zero fill, as a prefix would.
    let (prefix, digits) = match body.strip_prefix("0x").or_else(|| body.strip_prefix("0X")) {
        Some(rest) => (body[..2].to_string(), rest.to_string()),
        None => (String::new(), body.clone()),
    };
    let fixed_length = sign.len() + prefix.len() + digits.len();
    if flags.zero && !flags.left && width > fixed_length {
        format!("{sign}{prefix}{}{digits}", "0".repeat(width - fixed_length))
    } else {
        pad_text(
            &format!("{sign}{prefix}{digits}"),
            flags,
            Some(width as i64),
            None,
        )
    }
}

/// `%f`: fixed notation with `precision` fraction digits; `#` keeps the
/// point when there are none.
fn fixed(magnitude: f64, precision: i64, alternate: bool) -> String {
    let precision = usize::try_from(precision).unwrap_or(0);
    let Digits { digits, decpt } = if magnitude == 0.0 {
        Digits {
            digits: String::new(),
            decpt: 1,
        }
    } else {
        mri_digits(magnitude, 3, precision as i32)
    };
    let whole = if decpt > 0 {
        let decpt = decpt as usize;
        let mut whole: String = digits.chars().take(decpt).collect();
        while whole.len() < decpt {
            whole.push('0');
        }
        whole
    } else {
        "0".to_string()
    };
    let mut fraction = if decpt >= 0 {
        digits.chars().skip(decpt as usize).collect::<String>()
    } else {
        format!("{}{digits}", "0".repeat((-decpt) as usize))
    };
    fraction.truncate(precision);
    while fraction.len() < precision {
        fraction.push('0');
    }
    if precision == 0 {
        if alternate {
            format!("{whole}.")
        } else {
            whole
        }
    } else {
        format!("{whole}.{fraction}")
    }
}

/// `%e`: `d.dddddde+XX`, the exponent with a sign and at least two digits.
fn exponential(magnitude: f64, precision: i64, alternate: bool) -> String {
    let precision = usize::try_from(precision).unwrap_or(0);
    let Digits { digits, decpt } = if magnitude == 0.0 {
        Digits {
            digits: "0".to_string(),
            decpt: 1,
        }
    } else {
        mri_digits(magnitude, 2, precision as i32 + 1)
    };
    exponential_from(&digits, decpt, precision, true, alternate)
}

/// The exponential spelling of digits whose value is `0.digits × 10^decpt`,
/// with `precision` fraction digits — padded with zeros only when `pad`
/// says so, which `%g` without `#` does not.
fn exponential_from(
    digits: &str,
    decpt: i32,
    precision: usize,
    pad: bool,
    alternate: bool,
) -> String {
    let mut characters = digits.chars();
    let first = characters.next().unwrap_or('0');
    let mut fraction: String = characters.collect();
    fraction.truncate(precision);
    if pad {
        while fraction.len() < precision {
            fraction.push('0');
        }
    }
    let exponent = decpt - 1;
    let sign = if exponent < 0 { '-' } else { '+' };
    let point = if fraction.is_empty() && !alternate {
        ""
    } else {
        "."
    };
    format!("{first}{point}{fraction}e{sign}{:02}", exponent.abs())
}

/// `%g`: C's rule — `precision` significant digits (6 by default, 0 reads
/// as 1), exponential when the exponent is below -4 or at least the
/// precision, fixed otherwise, and trailing zeros dropped unless `#`.
fn general(magnitude: f64, precision: Option<i64>, alternate: bool) -> String {
    let significant = match precision {
        None => 6,
        Some(0) => 1,
        Some(count) => usize::try_from(count).unwrap_or(1),
    };
    let Digits { digits, decpt } = if magnitude == 0.0 {
        Digits {
            digits: "0".to_string(),
            decpt: 1,
        }
    } else {
        mri_digits(magnitude, 2, significant as i32)
    };
    if decpt <= -4 || decpt > significant as i32 {
        return exponential_from(&digits, decpt, significant - 1, alternate, alternate);
    }
    let whole = if decpt > 0 {
        let decpt = decpt as usize;
        let mut whole: String = digits.chars().take(decpt).collect();
        while whole.len() < decpt {
            whole.push('0');
        }
        whole
    } else {
        "0".to_string()
    };
    let mut fraction = if decpt >= 0 {
        digits.chars().skip(decpt as usize).collect::<String>()
    } else {
        format!("{}{digits}", "0".repeat((-decpt) as usize))
    };
    if alternate {
        let wanted = (significant as i32 - decpt).max(0) as usize;
        while fraction.len() < wanted {
            fraction.push('0');
        }
    }
    if fraction.is_empty() && !alternate {
        whole
    } else {
        format!("{whole}.{fraction}")
    }
}

// ---- MRI's digits ---------------------------------------------------------
//
// The digits Ruby prints for a positive finite double come from David Gay's
// dtoa in its mode 2 (so many significant digits, for %e and %g) and mode 3
// (so many past the point, for %f) — as MRI's missing/dtoa.c modifies it,
// which is not a correctly rounded conversion. The fast floating-point path
// generates the digits; when the remainder after the last one lies within
// its error bound of a half, MRI sets a `half` flag and rounds that digit
// half to even on the digits alone, never consulting the exact value. So
// `format("%.2f", 2.675)` is `2.68` where the exact 2.67499… would give
// `2.67`, and `1.005` is `1.00`; outside that band the result is exact.
// Ported here so Portland's answers are Ruby's byte for byte (principle 2).

/// Digits without trailing zeros whose value is `0.digits × 10^decpt`; an
/// empty string is no digits at all.
struct Digits {
    digits: String,
    decpt: i32,
}

const TENS: [f64; 23] = [
    1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16,
    1e17, 1e18, 1e19, 1e20, 1e21, 1e22,
];
const BIGTENS: [f64; 5] = [1e16, 1e32, 1e64, 1e128, 1e256];

/// `value = d2 × 2^exponent` with `d2` in [1, 2) — dtoa's `d2` and `i`.
fn normalized(value: f64) -> (f64, i32) {
    let bits = value.to_bits();
    let raw_exponent = ((bits >> 52) & 0x7ff) as i32;
    if raw_exponent != 0 {
        let d2 = f64::from_bits((bits & 0x000f_ffff_ffff_ffff) | 0x3ff0_0000_0000_0000);
        return (d2, raw_exponent - 1023);
    }
    // Subnormal: scale up by a power of two until it is normal.
    let scaled = value * 2f64.powi(64);
    let (d2, exponent) = normalized(scaled);
    (d2, exponent - 64)
}

// dtoa's own log10(2) to fifteen places, kept as written so `k` comes out as
// MRI's does — the library constant is a different double.
#[allow(clippy::approx_constant)]
fn mri_digits(value: f64, mode: u8, ndigits: i32) -> Digits {
    let (d2, i) = normalized(value);
    let ds = (d2 - 1.5) * 0.289529654602168 + 0.1760912590558 + f64::from(i) * 0.301029995663981;
    let mut k = ds as i32;
    if ds < 0.0 && ds != f64::from(k) {
        k -= 1;
    }
    let mut k_check = true;
    if (0..=22).contains(&k) {
        if value < TENS[k as usize] {
            k -= 1;
        }
        k_check = false;
    }
    let (ilim, ilim1) = if mode == 2 {
        let count = ndigits.max(1);
        (count, count)
    } else {
        let count = ndigits + k + 1;
        (count, count - 1)
    };
    let mut half = false;
    if (0..=14).contains(&ilim) {
        match fast_path(value, k, k_check, ilim, ilim1, ndigits) {
            Fast::Decisive(digits) => return digits,
            Fast::Half => half = true,
            Fast::Failed => {}
        }
    }
    exact_round(exact_decimal(value), mode, ndigits, half)
}

enum Fast {
    Decisive(Digits),
    Failed,
    Half,
}

/// dtoa's "try to get by with floating-point arithmetic", modes 2 and 3.
fn fast_path(
    mut d: f64,
    mut k: i32,
    k_check: bool,
    mut ilim: i32,
    ilim1: i32,
    ndigits: i32,
) -> Fast {
    let mut i = 0usize;
    let mut ieps = 2;
    if k > 0 {
        let mut ds = TENS[(k & 0xf) as usize];
        let mut j = k >> 4;
        if j & 0x10 != 0 {
            j &= 0x10 - 1;
            d /= BIGTENS[4];
            ieps += 1;
        }
        while j != 0 {
            if j & 1 != 0 {
                ieps += 1;
                ds *= BIGTENS[i];
            }
            j >>= 1;
            i += 1;
        }
        d /= ds;
    } else if k != 0 {
        let j1 = -k;
        d *= TENS[(j1 & 0xf) as usize];
        let mut j = j1 >> 4;
        while j != 0 {
            if j & 1 != 0 {
                ieps += 1;
                d *= BIGTENS[i];
            }
            j >>= 1;
            i += 1;
        }
    }
    if k_check && d < 1.0 && ilim > 0 {
        if ilim1 <= 0 {
            return Fast::Failed;
        }
        ilim = ilim1;
        k -= 1;
        d *= 10.0;
        ieps += 1;
    }
    // `word0(eps) -= (P-1)*Exp_msk1`: the exponent field down by 52.
    let eps = (f64::from(ieps) * d + 7.0) * 2f64.powi(-52);
    if ilim == 0 {
        d -= 5.0;
        if d > eps {
            return Fast::Decisive(Digits {
                digits: "1".to_string(),
                decpt: k + 2,
            });
        }
        if d < -eps {
            return Fast::Decisive(Digits {
                digits: String::new(),
                decpt: -ndigits,
            });
        }
        return Fast::Failed;
    }
    let eps = eps * TENS[(ilim - 1) as usize];
    let mut digits = String::new();
    let mut count = 1;
    loop {
        let digit = d as i64;
        d -= digit as f64;
        if d == 0.0 {
            ilim = count;
        }
        digits.push(char::from(b'0' + digit as u8));
        if count == ilim {
            if d > 0.5 + eps {
                return Fast::Decisive(bump_up(digits, k));
            }
            if d < 0.5 - eps {
                let trimmed = digits.trim_end_matches('0').to_string();
                return Fast::Decisive(Digits {
                    digits: trimmed,
                    decpt: k + 1,
                });
            }
            if digit & 1 == 1 {
                return Fast::Decisive(bump_up(digits, k));
            }
            return Fast::Half;
        }
        count += 1;
        d *= 10.0;
    }
}

/// dtoa's `bump_up`: trailing nines fall away and the digit before them
/// goes up; all nines become a lone one a place higher.
fn bump_up(digits: String, k: i32) -> Digits {
    let trimmed = digits.trim_end_matches('9');
    if trimmed.is_empty() {
        return Digits {
            digits: "1".to_string(),
            decpt: k + 2,
        };
    }
    let mut kept: Vec<char> = trimmed.chars().collect();
    let last = kept.len() - 1;
    kept[last] = char::from(kept[last] as u8 + 1);
    Digits {
        digits: kept.into_iter().collect(),
        decpt: k + 1,
    }
}

/// The bignum path, with MRI's `half` flag: the exact digits cut at the
/// requested count and rounded — half to even when the exact value says so,
/// or, under `half`, up only when the last digit is odd.
fn exact_round(exact: (String, i32), mode: u8, ndigits: i32, half: bool) -> Digits {
    let (digits, decpt) = exact;
    let ilim = if mode == 2 {
        ndigits.max(1)
    } else {
        ndigits + decpt
    };
    if mode == 3 && ilim <= 0 {
        // dtoa's `ilim <= 0` for mode 3: one digit when the value is past
        // a half of the last place, none otherwise.
        if ilim == 0 && digits.as_str() > "5" {
            return Digits {
                digits: "1".to_string(),
                decpt: decpt + 1,
            };
        }
        return Digits {
            digits: String::new(),
            decpt: -ndigits,
        };
    }
    let ilim = ilim as usize;
    if digits.len() <= ilim {
        return Digits { digits, decpt };
    }
    let kept = &digits[..ilim];
    let rest = &digits[ilim..];
    let comparison = rest.cmp("5");
    let last_odd = kept.ends_with(['1', '3', '5', '7', '9']);
    let bump = if half {
        comparison != std::cmp::Ordering::Less && last_odd
    } else {
        comparison == std::cmp::Ordering::Greater
            || (comparison == std::cmp::Ordering::Equal && last_odd)
    };
    if bump {
        bump_up(kept.to_string(), decpt - 1)
    } else {
        Digits {
            digits: kept.trim_end_matches('0').to_string(),
            decpt,
        }
    }
}

/// A double's exact decimal expansion: `mantissa × 2^exponent` is
/// `mantissa × 5^-exponent / 10^-exponent` below one, so the digits are a
/// bignum's and the point is placed by counting. Answers digits without
/// trailing zeros and the point's place.
fn exact_decimal(value: f64) -> (String, i32) {
    let bits = value.to_bits();
    let raw_exponent = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1u64 << 52) - 1);
    let (mantissa, exponent) = if raw_exponent == 0 {
        (fraction, -1074)
    } else {
        (fraction | (1u64 << 52), raw_exponent - 1075)
    };
    let mut number = BigUnsigned::from_u64(mantissa);
    let mut decimal_shift = 0;
    if exponent >= 0 {
        for _ in 0..exponent {
            number.multiply(2);
        }
    } else {
        for _ in 0..-exponent {
            number.multiply(5);
        }
        decimal_shift = -exponent;
    }
    let text = number.to_decimal();
    let decpt = text.len() as i32 - decimal_shift;
    (text.trim_end_matches('0').to_string(), decpt)
}

/// Just enough bignum for the exact expansion: limbs of a billion, little
/// end first, multiplied by small numbers and written out in decimal.
struct BigUnsigned {
    limbs: Vec<u64>,
}

impl BigUnsigned {
    const BASE: u64 = 1_000_000_000;

    fn from_u64(mut value: u64) -> Self {
        let mut limbs = Vec::new();
        loop {
            limbs.push(value % Self::BASE);
            value /= Self::BASE;
            if value == 0 {
                break;
            }
        }
        Self { limbs }
    }

    fn multiply(&mut self, factor: u64) {
        let mut carry = 0;
        for limb in &mut self.limbs {
            let product = *limb * factor + carry;
            *limb = product % Self::BASE;
            carry = product / Self::BASE;
        }
        while carry > 0 {
            self.limbs.push(carry % Self::BASE);
            carry /= Self::BASE;
        }
    }

    fn to_decimal(&self) -> String {
        let mut text = String::new();
        for (index, limb) in self.limbs.iter().rev().enumerate() {
            if index == 0 {
                text.push_str(&limb.to_string());
            } else {
                text.push_str(&format!("{limb:09}"));
            }
        }
        text
    }
}

/// `%a`: `0xh.hhhhp+d` — the double's own bits as hex digits, trailing
/// zeros dropped unless a precision asks for them (and rounds to them,
/// half to even), `#` keeping the point.
fn hex_float(magnitude: f64, precision: Option<i64>, alternate: bool) -> String {
    if magnitude == 0.0 {
        let fraction = precision.map_or(String::new(), |count| {
            "0".repeat(usize::try_from(count).unwrap_or(0))
        });
        let point = if fraction.is_empty() && !alternate {
            ""
        } else {
            "."
        };
        return format!("0x0{point}{fraction}p+0");
    }
    let bits = magnitude.to_bits();
    let raw_exponent = ((bits >> 52) & 0x7ff) as i32;
    let mut mantissa = bits & ((1u64 << 52) - 1);
    let mut lead: u64 = 1;
    let mut exponent = raw_exponent - 1023;
    if raw_exponent == 0 {
        // Subnormal: no implicit one, exponent at the floor.
        lead = 0;
        exponent = -1022;
    }
    let mut digits = 13usize;
    if let Some(count) = precision {
        let count = usize::try_from(count).unwrap_or(0).min(13);
        let dropped_bits = 4 * (13 - count);
        if dropped_bits > 0 {
            let half = 1u64 << (dropped_bits - 1);
            let dropped = mantissa & ((1u64 << dropped_bits) - 1);
            mantissa >>= dropped_bits;
            if dropped > half || (dropped == half && mantissa & 1 == 1) {
                mantissa += 1;
                if mantissa >> (4 * count) != 0 {
                    mantissa = 0;
                    lead += 1;
                }
            }
        }
        digits = count;
    }
    let mut fraction = if digits == 0 {
        String::new()
    } else {
        format!("{mantissa:0width$x}", width = digits)
    };
    if precision.is_none() {
        fraction = fraction.trim_end_matches('0').to_string();
    }
    let point = if fraction.is_empty() && !alternate {
        ""
    } else {
        "."
    };
    let sign = if exponent < 0 { '-' } else { '+' };
    format!("0x{lead}{point}{fraction}p{sign}{}", exponent.abs())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(template: &str, values: &[Value]) -> String {
        format_values(template, values, &[])
    }

    fn integer(number: i64) -> Value {
        Value::Integer(number)
    }

    fn float(number: f64) -> Value {
        Value::Float(number)
    }

    fn string(text: &str) -> Value {
        Value::String(text.to_string())
    }

    #[test]
    fn integers_in_every_base_match_ruby() {
        assert_eq!(
            run(
                "%b %o %x %X %d",
                &[
                    integer(10),
                    integer(87),
                    integer(196),
                    integer(196),
                    integer(-112)
                ]
            ),
            "1010 127 c4 C4 -112"
        );
        assert_eq!(run("%b", &[integer(-10)]), "..10110");
        assert_eq!(run("%o", &[integer(-87)]), "..7651");
        assert_eq!(run("%x", &[integer(-196)]), "..f3c");
        assert_eq!(run("%X", &[integer(-1)]), "..F");
        assert_eq!(
            run("%#b %#x %#o", &[integer(10), integer(-196), integer(87)]),
            "0b1010 0x..f3c 0127"
        );
        assert_eq!(
            run("%010b|%010x", &[integer(-10), integer(-196)]),
            "..11110110|..ffffff3c"
        );
        assert_eq!(
            run("%08.3x|%.6b", &[integer(-1), integer(-10)]),
            "     ..f|..10110"
        );
        assert_eq!(
            run(
                "%+x % o %5.3d %-8b|",
                &[integer(-196), integer(87), integer(7), integer(-10)]
            ),
            "-c4  127   007 ..10110 |"
        );
        assert_eq!(
            run("%.0d|%2.0x|%#.0o", &[integer(0), integer(0), integer(0)]),
            "|  |0"
        );
        assert_eq!(run("%d %d", &[float(3.99), float(-3.99)]), "3 -3");
        assert_eq!(
            run(
                "%d %d %d %d",
                &[
                    string("0b1010"),
                    string("0127"),
                    string("0xc4"),
                    string(" -0x1f ")
                ]
            ),
            "10 87 196 -31"
        );
    }

    #[test]
    fn floats_match_ruby() {
        assert_eq!(
            run("%e %E", &[float(109.52), float(-0.10952)]),
            "1.095200e+02 -1.095200E-01"
        );
        assert_eq!(
            run(
                "%f %.10f %#.0f",
                &[float(1.555555555), float(10.952), float(123.4)]
            ),
            "1.555556 10.9520000000 123."
        );
        assert_eq!(
            run(
                "%g %g %g %g",
                &[
                    float(0.0000123456),
                    float(1234567.0),
                    float(0.0001),
                    float(123456.1234567)
                ]
            ),
            "1.23456e-05 1.23457e+06 0.0001 123456"
        );
        assert_eq!(
            run(
                "%#g %#.0g %.10g %g",
                &[float(123.4), float(123.4), float(12.1234), float(0.0)]
            ),
            "123.400 1.e+02 12.1234 0"
        );
        assert_eq!(
            run(
                "%f %+e % g %010f",
                &[
                    float(f64::INFINITY),
                    float(f64::INFINITY),
                    float(f64::NAN),
                    float(f64::NEG_INFINITY)
                ]
            ),
            "Inf +Inf  NaN       -Inf"
        );
        assert_eq!(
            run("%020e|%-20g|", &[float(109.52), float(12.1234)]),
            "000000001.095200e+02|12.1234             |"
        );
        assert_eq!(
            run(
                "%a %a %a %A",
                &[float(196.0), float(196.1), float(0.01), float(-0.01)]
            ),
            "0x1.88p+7 0x1.8833333333333p+7 0x1.47ae147ae147bp-7 -0X1.47AE147AE147BP-7"
        );
        assert_eq!(
            run(
                "%#.0a %.10a %020a",
                &[float(16.25), float(196.0), float(196.0)]
            ),
            "0x1.p+4 0x1.8800000000p+7 0x000000000001.88p+7"
        );
    }

    #[test]
    fn text_directives_match_ruby() {
        assert_eq!(
            run(
                "%c%c%c|%c|",
                &[integer(97), string("abc"), string("あいう"), string("")]
            ),
            "aaあ||"
        );
        assert_eq!(
            run(
                "%p %p %s %s|",
                &[Value::Nil, string("a"), Value::Nil, float(1.0)]
            ),
            "nil \"a\"  1.0|"
        );
        assert_eq!(
            run(
                "%.3s %-3.3s| %6s %4.6s %.2s",
                &[
                    string("hello"),
                    string("hello"),
                    string("abc"),
                    string("abcdefg"),
                    string("été")
                ]
            ),
            "hel hel|    abc abcdef ét"
        );
        assert_eq!(run("%% %%d %05s|", &[string("ab")]), "% %d    ab|");
    }

    #[test]
    fn positions_and_names_match_ruby() {
        assert_eq!(run("%2$s %1$s", &[string("x"), string("y")]), "y x");
        assert_eq!(run("%1$*2$d|%-2$d", &[integer(7), integer(5)]), "    7|5");
        assert_eq!(
            run(
                "%*d|%*d|%.*f",
                &[
                    integer(5),
                    integer(7),
                    integer(-5),
                    integer(7),
                    integer(2),
                    float(2.675)
                ]
            ),
            "    7|7    |2.68"
        );
        let named = [("foo".to_string(), float(10.952))];
        assert_eq!(
            format_values("%+20.10<foo>f|%<foo>+15.5f", &[], &named),
            "      +10.9520000000|      +10.95200"
        );
        let named = [("foo".to_string(), string("123456789"))];
        assert_eq!(
            format_values("%-20.5{foo}|%{foo}d", &[], &named),
            "12345               |123456789d"
        );
    }

    #[test]
    #[should_panic(expected = "too few arguments")]
    fn too_few_arguments_is_rubys() {
        run("%d %d", &[integer(1)]);
    }

    #[test]
    #[should_panic(expected = "unnumbered(1) mixed with numbered")]
    fn mixing_numbered_is_rubys() {
        run("%1$d %d", &[integer(1), integer(2)]);
    }

    #[test]
    #[should_panic(expected = "malformed format string - %q")]
    fn unknown_directive_is_rubys() {
        run("%q", &[integer(1)]);
    }

    #[test]
    #[should_panic(expected = "incomplete format specifier; use %% (double %) instead")]
    fn a_lone_percent_is_rubys() {
        run("%", &[]);
    }

    #[test]
    #[should_panic(expected = "can't convert nil into Integer")]
    fn nil_for_a_number_is_rubys() {
        run("%d", &[Value::Nil]);
    }

    #[test]
    #[should_panic(expected = "key<foo> not found")]
    fn a_missing_key_is_rubys() {
        format_values("%<foo>s", &[], &[("bar".to_string(), integer(1))]);
    }
}
