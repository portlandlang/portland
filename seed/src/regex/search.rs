//! `onig_search`: where to attempt a match, from `regexec.c`. The
//! optimizer's findings narrow the search before the matcher runs: a
//! pattern pinned to the start or the position tries once; one pinned to
//! the end by a bounded length tries only inside the last window; a
//! leading `.*` tries once per line; a literal every match contains is
//! found first (by a plain scan, Boyer-Moore, or their folded forms) and
//! attempts run only at the distances the literal allows; a map of opening
//! bytes skips to the next one. Ruby searches forward from a position with
//! the whole text in view, so only the forward search is here.

use super::compile::Program;
use super::optimize::{
    ANCHOR_ANYCHAR_STAR, ANCHOR_ANYCHAR_STAR_ML, ANCHOR_BEGIN_BUF, ANCHOR_BEGIN_LINE,
    ANCHOR_BEGIN_POSITION, ANCHOR_END_BUF, ANCHOR_END_LINE, ANCHOR_LOOK_BEHIND,
    ANCHOR_PREC_READ_NOT, ANCHOR_SEMI_END_BUF, INFINITE_DISTANCE, Info, Optimize,
};
use super::unicode;
use super::vm::Matcher;
use super::{Captures, RegexError};

/// The search over `text` from the byte offset `start`, with `budget`
/// matcher steps.
pub fn search(
    program: &Program,
    text: &str,
    start: usize,
    budget: u64,
) -> Result<Option<Captures>, RegexError> {
    let bytes = text.as_bytes();
    let end = bytes.len();
    let info = &program.optimize;
    let global_pos = start;
    let mut start = start;
    let mut range = end;
    if start > end {
        return Ok(None);
    }
    let mut matcher = Matcher::new(program, text, budget);
    // `MATCH_AND_RETURN_CHECK`.
    macro_rules! attempt {
        ($s:expr) => {
            if let Some(groups) = matcher.match_at($s, global_pos)? {
                return Ok(Some(Captures { groups }));
            }
        };
    }

    // The anchors narrow the start and the range.
    if info.anchor != 0 && end > 0 {
        if info.anchor & ANCHOR_BEGIN_POSITION != 0 || info.anchor & ANCHOR_ANYCHAR_STAR_ML != 0 {
            // Only the start position; a leading `(?m).*` reaches every
            // later position from it already.
            range = if range > start { start + 1 } else { start };
        } else if info.anchor & ANCHOR_BEGIN_BUF != 0 {
            if range > start {
                if start != 0 {
                    return Ok(None);
                }
                range = 1;
            } else if range == 0 {
                start = 0;
            } else {
                return Ok(None);
            }
        } else if info.anchor & (ANCHOR_END_BUF | ANCHOR_SEMI_END_BUF) != 0 {
            // `\z`: the match ends at the end; `\Z`: there or before a final
            // newline. With the pattern's length bounds, that pins where
            // it can start.
            let max_semi_end = end;
            let min_semi_end = if info.anchor & ANCHOR_END_BUF != 0 {
                Some(end)
            } else {
                let pre_end = prev_head(bytes, end);
                if bytes[pre_end] == b'\n' {
                    if pre_end > 0 && start <= pre_end {
                        Some(pre_end)
                    } else {
                        None
                    }
                } else {
                    Some(end)
                }
            };
            if let Some(min_semi_end) = min_semi_end {
                if (max_semi_end as u64) < info.anchor_dmin {
                    return Ok(None);
                }
                if range > start {
                    if (min_semi_end - start) as u64 > info.anchor_dmax {
                        start = min_semi_end - info.anchor_dmax as usize;
                        if start < end {
                            start = right_adjust(text, start);
                        }
                    }
                    if ((max_semi_end - (range - 1)) as u64) < info.anchor_dmin {
                        if (max_semi_end as u64 + 1) < info.anchor_dmin {
                            return Ok(None);
                        }
                        range = max_semi_end - info.anchor_dmin as usize + 1;
                    }
                    if start > range {
                        return Ok(None);
                    }
                } else {
                    if (min_semi_end - range) as u64 > info.anchor_dmax {
                        range = min_semi_end - info.anchor_dmax as usize;
                    }
                    if ((max_semi_end - start) as u64) < info.anchor_dmin {
                        if (max_semi_end as u64) < info.anchor_dmin {
                            return Ok(None);
                        }
                        start = max_semi_end - info.anchor_dmin as usize;
                        start = left_adjust(text, start);
                    }
                    if range > start {
                        return Ok(None);
                    }
                }
            }
        }
    } else if end == 0 {
        // The empty text: one attempt, unless the pattern needs bytes.
        if info.threshold_len == 0 {
            attempt!(0);
        }
        return Ok(None);
    }

    let mut s = start;
    if range <= start {
        // The window closed to the start itself: one attempt there.
        if info.optimize != Optimize::None && ((end - s) as u64) < info.threshold_len {
            return Ok(None);
        }
        attempt!(s);
        return Ok(None);
    }
    if info.optimize != Optimize::None {
        let sch_range = if info.dmax != 0 {
            if info.dmax == INFINITE_DISTANCE || ((end - range) as u64) < info.dmax {
                end
            } else {
                range + info.dmax as usize
            }
        } else {
            range
        };
        if ((end - start) as u64) < info.threshold_len {
            return Ok(None);
        }
        if info.dmax != INFINITE_DISTANCE {
            // The literal's distance bounds say where attempts can start:
            // a window behind each occurrence.
            loop {
                let Some((low, high)) = forward_search_range(info, text, s, sch_range) else {
                    return Ok(None);
                };
                if s < low {
                    s = low;
                }
                while s <= high {
                    attempt!(s);
                    if s >= end {
                        break;
                    }
                    s += matcher.char_len_at(s);
                }
                if s >= range {
                    return Ok(None);
                }
            }
        }
        // Only a check that the literal or a map byte is there at all.
        if forward_search_range(info, text, s, sch_range).is_none() {
            return Ok(None);
        }
        if info.anchor & ANCHOR_ANYCHAR_STAR != 0 {
            // A leading `.*` covers the rest of its line: after one attempt
            // the next is at the next line.
            loop {
                attempt!(s);
                let mut prev = s;
                s += matcher.char_len_at(s);
                if info.anchor & (ANCHOR_LOOK_BEHIND | ANCHOR_PREC_READ_NOT) == 0 {
                    while bytes[prev] != b'\n' && s < range {
                        prev = s;
                        s += matcher.char_len_at(s);
                    }
                }
                if s >= range {
                    return Ok(None);
                }
            }
        }
    }
    loop {
        attempt!(s);
        s += matcher.char_len_at(s);
        if s >= range {
            break;
        }
    }
    if s == range {
        // An empty match at the very end, as `/$/` has.
        attempt!(s);
    }
    Ok(None)
}

