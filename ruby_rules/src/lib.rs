//! Ruby's rules for text and numbers that Portland keeps (#581): how a
//! String splits, how one reads as an Integer, how one is padded, what
//! two Integers share, and how an Integer is written in a base.
//!
//! One copy, on purpose. The seed answers with it, and so does the
//! runtime every native program is linked with (ADR 0095). `split` once
//! had a copy in each, held together only by the cases a fixture tried.
//!
//! A rule that refuses answers the sentence to refuse with, and each
//! caller refuses in its own way: the seed panics, and the runtime
//! leaves as a native program does.

use unicode_segmentation::UnicodeSegmentation;

/// `String#split` by Ruby's rules (#142), each checked against Ruby 4.0.9:
/// no separator or a single space splits on runs of whitespace, leading
/// whitespace ignored; an empty separator splits into characters; a
/// positive limit caps the pieces, the last holding the rest; and trailing
/// empty fields drop unless a limit is given — a negative one keeps them.
/// An empty string has no fields at all.
pub fn split(text: &str, separator: Option<&str>, limit: i64) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    let cap = usize::try_from(limit).ok().filter(|cap| *cap > 0);
    let full = |pieces: &Vec<String>| cap.is_some_and(|cap| pieces.len() + 1 == cap);
    let mut pieces: Vec<String> = Vec::new();
    // Ruby's whitespace is ASCII's six — a U+2008 punctuation space is not
    // a field break.
    let ascii_space =
        |character: char| matches!(character, ' ' | '\t' | '\n' | '\u{b}' | '\u{c}' | '\r');
    match separator {
        None | Some(" ") => {
            let mut rest = text.trim_start_matches(ascii_space);
            let mut took_the_rest = false;
            while !rest.is_empty() {
                if full(&pieces) {
                    pieces.push(rest.to_string());
                    took_the_rest = true;
                    break;
                }
                let end = rest.find(ascii_space).unwrap_or(rest.len());
                pieces.push(rest[..end].to_string());
                rest = rest[end..].trim_start_matches(ascii_space);
            }
            // Trailing whitespace is one empty field, which a limit keeps:
            // `" 1 2 ".split(" ", 3)` is `["1", "2", ""]`, and `" "` with a
            // limit is `[""]`. Without one it drops below with the rest.
            if limit != 0 && !took_the_rest && text.ends_with(ascii_space) {
                pieces.push(String::new());
            }
        }
        Some("") => {
            let characters: Vec<&str> = text.graphemes(true).collect();
            for (index, character) in characters.iter().enumerate() {
                if full(&pieces) {
                    pieces.push(characters[index..].concat());
                    break;
                }
                pieces.push((*character).to_string());
            }
            // A limit past the last character — or a negative one — keeps
            // the empty field after it, as Ruby's `"ab".split("", -1)` does.
            if limit < 0 || cap.is_some_and(|cap| cap > characters.len()) {
                pieces.push(String::new());
            }
        }
        Some(separator) => {
            pieces = match cap {
                Some(cap) => text.splitn(cap, separator).map(str::to_string).collect(),
                None => text.split(separator).map(str::to_string).collect(),
            };
        }
    }
    if limit == 0 {
        while pieces.last().is_some_and(String::is_empty) {
            pieces.pop();
        }
    }
    pieces
}

