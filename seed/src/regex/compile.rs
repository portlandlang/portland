//! Tree to program: Onigmo's `regcomp.c` emitters, with forward addresses
//! patched after the fact instead of measured first. The instruction set
//! is Onigmo's, minus the byte-level specializations a UTF-8-only engine
//! over characters does not need.

use super::ast::{AnchorKind, CharSet, EmptyInfo, EncloseKind, Node, QuantifierNode};
use super::casefold;
use super::error::RegexError;
use super::setup::Prepared;

/// One instruction; addresses are indexes into `Program::ops`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    /// The bottom of the stack: no alternative left, the attempt failed.
    Finish,
    End,
    Exact(Vec<char>),
    /// The pattern's full case fold; the text is folded a character at a
    /// time and the folded streams must agree.
    ExactIc(Vec<u32>),
    Class {
        set: CharSet,
        not: bool,
    },
    Word {
        not: bool,
        ascii: bool,
    },
    AnyChar,
    AnyCharMl,
    GraphemeCluster,
    BeginBuf,
    EndBuf,
    SemiEndBuf,
    BeginLine,
    EndLine,
    BeginPosition,
    WordBound {
        ascii: bool,
    },
    NotWordBound {
        ascii: bool,
    },
    Keep,
    MemStart(usize),
    /// `rec` finds the matching start by recursion level.
    MemEnd {
        mem: usize,
        rec: bool,
    },
    /// The groups to try, last-defined first.
    BackRef {
        mems: Vec<usize>,
        ignore_case: bool,
    },
    BackRefLevel {
        mems: Vec<usize>,
        level: i32,
        ignore_case: bool,
    },
    NullCheckStart(usize),
    NullCheckEnd {
        id: usize,
        kind: EmptyInfo,
    },
    Jump(usize),
    Push(usize),
    Pop,
    Repeat {
        id: usize,
        greedy: bool,
        after: usize,
    },
    RepeatInc {
        id: usize,
        greedy: bool,
        by_stack: bool,
    },
    PushPos,
    PopPos,
    PushPosNot(usize),
    FailPos,
    PushStopBt,
    PopStopBt,
    LookBehind(usize),
    PushLookBehindNot {
        addr: usize,
        len: usize,
    },
    FailLookBehindNot,
    PushAbsentPos,
    Absent(usize),
    AbsentEnd,
    Call(usize),
    Return,
    Condition {
        mem: usize,
        addr: usize,
    },
    Fail,
}

#[derive(Debug)]
pub struct Program {
    pub ops: Vec<Op>,
    /// Lower and upper bound per repeat id.
    pub repeat_ranges: Vec<(u32, Option<u32>)>,
    pub num_mem: usize,
    pub num_call: usize,
}

struct Compiler {
    ops: Vec<Op>,
    repeat_ranges: Vec<(u32, Option<u32>)>,
    num_null_check: usize,
    num_call: usize,
    /// Group number to the address a call enters at.
    call_entries: Vec<(usize, usize)>,
    /// Call instructions waiting for their group's entry address.
    pending_calls: Vec<(usize, usize)>,
}

pub fn compile(prepared: &Prepared) -> Result<Program, RegexError> {
    let mut compiler = Compiler {
        ops: Vec::new(),
        repeat_ranges: Vec::new(),
        num_null_check: 0,
        num_call: prepared.num_call,
        call_entries: Vec::new(),
        pending_calls: Vec::new(),
    };
    compiler.tree(&prepared.root, false)?;
    compiler.emit(Op::End);
    for (index, group) in std::mem::take(&mut compiler.pending_calls) {
        let entry = compiler
            .call_entries
            .iter()
            .find(|(number, _)| *number == group)
            .map(|(_, entry)| *entry)
            .ok_or_else(|| RegexError::new("internal parser error (bug)"))?;
        compiler.ops[index] = Op::Call(entry);
    }
    Ok(Program {
        ops: compiler.ops,
        repeat_ranges: compiler.repeat_ranges,
        num_mem: prepared.num_mem,
        num_call: prepared.num_call,
    })
}

impl Compiler {
    fn emit(&mut self, op: Op) -> usize {
        self.ops.push(op);
        self.ops.len() - 1
    }

    fn here(&self) -> usize {
        self.ops.len()
    }

    fn patch(&mut self, index: usize, op: Op) {
        self.ops[index] = op;
    }

