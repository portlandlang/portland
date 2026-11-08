//! Tree to program: Onigmo's `regcomp.c` emitters, with forward addresses
//! patched after the fact instead of measured first. The instruction set
//! is Onigmo's, minus the byte-level specializations a UTF-8-only engine
//! over characters does not need.

use super::ast::{AnchorKind, EmptyInfo, EncloseKind, Node, QuantifierNode};
use super::cache;
use super::error::RegexError;
use super::length;
use super::optimize;
use super::setup::Prepared;

/// One instruction; addresses are indexes into `Program::ops`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    /// The bottom of the stack: no alternative left, the attempt failed.
    Finish,
    End,
    /// A literal run, compared as bytes.
    Exact(String),
    /// The pattern's full case fold; the text is folded a character at a
    /// time and the folded streams must agree.
    ExactIc(Vec<u32>),
    /// Onigmo's class halves: the bitset for single-byte codes, the ranges
    /// for the rest. Which half a text character consults, and what a
    /// multibyte character means to a class with no ranges, follows the
    /// six `OP_CCLASS*` opcodes.
    Class {
        bitset: [u64; 4],
        mbuf: Option<Vec<(u32, u32)>>,
        not: bool,
    },
    Word {
        not: bool,
        ascii: bool,
    },
    AnyChar,
    AnyCharMl,
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
    /// `push` keeps the start on the stack, restored by backtracking;
    /// otherwise it is assigned in place (`OP_MEMORY_START`).
    MemStart {
        mem: usize,
        push: bool,
    },
    /// `rec` finds the matching start by recursion level; `push` keeps the
    /// end on the stack.
    MemEnd {
        mem: usize,
        push: bool,
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
    /// `OP_PUSH_OR_JUMP_EXACT1`: a loop whose body opens with `byte`
    /// pushes its alternative only where that byte is next, and jumps out
    /// otherwise.
    PushOrJumpExact1 {
        addr: usize,
        byte: u8,
    },
    /// `OP_PUSH_IF_PEEK_NEXT`: a loop followed by `byte` pushes its
    /// alternative only where that byte is next, and runs its body either
    /// way.
    PushIfPeekNext {
        addr: usize,
        byte: u8,
    },
    /// `OP_ANYCHAR_STAR` and `_ML_STAR`, with `_PEEK_NEXT` when `peek` is
    /// set: `.*` as one instruction, an alternative per character.
    AnyCharStar {
        multiline: bool,
        peek: Option<u8>,
    },
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
    /// What the search can skip by: anchors, a literal, or a byte map.
    pub optimize: optimize::Info,
    /// The instructions the match cache memoizes, when it can cover the
    /// program.
    pub cache: Option<cache::CacheOpcodes>,
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
    bt_mem_start: Vec<bool>,
    bt_mem_end: Vec<bool>,
}

pub fn compile(prepared: &Prepared) -> Result<Program, RegexError> {
    let mut compiler = Compiler {
        ops: Vec::new(),
        repeat_ranges: Vec::new(),
        num_null_check: 0,
        num_call: prepared.num_call,
        call_entries: Vec::new(),
        pending_calls: Vec::new(),
        bt_mem_start: prepared.bt_mem_start.clone(),
        bt_mem_end: prepared.bt_mem_end.clone(),
    };
    compiler.tree(&prepared.root)?;
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
    let optimize = optimize::from_tree(&prepared.root, prepared.num_mem);
    let cache = cache::collect(&compiler.ops, &compiler.repeat_ranges);
    Ok(Program {
        ops: compiler.ops,
        repeat_ranges: compiler.repeat_ranges,
        num_mem: prepared.num_mem,
        num_call: prepared.num_call,
        optimize,
        cache,
    })
}