/// `forward_search_range`: the next place the literal or map byte occurs
/// at or after `s`, as the window `(low, high)` of attempt positions it
/// allows; `None` when there is none before `range`.
fn forward_search_range(info: &Info, text: &str, s: usize, range: usize) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    let end = bytes.len();
    if info.dmin > end as u64 {
        return None;
    }
    let mut p = s;
    if info.dmin != 0 {
        if (end - p) as u64 <= info.dmin {
            return None;
        }
        let q = p + info.dmin as usize;
        while p < q {
            p += char_len(bytes, p);
        }
    }
    loop {
        let found = match info.optimize {
            Optimize::None => return None,
            Optimize::Exact => slow_search(&info.exact, bytes, p, range),
            Optimize::ExactIc => slow_search_ic(&info.exact, text, p, range),
            Optimize::ExactBm => bm_search(&info.exact, &info.bm_skip, text, p, range),
            Optimize::ExactBmIc => bm_search_ic(&info.exact, &info.bm_skip, text, p, range),
            Optimize::Map => map_search(&info.map, bytes, p, range),
        }?;
        if found >= range {
            return None;
        }
        p = found;
        let accepted = (p - s) as u64 >= info.dmin && !fails_sub_anchor(info, bytes, p);
        if !accepted {
            // Too close to the start, or on the wrong side of a newline:
            // look again from the next character.
            p += char_len(bytes, p);
            continue;
        }
        let low = if info.dmax == 0 {
            p
        } else if info.dmax != INFINITE_DISTANCE {
            if (p as u64) < info.dmax {
                0
            } else {
                let low = p - info.dmax as usize;
                if low > s {
                    right_adjust(text, low)
                } else {
                    low
                }
            }
        } else {
            // Unused by the caller: the check-only path.
            0
        };
        let high = if info.dmax == 0 {
            p
        } else if (p as u64) < info.dmin {
            0
        } else {
            p - info.dmin as usize
        };
        return Some((low, high));
    }
}

