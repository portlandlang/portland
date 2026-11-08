//! The optimizer: `set_optimize_info_from_tree` and `optimize_node_left`
//! from `regcomp.c`, which read off the tree what a search can use to
//! skip positions. Three things come out: the anchors a match is pinned
//! by (`\A`, `\G`, a leading `.*`, `\z` and `\Z` with the pattern's byte
//! length bounds), one literal every match contains with its distance
//! bounds from the match's start (Onigmo's `exact`, searched by Boyer-Moore
//! when long enough, folded when under `i`), or failing a literal, the set
//! of bytes a match can begin with (the `map`). Which literal wins among
//! several is Onigmo's own weighing of length, position, and spread.
//!
//! Lengths here are bytes, as Onigmo's `OnigDistance` is, with
//! `ONIG_INFINITE_DISTANCE` for an unbounded one; a character is one to
//! four bytes in UTF-8.

use super::ast::{AnchorKind, EncloseKind, Node};
use super::casefold;

pub const INFINITE_DISTANCE: u64 = u64::MAX;
const MBC_MINLEN: u64 = 1;
const MBC_MAXLEN_DIST: u64 = 4;
pub const OPT_EXACT_MAXLEN: usize = 24;

pub const ANCHOR_BEGIN_BUF: u32 = 1 << 0;
pub const ANCHOR_BEGIN_LINE: u32 = 1 << 1;
pub const ANCHOR_BEGIN_POSITION: u32 = 1 << 2;
pub const ANCHOR_END_BUF: u32 = 1 << 3;
pub const ANCHOR_SEMI_END_BUF: u32 = 1 << 4;
pub const ANCHOR_END_LINE: u32 = 1 << 5;
pub const ANCHOR_PREC_READ: u32 = 1 << 10;
pub const ANCHOR_PREC_READ_NOT: u32 = 1 << 11;
pub const ANCHOR_LOOK_BEHIND: u32 = 1 << 12;
pub const ANCHOR_ANYCHAR_STAR: u32 = 1 << 14;
pub const ANCHOR_ANYCHAR_STAR_ML: u32 = 1 << 15;
pub const ANCHOR_ANYCHAR_STAR_MASK: u32 = ANCHOR_ANYCHAR_STAR | ANCHOR_ANYCHAR_STAR_ML;

/// `reg->optimize`: how the search looks for a candidate position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Optimize {
    None,
    /// A literal, found by a plain scan.
    Exact,
    /// A literal, found by Boyer-Moore.
    ExactBm,
    /// A folded literal, found by a plain scan comparing folded.
    ExactIc,
    /// A folded literal, found by Boyer-Moore over its spellings' bytes.
    ExactBmIc,
    /// The bytes a match can begin with.
    Map,
}

/// What a compiled pattern's search knows (the optimize fields of
/// Onigmo's `regex_t`).
#[derive(Clone, Debug)]
pub struct Info {
    pub anchor: u32,
    pub anchor_dmin: u64,
    pub anchor_dmax: u64,
    pub sub_anchor: u32,
    pub optimize: Optimize,
    pub exact: Vec<u8>,
    pub dmin: u64,
    pub dmax: u64,
    pub threshold_len: u64,
    pub map: [bool; 256],
    pub bm_skip: [usize; 256],
}

impl Default for Info {
    fn default() -> Info {
        Info {
            anchor: 0,
            anchor_dmin: 0,
            anchor_dmax: 0,
            sub_anchor: 0,
            optimize: Optimize::None,
            exact: Vec::new(),
            dmin: 0,
            dmax: 0,
            threshold_len: 0,
            map: [false; 256],
            bm_skip: [0; 256],
        }
    }
}

pub fn distance_add(d1: u64, d2: u64) -> u64 {
    if d1 == INFINITE_DISTANCE || d2 == INFINITE_DISTANCE {
        INFINITE_DISTANCE
    } else if d1 <= INFINITE_DISTANCE - d2 {
        d1 + d2
    } else {
        INFINITE_DISTANCE
    }
}

