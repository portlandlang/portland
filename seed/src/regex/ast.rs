//! The pattern tree, Onigmo's `Node` kinds. Options that change meaning
//! (`i`, `m`, the ASCII-range family) are recorded on the nodes they affect
//! at parse time, so there is no option node: Onigmo keeps one and applies it
//! while compiling, which comes to the same thing.

use super::unicode;

/// The last code point a class can hold.
pub const LAST_CODE_POINT: u32 = 0x10ffff;

/// A set of code points as sorted, disjoint, inclusive ranges.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CharSet {
    pub ranges: Vec<(u32, u32)>,
}

impl CharSet {
    pub fn new() -> CharSet {
        CharSet { ranges: Vec::new() }
    }

    pub fn from_ranges(ranges: &[(u32, u32)]) -> CharSet {
        let mut set = CharSet::new();
        for (low, high) in ranges {
            set.add_range(*low, *high);
        }
        set
    }

    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    pub fn contains(&self, code: u32) -> bool {
        unicode::ranges_contain(&self.ranges, code)
    }

    pub fn add_code(&mut self, code: u32) {
        self.add_range(code, code);
    }

    /// Inserts a range, merging with neighbours so the list stays sorted
    /// and disjoint.
    pub fn add_range(&mut self, low: u32, high: u32) {
        if low > high {
            return;
        }
        let mut merged: Vec<(u32, u32)> = Vec::with_capacity(self.ranges.len() + 1);
        let mut new_low = low;
        let mut new_high = high;
        let mut placed = false;
        for &(range_low, range_high) in &self.ranges {
            if range_high.saturating_add(1) < new_low {
                merged.push((range_low, range_high));
            } else if new_high.saturating_add(1) < range_low {
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
        self.ranges = merged;
    }

    pub fn union(&self, other: &CharSet) -> CharSet {
        let mut result = self.clone();
        for &(low, high) in &other.ranges {
            result.add_range(low, high);
        }
        result
    }

    pub fn intersection(&self, other: &CharSet) -> CharSet {
        let mut result = CharSet::new();
        let mut index = 0;
        let mut other_index = 0;
        while index < self.ranges.len() && other_index < other.ranges.len() {
            let (low, high) = self.ranges[index];
            let (other_low, other_high) = other.ranges[other_index];
            let start = low.max(other_low);
            let end = high.min(other_high);
            if start <= end {
                result.ranges.push((start, end));
            }
            if high < other_high {
                index += 1;
            } else {
                other_index += 1;
            }
        }
        result
    }

    /// The complement within `0..=LAST_CODE_POINT`.
    pub fn negated(&self) -> CharSet {
        let mut result = CharSet::new();
        let mut next = 0u32;
        for &(low, high) in &self.ranges {
            if low > next {
                result.ranges.push((next, low - 1));
            }
            next = high.saturating_add(1);
            if high >= LAST_CODE_POINT {
                return result;
            }
        }
        if next <= LAST_CODE_POINT {
            result.ranges.push((next, LAST_CODE_POINT));
        }
        result
    }

    /// The one code point of a one-member set.
    pub fn single_code(&self) -> Option<u32> {
        match self.ranges.as_slice() {
            [(low, high)] if low == high => Some(*low),
            _ => None,
        }
    }
}

/// A `[...]` class, `\d`, `\s`, `\h`, or `\p{...}`: the set and whether it
/// is negated. The ASCII twin exists only under `i`: it is the class as
/// built without the word-class and ASCII-property additions, which is what
/// decides whether a case-fold crossing the ASCII boundary may be added.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassNode {
    pub set: CharSet,
    pub not: bool,
    pub ascii_twin: Option<Box<ClassNode>>,
}

impl ClassNode {
    pub fn new() -> ClassNode {
        ClassNode {
            set: CharSet::new(),
            not: false,
            ascii_twin: None,
        }
    }

    pub fn matches(&self, code: u32) -> bool {
        self.set.contains(code) != self.not
    }
}

impl Default for ClassNode {
    fn default() -> Self {
        ClassNode::new()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnchorKind {
    BeginBuf,
    EndBuf,
    SemiEndBuf,
    BeginLine,
    EndLine,
    BeginPosition,
    WordBound,
    NotWordBound,
    Keep,
    PrecRead,
    PrecReadNot,
    LookBehind,
    LookBehindNot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnchorNode {
    pub kind: AnchorKind,
    /// `\b` and `\B` under `(?a)`: word characters are ASCII only.
    pub ascii_range: bool,
    /// The body of a lookaround.
    pub target: Option<Box<Node>>,
    /// A lookbehind body's fixed length in characters, set by setup.
    pub char_len: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncloseKind {
    /// A capturing group; `named` is whether it was `(?<name>...)`.
    Memory { regnum: usize, named: bool },
    /// `(?>...)`, and the wrapper of a possessive quantifier.
    StopBacktrack,
    /// `(?(regnum)yes|no)`; the target is always an alternation of two.
    Condition { regnum: usize },
    /// `(?~...)`.
    Absent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncloseNode {
    pub kind: EncloseKind,
    pub target: Box<Node>,
    /// Set by setup: the group is a subexpression call target or is
    /// referred to from inside a repeat, so its start is kept on the stack.
    pub call_target: bool,
    /// Set by setup: a call to this group can reach itself.
    pub recursive: bool,
    /// Set by setup for a group whose body has a fixed length.
    pub char_len: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuantifierNode {
    pub target: Box<Node>,
    pub lower: u32,
    /// `None` is unbounded.
    pub upper: Option<u32>,
    pub greedy: bool,
    /// Written as `{n,m}` rather than `*`, `+`, or `?`.
    pub by_number: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackRefNode {
    /// The groups referred to; several for a name defined more than once,
    /// tried last to first.
    pub groups: Vec<usize>,
    pub by_name: bool,
    /// `\k<n+level>`: the recursion level relative to the reference.
    pub level: Option<i32>,
    pub ignore_case: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallNode {
    /// The name as written, empty for a numbered call.
    pub name: String,
    /// The group called, as parsed for a number and as resolved by setup
    /// for a name; 0 is the whole pattern.
    pub group: usize,
    /// Whether the number came from the pattern (`\g<2>`) rather than a name.
    pub by_number: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    Empty,
    /// A literal run; `ignore_case` makes it compare case-folded.
    Str { chars: Vec<char>, ignore_case: bool },
    Class(ClassNode),
    /// `\w` and `\W`, kept apart from classes as Onigmo does.
    Word { not: bool, ascii_range: bool },
    /// `.`; `multiline` lets it match a newline.
    AnyChar { multiline: bool },
    List(Vec<Node>),
    Alt(Vec<Node>),
    Quantifier(QuantifierNode),
    Enclose(EncloseNode),
    Anchor(AnchorNode),
    BackRef(BackRefNode),
    Call(CallNode),
}

impl Node {
    pub fn str_from(chars: Vec<char>, ignore_case: bool) -> Node {
        Node::Str { chars, ignore_case }
    }

    pub fn anchor(kind: AnchorKind) -> Node {
        Node::Anchor(AnchorNode {
            kind,
            ascii_range: false,
            target: None,
            char_len: 0,
        })
    }

    pub fn enclose(kind: EncloseKind, target: Node) -> Node {
        Node::Enclose(EncloseNode {
            kind,
            target: Box::new(target),
            call_target: false,
            recursive: false,
            char_len: None,
        })
    }
}