/// The `sub_anchor` checks: a literal that must open a line, or close one.
/// Onigmo switches on the whole value, so with both set neither applies.
fn fails_sub_anchor(info: &Info, bytes: &[u8], p: usize) -> bool {
    match info.sub_anchor {
        ANCHOR_BEGIN_LINE => p != 0 && bytes[prev_head(bytes, p)] != b'\n',
        // At the end a newline before counts as an empty last line
        // (`USE_NEWLINE_AT_END_OF_STRING_HAS_EMPTY_LINE`), so nothing is
        // refused there.
        ANCHOR_END_LINE => p < bytes.len() && bytes[p] != b'\n',
        _ => false,
    }
}

fn char_len(bytes: &[u8], index: usize) -> usize {
    let lead = bytes[index];
    if lead < 0x80 {
        1
    } else if lead < 0xe0 {
        2
    } else if lead < 0xf0 {
        3
    } else {
        4
    }
}

/// `onigenc_get_prev_char_head`: the head of the character before `s`.
fn prev_head(bytes: &[u8], s: usize) -> usize {
    let mut index = s - 1;
    while index > 0 && (bytes[index] & 0xc0) == 0x80 {
        index -= 1;
    }
    index
}

/// `onigenc_get_right_adjust_char_head`: `s`, or the next head after it.
fn right_adjust(text: &str, s: usize) -> usize {
    let mut index = s;
    while index < text.len() && !text.is_char_boundary(index) {
        index += 1;
    }
    index
}

