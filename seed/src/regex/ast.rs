//! The pattern tree, Onigmo's `Node` kinds. Options that change meaning
//! (`i`, `m`, the ASCII-range family) are recorded on the nodes they affect
//! at parse time, so there is no option node: Onigmo keeps one and applies it
//! while compiling, which comes to the same thing.

pub use super::cclass::ClassNode;

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
    /// `by_name` is whether the condition named its group.
    Condition { regnum: usize, by_name: bool },
    /// `(?~...)`.
    Absent,
    /// `(?imx:...)` and the rest of a group after `(?imx)`: the options are
    /// already recorded on the nodes inside, so this only keeps a quantifier
    /// from reaching in, as Onigmo's option node does.
    Option,
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

/// What a quantifier's body can do when it matches nothing: nothing to
/// watch, captures to watch, or recursion to watch (Onigmo's
/// `NQ_TARGET_IS_EMPTY`, `_MEM`, `_REC`); set by setup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmptyInfo {
    Never,
    Plain,
    Captures,
    Recursion,
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
    pub empty: EmptyInfo,
    /// A `{0}` body holding a called group, which must still be emitted.
    pub referred: bool,
    /// Nested inside another quantifier, so its counter is found on the
    /// stack rather than by id (Onigmo's `_SG` opcodes).
    pub in_repeat: bool,
}

impl QuantifierNode {
    pub fn new(
        target: Node,
        lower: u32,
        upper: Option<u32>,
        greedy: bool,
        by_number: bool,
    ) -> QuantifierNode {
        QuantifierNode {
            target: Box::new(target),
            lower,
            upper,
            greedy,
            by_number,
            empty: EmptyInfo::Never,
            referred: false,
            in_repeat: false,
        }
    }
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
    Str {
        chars: Vec<char>,
        ignore_case: bool,
    },
    Class(ClassNode),
    /// `\w` and `\W`, kept apart from classes as Onigmo does.
    Word {
        not: bool,
        ascii_range: bool,
    },
    /// `.`; `multiline` lets it match a newline.
    AnyChar {
        multiline: bool,
    },
    /// `\X`: one extended grapheme cluster.
    GraphemeCluster,
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
