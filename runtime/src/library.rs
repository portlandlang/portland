//! The methods of a String and of an array that a native program calls
//! by name (#560): each is what the seed answers, written over the same
//! crates, and held to the seed by a fixture.
//!
//! Every function here borrows what it is handed: the compiled program
//! holds its arguments and lets go of them after the call. What a
//! function answers is new and held once.

use super::*;
use unicode_segmentation::UnicodeSegmentation;

/// A String's text.
///
/// # Safety
/// `text` points at a live String, whose bytes are UTF-8.
unsafe fn text_of<'text>(text: *const StringBlock) -> &'text str {
    unsafe { std::str::from_utf8_unchecked(bytes_of(text)) }
}

/// A new String holding this text, held once.
fn string_of(text: &str) -> *mut StringBlock {
    string_from(&[text.as_bytes()])
}

/// A new array of new Strings, held once.
fn strings_of<'piece>(pieces: impl Iterator<Item = &'piece str>) -> *mut ArrayBlock {
    let mut array = array_with(STRINGS, SMALLEST_ROOM);
    for piece in pieces {
        array = unsafe { portland_array_append(array, string_of(piece) as i64, STRINGS) };
    }
    array
}

/// Where a run starts and how long it is, by the seed's `slice(start,
/// length)`: a negative start counts from the end, a start past either
/// end is a run of nothing, and a negative length refuses.
fn run_of(count: usize, start: i64, length: i64) -> (usize, usize) {
    let length = usize::try_from(length).unwrap_or_else(|_| {
        refuse_program(&format!(
            "slice length must not be negative, got {length}"
        ))
    });
    let start = if start < 0 {
        start + count as i64
    } else {
        start
    };
    match usize::try_from(start) {
        Ok(start) if start <= count => (start, length.min(count - start)),
        _ => (count, 0),
    }
}

/// `array.drop(count)`: a new array without its first `count` elements.
///
/// # Safety
/// `array` is null or points at a live array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_array_drop(array: *mut ArrayBlock, count: i64) -> *mut ArrayBlock {
    if count < 0 {
        refuse_program(&format!("attempt to drop negative size, got {count}"));
    }
    unsafe { portland_array_slice(array, count, i64::MAX) }
}

/// `array.include?(value)`: whether an element equals the value, a
/// String by the equality a String has.
///
/// # Safety
/// `array` is null or points at a live array, and `value` is a word of
/// its kind.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_array_include(array: *mut ArrayBlock, value: i64) -> bool {
    unsafe {
        if array.is_null() {
            return false;
        }
        let elements = std::slice::from_raw_parts(elements_of(array), (*array).length as usize);
        match (*array).kind {
            STRINGS => elements.iter().any(|element| {
                portland_string_equal(*element as *const StringBlock, value as *const StringBlock)
            }),
            _ => elements.contains(&value),
        }
    }
}

/// `array.join`: the Strings of an array as one.
///
/// # Safety
/// `array` is null or points at a live array of Strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_array_join(array: *mut ArrayBlock) -> *mut StringBlock {
    unsafe { joined(array, "") }
}

/// `array.join(separator)`: the Strings of an array with the separator
/// between each two.
///
/// # Safety
/// `array` is null or points at a live array of Strings, and `separator`
/// points at a live String.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_array_join_with(
    array: *mut ArrayBlock,
    separator: *const StringBlock,
) -> *mut StringBlock {
    unsafe { joined(array, text_of(separator)) }
}

/// # Safety
/// `array` is null or points at a live array of Strings.
unsafe fn joined(array: *mut ArrayBlock, separator: &str) -> *mut StringBlock {
    unsafe {
        if array.is_null() {
            return string_of("");
        }
        let elements = std::slice::from_raw_parts(elements_of(array), (*array).length as usize);
        let pieces: Vec<&str> = elements
            .iter()
            .map(|element| text_of(*element as *const StringBlock))
            .collect();
        string_of(&pieces.join(separator))
    }
}

