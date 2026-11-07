//! A character class as Onigmo keeps one (`CClassNode`): a 256-bit set for
//! single-byte codes and a sorted list of code point ranges (`mbuf`) for the
//! rest, with a negation flag on top. The two halves are combined by the
//! arithmetic of `regparse.c` — `or_cclass`, `and_cclass`,
//! `not_code_range_buf`, and kin — ported here as written, because Ruby's
//! answers for nested and negated classes follow that arithmetic rather than
//! set theory: `[^[^\D]]` is the ASCII non-digits, not the non-digits.
//!
//! The range list is `None` where Onigmo's buffer is null; it is never an
//! empty `Some`, since every path that would leave it empty leaves it null.

/// `ONIG_LAST_CODE_POINT`: the ranges run to the top of the type.
pub const LAST_CODE_POINT: u32 = u32::MAX;
/// `SINGLE_BYTE_SIZE`.
pub const SINGLE_BYTE_SIZE: u32 = 256;
/// `MBCODE_START_POS` for UTF-8, and the `sb_out` of its ctype ranges.
pub const MBCODE_START: u32 = 0x80;

/// A `[...]` class, `\d`, `\s`, `\h`, or `\p{...}`. The ASCII twin exists
/// only under `i`: it is the class as built without the word-class and
/// ASCII-property additions, which is what decides whether a case fold
/// crossing the ASCII boundary may be added.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ClassNode {
    pub bitset: [u64; 4],
    pub mbuf: Option<Vec<(u32, u32)>>,
    pub not: bool,
    pub ascii_twin: Option<Box<ClassNode>>,
}

impl ClassNode {
    pub fn new() -> ClassNode {
        ClassNode::default()
    }

    pub fn bit(&self, code: u32) -> bool {
        code < SINGLE_BYTE_SIZE && (self.bitset[(code / 64) as usize] >> (code % 64)) & 1 == 1
    }

    pub fn set_bit(&mut self, code: u32) {
        if code < SINGLE_BYTE_SIZE {
            self.bitset[(code / 64) as usize] |= 1u64 << (code % 64);
        }
    }

    /// `bitset_set_range`, the upper end clipped to the bitset.
    pub fn set_bit_range(&mut self, from: u32, to: u32) {
        let top = to.min(SINGLE_BYTE_SIZE - 1);
        let mut code = from;
        while code <= top {
            self.set_bit(code);
            if code == top {
                break;
            }
            code += 1;
        }
    }

    pub fn bitset_is_empty(&self) -> bool {
        self.bitset == [0; 4]
    }

    pub fn bitset_invert(&mut self) {
        for word in self.bitset.iter_mut() {
            *word = !*word;
        }
    }

    /// `add_code_range_to_buf`: into the range list, merging what overlaps
    /// or touches.
    pub fn add_code_range(&mut self, from: u32, to: u32) {
        let (from, to) = if from > to { (to, from) } else { (from, to) };
        let ranges = self.mbuf.get_or_insert_with(Vec::new);
        add_range_to(ranges, from, to);
    }

    /// `onig_is_code_in_cc` without the negation: whether the code is in
    /// the raw set, the bitset for single-byte codes and the ranges for
    /// the rest.
    pub fn code_in_raw(&self, code: u32) -> bool {
        if code >= MBCODE_START {
            match &self.mbuf {
                Some(ranges) => in_ranges(ranges, code),
                None => false,
            }
        } else {
            self.bit(code)
        }
    }

    /// `onig_is_code_in_cc`.
    pub fn matches(&self, code: u32) -> bool {
        self.code_in_raw(code) != self.not
    }

    /// `is_onechar_cclass`: the one code point a one-member class holds.
    pub fn single_code(&self) -> Option<u32> {
        if self.not {
            return None;
        }
        let mut found: Option<u32> = None;
        if let Some(ranges) = &self.mbuf {
            if ranges.len() == 1 && ranges[0].0 == ranges[0].1 {
                let code = ranges[0].0;
                if !(code < SINGLE_BYTE_SIZE && self.bit(code)) {
                    found = Some(code);
                }
            } else {
                return None;
            }
        }
        for (index, word) in self.bitset.iter().enumerate() {
            if *word != 0 {
                if word & (word - 1) == 0 && found.is_none() {
                    found = Some(index as u32 * 64 + word.trailing_zeros());
                } else {
                    return None;
                }
            }
        }
        found
    }

