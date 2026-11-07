//! CRuby's `rb_reg_preprocess` (`re.c`): the pass Ruby runs over a pattern
//! before handing it to Onigmo. It turns `\xHH`, `\0oo`, `\cX`, `\C-X`, and
//! `\M-X` into bytes (several of them into one UTF-8 character when they form
//! one), `\uHHHH` and `\u{...}` into characters, strips `(?#...)` comments and
//! the `#` comments of extended mode, and leaves everything else as written.
//! An ASCII result of a byte escape is written back as `\xHH` so Onigmo sees
//! it as the escape it was. The refusals are Ruby's sentences.

use super::error::{self, RegexError};

pub fn preprocess(pattern: &str, extended: bool) -> Result<String, RegexError> {
    let bytes = pattern.as_bytes();
    let mut position = 0;
    let mut output: Vec<u8> = Vec::with_capacity(bytes.len());
    unescape(bytes, &mut position, &mut output, extended, false)?;
    String::from_utf8(output).map_err(|_| RegexError::new(error::INVALID_MULTIBYTE_CHARACTER))
}

fn unescape(
    bytes: &[u8],
    position: &mut usize,
    output: &mut Vec<u8>,
    extended: bool,
    recurse: bool,
) -> Result<(), RegexError> {
    let end = bytes.len();
    let mut in_char_class = 0;
    let mut parens = 1;
    let mut extended_mode = extended;
    while *position < end {
        let byte = bytes[*position];
        if byte >= 0x80 {
            // A multibyte character is copied whole; the pattern is valid
            // UTF-8 already, so its length is in the lead byte.
            let length = utf8_length(byte);
            output.extend_from_slice(&bytes[*position..*position + length]);
            *position += length;
            continue;
        }
        *position += 1;
        match byte {
            b'\\' => {
                if *position == end {
                    return Err(RegexError::new(error::TOO_SHORT_ESCAPE_SEQUENCE));
                }
                let escaped = bytes[*position];
                if escaped >= 0x80 {
                    // An escaped multibyte character: the backslash and the
                    // character pass through together.
                    let length = utf8_length(escaped);
                    output.push(b'\\');
                    output.extend_from_slice(&bytes[*position..*position + length]);
                    *position += length;
                    continue;
                }
                *position += 1;
                match escaped {
                    b'1'..=b'7' => {
                        // \O, \OO, \OOO, or a backreference: a value up to
                        // 0177 passes through, since re-escaping could
                        // break the backreference.
                        let (value, _) = scan_octal(bytes, *position - 1, 3);
                        if value <= 0o177 {
                            output.push(b'\\');
                            output.push(escaped);
                        } else {
                            *position -= 2;
                            unescape_escaped_nonascii(bytes, position, output)?;
                        }
                    }
                    b'0' | b'x' | b'c' | b'C' | b'M' => {
                        *position -= 2;
                        unescape_escaped_nonascii(bytes, position, output)?;
                    }
                    b'u' => {
                        if *position == end {
                            return Err(RegexError::new(error::TOO_SHORT_ESCAPE_SEQUENCE));
                        }
                        if bytes[*position] == b'{' {
                            *position += 1;
                            unescape_unicode_list(bytes, position, output)?;
                            if *position == end || bytes[*position] != b'}' {
                                return Err(RegexError::new(error::INVALID_UNICODE_LIST));
                            }
                            *position += 1;
                        } else {
                            unescape_unicode_bmp(bytes, position, output)?;
                        }
                    }
                    _ => {
                        output.push(b'\\');
                        output.push(escaped);
                    }
                }
            }
            b'#' => {
                if extended_mode && in_char_class == 0 {
                    while *position < end {
                        let comment_byte = bytes[*position];
                        *position += 1;
                        if comment_byte == b'\n' {
                            break;
                        }
                    }
                } else {
                    output.push(byte);
                }
            }
            b'[' => {
                in_char_class += 1;
                output.push(byte);
            }
            b']' => {
                if in_char_class > 0 {
                    in_char_class -= 1;
                }
                output.push(byte);
            }
            b')' => {
                output.push(byte);
                if in_char_class == 0 && recurse {
                    parens -= 1;
                    if parens == 0 {
                        return Ok(());
                    }
                }
            }
            b'(' => {
                if in_char_class == 0 && *position + 1 < end && bytes[*position] == b'?' {
                    if bytes[*position + 1] == b'#' {
                        // (?# is a comment anywhere; an unterminated one is
                        // left for the parser to refuse.
                        let original = *position;
                        let mut closed = false;
                        while *position < end {
                            let comment_byte = bytes[*position];
                            *position += 1;
                            if comment_byte == b'\\' {
                                if *position < end {
                                    *position += utf8_length(bytes[*position]).max(1);
                                }
                            } else if comment_byte == b')' {
                                closed = true;
                                break;
                            }
                        }
                        if !closed {
                            *position = original;
                            output.push(b'(');
                        }
                        continue;
                    }
                    // A potential change of the extended option.
                    if recurse {
                        parens += 1;
                    }
                    let mut invert = false;
                    let mut local_extend = 0;
                    let mut scan = *position + 1;
                    let mut handled = false;
                    while scan < end {
                        match bytes[scan] {
                            b'x' => local_extend = if invert { -1 } else { 1 },
                            b'-' => invert = true,
                            b':' | b')' => {
                                if local_extend == 0
                                    || (local_extend == -1 && !extended_mode)
                                    || (local_extend == 1 && extended_mode)
                                {
                                    break;
                                }
                                if bytes[scan] == b':' {
                                    output.push(b'(');
                                    unescape(bytes, position, output, local_extend == 1, true)?;
                                    handled = true;
                                } else {
                                    extended_mode = local_extend == 1;
                                }
                                break;
                            }
                            b'i' | b'm' | b'a' | b'd' | b'u' => {}
                            _ => break,
                        }
                        scan += 1;
                    }
                    if handled {
                        continue;
                    }
                } else if in_char_class == 0 && recurse {
                    parens += 1;
                }
                output.push(byte);
            }
            _ => output.push(byte),
        }
    }
    Ok(())
}