/// `String#to_i` by Ruby's rule (ADR 0061), which never fails: leading
/// whitespace, one sign, the prefix of the base asked for — any of them,
/// or a bare `0` for octal, when the base is 0 — then digits of the base
/// with single underscores between digits; the first thing that is none
/// of these ends the number, and nothing parsed is 0. A base outside 2 to
/// 36 refuses in Ruby's words; a number past 64 bits refuses (#156).
pub fn to_i(text: &str, base: i64) -> Result<i64, String> {
    if base == 1 || !(0..=36).contains(&base) {
        return Err(format!("invalid radix {base}"));
    }
    let trimmed = text.trim_start_matches([' ', '\t', '\n', '\u{b}', '\u{c}', '\r']);
    let (negative, rest) = match trimmed.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };
    let lowered = rest.to_ascii_lowercase();
    let (base, rest) = if base == 0 {
        if let Some(after) = lowered.strip_prefix("0b") {
            (2, after.to_string())
        } else if let Some(after) = lowered.strip_prefix("0o") {
            (8, after.to_string())
        } else if let Some(after) = lowered.strip_prefix("0d") {
            (10, after.to_string())
        } else if let Some(after) = lowered.strip_prefix("0x") {
            (16, after.to_string())
        } else if lowered.starts_with('0') {
            (8, lowered.clone())
        } else {
            (10, lowered.clone())
        }
    } else {
        let prefix = match base {
            2 => "0b",
            8 => "0o",
            10 => "0d",
            16 => "0x",
            _ => "",
        };
        let after = if prefix.is_empty() {
            lowered.clone()
        } else {
            lowered
                .strip_prefix(prefix)
                .map_or_else(|| lowered.clone(), str::to_string)
        };
        (base as u32, after)
    };
    let mut value: i64 = 0;
    let mut previous_underscore = true;
    for character in rest.chars() {
        if character == '_' {
            if previous_underscore {
                break;
            }
            previous_underscore = true;
            continue;
        }
        let Some(digit) = character.to_digit(base) else {
            break;
        };
        value = value
            .checked_mul(i64::from(base))
            .and_then(|scaled| scaled.checked_add(i64::from(digit)))
            .ok_or_else(|| {
                format!(
                    "to_i overflows a 64-bit integer on {text:?} — integers are 64-bit here (#156)"
                )
            })?;
        previous_underscore = false;
    }
    Ok(if negative { -value } else { value })
}

/// `center`, `ljust`, and `rjust` (#106): the text padded to `width`
/// graphemes with `padding` repeated, `center` giving the extra to the
/// right as Ruby's does. An empty padding refuses in Ruby's words.
pub fn padded(text: &str, width: i64, padding: &str, method: &str) -> Result<String, String> {
    if padding.is_empty() {
        return Err("zero width padding".to_string());
    }
    let length = text.graphemes(true).count() as i64;
    if width <= length {
        return Ok(text.to_string());
    }
    let total = (width - length) as usize;
    let (left, right) = match method {
        "ljust" => (0, total),
        "rjust" => (total, 0),
        _ => (total / 2, total - total / 2),
    };
    let fill = |count: usize| -> String { padding.graphemes(true).cycle().take(count).collect() };
    Ok(format!("{}{}{}", fill(left), text, fill(right)))
}