/// `array.slice(start, length)`: a new array of the run, each element
/// held once more.
///
/// # Safety
/// `array` is null or points at a live array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_array_slice(
    array: *mut ArrayBlock,
    start: i64,
    length: i64,
) -> *mut ArrayBlock {
    unsafe {
        if array.is_null() {
            return array_with(INTEGERS, SMALLEST_ROOM);
        }
        let kind = (*array).kind;
        let (start, length) = run_of((*array).length as usize, start, length);
        let mut run = array_with(kind, length.max(SMALLEST_ROOM));
        for index in start..start + length {
            let word = elements_of(array).add(index).read();
            hold_word(kind, word, true);
            run = portland_array_append(run, word, kind);
        }
        run
    }
}

/// `text.bytesize`.
///
/// # Safety
/// `text` points at a live String.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_string_bytesize(text: *const StringBlock) -> i64 {
    unsafe { bytes_of(text).len() as i64 }
}

/// `text.chars`: its characters, each a grapheme cluster, as the seed
/// counts them.
///
/// # Safety
/// `text` points at a live String, whose bytes are UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_string_chars(text: *const StringBlock) -> *mut ArrayBlock {
    unsafe { strings_of(text_of(text).graphemes(true)) }
}

/// `text.chop`: without its last character.
///
/// # Safety
/// `text` points at a live String, whose bytes are UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_string_chop(text: *const StringBlock) -> *mut StringBlock {
    unsafe {
        let mut characters: Vec<&str> = text_of(text).graphemes(true).collect();
        characters.pop();
        string_of(&characters.concat())
    }
}

/// `text.chr`: its first character, or the empty String.
///
/// # Safety
/// `text` points at a live String, whose bytes are UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_string_chr(text: *const StringBlock) -> *mut StringBlock {
    unsafe { string_of(text_of(text).graphemes(true).next().unwrap_or("")) }
}

/// `text.codepoints`: each code point as an Integer.
///
/// # Safety
/// `text` points at a live String, whose bytes are UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_string_codepoints(text: *const StringBlock) -> *mut ArrayBlock {
    unsafe {
        let mut array = array_with(INTEGERS, SMALLEST_ROOM);
        for character in text_of(text).chars() {
            array = portland_array_append(array, character as i64, INTEGERS);
        }
        array
    }
}

/// `text.downcase`.
///
/// # Safety
/// `text` points at a live String, whose bytes are UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_string_downcase(text: *const StringBlock) -> *mut StringBlock {
    unsafe { string_of(&text_of(text).to_lowercase()) }
}

/// `text.end_with?(suffix)`.
///
/// # Safety
/// Both point at live Strings, whose bytes are UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_string_end_with(
    text: *const StringBlock,
    suffix: *const StringBlock,
) -> bool {
    unsafe { text_of(text).ends_with(text_of(suffix)) }
}

/// `text.include?(needle)`.
///
/// # Safety
/// Both point at live Strings, whose bytes are UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_string_include(
    text: *const StringBlock,
    needle: *const StringBlock,
) -> bool {
    unsafe { text_of(text).contains(text_of(needle)) }
}

/// `text.index(needle)`: how many characters come before the first place
/// the needle is, or `i64::MIN` where it is nowhere, which the compiled
/// program reads as nothing.
///
/// # Safety
/// Both point at live Strings, whose bytes are UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_string_index(
    text: *const StringBlock,
    needle: *const StringBlock,
) -> i64 {
    unsafe {
        let text = text_of(text);
        text.find(text_of(needle)).map_or(i64::MIN, |position| {
            text[..position].graphemes(true).count() as i64
        })
    }
}

/// `text.slice(start, length)`: the run of characters, which is empty
/// past either end.
///
/// # Safety
/// `text` points at a live String, whose bytes are UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_string_slice(
    text: *const StringBlock,
    start: i64,
    length: i64,
) -> *mut StringBlock {
    unsafe {
        let characters: Vec<&str> = text_of(text).graphemes(true).collect();
        let (start, length) = run_of(characters.len(), start, length);
        string_of(&characters[start..start + length].concat())
    }
}