fn utf8_length(lead: u8) -> usize {
    if lead < 0x80 {
        1
    } else if lead >> 5 == 0b110 {
        2
    } else if lead >> 4 == 0b1110 {
        3
    } else {
        4
    }
}

/// `scan_oct`: up to `max` octal digits from `start`; the value and the count.
fn scan_octal(bytes: &[u8], start: usize, max: usize) -> (u32, usize) {
    let mut value = 0u32;
    let mut count = 0;
    while count < max && start + count < bytes.len() {
        let digit = bytes[start + count];
        if !(b'0'..=b'7').contains(&digit) {
            break;
        }
        value = value * 8 + u32::from(digit - b'0');
        count += 1;
    }
    (value, count)
}

/// `scan_hex`: up to `max` hex digits from `start`; the value and the count.
fn scan_hex(bytes: &[u8], start: usize, max: usize) -> (u32, usize) {
    let mut value = 0u32;
    let mut count = 0;
    while count < max && start + count < bytes.len() {
        let digit = bytes[start + count];
        let Some(digit_value) = (digit as char).to_digit(16) else {
            break;
        };
        value = value * 16 + digit_value;
        count += 1;
    }
    (value, count)
}

/// `read_escaped_byte`: one byte from a `\xHH`, `\ooo`, `\cX`, `\C-X`, or
/// `\M-X` escape at `position`, with Ruby's refusals.
fn read_escaped_byte(bytes: &[u8], position: &mut usize) -> Result<u8, RegexError> {
    let end = bytes.len();
    if *position == end || bytes[*position] != b'\\' {
        return Err(RegexError::new("too short escaped multibyte character"));
    }
    *position += 1;
    let mut meta_prefix = false;
    let mut control_prefix = false;
    let code: u32;
    loop {
        if *position == end {
            return Err(RegexError::new(error::TOO_SHORT_ESCAPE_SEQUENCE));
        }
        let escaped = bytes[*position];
        *position += 1;
        match escaped {
            b'\\' => {
                code = u32::from(b'\\');
                break;
            }
            b'n' => {
                code = 10;
                break;
            }
            b't' => {
                code = 9;
                break;
            }
            b'r' => {
                code = 13;
                break;
            }
            b'f' => {
                code = 12;
                break;
            }
            b'v' => {
                code = 11;
                break;
            }
            b'a' => {
                code = 7;
                break;
            }
            b'e' => {
                code = 27;
                break;
            }
            b'0'..=b'7' => {
                let (value, count) = scan_octal(bytes, *position - 1, 3);
                *position = *position - 1 + count;
                code = value;
                break;
            }
            b'x' => {
                let (value, count) = scan_hex(bytes, *position, 2);
                if count < 1 {
                    return Err(RegexError::new(error::INVALID_HEX_ESCAPE));
                }
                *position += count;
                code = value;
                break;
            }
            b'M' => {
                if meta_prefix {
                    return Err(RegexError::new(error::DUPLICATE_META_ESCAPE));
                }
                meta_prefix = true;
                if *position + 1 < end && bytes[*position] == b'-' && bytes[*position + 1] < 0x80 {
                    *position += 1;
                    if bytes[*position] == b'\\' {
                        *position += 1;
                        continue;
                    }
                    code = u32::from(bytes[*position]);
                    *position += 1;
                    break;
                }
                return Err(RegexError::new(error::TOO_SHORT_META_ESCAPE));
            }
            b'C' | b'c' => {
                if escaped == b'C' {
                    if *position == end || bytes[*position] != b'-' {
                        return Err(RegexError::new(error::TOO_SHORT_CONTROL_ESCAPE));
                    }
                    *position += 1;
                }
                if control_prefix {
                    return Err(RegexError::new(error::DUPLICATE_CONTROL_ESCAPE));
                }
                control_prefix = true;
                if *position < end && bytes[*position] < 0x80 {
                    if bytes[*position] == b'\\' {
                        *position += 1;
                        continue;
                    }
                    code = u32::from(bytes[*position]);
                    *position += 1;
                    break;
                }
                return Err(RegexError::new(error::TOO_SHORT_CONTROL_ESCAPE));
            }
            _ => return Err(RegexError::new(error::UNEXPECTED_ESCAPE_SEQUENCE)),
        }
    }
    if code > 0xff {
        return Err(RegexError::new(error::INVALID_ESCAPE_CODE));
    }
    let mut value = code as u8;
    if control_prefix {
        value &= 0x1f;
    }
    if meta_prefix {
        value |= 0x80;
    }
    Ok(value)
}