    /// `compile_tree`.
    fn tree(&mut self, node: &Node, in_lookbehind: bool) -> Result<(), RegexError> {
        match node {
            Node::Empty => {}
            Node::List(parts) => {
                for part in parts {
                    self.tree(part, in_lookbehind)?;
                }
            }
            Node::Alt(branches) => {
                let mut jumps = Vec::new();
                for (index, branch) in branches.iter().enumerate() {
                    let last = index + 1 == branches.len();
                    let push = if last {
                        None
                    } else {
                        Some(self.emit(Op::Push(0)))
                    };
                    self.tree(branch, in_lookbehind)?;
                    if let Some(push) = push {
                        jumps.push(self.emit(Op::Jump(0)));
                        let next = self.here();
                        self.patch(push, Op::Push(next));
                    }
                }
                let end = self.here();
                for jump in jumps {
                    self.patch(jump, Op::Jump(end));
                }
            }
            Node::Str { chars, ignore_case } => {
                if *ignore_case {
                    self.case_fold_string(chars, in_lookbehind);
                } else {
                    self.emit(Op::Exact(chars.clone()));
                }
            }
            Node::Class(class) => {
                self.emit(Op::Class {
                    set: class.set.clone(),
                    not: class.not,
                });
            }
            Node::Word { not, ascii_range } => {
                self.emit(Op::Word {
                    not: *not,
                    ascii: *ascii_range,
                });
            }
            Node::AnyChar { multiline } => {
                self.emit(if *multiline {
                    Op::AnyCharMl
                } else {
                    Op::AnyChar
                });
            }
            Node::GraphemeCluster => {
                self.emit(Op::GraphemeCluster);
            }
            Node::Quantifier(quantifier) => self.quantifier(quantifier, in_lookbehind)?,
            Node::Enclose(enclose) => match enclose.kind {
                EncloseKind::Memory { regnum, .. } => {
                    let mut after_jump = None;
                    if enclose.call_target {
                        let call = self.emit(Op::Call(0));
                        after_jump = Some(self.emit(Op::Jump(0)));
                        let entry = self.here();
                        self.patch(call, Op::Call(entry));
                        self.call_entries.push((regnum, entry));
                    }
                    self.emit(Op::MemStart(regnum));
                    self.tree(&enclose.target, in_lookbehind)?;
                    self.emit(Op::MemEnd {
                        mem: regnum,
                        rec: enclose.recursive,
                    });
                    if let Some(jump) = after_jump {
                        self.emit(Op::Return);
                        let after = self.here();
                        self.patch(jump, Op::Jump(after));
                    }
                }
                EncloseKind::StopBacktrack => {
                    self.emit(Op::PushStopBt);
                    self.tree(&enclose.target, in_lookbehind)?;
                    self.emit(Op::PopStopBt);
                }
                EncloseKind::Condition { regnum, .. } => {
                    let Node::Alt(branches) = enclose.target.as_ref() else {
                        return Err(RegexError::new("internal parser error (bug)"));
                    };
                    let condition = self.emit(Op::Condition {
                        mem: regnum,
                        addr: 0,
                    });
                    self.tree(&branches[0], in_lookbehind)?;
                    let jump = self.emit(Op::Jump(0));
                    let no_start = self.here();
                    self.patch(
                        condition,
                        Op::Condition {
                            mem: regnum,
                            addr: no_start,
                        },
                    );
                    if let Some(no) = branches.get(1) {
                        self.tree(no, in_lookbehind)?;
                    }
                    let end = self.here();
                    self.patch(jump, Op::Jump(end));
                }
                EncloseKind::Absent => {
                    self.emit(Op::PushAbsentPos);
                    let absent = self.emit(Op::Absent(0));
                    self.tree(&enclose.target, in_lookbehind)?;
                    self.emit(Op::AbsentEnd);
                    let after = self.here();
                    self.patch(absent, Op::Absent(after));
                }
            },
            Node::Anchor(anchor) => match anchor.kind {
                AnchorKind::BeginBuf => {
                    self.emit(Op::BeginBuf);
                }
                AnchorKind::EndBuf => {
                    self.emit(Op::EndBuf);
                }
                AnchorKind::SemiEndBuf => {
                    self.emit(Op::SemiEndBuf);
                }
                AnchorKind::BeginLine => {
                    self.emit(Op::BeginLine);
                }
                AnchorKind::EndLine => {
                    self.emit(Op::EndLine);
                }
                AnchorKind::BeginPosition => {
                    self.emit(Op::BeginPosition);
                }
                AnchorKind::WordBound => {
                    self.emit(Op::WordBound {
                        ascii: anchor.ascii_range,
                    });
                }
                AnchorKind::NotWordBound => {
                    self.emit(Op::NotWordBound {
                        ascii: anchor.ascii_range,
                    });
                }
                AnchorKind::Keep => {
                    self.emit(Op::Keep);
                }
                AnchorKind::PrecRead => {
                    self.emit(Op::PushPos);
                    self.tree(anchor.target.as_ref().expect("a body"), in_lookbehind)?;
                    self.emit(Op::PopPos);
                }
                AnchorKind::PrecReadNot => {
                    let push = self.emit(Op::PushPosNot(0));
                    self.tree(anchor.target.as_ref().expect("a body"), in_lookbehind)?;
                    self.emit(Op::FailPos);
                    let after = self.here();
                    self.patch(push, Op::PushPosNot(after));
                }
                AnchorKind::LookBehind => {
                    self.emit(Op::LookBehind(anchor.char_len));
                    self.tree(anchor.target.as_ref().expect("a body"), true)?;
                }
                AnchorKind::LookBehindNot => {
                    let push = self.emit(Op::PushLookBehindNot {
                        addr: 0,
                        len: anchor.char_len,
                    });
                    self.tree(anchor.target.as_ref().expect("a body"), true)?;
                    self.emit(Op::FailLookBehindNot);
                    let after = self.here();
                    self.patch(
                        push,
                        Op::PushLookBehindNot {
                            addr: after,
                            len: anchor.char_len,
                        },
                    );
                }
            },
            Node::BackRef(backref) => {
                let mut mems = backref.groups.clone();
                mems.reverse();
                match backref.level {
                    Some(level) => {
                        self.emit(Op::BackRefLevel {
                            mems,
                            level,
                            ignore_case: backref.ignore_case,
                        });
                    }
                    None => {
                        self.emit(Op::BackRef {
                            mems,
                            ignore_case: backref.ignore_case,
                        });
                    }
                }
            }
            Node::Call(call) => {
                let index = self.emit(Op::Call(0));
                self.pending_calls.push((index, call.group));
            }
        }
        Ok(())
    }

