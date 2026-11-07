//! What `enc/unicode.c` does for the engine: property membership by name,
//! the builtin character types, and full case folding, all over the tables
//! generated from Ruby's own copies.

use super::unicode_tables::{
    CASE_FOLD, CASE_UNFOLD_ONE, CASE_UNFOLD_THREE, CASE_UNFOLD_TWO, PROPERTY_NAMES, PROPERTY_RANGES,
};

/// The builtin character types, in Onigmo's `ONIGENC_CTYPE_*` order, which
/// is also their index into the property tables.
pub const CTYPE_NEWLINE: u16 = 0;
pub const CTYPE_ALPHA: u16 = 1;
pub const CTYPE_BLANK: u16 = 2;
pub const CTYPE_CNTRL: u16 = 3;
pub const CTYPE_DIGIT: u16 = 4;
pub const CTYPE_GRAPH: u16 = 5;
pub const CTYPE_LOWER: u16 = 6;
pub const CTYPE_PRINT: u16 = 7;
pub const CTYPE_PUNCT: u16 = 8;
pub const CTYPE_SPACE: u16 = 9;
pub const CTYPE_UPPER: u16 = 10;
pub const CTYPE_XDIGIT: u16 = 11;
pub const CTYPE_WORD: u16 = 12;
pub const CTYPE_ALNUM: u16 = 13;
pub const CTYPE_ASCII: u16 = 14;

/// Looks a property name up the way Onigmo does: spaces, hyphens, and
/// underscores dropped, ASCII letters lowered; a non-ASCII character or an
/// unknown name is `None`.
pub fn property_ctype(name: &str) -> Option<u16> {
    let mut normalized = String::with_capacity(name.len());
    for character in name.chars() {
        if character == ' ' || character == '-' || character == '_' {
            continue;
        }
        if !character.is_ascii() {
            return None;
        }
        normalized.push(character.to_ascii_lowercase());
    }
    PROPERTY_NAMES
        .binary_search_by(|(candidate, _)| candidate.cmp(&normalized.as_str()))
        .ok()
        .map(|index| PROPERTY_NAMES[index].1)
}

/// The inclusive code point ranges of a property.
pub fn ctype_ranges(ctype: u16) -> &'static [(u32, u32)] {
    PROPERTY_RANGES[ctype as usize]
}

/// Whether `code` is in the property.
pub fn ctype_contains(ctype: u16, code: u32) -> bool {
    ranges_contain(ctype_ranges(ctype), code)
}

/// Binary search over sorted, disjoint, inclusive ranges.
pub fn ranges_contain(ranges: &[(u32, u32)], code: u32) -> bool {
    ranges
        .binary_search_by(|(low, high)| {
            if code < *low {
                std::cmp::Ordering::Greater
            } else if code > *high {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

/// The full case fold of a code point, if it has one.
pub fn fold(code: u32) -> Option<&'static [u32]> {
    CASE_FOLD
        .binary_search_by(|(candidate, _)| candidate.cmp(&code))
        .ok()
        .map(|index| CASE_FOLD[index].1)
}

/// The fold of a code point as a sequence, itself when it has none.
pub fn fold_sequence(code: u32) -> Vec<u32> {
    match fold(code) {
        Some(folded) => folded.to_vec(),
        None => vec![code],
    }
}

/// The fold of a code point as a borrowed slice, with no allocation: the
/// table's entry, or the code itself held in `scratch`.
pub fn fold_slice(code: u32, scratch: &mut [u32; 1]) -> &[u32] {
    match fold(code) {
        Some(folded) => folded,
        None => {
            scratch[0] = code;
            &scratch[..]
        }
    }
}

/// The code points whose single-code fold is `code`.
pub fn unfold_one(code: u32) -> Option<&'static [u32]> {
    CASE_UNFOLD_ONE
        .binary_search_by(|(candidate, _)| candidate.cmp(&code))
        .ok()
        .map(|index| CASE_UNFOLD_ONE[index].1)
}

/// The code points whose full fold is the two-code sequence.
pub fn unfold_two(pair: [u32; 2]) -> Option<&'static [u32]> {
    CASE_UNFOLD_TWO
        .iter()
        .find(|(candidate, _)| *candidate == pair)
        .map(|(_, codes)| *codes)
}

/// The code points whose full fold is the three-code sequence.
pub fn unfold_three(triple: [u32; 3]) -> Option<&'static [u32]> {
    CASE_UNFOLD_THREE
        .iter()
        .find(|(candidate, _)| *candidate == triple)
        .map(|(_, codes)| *codes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn property_names_normalize_as_ruby_does() {
        assert_eq!(property_ctype("Alpha"), Some(CTYPE_ALPHA));
        assert_eq!(property_ctype("ALPHA"), Some(CTYPE_ALPHA));
        assert_eq!(
            property_ctype("In_Basic_Latin"),
            property_ctype("inbasiclatin")
        );
        assert!(property_ctype("Other Default Ignorable Code Point").is_some());
        assert_eq!(property_ctype("あ"), None);
        assert_eq!(property_ctype("foobarbaz"), None);
    }

    #[test]
    fn membership_follows_the_ranges() {
        assert!(ctype_contains(CTYPE_WORD, 'a' as u32));
        assert!(ctype_contains(CTYPE_WORD, 'お' as u32));
        assert!(!ctype_contains(CTYPE_DIGIT, 'a' as u32));
        assert!(ctype_contains(CTYPE_SPACE, 0x85));
        let greek = property_ctype("Greek").unwrap();
        assert!(ctype_contains(greek, 'α' as u32));
        assert!(!ctype_contains(greek, 'a' as u32));
    }

    #[test]
    fn folding_is_full() {
        assert_eq!(fold('A' as u32), Some(&['a' as u32][..]));
        assert_eq!(fold(0xdf), Some(&[0x73, 0x73][..]));
        assert_eq!(fold('a' as u32), None);
        assert_eq!(unfold_one('a' as u32), Some(&['A' as u32][..]));
        assert_eq!(unfold_two([0x73, 0x73]), Some(&[0xdf, 0x1e9e][..]));
    }
}