/// `unescape_escaped_nonascii`: byte escapes that together form one UTF-8
/// character become that character; a lone ASCII byte is written back as
/// `\xHH`.
fn unescape_escaped_nonascii(
    bytes: &[u8],
    position: &mut usize,
    output: &mut Vec<u8>,
) -> Result<(), RegexError> {
    let mut character: Vec<u8> = Vec::with_capacity(4);
    character.push(read_escaped_byte(bytes, position)?);
    let needed = if character[0] < 0x80 {
        1
    } else {
        utf8_length(character[0])
    };
    while character.len() < needed && character.len() < 4 {
        if *position == bytes.len() || bytes[*position] != b'\\' {
            break;
        }
        character.push(read_escaped_byte(bytes, position)?);
    }
    if character.len() == 1 && character[0] < 0x80 {
        output.extend_from_slice(format!("\\x{:02X}", character[0]).as_bytes());
        return Ok(());
    }
    if std::str::from_utf8(&character).is_err() {
        return Err(RegexError::new(error::INVALID_MULTIBYTE_ESCAPE));
    }
    output.extend_from_slice(&character);
    Ok(())
}

fn check_unicode_range(code: u32) -> Result<(), RegexError> {
    if (0xd800..=0xdfff).contains(&code) || code > 0x10ffff {
        return Err(RegexError::new(error::INVALID_UNICODE_RANGE));
    }
    Ok(())
}

fn append_utf8(code: u32, output: &mut Vec<u8>) -> Result<(), RegexError> {
    check_unicode_range(code)?;
    if code < 0x80 {
        output.extend_from_slice(format!("\\x{code:02X}").as_bytes());
    } else {
        let character = char::from_u32(code).expect("checked range");
        let mut buffer = [0u8; 4];
        output.extend_from_slice(character.encode_utf8(&mut buffer).as_bytes());
    }
    Ok(())
}

fn unescape_unicode_list(
    bytes: &[u8],
    position: &mut usize,
    output: &mut Vec<u8>,
) -> Result<(), RegexError> {
    let end = bytes.len();
    let mut has_unicode = false;
    while *position < end && bytes[*position].is_ascii_whitespace() {
        *position += 1;
    }
    loop {
        let (code, count) = scan_hex(bytes, *position, usize::MAX);
        if count == 0 {
            break;
        }
        if count > 6 {
            return Err(RegexError::new(error::INVALID_UNICODE_RANGE));
        }
        *position += count;
        append_utf8(code, output)?;
        has_unicode = true;
        while *position < end && bytes[*position].is_ascii_whitespace() {
            *position += 1;
        }
    }
    if !has_unicode {
        return Err(RegexError::new(error::INVALID_UNICODE_LIST));
    }
    Ok(())
}

fn unescape_unicode_bmp(
    bytes: &[u8],
    position: &mut usize,
    output: &mut Vec<u8>,
) -> Result<(), RegexError> {
    if bytes.len() < *position + 4 {
        return Err(RegexError::new(error::INVALID_UNICODE_ESCAPE));
    }
    let (code, count) = scan_hex(bytes, *position, 4);
    if count != 4 {
        return Err(RegexError::new(error::INVALID_UNICODE_ESCAPE));
    }
    append_utf8(code, output)?;
    *position += 4;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_escapes_become_characters_or_stay_escapes() {
        assert_eq!(preprocess("\\xE8\\xBF\\xBA", false).unwrap(), "迺");
        assert_eq!(preprocess("\\x41", false).unwrap(), "\\x41");
        assert_eq!(preprocess("\\u3042", false).unwrap(), "あ");
        assert_eq!(preprocess("\\u{41 3042}", false).unwrap(), "\\x41あ");
        assert_eq!(preprocess("\\x{3042}", false).unwrap_err().message, "invalid hex escape");
        assert_eq!(preprocess("\\c\\\\", false).unwrap(), "\\x1C");
        assert_eq!(
            preprocess("\\M-\\C-a", false).unwrap_err().message,
            "invalid multibyte escape"
        );
        assert_eq!(preprocess("\\M-a", false).unwrap_err().message, "invalid multibyte escape");
    }

    #[test]
    fn comments_go_and_the_rest_stays() {
        assert_eq!(preprocess("a(?#note)b", false).unwrap(), "ab");
        assert_eq!(preprocess("a # note\nb", true).unwrap(), "a b");
        assert_eq!(preprocess("(?x)a # note\nb", false).unwrap(), "(?x)a b");
        assert_eq!(preprocess("[#]", true).unwrap(), "[#]");
        assert_eq!(preprocess("\\d+\\.\\w", false).unwrap(), "\\d+\\.\\w");
    }
}