    /// `compile_tree_empty_check`.
    fn body_with_empty_check(
        &mut self,
        body: &Node,
        empty: EmptyInfo,
        in_lookbehind: bool,
    ) -> Result<(), RegexError> {
        if empty == EmptyInfo::Never {
            return self.tree(body, in_lookbehind);
        }
        let id = self.num_null_check;
        self.num_null_check += 1;
        self.emit(Op::NullCheckStart(id));
        self.tree(body, in_lookbehind)?;
        self.emit(Op::NullCheckEnd { id, kind: empty });
        Ok(())
    }

    /// `compile_quantifier_node`.
    fn quantifier(
        &mut self,
        quantifier: &QuantifierNode,
        in_lookbehind: bool,
    ) -> Result<(), RegexError> {
        let target = quantifier.target.as_ref();
        let infinite = quantifier.upper.is_none();
        if infinite && quantifier.lower <= 1 {
            for _ in 0..quantifier.lower {
                self.tree(target, in_lookbehind)?;
            }
            if quantifier.greedy {
                let push = self.emit(Op::Push(0));
                self.body_with_empty_check(target, quantifier.empty, in_lookbehind)?;
                self.emit(Op::Jump(push));
                let after = self.here();
                self.patch(push, Op::Push(after));
            } else {
                let jump = self.emit(Op::Jump(0));
                let body = self.here();
                self.body_with_empty_check(target, quantifier.empty, in_lookbehind)?;
                let push = self.here();
                self.emit(Op::Push(body));
                self.patch(jump, Op::Jump(push));
            }
            return Ok(());
        }
        if quantifier.upper == Some(0) {
            if quantifier.referred {
                // A called group inside `{0}`: emitted, never entered here.
                let jump = self.emit(Op::Jump(0));
                self.tree(target, in_lookbehind)?;
                let after = self.here();
                self.patch(jump, Op::Jump(after));
            }
            return Ok(());
        }
        if !infinite && quantifier.greedy && quantifier.upper == Some(1) {
            for _ in 0..quantifier.lower {
                self.tree(target, in_lookbehind)?;
            }
            if quantifier.lower == 0 {
                let push = self.emit(Op::Push(0));
                self.tree(target, in_lookbehind)?;
                let after = self.here();
                self.patch(push, Op::Push(after));
            }
            return Ok(());
        }
        if !quantifier.greedy && quantifier.upper == Some(1) && quantifier.lower == 0 {
            let push = self.emit(Op::Push(0));
            let jump = self.emit(Op::Jump(0));
            let body = self.here();
            self.patch(push, Op::Push(body));
            self.tree(target, in_lookbehind)?;
            let after = self.here();
            self.patch(jump, Op::Jump(after));
            return Ok(());
        }
        // `compile_range_repeat_node`.
        let id = self.repeat_ranges.len();
        self.repeat_ranges
            .push((quantifier.lower, quantifier.upper));
        let repeat = self.emit(Op::Repeat {
            id,
            greedy: quantifier.greedy,
            after: 0,
        });
        self.body_with_empty_check(target, quantifier.empty, in_lookbehind)?;
        self.emit(Op::RepeatInc {
            id,
            greedy: quantifier.greedy,
            by_stack: self.num_call > 0,
        });
        let after = self.here();
        self.patch(
            repeat,
            Op::Repeat {
                id,
                greedy: quantifier.greedy,
                after,
            },
        );
        Ok(())
    }