    /// `or_cclass(dest = self, cc = other)`.
    pub fn or_with(&mut self, other: &ClassNode) {
        let not1 = self.not;
        let not2 = other.not;
        let mut bits1 = self.bitset;
        let mut bits2 = other.bitset;
        if not1 {
            invert(&mut bits1);
        }
        if not2 {
            invert(&mut bits2);
        }
        for index in 0..4 {
            bits1[index] |= bits2[index];
        }
        if not1 {
            invert(&mut bits1);
        }
        self.bitset = bits1;
        let merged = if not1 && not2 {
            and_code_range_buf(self.mbuf.as_deref(), false, other.mbuf.as_deref(), false)
        } else {
            let joined = or_code_range_buf(self.mbuf.as_deref(), not1, other.mbuf.as_deref(), not2);
            if not1 {
                not_code_range_buf(joined.as_deref())
            } else {
                joined
            }
        };
        self.mbuf = merged;
    }

    /// `and_cclass(dest = self, cc = other)`.
    pub fn and_with(&mut self, other: &ClassNode) {
        let not1 = self.not;
        let not2 = other.not;
        let mut bits1 = self.bitset;
        let mut bits2 = other.bitset;
        if not1 {
            invert(&mut bits1);
        }
        if not2 {
            invert(&mut bits2);
        }
        for index in 0..4 {
            bits1[index] &= bits2[index];
        }
        if not1 {
            invert(&mut bits1);
        }
        self.bitset = bits1;
        let merged = if not1 && not2 {
            or_code_range_buf(self.mbuf.as_deref(), false, other.mbuf.as_deref(), false)
        } else {
            let joined =
                and_code_range_buf(self.mbuf.as_deref(), not1, other.mbuf.as_deref(), not2);
            if not1 {
                not_code_range_buf(joined.as_deref())
            } else {
                joined
            }
        };
        self.mbuf = merged;
    }
}

fn invert(bits: &mut [u64; 4]) {
    for word in bits.iter_mut() {
        *word = !*word;
    }
}