/// The first byte of a head string, what `OP_PUSH_OR_JUMP_EXACT1` peeks.
fn first_byte(head: &Node) -> u8 {
    let Node::Str { chars, .. } = head else {
        unreachable!("a head is a string")
    };
    let mut bytes = [0u8; 4];
    chars[0].encode_utf8(&mut bytes);
    bytes[0]
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
    fn tree(&mut self, node: &Node) -> Result<(), RegexError> {
        match node {
            Node::Empty => {}
            Node::List(parts) => {
                for part in parts {
                    self.tree(part)?;
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
                    self.tree(branch)?;
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
            Node::Str {
                chars, ignore_case, ..
            } => {
                if *ignore_case {
                    // Already its full fold, from setup's expansion.
                    self.emit(Op::ExactIc(
                        chars.iter().map(|character| *character as u32).collect(),
                    ));
                } else {
                    self.emit(Op::Exact(chars.iter().collect()));
                }
            }
            Node::Class(class) => {
                self.emit(Op::Class {
                    bitset: class.bitset,
                    mbuf: class.mbuf.clone(),
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
            Node::Quantifier(quantifier) => self.quantifier(quantifier)?,
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
                    self.emit(Op::MemStart {
                        mem: regnum,
                        push: self.bt_mem_start[regnum],
                    });
                    self.tree(&enclose.target)?;
                    self.emit(Op::MemEnd {
                        mem: regnum,
                        push: self.bt_mem_end[regnum],
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
                    self.tree(&enclose.target)?;
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
                    self.tree(&branches[0])?;
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
                        self.tree(no)?;
                    }
                    let end = self.here();
                    self.patch(jump, Op::Jump(end));
                }
                EncloseKind::Option => self.tree(&enclose.target)?,
                EncloseKind::Absent => {
                    self.emit(Op::PushAbsentPos);
                    let absent = self.emit(Op::Absent(0));
                    self.tree(&enclose.target)?;
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
                    self.tree(anchor.target.as_ref().expect("a body"))?;
                    self.emit(Op::PopPos);
                }
                AnchorKind::PrecReadNot => {
                    let push = self.emit(Op::PushPosNot(0));
                    self.tree(anchor.target.as_ref().expect("a body"))?;
                    self.emit(Op::FailPos);
                    let after = self.here();
                    self.patch(push, Op::PushPosNot(after));
                }
                AnchorKind::LookBehind => {
                    self.emit(Op::LookBehind(anchor.char_len));
                    self.tree(anchor.target.as_ref().expect("a body"))?;
                }
                AnchorKind::LookBehindNot => {
                    let push = self.emit(Op::PushLookBehindNot {
                        addr: 0,
                        len: anchor.char_len,
                    });
                    self.tree(anchor.target.as_ref().expect("a body"))?;
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
    fn body_with_empty_check(&mut self, body: &Node, empty: EmptyInfo) -> Result<(), RegexError> {
        if empty == EmptyInfo::Never {
            return self.tree(body);
        }
        let id = self.num_null_check;
        self.num_null_check += 1;
        self.emit(Op::NullCheckStart(id));
        self.tree(body)?;
        self.emit(Op::NullCheckEnd { id, kind: empty });
        Ok(())
    }

    /// `compile_quantifier_node`: the shapes Onigmo gives a quantifier,
    /// chosen by the body's size in Onigmo's bytes. A short body is
    /// unrolled, `a{2,4}` into `aa` and two nested pushes over `a`, and
    /// `a{2,}` into `aa` and a loop; a long one counts in a repeat.
    fn quantifier(&mut self, quantifier: &QuantifierNode) -> Result<(), RegexError> {
        let target = quantifier.target.as_ref();
        let infinite = quantifier.upper.is_none();
        let lower = quantifier.lower;
        let tlen = length::tree_length(target);
        if let Node::AnyChar { multiline } = target
            && quantifier.greedy
            && infinite
        {
            // `is_anychar_star_quantifier`: `.*` is one instruction.
            for _ in 0..lower {
                self.tree(target)?;
            }
            self.emit(Op::AnyCharStar {
                multiline: *multiline,
                peek: quantifier.next_head_byte,
            });
            return Ok(());
        }
        if infinite && (lower <= 1 || length::is_expand_limit_ok(tlen, lower)) {
            // A body too long to write out once is entered by a jump into
            // the loop instead.
            let enter = if lower == 1 && tlen > length::QUANTIFIER_EXPAND_LIMIT_SIZE {
                Some(self.emit(Op::Jump(0)))
            } else {
                for _ in 0..lower {
                    self.tree(target)?;
                }
                None
            };
            if quantifier.greedy {
                // The alternative is pushed only where the body's own
                // first byte, or the byte after the loop, is next.
                let push = self.emit(Op::Push(0));
                let body = self.here();
                self.body_with_empty_check(target, quantifier.empty)?;
                self.emit(Op::Jump(push));
                let after = self.here();
                let push_op = match (&quantifier.head_exact, quantifier.next_head_byte) {
                    (Some(head), _) => Op::PushOrJumpExact1 {
                        addr: after,
                        byte: first_byte(head),
                    },
                    (None, Some(byte)) => Op::PushIfPeekNext { addr: after, byte },
                    (None, None) => Op::Push(after),
                };
                self.patch(push, push_op);
                if let Some(enter) = enter {
                    self.patch(enter, Op::Jump(body));
                }
            } else {
                let jump = self.emit(Op::Jump(0));
                let body = self.here();
                self.body_with_empty_check(target, quantifier.empty)?;
                let push = self.here();
                self.emit(Op::Push(body));
                self.patch(jump, Op::Jump(push));
                if let Some(enter) = enter {
                    self.patch(enter, Op::Jump(body));
                }
            }
            return Ok(());
        }
        // An open repeat with a lower bound too long to write out counts
        // in a repeat like a closed one.
        let upper = quantifier.upper.unwrap_or(u32::MAX);
        if upper == 0 && quantifier.referred {
            // A called group inside `{0}`: emitted, never entered here.
            let jump = self.emit(Op::Jump(0));
            self.tree(target)?;
            let after = self.here();
            self.patch(jump, Op::Jump(after));
            return Ok(());
        }
        if !infinite
            && quantifier.greedy
            && (upper == 1 || length::is_expand_limit_ok(tlen + length::SIZE_OP_PUSH, upper))
        {
            // Unrolled: the body `lower` times, then once per optional
            // run behind a push that skips to the end.
            for _ in 0..lower {
                self.tree(target)?;
            }
            let mut pushes = Vec::new();
            for _ in lower..upper {
                pushes.push(self.emit(Op::Push(0)));
                self.tree(target)?;
            }
            let end = self.here();
            for push in pushes {
                self.patch(push, Op::Push(end));
            }
            return Ok(());
        }
        if !quantifier.greedy && upper == 1 && lower == 0 {
            let push = self.emit(Op::Push(0));
            let jump = self.emit(Op::Jump(0));
            let body = self.here();
            self.patch(push, Op::Push(body));
            self.tree(target)?;
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
        self.body_with_empty_check(target, quantifier.empty)?;
        self.emit(Op::RepeatInc {
            id,
            greedy: quantifier.greedy,
            by_stack: self.num_call > 0 || quantifier.in_repeat,
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
}