/// `ONIGENC_LEFT_ADJUST_CHAR_HEAD`: `s`, or the head before it.
fn left_adjust(text: &str, s: usize) -> usize {
    let mut index = s.min(text.len());
    while !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

/// `slow_search`: a plain scan, character by character.
fn slow_search(target: &[u8], bytes: &[u8], text: usize, range: usize) -> Option<usize> {
    let end = (bytes.len() + 1).saturating_sub(target.len()).min(range);
    let mut s = text;
    while s < end {
        if bytes[s] == target[0] && bytes[s..].starts_with(target) {
            return Some(s);
        }
        s += char_len(bytes, s);
    }
    None
}

/// `str_lower_case_match`: whether the text at `p` folds to `target`.
fn lower_case_match(target: &[u8], text: &str, p: usize, end: usize) -> bool {
    let mut matched = 0;
    let mut position = p;
    let mut scratch = [0u32; 1];
    let mut buffer = [0u8; 4];
    let bytes = text.as_bytes();
    while matched < target.len() {
        if position >= end {
            return false;
        }
        // An ASCII character folds to its ASCII lowercase, one byte.
        let lead = bytes[position];
        if lead < 0x80 {
            if target[matched] != lead.to_ascii_lowercase() {
                return false;
            }
            matched += 1;
            position += 1;
            continue;
        }
        let character = text[position..].chars().next().expect("a character");
        position += character.len_utf8();
        for code in unicode::fold_slice(character as u32, &mut scratch) {
            let Some(folded) = char::from_u32(*code) else {
                return false;
            };
            let encoded = folded.encode_utf8(&mut buffer);
            for byte in encoded.bytes() {
                if matched >= target.len() || target[matched] != byte {
                    return false;
                }
                matched += 1;
            }
        }
    }
    true
}

/// `slow_search_ic`: a plain scan comparing folded.
fn slow_search_ic(target: &[u8], text: &str, start: usize, range: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let end = (bytes.len() + 1).saturating_sub(target.len()).min(range);
    let mut s = start;
    while s < end {
        if lower_case_match(target, text, s, bytes.len()) {
            return Some(s);
        }
        s += char_len(bytes, s);
    }
    None
}

/// `bm_search`: Onigmo's Boyer-Moore, the window compared from its tail
/// and moved by the skip of the byte past it; a UTF-8 needle found this
/// way lies on a character boundary, since its first byte can never be a
/// continuation byte. A short needle goes to the standard library's
/// search, which hunts its rarest byte and is the faster of the two there.
pub fn bm_search(
    target: &[u8],
    skip: &[usize; 256],
    source: &str,
    text: usize,
    range: usize,
) -> Option<usize> {
    const BM_FROM: usize = 8;
    let bytes = source.as_bytes();
    let tail = target.len() - 1;
    let end = (range + tail).min(bytes.len());
    if target.len() < BM_FROM {
        return find_str(source, target, text, end);
    }
    let mut window_end = text + tail;
    while window_end < end {
        let mut text_index = window_end;
        let mut needle_index = tail;
        while bytes[text_index] == target[needle_index] {
            if needle_index == 0 {
                return Some(text_index);
            }
            text_index -= 1;
            needle_index -= 1;
        }
        if window_end + 1 >= end {
            break;
        }
        window_end += skip[bytes[window_end + 1] as usize];
    }
    None
}

/// The needle in `text[start..end]`, by the standard library's search
/// over `str`. `start` is a character boundary; `end` moves up to the next
/// one, which can only let through an occurrence starting past the range,
/// and the caller refuses those. A needle cut mid-character — the
/// optimizer trims a lookahead's literal to a byte count, as Onigmo's does
/// — is searched byte by byte instead.
fn find_str(text: &str, needle: &[u8], start: usize, end: usize) -> Option<usize> {
    let end = right_adjust(text, end);
    if needle.len() > end - start {
        return None;
    }
    match std::str::from_utf8(needle) {
        Ok(needle) => text[start..end].find(needle).map(|found| start + found),
        Err(_) => text.as_bytes()[start..end]
            .windows(needle.len())
            .position(|window| window == needle)
            .map(|found| start + found),
    }
}

/// `bm_search_ic`: Boyer-Moore over the spellings' bytes, each window
/// compared folded.
fn bm_search_ic(
    target: &[u8],
    skip: &[usize; 256],
    text: &str,
    start: usize,
    range: usize,
) -> Option<usize> {
    let bytes = text.as_bytes();
    let tail = target.len() - 1;
    let end = (range + tail).min(bytes.len());
    let mut s = start + tail;
    while s < end {
        let p = s - tail;
        if text.is_char_boundary(p) && lower_case_match(target, text, p, s + 1) {
            return Some(p);
        }
        if s + 1 >= end {
            break;
        }
        s += skip[bytes[s + 1] as usize];
    }
    None
}

/// `map_search`: the next byte a match can begin with.
fn map_search(map: &[bool; 256], bytes: &[u8], text: usize, range: usize) -> Option<usize> {
    let mut s = text;
    while s < range {
        if map[bytes[s] as usize] {
            return Some(s);
        }
        s += char_len(bytes, s);
    }
    None
}