/// Binary search over sorted, disjoint, inclusive ranges.
pub fn in_ranges(ranges: &[(u32, u32)], code: u32) -> bool {
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

/// `add_code_range_to_buf`: insert with merging of overlapping or touching
/// ranges, the list kept sorted.
fn add_range_to(ranges: &mut Vec<(u32, u32)>, from: u32, to: u32) {
    let mut merged: Vec<(u32, u32)> = Vec::with_capacity(ranges.len() + 1);
    let mut new_low = from;
    let mut new_high = to;
    let mut placed = false;
    for &(range_low, range_high) in ranges.iter() {
        if range_high < u32::MAX && range_high + 1 < new_low {
            merged.push((range_low, range_high));
        } else if new_high < u32::MAX && new_high + 1 < range_low {
            if !placed {
                merged.push((new_low, new_high));
                placed = true;
            }
            merged.push((range_low, range_high));
        } else {
            new_low = new_low.min(range_low);
            new_high = new_high.max(range_high);
        }
    }
    if !placed {
        merged.push((new_low, new_high));
    }
    *ranges = merged;
}

fn to_buf(ranges: Vec<(u32, u32)>) -> Option<Vec<(u32, u32)>> {
    if ranges.is_empty() {
        None
    } else {
        Some(ranges)
    }
}

/// `SET_ALL_MULTI_BYTE_RANGE`.
fn all_multibyte() -> Option<Vec<(u32, u32)>> {
    Some(vec![(MBCODE_START, LAST_CODE_POINT)])
}

/// `not_code_range_buf`: the complement of the ranges, taken from the
/// multibyte start.
pub fn not_code_range_buf(ranges: Option<&[(u32, u32)]>) -> Option<Vec<(u32, u32)>> {
    let Some(ranges) = ranges else {
        return all_multibyte();
    };
    if ranges.is_empty() {
        return all_multibyte();
    }
    let mut result: Vec<(u32, u32)> = Vec::new();
    let mut pre = MBCODE_START;
    let mut to = 0u32;
    for &(from, range_to) in ranges {
        to = range_to;
        if from > 0 && pre < from {
            add_range_to(&mut result, pre, from - 1);
        }
        if to == LAST_CODE_POINT {
            break;
        }
        pre = to + 1;
    }
    if to < LAST_CODE_POINT {
        add_range_to(&mut result, to + 1, LAST_CODE_POINT);
    }
    to_buf(result)
}

/// `or_code_range_buf`.
pub fn or_code_range_buf(
    buf1: Option<&[(u32, u32)]>,
    not1: bool,
    buf2: Option<&[(u32, u32)]>,
    not2: bool,
) -> Option<Vec<(u32, u32)>> {
    let (mut buf1, mut not1, mut buf2, mut not2) = (buf1, not1, buf2, not2);
    if buf1.is_none() && buf2.is_none() {
        return if not1 || not2 { all_multibyte() } else { None };
    }
    if buf2.is_none() {
        std::mem::swap(&mut buf1, &mut buf2);
        std::mem::swap(&mut not1, &mut not2);
    }
    if buf1.is_none() {
        if not1 {
            return all_multibyte();
        }
        return if not2 {
            not_code_range_buf(buf2)
        } else {
            buf2.map(|ranges| ranges.to_vec())
        };
    }
    if not1 {
        std::mem::swap(&mut buf1, &mut buf2);
        std::mem::swap(&mut not1, &mut not2);
    }
    let data1 = buf1.expect("set above");
    let mut result: Vec<(u32, u32)> = if !not2 && !not1 {
        buf2.map(|ranges| ranges.to_vec()).unwrap_or_default()
    } else if !not1 {
        not_code_range_buf(buf2).unwrap_or_default()
    } else {
        Vec::new()
    };
    for &(from, to) in data1 {
        add_range_to(&mut result, from, to);
    }
    to_buf(result)
}

/// `and_code_range_buf`.
pub fn and_code_range_buf(
    buf1: Option<&[(u32, u32)]>,
    not1: bool,
    buf2: Option<&[(u32, u32)]>,
    not2: bool,
) -> Option<Vec<(u32, u32)>> {
    let (mut buf1, mut not1, mut buf2, mut not2) = (buf1, not1, buf2, not2);
    match (buf1, buf2) {
        (None, _) => {
            return if not1 && buf2.is_some() {
                buf2.map(|ranges| ranges.to_vec())
            } else {
                None
            };
        }
        (_, None) => {
            return if not2 {
                buf1.map(|ranges| ranges.to_vec())
            } else {
                None
            };
        }
        _ => {}
    }
    if not1 {
        std::mem::swap(&mut buf1, &mut buf2);
        std::mem::swap(&mut not1, &mut not2);
    }
    let data1 = buf1.expect("set above");
    let data2 = buf2.expect("set above");
    let mut result: Vec<(u32, u32)> = Vec::new();
    if !not2 && !not1 {
        for &(from1, to1) in data1 {
            for &(from2, to2) in data2 {
                if from2 > to1 {
                    break;
                }
                if to2 < from1 {
                    continue;
                }
                add_range_to(&mut result, from1.max(from2), to1.min(to2));
            }
        }
    } else if !not1 {
        for &(from1, to1) in data1 {
            and_code_range1(&mut result, from1, to1, data2);
        }
    }
    to_buf(result)
}

/// `and_code_range1`: `[from1, to1]` minus the ranges of `data`.
fn and_code_range1(result: &mut Vec<(u32, u32)>, from1: u32, to1: u32, data: &[(u32, u32)]) {
    let mut from1 = from1;
    let mut to1 = to1;
    for &(from2, to2) in data {
        if from2 < from1 {
            if to2 < from1 {
                continue;
            }
            from1 = to2.saturating_add(1);
        } else if from2 <= to1 {
            if to2 < to1 {
                if from2 > 0 && from1 < from2 {
                    add_range_to(result, from1, from2 - 1);
                }
                from1 = to2.saturating_add(1);
            } else {
                to1 = from2.wrapping_sub(1);
                if from2 == 0 {
                    return;
                }
            }
        } else {
            from1 = from2;
        }
        if from1 > to1 {
            break;
        }
    }
    if from1 <= to1 {
        add_range_to(result, from1, to1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_double_negated_non_digit_class_is_the_ascii_non_digits() {
        // [^\D]: \D is the non-digits, ASCII in the bitset and all of
        // 0x80.. in the ranges; the class is then negated.
        let mut inner = ClassNode::new();
        for code in 0..MBCODE_START {
            if !(0x30..=0x39).contains(&code) {
                inner.set_bit(code);
            }
        }
        inner.add_code_range(MBCODE_START, LAST_CODE_POINT);
        inner.not = true;
        // [^ [^\D] ]
        let mut outer = ClassNode::new();
        outer.or_with(&inner);
        outer.not = true;
        assert!(outer.matches('a' as u32));
        assert!(!outer.matches('5' as u32));
        assert_eq!(outer.mbuf, None);
        // The bits for 0x80.. are set by the inversion, which is what the
        // matcher reads for a multibyte character's lead byte.
        assert!(outer.bit(0xc3));
    }

    #[test]
    fn complements_start_at_the_multibyte_boundary() {
        assert_eq!(not_code_range_buf(None), Some(vec![(0x80, u32::MAX)]));
        assert_eq!(not_code_range_buf(Some(&[(0x80, u32::MAX)])), None);
        assert_eq!(
            not_code_range_buf(Some(&[(0x100, 0x1ff)])),
            Some(vec![(0x80, 0xff), (0x200, u32::MAX)])
        );
    }
}