    /// `expand_case_fold_string`: a string under `i` becomes, at each
    /// position where a fold could be spelled differently, an alternation
    /// of the spellings, up to eight branches; the rest compares folded.
    fn case_fold_string(&mut self, chars: &[char], in_lookbehind: bool) {
        const THRESHOLD_CASE_FOLD_ALT_FOR_EXPANSION: usize = 8;
        let mut alt_num = 1usize;
        self.case_fold_from(
            chars,
            0,
            &mut alt_num,
            in_lookbehind,
            THRESHOLD_CASE_FOLD_ALT_FOR_EXPANSION,
        );
    }

    fn case_fold_from(
        &mut self,
        chars: &[char],
        start: usize,
        alt_num: &mut usize,
        in_lookbehind: bool,
        threshold: usize,
    ) {
        let mut plain: Vec<char> = Vec::new();
        let mut index = start;
        while index < chars.len() {
            let items = casefold::case_fold_items(chars, index);
            let variable = items
                .iter()
                .any(|item| item.char_count != 1 || item.codes.len() != 1);
            if items.is_empty() || !variable || in_lookbehind {
                plain.push(chars[index]);
                index += 1;
                continue;
            }
            *alt_num *= items.len() + 1;
            if *alt_num > threshold {
                break;
            }
            if !plain.is_empty() {
                let folded = Self::folded(&std::mem::take(&mut plain));
                self.emit(Op::ExactIc(folded));
            }
            // The character as written, then each other spelling, each
            // followed by the rest of the string.
            let mut branches: Vec<(Vec<u32>, usize)> =
                vec![(Self::folded(&[chars[index]]), index + 1)];
            for item in &items {
                if item.char_count == 1 && item.codes.len() == 1 {
                    continue;
                }
                let spelled: Vec<char> = item
                    .codes
                    .iter()
                    .filter_map(|code| char::from_u32(*code))
                    .collect();
                branches.push((Self::folded(&spelled), index + item.char_count));
            }
            let mut jumps = Vec::new();
            for (branch_index, (spelled, resume)) in branches.iter().enumerate() {
                let last = branch_index + 1 == branches.len();
                let push = if last {
                    None
                } else {
                    Some(self.emit(Op::Push(0)))
                };
                self.emit(Op::ExactIc(spelled.clone()));
                self.case_fold_from(chars, *resume, alt_num, in_lookbehind, threshold);
                if let Some(push) = push {
                    jumps.push(self.emit(Op::Jump(0)));
                    let next = self.here();
                    self.patch(push, Op::Push(next));
                }
            }
            let end = self.here();
            for jump in jumps {
                self.patch(jump, Op::Jump(end));
            }
            return;
        }
        if index < chars.len() {
            plain.extend_from_slice(&chars[index..]);
        }
        if !plain.is_empty() {
            self.emit(Op::ExactIc(Self::folded(&plain)));
        }
    }

    /// `update_string_node_case_fold`: the full fold of a string.
    fn folded(chars: &[char]) -> Vec<u32> {
        chars
            .iter()
            .flat_map(|character| super::unicode::fold_sequence(*character as u32))
            .collect()
    }
}