/// `String#split` by Ruby's rules, the seed's `ruby_split` line for line
/// (#142): a single space splits on runs of whitespace, an empty
/// separator into characters, a positive limit caps the pieces, and
/// trailing empty fields drop unless a limit is given.
fn ruby_split(text: &str, separator: Option<&str>, limit: i64) -> Vec<String> {
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

/// `text.split(separator)`.
///
/// # Safety
/// Both point at live Strings, whose bytes are UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_string_split(
    text: *const StringBlock,
    separator: *const StringBlock,
) -> *mut ArrayBlock {
    unsafe { portland_string_split_limit(text, separator, 0) }
}

/// `text.split(separator, limit)`.
///
/// # Safety
/// Both point at live Strings, whose bytes are UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_string_split_limit(
    text: *const StringBlock,
    separator: *const StringBlock,
    limit: i64,
) -> *mut ArrayBlock {
    unsafe {
        let pieces = ruby_split(text_of(text), Some(text_of(separator)), limit);
        strings_of(pieces.iter().map(String::as_str))
    }
}

/// `text.start_with?(prefix)`.
///
/// # Safety
/// Both point at live Strings, whose bytes are UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_string_start_with(
    text: *const StringBlock,
    prefix: *const StringBlock,
) -> bool {
    unsafe { text_of(text).starts_with(text_of(prefix)) }
}

/// `text.strip`: without the whitespace and the NULs at either end.
///
/// # Safety
/// `text` points at a live String, whose bytes are UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_string_strip(text: *const StringBlock) -> *mut StringBlock {
    unsafe {
        string_of(
            text_of(text)
                .trim_matches(|character: char| character.is_whitespace() || character == '\0'),
        )
    }
}

/// `text.upcase`.
///
/// # Safety
/// `text` points at a live String, whose bytes are UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_string_upcase(text: *const StringBlock) -> *mut StringBlock {
    unsafe { string_of(&text_of(text).to_uppercase()) }
}

/// Where a range starts and ends in a collection of this length, as the
/// seed's `slice_bounds` reads one: a negative end counts from the far
/// end, everything past either end is brought in, and a range that runs
/// backwards holds nothing. `mode` says whether the range leaves its last
/// place out (1), has no end (2), or both.
fn bounds_of(length: usize, start: i64, end: i64, mode: i64) -> (usize, usize) {
    let length = length as i64;
    let resolve = |bound: i64| if bound < 0 { length + bound } else { bound };
    let from = resolve(start).clamp(0, length);
    let to = if mode & 2 != 0 {
        length
    } else {
        let bound = resolve(end);
        (if mode & 1 != 0 { bound } else { bound.saturating_add(1) }).clamp(0, length)
    };
    (from as usize, to.max(from) as usize)
}

/// `array[start..end]`: a new array of the run, each element held once
/// more.
///
/// # Safety
/// `array` is null or points at a live array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_array_range(
    array: *mut ArrayBlock,
    start: i64,
    end: i64,
    mode: i64,
) -> *mut ArrayBlock {
    unsafe {
        if array.is_null() {
            return array_with(INTEGERS, SMALLEST_ROOM);
        }
        let (from, to) = bounds_of((*array).length as usize, start, end, mode);
        portland_array_slice(array, from as i64, (to - from) as i64)
    }
}

/// `text[index]`: the character there, counting from the end where the
/// index is negative, or nothing past either end.
///
/// # Safety
/// `text` points at a live String, whose bytes are UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_string_character(
    text: *const StringBlock,
    index: i64,
) -> *mut StringBlock {
    unsafe {
        let characters: Vec<&str> = text_of(text).graphemes(true).collect();
        let place = if index < 0 {
            index + characters.len() as i64
        } else {
            index
        };
        match usize::try_from(place).ok().and_then(|place| characters.get(place)) {
            Some(character) => string_of(character),
            None => std::ptr::null_mut(),
        }
    }
}

/// `text[start..end]`: the run of characters.
///
/// # Safety
/// `text` points at a live String, whose bytes are UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_string_range(
    text: *const StringBlock,
    start: i64,
    end: i64,
    mode: i64,
) -> *mut StringBlock {
    unsafe {
        let characters: Vec<&str> = text_of(text).graphemes(true).collect();
        let (from, to) = bounds_of(characters.len(), start, end, mode);
        string_of(&characters[from..to].concat())
    }
}