/// Euclid on magnitudes, taken unsigned so the smallest integer has one.
pub fn magnitude_gcd(left: i64, right: i64) -> u64 {
    let (mut left, mut right) = (left.unsigned_abs(), right.unsigned_abs());
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

/// Integer `**` (ADR 0033) for an exponent that is not negative. Past-i64
/// results refuse the way past-i64 literals do.
pub fn power(left: i64, right: i64) -> Result<i64, String> {
    // A magnitude of one never grows, however tall the tower — Ruby's
    // answers, reached before the width check could refuse them.
    if left == 0 && right > 0 {
        return Ok(0);
    }
    if left == 1 {
        return Ok(1);
    }
    if left == -1 {
        return Ok(if right % 2 == 0 { 1 } else { -1 });
    }
    let overflows = || format!("{left} ** {right} overflows the 64-bit integers");
    let exponent = u32::try_from(right).map_err(|_| overflows())?;
    left.checked_pow(exponent).ok_or_else(overflows)
}

/// `gcd`: never a sign, and a zero yields the other side's magnitude (Ruby
/// 4.0.9, probed). The smallest integer's magnitude is past the edge.
pub fn gcd(number: i64, other: i64) -> Result<i64, String> {
    i64::try_from(magnitude_gcd(number, other))
        .map_err(|_| format!("{number}.gcd({other}) overflows the 64-bit integers"))
}

/// An Integer written in a base, 2 through 36, lowercase digits, the sign
/// out front: Ruby's shapes, including the refusal's bounds.
pub fn in_base(number: i64, base: i64) -> Result<String, String> {
    if !(2..=36).contains(&base) {
        return Err(format!("invalid radix {base}"));
    }
    Ok(integer_in_base(number, base))
}

/// A whole number written in another base, 2 through 36 — lowercase digits,
/// the sign out front, Ruby's shapes (probed on 4.0.9).
/// An Integer in a base the caller has already held to 2 through 36.
fn integer_in_base(number: i64, base: i64) -> String {
    if number == 0 {
        return "0".to_string();
    }
    let digits = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut magnitude = number.unsigned_abs();
    let mut written = Vec::new();
    while magnitude > 0 {
        written.push(digits[(magnitude % base as u64) as usize]);
        magnitude /= base as u64;
    }
    if number < 0 {
        written.push(b'-');
    }
    written.reverse();
    String::from_utf8(written).expect("base digits are ascii")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_follows_rubys_rules() {
        assert_eq!(split(" a  b ", None, 0), ["a", "b"]);
        assert_eq!(split("a,b,,", Some(","), 0), ["a", "b"]);
        assert_eq!(split("a,b,,", Some(","), -1), ["a", "b", "", ""]);
        assert_eq!(split("a,b,c", Some(","), 2), ["a", "b,c"]);
        assert_eq!(split("ab", Some(""), 0), ["a", "b"]);
        assert!(split("", Some(","), 0).is_empty());
    }

    #[test]
    fn to_i_never_fails_on_text_and_refuses_a_bad_base() {
        assert_eq!(to_i("42", 10), Ok(42));
        assert_eq!(to_i("  -17 apples", 10), Ok(-17));
        assert_eq!(to_i("1_000", 10), Ok(1000));
        assert_eq!(to_i("none", 10), Ok(0));
        assert_eq!(to_i("0x1f", 0), Ok(31));
        assert_eq!(to_i("ff", 16), Ok(255));
        assert_eq!(to_i("1", 1), Err("invalid radix 1".to_string()));
        assert!(to_i("99999999999999999999", 10).is_err());
    }

    #[test]
    fn padded_fills_to_a_width() {
        assert_eq!(padded("7", 4, "0", "rjust"), Ok("0007".to_string()));
        assert_eq!(padded("7", 4, "0", "ljust"), Ok("7000".to_string()));
        assert_eq!(padded("ab", 7, "xyz", "center"), Ok("xyabxyz".to_string()));
        assert_eq!(padded("seven", 3, "0", "rjust"), Ok("seven".to_string()));
        assert_eq!(
            padded("a", 3, "", "rjust"),
            Err("zero width padding".to_string())
        );
    }

    #[test]
    fn power_refuses_past_the_edge() {
        assert_eq!(power(2, 10), Ok(1024));
        assert_eq!(power(0, 0), Ok(1));
        assert_eq!(power(-1, 9_000_000_000), Ok(1));
        assert_eq!(
            power(2, 64),
            Err("2 ** 64 overflows the 64-bit integers".to_string())
        );
    }

    #[test]
    fn gcd_is_a_magnitude() {
        assert_eq!(gcd(12, 18), Ok(6));
        assert_eq!(gcd(-12, 0), Ok(12));
        assert_eq!(gcd(0, 0), Ok(0));
        assert!(gcd(i64::MIN, 0).is_err());
    }

    #[test]
    fn in_base_writes_the_sign_out_front() {
        assert_eq!(in_base(255, 16), Ok("ff".to_string()));
        assert_eq!(in_base(-255, 2), Ok("-11111111".to_string()));
        assert_eq!(in_base(0, 36), Ok("0".to_string()));
        assert_eq!(in_base(1, 37), Err("invalid radix 37".to_string()));
    }
}