pub fn distance_multiply(d: u64, m: u32) -> u64 {
    if m == 0 {
        0
    } else if d < INFINITE_DISTANCE / u64::from(m) {
        d * u64::from(m)
    } else {
        INFINITE_DISTANCE
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct MinMaxLen {
    min: u64,
    max: u64,
}

impl MinMaxLen {
    fn add(&mut self, from: &MinMaxLen) {
        self.min = distance_add(self.min, from.min);
        self.max = distance_add(self.max, from.max);
    }

    fn alt_merge(&mut self, from: &MinMaxLen) {
        if self.min > from.min {
            self.min = from.min;
        }
        if self.max < from.max {
            self.max = from.max;
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct AncInfo {
    left: u32,
    right: u32,
}

fn is_left_anchor(anc: u32) -> bool {
    !matches!(
        anc,
        ANCHOR_END_BUF
            | ANCHOR_SEMI_END_BUF
            | ANCHOR_END_LINE
            | ANCHOR_PREC_READ
            | ANCHOR_PREC_READ_NOT
    )
}

impl AncInfo {
    fn concat(left: &AncInfo, right: &AncInfo, left_len: u64, right_len: u64) -> AncInfo {
        let mut to = AncInfo {
            left: left.left,
            right: right.right,
        };
        if left_len == 0 {
            to.left |= right.left;
        }
        if right_len == 0 {
            to.right |= left.right;
        } else {
            to.right |= left.right & ANCHOR_PREC_READ_NOT;
        }
        to
    }

    fn is_set(&self, anc: u32) -> bool {
        (self.left & anc) != 0 || (self.right & anc) != 0
    }

    fn add(&mut self, anc: u32) {
        if is_left_anchor(anc) {
            self.left |= anc;
        } else {
            self.right |= anc;
        }
    }

    fn remove(&mut self, anc: u32) {
        if is_left_anchor(anc) {
            self.left &= !anc;
        } else {
            self.right &= !anc;
        }
    }

    fn alt_merge(&mut self, add: &AncInfo) {
        self.left &= add.left;
        self.right &= add.right;
    }
}

/// `OptExactInfo`: a literal with where it sits.
#[derive(Clone, Debug)]
struct ExactInfo {
    mmd: MinMaxLen,
    anc: AncInfo,
    reach_end: bool,
    /// -1 unset, 0 case-sensitive, 1 folded.
    ignore_case: i8,
    bytes: Vec<u8>,
}

impl Default for ExactInfo {
    fn default() -> ExactInfo {
        ExactInfo {
            mmd: MinMaxLen::default(),
            anc: AncInfo::default(),
            reach_end: false,
            ignore_case: -1,
            bytes: Vec::new(),
        }
    }
}

/// `map_position_value`: how cheap a byte is to search for, by how rare
/// it is in text.
fn map_position_value(byte: usize) -> i32 {
    const BYTE_VAL_TABLE: [i32; 128] = [
        5, 1, 1, 1, 1, 1, 1, 1, 1, 10, 10, 1, 1, 10, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 12, 4, 7, 4, 4, 4, 4, 4, 4, 5, 5, 5, 5, 5, 5, 5, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 5,
        5, 5, 5, 5, 5, 5, 6, 6, 6, 6, 7, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
        6, 6, 5, 6, 5, 5, 5, 5, 6, 6, 6, 6, 7, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
        6, 6, 6, 6, 5, 5, 5, 5, 1,
    ];
    if byte < BYTE_VAL_TABLE.len() {
        BYTE_VAL_TABLE[byte]
    } else {
        4
    }
}

/// `distance_value`: how much a literal's position being pinned down is
/// worth.
fn distance_value(mm: &MinMaxLen) -> i32 {
    const DIST_VALS: [i32; 100] = [
        1000, 500, 333, 250, 200, 167, 143, 125, 111, 100, 91, 83, 77, 71, 67, 63, 59, 56, 53, 50,
        48, 45, 43, 42, 40, 38, 37, 36, 34, 33, 32, 31, 30, 29, 29, 28, 27, 26, 26, 25, 24, 24, 23,
        23, 22, 22, 21, 21, 20, 20, 20, 19, 19, 19, 18, 18, 18, 17, 17, 17, 16, 16, 16, 16, 15, 15,
        15, 15, 14, 14, 14, 14, 14, 14, 13, 13, 13, 13, 13, 13, 12, 12, 12, 12, 12, 12, 11, 11, 11,
        11, 11, 11, 11, 11, 11, 10, 10, 10, 10, 10,
    ];
    if mm.max == INFINITE_DISTANCE {
        return 0;
    }
    let d = mm.max - mm.min;
    if d < DIST_VALS.len() as u64 {
        DIST_VALS[d as usize]
    } else {
        1
    }
}

fn comp_distance_value(d1: &MinMaxLen, d2: &MinMaxLen, v1: i32, v2: i32) -> i32 {
    if v2 <= 0 {
        return -1;
    }
    if v1 <= 0 {
        return 1;
    }
    let v1 = v1 * distance_value(d1);
    let v2 = v2 * distance_value(d2);
    if v2 > v1 {
        return 1;
    }
    if v2 < v1 {
        return -1;
    }
    if d2.min < d1.min {
        return 1;
    }
    if d2.min > d1.min {
        return -1;
    }
    0
}

fn char_len_at(bytes: &[u8], index: usize) -> usize {
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

impl ExactInfo {
    fn clear(&mut self) {
        *self = ExactInfo::default();
    }

    fn is_full(&self) -> bool {
        self.bytes.len() >= OPT_EXACT_MAXLEN
    }

    /// `concat_opt_exact_info`.
    fn concat(&mut self, add: &ExactInfo) {
        if self.ignore_case < 0 {
            self.ignore_case = add.ignore_case;
        } else if self.ignore_case != add.ignore_case {
            return;
        }
        let mut index = 0;
        while index < add.bytes.len() {
            let len = char_len_at(&add.bytes, index);
            if self.bytes.len() + len > OPT_EXACT_MAXLEN {
                break;
            }
            self.bytes.extend_from_slice(&add.bytes[index..index + len]);
            index += len;
        }
        self.reach_end = if index == add.bytes.len() {
            add.reach_end
        } else {
            false
        };
        let mut tanc = AncInfo::concat(&self.anc, &add.anc, 1, 1);
        if !self.reach_end {
            tanc.right = 0;
        }
        self.anc = tanc;
    }

    /// `concat_opt_exact_info_str`.
    fn concat_str(&mut self, bytes: &[u8]) {
        let mut index = 0;
        while index < bytes.len() && self.bytes.len() < OPT_EXACT_MAXLEN {
            let len = char_len_at(bytes, index);
            if self.bytes.len() + len > OPT_EXACT_MAXLEN {
                break;
            }
            self.bytes.extend_from_slice(&bytes[index..index + len]);
            index += len;
        }
    }

    /// `alt_merge_opt_exact_info`: the common prefix of two branches'
    /// literals, at the same position.
    fn alt_merge(&mut self, add: &ExactInfo) {
        if add.bytes.is_empty() || self.bytes.is_empty() {
            self.clear();
            return;
        }
        if self.mmd != add.mmd {
            self.clear();
            return;
        }
        let mut index = 0;
        while index < self.bytes.len() && index < add.bytes.len() {
            if self.bytes[index] != add.bytes[index] {
                break;
            }
            let len = char_len_at(&self.bytes, index);
            let mut inner = 1;
            while inner < len {
                if self.bytes.get(index + inner) != add.bytes.get(index + inner) {
                    break;
                }
                inner += 1;
            }
            if inner < len {
                break;
            }
            index += len;
        }
        if !add.reach_end || index < add.bytes.len() || index < self.bytes.len() {
            self.reach_end = false;
        }
        self.bytes.truncate(index);
        if self.ignore_case < 0 {
            self.ignore_case = add.ignore_case;
        } else if add.ignore_case >= 0 {
            self.ignore_case |= add.ignore_case;
        }
        self.anc.alt_merge(&add.anc);
        if !self.reach_end {
            self.anc.right = 0;
        }
    }

    /// `select_opt_exact_info`: `alt` replaces `self` when it is the better
    /// literal to look for.
    fn select(&mut self, alt: &ExactInfo) {
        let mut v1 = self.bytes.len() as i32;
        let mut v2 = alt.bytes.len() as i32;
        if v2 == 0 {
            return;
        }
        if v1 == 0 {
            *self = alt.clone();
            return;
        }
        if v1 <= 2 && v2 <= 2 {
            // A rare first byte is worth more than a long literal here.
            v2 = map_position_value(self.bytes[0] as usize);
            v1 = map_position_value(alt.bytes[0] as usize);
            if self.bytes.len() > 1 {
                v1 += 5;
            }
            if alt.bytes.len() > 1 {
                v2 += 5;
            }
        }
        if self.ignore_case <= 0 {
            v1 *= 2;
        }
        if alt.ignore_case <= 0 {
            v2 *= 2;
        }
        if comp_distance_value(&self.mmd, &alt.mmd, v1, v2) > 0 {
            *self = alt.clone();
        }
    }
}

/// `OptMapInfo`: the bytes a match can begin with, weighted by rarity.
#[derive(Clone, Debug)]
struct MapInfo {
    mmd: MinMaxLen,
    anc: AncInfo,
    value: i32,
    map: [bool; 256],
}

impl Default for MapInfo {
    fn default() -> MapInfo {
        MapInfo {
            mmd: MinMaxLen::default(),
            anc: AncInfo::default(),
            value: 0,
            map: [false; 256],
        }
    }
}

impl MapInfo {
    fn clear(&mut self) {
        *self = MapInfo::default();
    }

    fn add_byte(&mut self, byte: u8) {
        if !self.map[byte as usize] {
            self.map[byte as usize] = true;
            self.value += map_position_value(byte as usize);
        }
    }

    /// `add_char_amb_opt_map_info`: the character's first byte and those
    /// of its single-code spellings.
    fn add_char_amb(&mut self, chars: &[char]) {
        let mut bytes = [0u8; 4];
        chars[0].encode_utf8(&mut bytes);
        self.add_byte(bytes[0]);
        for item in casefold::case_fold_items(chars, 0) {
            // `DISABLE_CASE_FOLD_MULTI_CHAR`: only one-code spellings.
            if item.codes.len() != 1 || item.char_count != 1 {
                continue;
            }
            if let Some(spelled) = char::from_u32(item.codes[0]) {
                spelled.encode_utf8(&mut bytes);
                self.add_byte(bytes[0]);
            }
        }
    }

    fn select(&mut self, alt: &MapInfo) {
        const BIG: i32 = 1 << 15;
        if alt.value == 0 {
            return;
        }
        if self.value == 0 {
            *self = alt.clone();
            return;
        }
        let v1 = BIG / self.value;
        let v2 = BIG / alt.value;
        if comp_distance_value(&self.mmd, &alt.mmd, v1, v2) > 0 {
            *self = alt.clone();
        }
    }

    fn alt_merge(&mut self, add: &MapInfo) {
        if self.value == 0 {
            return;
        }
        if add.value == 0 || self.mmd.max < add.mmd.min {
            self.clear();
            return;
        }
        self.mmd.alt_merge(&add.mmd);
        let mut value = 0;
        for byte in 0..256 {
            if add.map[byte] {
                self.map[byte] = true;
            }
            if self.map[byte] {
                value += map_position_value(byte);
            }
        }
        self.value = value;
        self.anc.alt_merge(&add.anc);
    }
}

/// `comp_opt_exact_or_map_info`: whether the map beats the literal.
fn comp_exact_or_map(exact: &ExactInfo, map: &MapInfo) -> i32 {
    const COMP_EM_BASE: i32 = 20;
    if map.value <= 0 {
        return -1;
    }
    let ve = COMP_EM_BASE * exact.bytes.len() as i32 * if exact.ignore_case > 0 { 1 } else { 2 };
    let vm = COMP_EM_BASE * 5 * 2 / map.value;
    comp_distance_value(&exact.mmd, &map.mmd, ve, vm)
}

#[derive(Clone, Debug, Default)]
struct NodeOptInfo {
    len: MinMaxLen,
    anc: AncInfo,
    /// A literal at the boundary: the match opens with it.
    exb: ExactInfo,
    /// A literal in the middle.
    exm: ExactInfo,
    /// A literal a lookahead demands.
    expr: ExactInfo,
    map: MapInfo,
}

impl NodeOptInfo {
    fn bound_to(&mut self, mmd: &MinMaxLen) {
        self.exb.mmd = *mmd;
        self.expr.mmd = *mmd;
        self.map.mmd = *mmd;
    }

    /// `concat_left_node_opt_info`: `add` follows `self` in a sequence.
    fn concat_left(&mut self, add: &mut NodeOptInfo) {
        self.anc = AncInfo::concat(&self.anc, &add.anc, self.len.max, add.len.max);
        if !add.exb.bytes.is_empty() && self.len.max == 0 {
            add.exb.anc = AncInfo::concat(&self.anc, &add.exb.anc, self.len.max, add.len.max);
        }
        if add.map.value > 0 && self.len.max == 0 && add.map.mmd.max == 0 {
            add.map.anc.left |= self.anc.left;
        }
        let exb_reach = self.exb.reach_end;
        let exm_reach = self.exm.reach_end;
        if add.len.max != 0 {
            self.exb.reach_end = false;
            self.exm.reach_end = false;
        }
        if !add.exb.bytes.is_empty() {
            if exb_reach {
                self.exb.concat(&add.exb);
                add.exb.clear();
            } else if exm_reach {
                self.exm.concat(&add.exb);
                add.exb.clear();
            }
        }
        self.exm.select(&add.exb);
        self.exm.select(&add.exm);
        if !self.expr.bytes.is_empty() {
            if add.len.max > 0 {
                if self.expr.bytes.len() as u64 > add.len.max {
                    self.expr.bytes.truncate(add.len.max as usize);
                }
                if self.expr.mmd.max == 0 {
                    let expr = self.expr.clone();
                    self.exb.select(&expr);
                } else {
                    let expr = self.expr.clone();
                    self.exm.select(&expr);
                }
            }
        } else if !add.expr.bytes.is_empty() {
            self.expr = add.expr.clone();
        }
        self.map.select(&add.map);
        self.len.add(&add.len);
    }

    fn alt_merge(&mut self, add: &NodeOptInfo) {
        self.anc.alt_merge(&add.anc);
        self.exb.alt_merge(&add.exb);
        self.exm.alt_merge(&add.exm);
        self.expr.alt_merge(&add.expr);
        self.map.alt_merge(&add.map);
        self.len.alt_merge(&add.len);
    }
}

/// What the walk needs beyond the tree: the capturing groups by number,
/// for backreferences and calls, and which groups a backreference names.
pub struct Env<'a> {
    pub groups: Vec<(usize, &'a Node)>,
    pub backrefed: Vec<bool>,
}

impl<'a> Env<'a> {
    pub fn new(root: &'a Node, num_mem: usize) -> Env<'a> {
        let mut groups = Vec::new();
        let mut backrefed = vec![false; num_mem + 1];
        collect(root, &mut groups, &mut backrefed);
        Env { groups, backrefed }
    }

    fn group(&self, regnum: usize) -> Option<&'a Node> {
        self.groups
            .iter()
            .find(|(number, _)| *number == regnum)
            .map(|(_, node)| *node)
    }
}

fn collect<'a>(node: &'a Node, groups: &mut Vec<(usize, &'a Node)>, backrefed: &mut [bool]) {
    match node {
        Node::List(parts) | Node::Alt(parts) => {
            for part in parts {
                collect(part, groups, backrefed);
            }
        }
        Node::Quantifier(quantifier) => collect(&quantifier.target, groups, backrefed),
        Node::Enclose(enclose) => {
            if let EncloseKind::Memory { regnum, .. } = enclose.kind {
                groups.push((regnum, node));
            }
            collect(&enclose.target, groups, backrefed);
        }
        Node::Anchor(anchor) => {
            if let Some(target) = &anchor.target {
                collect(target, groups, backrefed);
            }
        }
        Node::BackRef(backref) => {
            for group in &backref.groups {
                if *group < backrefed.len() {
                    backrefed[*group] = true;
                }
            }
        }
        _ => {}
    }
}

fn str_bytes(chars: &[char]) -> Vec<u8> {
    chars.iter().collect::<String>().into_bytes()
}

/// `get_min_match_length`, in bytes; a group being measured again inside
/// itself counts as empty.
fn min_length(node: &Node, env: &Env, visiting: &mut Vec<usize>) -> u64 {
    match node {
        Node::Empty | Node::Anchor(_) => 0,
        Node::BackRef(backref) => {
            if backref.level.is_some() {
                return 0;
            }
            backref
                .groups
                .iter()
                .filter_map(|regnum| env.group(*regnum))
                .map(|group| min_length(group, env, visiting))
                .min()
                .unwrap_or(0)
        }
        Node::Call(call) => match env.group(call.group) {
            Some(Node::Enclose(enclose)) if enclose.recursive => 0,
            Some(group) => min_length(group, env, visiting),
            None => 0,
        },
        Node::List(parts) => parts.iter().fold(0, |total, part| {
            distance_add(total, min_length(part, env, visiting))
        }),
        Node::Alt(branches) => branches
            .iter()
            .map(|branch| min_length(branch, env, visiting))
            .min()
            .unwrap_or(0),
        Node::Str { chars, .. } => str_bytes(chars).len() as u64,
        Node::Class(_) | Node::Word { .. } | Node::AnyChar { .. } => MBC_MINLEN,
        Node::Quantifier(quantifier) => {
            if quantifier.lower > 0 {
                distance_multiply(
                    min_length(&quantifier.target, env, visiting),
                    quantifier.lower,
                )
            } else {
                0
            }
        }
        Node::Enclose(enclose) => match enclose.kind {
            EncloseKind::Memory { regnum, .. } => {
                if visiting.contains(&regnum) {
                    return 0;
                }
                visiting.push(regnum);
                let length = min_length(&enclose.target, env, visiting);
                visiting.pop();
                length
            }
            EncloseKind::Absent => 0,
            _ => min_length(&enclose.target, env, visiting),
        },
    }
}

/// `get_max_match_length`, in bytes; a group being measured again inside
/// itself counts as unbounded.
fn max_length(node: &Node, env: &Env, visiting: &mut Vec<usize>) -> u64 {
    match node {
        Node::Empty | Node::Anchor(_) => 0,
        Node::BackRef(backref) => {
            if backref.level.is_some() {
                return INFINITE_DISTANCE;
            }
            backref
                .groups
                .iter()
                .filter_map(|regnum| env.group(*regnum))
                .map(|group| max_length(group, env, visiting))
                .max()
                .unwrap_or(0)
        }
        Node::Call(call) => match env.group(call.group) {
            Some(Node::Enclose(enclose)) if enclose.recursive => INFINITE_DISTANCE,
            Some(group) => max_length(group, env, visiting),
            None => 0,
        },
        Node::List(parts) => parts.iter().fold(0, |total, part| {
            distance_add(total, max_length(part, env, visiting))
        }),
        Node::Alt(branches) => branches
            .iter()
            .map(|branch| max_length(branch, env, visiting))
            .max()
            .unwrap_or(0),
        Node::Str { chars, .. } => str_bytes(chars).len() as u64,
        Node::Class(_) | Node::Word { .. } | Node::AnyChar { .. } => MBC_MAXLEN_DIST,
        Node::Quantifier(quantifier) => {
            if quantifier.upper == Some(0) {
                return 0;
            }
            let inner = max_length(&quantifier.target, env, visiting);
            if inner == 0 {
                return 0;
            }
            match quantifier.upper {
                Some(upper) => distance_multiply(inner, upper),
                None => INFINITE_DISTANCE,
            }
        }
        Node::Enclose(enclose) => match enclose.kind {
            EncloseKind::Memory { regnum, .. } => {
                if visiting.contains(&regnum) {
                    return INFINITE_DISTANCE;
                }
                visiting.push(regnum);
                let length = max_length(&enclose.target, env, visiting);
                visiting.pop();
                length
            }
            EncloseKind::Absent => INFINITE_DISTANCE,
            _ => max_length(&enclose.target, env, visiting),
        },
    }
}

/// `optimize_node_left`: the information of `node`, placed `mmd` bytes
/// from the match's start.
fn optimize_node(node: &Node, mmd: &MinMaxLen, env: &Env) -> NodeOptInfo {
    let mut opt = NodeOptInfo::default();
    opt.bound_to(mmd);
    match node {
        Node::Empty => {}
        Node::List(parts) => {
            let mut position = *mmd;
            for part in parts {
                let mut nopt = optimize_node(part, &position, env);
                position.add(&nopt.len);
                opt.concat_left(&mut nopt);
            }
        }
        Node::Alt(branches) => {
            for (index, branch) in branches.iter().enumerate() {
                let nopt = optimize_node(branch, mmd, env);
                if index == 0 {
                    opt = nopt;
                } else {
                    opt.alt_merge(&nopt);
                }
            }
        }
        Node::Str {
            chars,
            ignore_case,
            remainder,
            ..
        } => {
            let bytes = str_bytes(chars);
            let slen = bytes.len() as u64;
            if !*ignore_case {
                opt.exb.concat_str(&bytes);
                opt.exb.ignore_case = 0;
                if slen > 0 {
                    opt.map.add_byte(bytes[0]);
                }
                opt.len = MinMaxLen {
                    min: slen,
                    max: slen,
                };
            } else {
                let max = if *remainder {
                    MBC_MAXLEN_DIST * chars.len() as u64
                } else {
                    opt.exb.concat_str(&bytes);
                    opt.exb.ignore_case = 1;
                    if slen > 0 {
                        opt.map.add_char_amb(chars);
                    }
                    slen
                };
                opt.len = MinMaxLen { min: slen, max };
            }
            if opt.exb.bytes.len() as u64 == slen {
                opt.exb.reach_end = true;
            }
        }
        Node::Class(class) => {
            if class.mbuf.is_some() || class.not {
                opt.len = MinMaxLen {
                    min: MBC_MINLEN,
                    max: MBC_MAXLEN_DIST,
                };
            } else {
                for byte in 0..256u32 {
                    if class.bit(byte) {
                        opt.map.add_byte(byte as u8);
                    }
                }
                opt.len = MinMaxLen { min: 1, max: 1 };
            }
        }
        // `\w` is a multibyte-encoding type: no map, one to four bytes.
        Node::Word { .. } | Node::AnyChar { .. } => {
            opt.len = MinMaxLen {
                min: MBC_MINLEN,
                max: MBC_MAXLEN_DIST,
            };
        }
        Node::Anchor(anchor) => match anchor.kind {
            AnchorKind::BeginBuf => opt.anc.add(ANCHOR_BEGIN_BUF),
            AnchorKind::BeginPosition => opt.anc.add(ANCHOR_BEGIN_POSITION),
            AnchorKind::BeginLine => opt.anc.add(ANCHOR_BEGIN_LINE),
            AnchorKind::EndBuf => opt.anc.add(ANCHOR_END_BUF),
            AnchorKind::SemiEndBuf => opt.anc.add(ANCHOR_SEMI_END_BUF),
            AnchorKind::EndLine => opt.anc.add(ANCHOR_END_LINE),
            AnchorKind::LookBehind => opt.anc.add(ANCHOR_LOOK_BEHIND),
            AnchorKind::PrecReadNot => opt.anc.add(ANCHOR_PREC_READ_NOT),
            AnchorKind::PrecRead => {
                if let Some(target) = &anchor.target {
                    let nopt = optimize_node(target, mmd, env);
                    if !nopt.exb.bytes.is_empty() {
                        opt.expr = nopt.exb.clone();
                    } else if !nopt.exm.bytes.is_empty() {
                        opt.expr = nopt.exm.clone();
                    }
                    opt.expr.reach_end = false;
                    if nopt.map.value > 0 {
                        opt.map = nopt.map.clone();
                    }
                }
            }
            _ => {}
        },
        Node::BackRef(backref) => {
            if backref.level.is_some() {
                opt.len = MinMaxLen {
                    min: 0,
                    max: INFINITE_DISTANCE,
                };
            } else {
                let mut min = INFINITE_DISTANCE;
                let mut max = 0;
                for regnum in &backref.groups {
                    if let Some(group) = env.group(*regnum) {
                        min = min.min(min_length(group, env, &mut Vec::new()));
                        max = max.max(max_length(group, env, &mut Vec::new()));
                    }
                }
                if min == INFINITE_DISTANCE {
                    min = 0;
                }
                opt.len = MinMaxLen { min, max };
            }
        }
        Node::Call(call) => match env.group(call.group) {
            Some(Node::Enclose(enclose)) if enclose.recursive => {
                opt.len = MinMaxLen {
                    min: 0,
                    max: INFINITE_DISTANCE,
                };
            }
            Some(group) => opt = optimize_node(group, mmd, env),
            None => {}
        },
        Node::Quantifier(quantifier) => {
            let nopt = optimize_node(&quantifier.target, mmd, env);
            if quantifier.lower == 0 && quantifier.upper.is_none() {
                if let Node::AnyChar { multiline } = quantifier.target.as_ref()
                    && mmd.max == 0
                    && quantifier.greedy
                {
                    // An implicit anchor: /.*a/ is /\A.*a/, line by line.
                    if *multiline {
                        opt.anc.add(ANCHOR_ANYCHAR_STAR_ML);
                    } else {
                        opt.anc.add(ANCHOR_ANYCHAR_STAR);
                    }
                }
            } else if quantifier.lower > 0 {
                opt = nopt.clone();
                if !nopt.exb.bytes.is_empty() && nopt.exb.reach_end {
                    let mut count = 2;
                    while count <= quantifier.lower && !opt.exb.is_full() {
                        opt.exb.concat(&nopt.exb);
                        count += 1;
                    }
                    if count < quantifier.lower {
                        opt.exb.reach_end = false;
                    }
                }
                if Some(quantifier.lower) != quantifier.upper {
                    opt.exb.reach_end = false;
                    opt.exm.reach_end = false;
                }
                if quantifier.lower > 1 {
                    opt.exm.reach_end = false;
                }
            }
            let min = distance_multiply(nopt.len.min, quantifier.lower);
            let max = match quantifier.upper {
                None => {
                    if nopt.len.max > 0 {
                        INFINITE_DISTANCE
                    } else {
                        0
                    }
                }
                Some(upper) => distance_multiply(nopt.len.max, upper),
            };
            opt.len = MinMaxLen { min, max };
        }
        Node::Enclose(enclose) => match enclose.kind {
            EncloseKind::Memory { regnum, .. } => {
                opt = optimize_node(&enclose.target, mmd, env);
                if opt.anc.is_set(ANCHOR_ANYCHAR_STAR_MASK)
                    && env.backrefed.get(regnum).copied().unwrap_or(false)
                {
                    opt.anc.remove(ANCHOR_ANYCHAR_STAR_MASK);
                }
            }
            EncloseKind::Absent => {
                opt.len = MinMaxLen {
                    min: 0,
                    max: INFINITE_DISTANCE,
                };
            }
            _ => opt = optimize_node(&enclose.target, mmd, env),
        },
    }
    opt
}

/// `set_bm_skip`: the Boyer-Moore table for `exact`, every byte of every
/// spelling under `i`; a character whose spellings change length stops
/// the table there, and the shortened length is answered.
fn set_bm_skip(exact: &[u8], ignore_case: bool, skip: &mut [usize; 256]) -> usize {
    let mut end = exact.len();
    let chars: Vec<char> = String::from_utf8_lossy(exact).chars().collect();
    let mut spellings: Vec<Vec<[u8; 4]>> = Vec::new();
    if ignore_case {
        let mut index = 0;
        for (position, character) in chars.iter().enumerate() {
            let clen = character.len_utf8();
            let mut alternatives = Vec::new();
            let mut stop = false;
            for item in casefold::case_fold_items(&chars, position) {
                if item.codes.len() != 1 || item.char_count != 1 {
                    // A spelling of another length is not supported.
                    stop = true;
                    break;
                }
                let Some(spelled) = char::from_u32(item.codes[0]) else {
                    continue;
                };
                if spelled.len_utf8() != clen {
                    stop = true;
                    break;
                }
                let mut bytes = [0u8; 4];
                spelled.encode_utf8(&mut bytes);
                alternatives.push(bytes);
            }
            if stop {
                end = index;
                break;
            }
            spellings.push(alternatives);
            index += clen;
        }
    }
    let len = end;
    for entry in skip.iter_mut() {
        *entry = len + 1;
    }
    let mut index = 0;
    let mut position = 0;
    while index < len {
        let clen = char_len_at(exact, index).min(len - index);
        for offset in 0..clen {
            skip[exact[index + offset] as usize] = len - index - offset;
            if let Some(alternatives) = spellings.get(position) {
                for bytes in alternatives {
                    skip[bytes[offset] as usize] = len - index - offset;
                }
            }
        }
        index += clen;
        position += 1;
    }
    len
}

/// `set_optimize_info_from_tree`.
pub fn from_tree(root: &Node, num_mem: usize) -> Info {
    let env = Env::new(root, num_mem);
    let mut opt = optimize_node(root, &MinMaxLen::default(), &env);
    let mut info = Info {
        anchor: opt.anc.left
            & (ANCHOR_BEGIN_BUF
                | ANCHOR_BEGIN_POSITION
                | ANCHOR_ANYCHAR_STAR
                | ANCHOR_ANYCHAR_STAR_ML
                | ANCHOR_LOOK_BEHIND),
        ..Info::default()
    };
    if (opt.anc.left & (ANCHOR_LOOK_BEHIND | ANCHOR_PREC_READ_NOT)) != 0 {
        info.anchor &= !ANCHOR_ANYCHAR_STAR_ML;
    }
    info.anchor |= opt.anc.right & (ANCHOR_END_BUF | ANCHOR_SEMI_END_BUF | ANCHOR_PREC_READ_NOT);
    if info.anchor & (ANCHOR_END_BUF | ANCHOR_SEMI_END_BUF) != 0 {
        info.anchor_dmin = opt.len.min;
        info.anchor_dmax = opt.len.max;
    }
    if !opt.exb.bytes.is_empty() || !opt.exm.bytes.is_empty() {
        let exm = opt.exm.clone();
        opt.exb.select(&exm);
        if opt.map.value > 0 && comp_exact_or_map(&opt.exb, &opt.map) > 0 {
            set_map(&mut info, &opt.map);
        } else {
            set_exact(&mut info, &opt.exb);
            set_sub_anchor(&mut info, &opt.exb.anc);
        }
    } else if opt.map.value > 0 {
        set_map(&mut info, &opt.map);
    } else {
        info.sub_anchor |= opt.anc.left & ANCHOR_BEGIN_LINE;
        if opt.len.max == 0 {
            info.sub_anchor |= opt.anc.right & ANCHOR_END_LINE;
        }
    }
    info
}

/// `set_optimize_exact_info`: UTF-8 always allows a reverse match, so a
/// literal of two bytes or more goes to Boyer-Moore.
fn set_exact(info: &mut Info, exact: &ExactInfo) {
    info.exact = exact.bytes.clone();
    if exact.ignore_case > 0 {
        if exact.bytes.len() >= 2 {
            let len = set_bm_skip(&exact.bytes, true, &mut info.bm_skip);
            if len >= 3 {
                info.exact.truncate(len);
                info.optimize = Optimize::ExactBmIc;
            } else {
                // The table could not cover the literal (a spelling of
                // another length early on): the plain folded scan over
                // the whole literal, as Ruby's Bug #21824 fix has it.
                info.optimize = Optimize::ExactIc;
            }
        } else {
            info.optimize = Optimize::ExactIc;
        }
    } else if exact.bytes.len() >= 2 {
        set_bm_skip(&exact.bytes, false, &mut info.bm_skip);
        info.optimize = Optimize::ExactBm;
    } else {
        info.optimize = Optimize::Exact;
    }
    info.dmin = exact.mmd.min;
    info.dmax = exact.mmd.max;
    if info.dmin != INFINITE_DISTANCE {
        info.threshold_len = info.dmin + info.exact.len() as u64;
    }
}

fn set_map(info: &mut Info, map: &MapInfo) {
    info.map = map.map;
    info.optimize = Optimize::Map;
    info.dmin = map.mmd.min;
    info.dmax = map.mmd.max;
    if info.dmin != INFINITE_DISTANCE {
        info.threshold_len = info.dmin + 1;
    }
    set_sub_anchor(info, &map.anc);
}

fn set_sub_anchor(info: &mut Info, anc: &AncInfo) {
    info.sub_anchor |= anc.left & ANCHOR_BEGIN_LINE;
    info.sub_anchor |= anc.right & ANCHOR_END_LINE;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::regex::Regex;

    fn info_of(pattern: &str) -> Info {
        Regex::new(pattern).unwrap().program.optimize.clone()
    }

    #[test]
    fn a_literal_opening_is_an_exact_at_distance_zero() {
        let info = info_of("abc\\d+");
        assert_eq!(info.optimize, Optimize::ExactBm);
        assert_eq!(info.exact, b"abc");
        assert_eq!((info.dmin, info.dmax), (0, 0));
        assert_eq!(info.threshold_len, 3);
    }

    #[test]
    fn a_literal_in_the_middle_carries_its_distance_bounds() {
        let info = info_of("\\d{2,3} miles");
        assert_eq!(info.exact, b" miles");
        assert_eq!((info.dmin, info.dmax), (2, 3));
    }

    #[test]
    fn an_end_anchor_pins_the_search_window() {
        let info = info_of("[a-z]{3}\\Z");
        assert_ne!(info.anchor & ANCHOR_SEMI_END_BUF, 0);
        assert_eq!((info.anchor_dmin, info.anchor_dmax), (3, 3));
        let info = info_of("\\w+\\z");
        assert_eq!(info.anchor_dmax, INFINITE_DISTANCE);
    }

    #[test]
    fn a_class_opening_is_a_map_and_a_leading_dot_star_an_anchor() {
        // `[ab]x` would look for the `x`; with no literal, the opening
        // class's bytes, which are rarer than the digits after it.
        let info = info_of("[ab]\\d");
        assert_eq!(info.optimize, Optimize::Map);
        assert!(info.map[b'a' as usize] && info.map[b'b' as usize] && !info.map[b'0' as usize]);
        let info = info_of(".*zebra");
        assert_ne!(info.anchor & ANCHOR_ANYCHAR_STAR, 0);
    }

    #[test]
    fn a_folded_literal_is_searched_folded_up_to_a_spelling_of_another_length() {
        // `s` may be spelled `ſ`, two bytes to one, so the table stops
        // before it and the literal searched is `wat`.
        let info = info_of("(?i)watson");
        assert_eq!(info.optimize, Optimize::ExactBmIc);
        assert_eq!(info.exact, b"wat");
        assert!(info.bm_skip[b'W' as usize] == info.bm_skip[b'w' as usize]);
        // Too short for the table once `k` (a Kelvin sign, three bytes)
        // stops it: the plain folded scan over the whole literal.
        let info = info_of("(?i)akte");
        assert_eq!(info.optimize, Optimize::ExactIc);
        assert_eq!(info.exact, b"akte");
    }
}
