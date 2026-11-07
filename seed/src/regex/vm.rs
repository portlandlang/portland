//! The backtracking matcher: Onigmo's `match_at` over a program, as Ruby's
//! copy of `regexec.c` has it. The text is a slice of characters; positions
//! are character indexes and are turned into byte offsets by the caller.
//! The stack is Onigmo's: besides the alternatives to resume, it records
//! capture starts and ends, repeat counts, null-check marks, call frames,
//! and the atomic and lookaround marks, so that backtracking restores all
//! of them in one walk.
//!
//! Two positions are signed: `range`, the bound reads may not pass, which
//! the absence operator lowers while it runs, and the position before the
//! current one, which is -1 at the start of the text; Onigmo has a null
//! pointer there, and the absence operator's arithmetic leans on it.

use super::ast::EmptyInfo;
use super::compile::{Op, Program};
use super::error::{self, RegexError};
use super::unicode::{self, CTYPE_WORD};
use unicode_segmentation::UnicodeSegmentation;

/// The groups of a match as character spans, group 0 first; a group that
/// took no part is `None`.
pub type Spans = Vec<Option<(usize, usize)>>;

fn bit_at(bitset: &[u64; 4], code: u32) -> bool {
    code < 256 && (bitset[(code / 64) as usize] >> (code % 64)) & 1 == 1
}

/// The first byte of a code point's UTF-8 form, which is what Onigmo's
/// bitset-only class opcodes read for a multibyte character.
fn lead_byte(code: u32) -> u32 {
    if code < 0x80 {
        code
    } else if code < 0x800 {
        0xc0 | (code >> 6)
    } else if code < 0x1_0000 {
        0xe0 | (code >> 12)
    } else {
        0xf0 | (code >> 18)
    }
}

/// Where a group's start or end lives: nowhere yet, in a stack entry that
/// backtracking restores, or assigned in place, where a failed path's value
/// outlives it (Onigmo's `mem_start_stk` holding an index or a pointer).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Slot {
    Invalid,
    Index(usize),
    Pos(usize),
}

#[derive(Clone, Copy, Debug)]
enum Entry {
    Alt {
        pc: usize,
        s: usize,
        pkeep: usize,
    },
    PosNot {
        pc: usize,
        s: usize,
        pkeep: usize,
    },
    LookBehindNot {
        pc: usize,
        s: usize,
        pkeep: usize,
    },
    Pos {
        s: usize,
    },
    StopBt,
    Repeat {
        id: usize,
        pc: usize,
        count: u32,
    },
    RepeatInc {
        index: usize,
    },
    MemStart {
        mem: usize,
        s: usize,
        prev_start: Slot,
        prev_end: Slot,
    },
    MemEnd {
        mem: usize,
        s: usize,
        prev_start: Slot,
        prev_end: Slot,
    },
    /// An end assigned in place still leaves a mark, so the level count
    /// that pairs a recursive group's start and end can see it.
    MemEndMark {
        mem: usize,
    },
    NullCheckStart {
        id: usize,
        s: usize,
    },
    NullCheckEnd {
        id: usize,
    },
    CallFrame {
        ret_pc: usize,
    },
    Return,
    Void,
    AbsentPos {
        abs_s: usize,
        range: i64,
    },
    Absent,
}

impl Entry {
    /// Onigmo's `STK_MASK_POP_USED`: the entries a failure resumes from.
    fn resumes(&self) -> bool {
        matches!(
            self,
            Entry::Alt { .. } | Entry::PosNot { .. } | Entry::LookBehindNot { .. }
        )
    }

    fn mem_s(&self) -> usize {
        match self {
            Entry::MemStart { s, .. } | Entry::MemEnd { s, .. } => *s,
            _ => unreachable!("a capture entry"),
        }
    }
}

pub struct Matcher<'a> {
    program: &'a Program,
    text: &'a [char],
    text_str: &'a str,
    byte_offsets: &'a [usize],
    stack: Vec<Entry>,
    mem_start: Vec<Slot>,
    mem_end: Vec<Slot>,
    repeat_stk: Vec<usize>,
    budget: u64,
    /// `REGEX_TRACE` in the environment prints every step to stderr.
    trace: bool,
}

impl<'a> Matcher<'a> {
    pub fn new(
        program: &'a Program,
        text: &'a [char],
        text_str: &'a str,
        byte_offsets: &'a [usize],
        budget: u64,
    ) -> Matcher<'a> {
        Matcher {
            program,
            text,
            text_str,
            byte_offsets,
            stack: Vec::new(),
            mem_start: vec![Slot::Invalid; program.num_mem + 1],
            mem_end: vec![Slot::Invalid; program.num_mem + 1],
            repeat_stk: vec![0; program.repeat_ranges.len()],
            budget,
            trace: std::env::var_os("REGEX_TRACE").is_some(),
        }
    }

    fn is_word_at(&self, index: usize, ascii: bool) -> bool {
        let character = self.text[index];
        if ascii {
            character.is_ascii_alphanumeric() || character == '_'
        } else {
            unicode::ctype_contains(CTYPE_WORD, character as u32)
        }
    }

    /// `OP_EXACTN_IC`: the text folded a character at a time against the
    /// pattern's folded codes. Answers the position after the text used.
    fn compare_folded_stream(&self, folded: &[u32], s: usize, range: i64) -> Option<usize> {
        let mut position = s;
        let mut index = 0;
        while index < folded.len() {
            if (position as i64) >= range {
                return None;
            }
            let mut scratch = [0u32; 1];
            let text_fold = unicode::fold_slice(self.text[position] as u32, &mut scratch);
            if folded.len() < index + text_fold.len()
                || folded[index..index + text_fold.len()] != *text_fold
            {
                return None;
            }
            index += text_fold.len();
            position += 1;
        }
        Some(position)
    }

    /// `string_cmp_ic`: `captured` against the text at `s`, each character
    /// folded, in lockstep. Answers the position after the text consumed.
    fn compare_folded_lockstep(&self, captured: &[char], s: usize, range: i64) -> Option<usize> {
        let mut position = s;
        for character in captured {
            if (position as i64) >= range {
                return None;
            }
            let mut scratch_captured = [0u32; 1];
            let mut scratch_text = [0u32; 1];
            if unicode::fold_slice(*character as u32, &mut scratch_captured)
                != unicode::fold_slice(self.text[position] as u32, &mut scratch_text)
            {
                return None;
            }
            position += 1;
        }
        Some(position)
    }

    /// The position a slot holds: read from the stack entry or taken as is.
    fn slot_pos(&self, slot: Slot) -> Option<usize> {
        match slot {
            Slot::Invalid => None,
            Slot::Index(index) => Some(self.stack[index].mem_s()),
            Slot::Pos(position) => Some(position),
        }
    }

    fn capture_span(&self, mem: usize) -> Option<(usize, usize)> {
        let start = self.slot_pos(self.mem_start[mem])?;
        let end = self.slot_pos(self.mem_end[mem])?;
        Some((start, end))
    }

    /// `STACK_GET_MEM_START`: the start entry that pairs with the end
    /// being recorded, counting nested completed instances.
    fn matching_start(&self, mem: usize) -> Option<usize> {
        let mut level = 0usize;
        let mut index = self.stack.len();
        while index > 0 {
            index -= 1;
            match self.stack[index] {
                Entry::MemEnd { mem: candidate, .. } | Entry::MemEndMark { mem: candidate }
                    if candidate == mem =>
                {
                    level += 1;
                }
                Entry::MemStart { mem: candidate, .. } if candidate == mem => {
                    if level == 0 {
                        return Some(index);
                    }
                    level -= 1;
                }
                _ => {}
            }
        }
        None
    }

    fn restore(&mut self, entry: Entry) {
        match entry {
            Entry::MemStart {
                mem,
                prev_start,
                prev_end,
                ..
            }
            | Entry::MemEnd {
                mem,
                prev_start,
                prev_end,
                ..
            } => {
                self.mem_start[mem] = prev_start;
                self.mem_end[mem] = prev_end;
            }
            Entry::RepeatInc { index } => {
                if let Entry::Repeat { count, .. } = &mut self.stack[index] {
                    *count -= 1;
                }
            }
            _ => {}
        }
    }

    /// `STACK_POP`: back to the newest resumable entry, restoring what the
    /// entries above it changed.
    fn pop(&mut self) -> Option<Entry> {
        while let Some(entry) = self.stack.pop() {
            self.restore(entry);
            if entry.resumes() {
                return Some(entry);
            }
        }
        None
    }

    /// `STACK_POP_TIL_*`: pop with restoring until `stop` says the entry
    /// is the one looked for (inclusive).
    fn pop_until(&mut self, stop: fn(&Entry) -> bool) {
        while let Some(entry) = self.stack.pop() {
            if stop(&entry) {
                return;
            }
            self.restore(entry);
        }
    }

    /// `STACK_POS_END` and `STACK_STOP_BT_END`: void the resumable entries
    /// back to the mark, and the mark itself.
    fn void_until(&mut self, is_mark: fn(&Entry) -> bool) {
        let mut index = self.stack.len();
        while index > 0 {
            index -= 1;
            let entry = self.stack[index];
            if is_mark(&entry) {
                self.stack[index] = Entry::Void;
                return;
            }
            if entry.resumes() {
                self.stack[index] = Entry::Void;
            }
        }
    }

    fn push_mem_start(&mut self, mem: usize, s: usize) {
        let entry = Entry::MemStart {
            mem,
            s,
            prev_start: self.mem_start[mem],
            prev_end: self.mem_end[mem],
        };
        self.stack.push(entry);
        self.mem_start[mem] = Slot::Index(self.stack.len() - 1);
        self.mem_end[mem] = Slot::Invalid;
    }

    fn push_mem_end(&mut self, mem: usize, s: usize) {
        let entry = Entry::MemEnd {
            mem,
            s,
            prev_start: self.mem_start[mem],
            prev_end: self.mem_end[mem],
        };
        self.stack.push(entry);
        self.mem_end[mem] = Slot::Index(self.stack.len() - 1);
    }

    /// `STACK_GET_REPEAT`: the repeat entry for `id` at the current call
    /// level.
    fn repeat_entry_by_stack(&self, id: usize) -> usize {
        let mut level: i32 = 0;
        let mut index = self.stack.len();
        while index > 0 {
            index -= 1;
            match self.stack[index] {
                Entry::Repeat { id: candidate, .. } => {
                    if level == 0 && candidate == id {
                        return index;
                    }
                }
                Entry::CallFrame { .. } => level -= 1,
                Entry::Return => level += 1,
                _ => {}
            }
        }
        self.repeat_stk[id]
    }

    /// `STACK_RETURN`: the frame a return goes back to.
    fn return_address(&self) -> usize {
        let mut level: i32 = 0;
        let mut index = self.stack.len();
        while index > 0 {
            index -= 1;
            match self.stack[index] {
                Entry::CallFrame { ret_pc } => {
                    if level == 0 {
                        return ret_pc;
                    }
                    level -= 1;
                }
                Entry::Return => level += 1,
                _ => {}
            }
        }
        unreachable!("a return has a frame")
    }

    /// `STACK_NULL_CHECK` family: 1 is an empty iteration, 0 not empty,
    /// -1 empty with a capture moved. Under the capture-watching kinds a
    /// group that had no end before this iteration, or whose previous
    /// capture was not empty, counts as progress, which is what lets
    /// `(?:()|())*` run once per group.
    fn null_check(&self, id: usize, s: usize, kind: EmptyInfo) -> i32 {
        let mut level = 0usize;
        let mut index = self.stack.len();
        while index > 0 {
            index -= 1;
            match self.stack[index] {
                Entry::NullCheckStart {
                    id: candidate,
                    s: start_s,
                } if candidate == id => {
                    if kind == EmptyInfo::Recursion && level > 0 {
                        level -= 1;
                        continue;
                    }
                    if start_s != s {
                        return 0;
                    }
                    if kind == EmptyInfo::Plain {
                        return 1;
                    }
                    let mut is_null = 1;
                    for entry in &self.stack[index..] {
                        if let Entry::MemStart {
                            prev_start,
                            prev_end,
                            ..
                        } = entry
                        {
                            let Some(end_pos) = self.slot_pos(*prev_end) else {
                                return 0;
                            };
                            let Some(start_pos) = self.slot_pos(*prev_start) else {
                                return 0;
                            };
                            if start_pos != end_pos {
                                return 0;
                            } else if end_pos != s {
                                is_null = -1;
                            }
                        }
                    }
                    return is_null;
                }
                Entry::NullCheckEnd { id: candidate }
                    if candidate == id && kind == EmptyInfo::Recursion =>
                {
                    level += 1;
                }
                _ => {}
            }
        }
        0
    }

    /// `backref_match_at_nested_level`.
    fn backref_at_level(
        &self,
        mems: &[usize],
        level_wanted: i32,
        ignore_case: bool,
        s: usize,
        range: i64,
    ) -> Option<usize> {
        let mut level: i32 = 0;
        let mut pend: Option<usize> = None;
        let mut index = self.stack.len();
        while index > 0 {
            index -= 1;
            match self.stack[index] {
                Entry::CallFrame { .. } => level -= 1,
                Entry::Return => level += 1,
                Entry::MemStart {
                    mem, s: start_pos, ..
                } if level == level_wanted && mems.contains(&mem) => {
                    if let Some(end_pos) = pend {
                        let captured = &self.text[start_pos..end_pos];
                        if (s + captured.len()) as i64 > range {
                            return None;
                        }
                        if ignore_case {
                            return self.compare_folded_lockstep(captured, s, range);
                        }
                        if self.text[s..s + captured.len()] == *captured {
                            return Some(s + captured.len());
                        }
                        return None;
                    }
                }
                Entry::MemEnd {
                    mem, s: end_pos, ..
                } if level == level_wanted && mems.contains(&mem) => {
                    pend = Some(end_pos);
                }
                _ => {}
            }
        }
        None
    }

    /// The length in characters of the grapheme cluster starting at `s`.
    fn grapheme_length(&self, s: usize, end: usize) -> usize {
        let from = self.byte_offsets[s];
        let to = self.byte_offsets[end];
        let slice = &self.text_str[from..to];
        match slice.graphemes(true).next() {
            Some(cluster) => cluster.chars().count(),
            None => 0,
        }
    }

    /// `match_at`: one attempt from `sstart`; `gpos` is where the search
    /// began, for `\G`. Answers the groups as character spans.
    pub fn match_at(&mut self, sstart: usize, gpos: usize) -> Result<Option<Spans>, RegexError> {
        let program = self.program;
        self.stack.clear();
        for slot in self.mem_start.iter_mut() {
            *slot = Slot::Invalid;
        }
        for slot in self.mem_end.iter_mut() {
            *slot = Slot::Invalid;
        }
        // The bottom of the stack: failing past it ends the attempt.
        self.stack.push(Entry::Alt {
            pc: usize::MAX,
            s: sstart,
            pkeep: sstart,
        });
        let mut pc = 0usize;
        let mut s = sstart;
        let mut pkeep = sstart;
        // Ruby's build lets the absence operator narrow the text's end
        // itself, so one bound serves the reads and the end anchors alike.
        let mut range: i64 = self.text.len() as i64;
        loop {
            if self.budget == 0 {
                return Err(RegexError::new(error::MATCH_STACK_LIMIT_OVER));
            }
            self.budget -= 1;
            if pc == usize::MAX {
                return Ok(None);
            }
            let sprev: i64 = s as i64 - 1;
            let mut fail = false;
            if self.trace {
                eprintln!(
                    "pc={pc} s={s} stack={} {:?}",
                    self.stack.len(),
                    program.ops[pc]
                );
            }
            match &program.ops[pc] {
                Op::Finish => return Ok(None),
                Op::End => {
                    let mut groups = Vec::with_capacity(program.num_mem + 1);
                    groups.push(Some((pkeep.min(s), s)));
                    for mem in 1..=program.num_mem {
                        groups.push(self.capture_span(mem));
                    }
                    return Ok(Some(groups));
                }
                Op::Exact(chars) => {
                    if (s + chars.len()) as i64 > range || self.text[s..s + chars.len()] != **chars
                    {
                        fail = true;
                    } else {
                        s += chars.len();
                        pc += 1;
                    }
                }
                Op::ExactIc(folded) => match self.compare_folded_stream(folded, s, range) {
                    Some(after) => {
                        s = after;
                        pc += 1;
                    }
                    None => fail = true,
                },
                Op::Class { bitset, mbuf, not } => {
                    if (s as i64) >= range {
                        fail = true;
                    } else {
                        let code = self.text[s] as u32;
                        let multibyte = code >= 0x80;
                        // The six OP_CCLASS* behaviours: without ranges the
                        // bitset is read at the character's lead byte; with
                        // ranges a multibyte character consults them and a
                        // single-byte one the bitset, or passes a negated
                        // class outright when the bitset is empty.
                        let found = match mbuf {
                            None => bit_at(bitset, lead_byte(code)),
                            Some(ranges) => {
                                if multibyte {
                                    super::cclass::in_ranges(ranges, code)
                                } else if bitset == &[0u64; 4] {
                                    // OP_CCLASS_MB fails on a single byte;
                                    // OP_CCLASS_MB_NOT passes it: not found
                                    // either way.
                                    false
                                } else {
                                    bit_at(bitset, code)
                                }
                            }
                        };
                        if found != *not {
                            s += 1;
                            pc += 1;
                        } else {
                            fail = true;
                        }
                    }
                }
                Op::Word { not, ascii } => {
                    if (s as i64) < range && (self.is_word_at(s, *ascii) != *not) {
                        s += 1;
                        pc += 1;
                    } else {
                        fail = true;
                    }
                }
                Op::AnyChar => {
                    if (s as i64) < range && self.text[s] != '\n' {
                        s += 1;
                        pc += 1;
                    } else {
                        fail = true;
                    }
                }
                Op::AnyCharMl => {
                    if (s as i64) < range {
                        s += 1;
                        pc += 1;
                    } else {
                        fail = true;
                    }
                }
                Op::GraphemeCluster => {
                    if (s as i64) < range {
                        let bound = (range.max(s as i64) as usize).min(self.text.len());
                        let length = self.grapheme_length(s, bound);
                        if length == 0 || (s + length) as i64 > range {
                            fail = true;
                        } else {
                            s += length;
                            pc += 1;
                        }
                    } else {
                        fail = true;
                    }
                }
                Op::BeginBuf => {
                    if s == 0 {
                        pc += 1;
                    } else {
                        fail = true;
                    }
                }
                Op::EndBuf => {
                    if (s as i64) == range {
                        pc += 1;
                    } else {
                        fail = true;
                    }
                }
                Op::SemiEndBuf => {
                    if (s as i64) == range
                        || (s < self.text.len() && self.text[s] == '\n' && (s as i64) + 1 == range)
                    {
                        pc += 1;
                    } else {
                        fail = true;
                    }
                }
                Op::BeginLine => {
                    if s == 0 || (self.text[s - 1] == '\n' && (s as i64) != range) {
                        pc += 1;
                    } else {
                        fail = true;
                    }
                }
                Op::EndLine => {
                    if (s as i64) == range || (s < self.text.len() && self.text[s] == '\n') {
                        pc += 1;
                    } else {
                        fail = true;
                    }
                }
                Op::BeginPosition => {
                    if s == gpos {
                        pc += 1;
                    } else {
                        fail = true;
                    }
                }
                Op::WordBound { ascii } => {
                    let ok = if s == 0 {
                        (s as i64) < range && self.is_word_at(s, *ascii)
                    } else if (s as i64) == range {
                        self.is_word_at(s - 1, *ascii)
                    } else {
                        self.is_word_at(s, *ascii) != self.is_word_at(s - 1, *ascii)
                    };
                    if ok {
                        pc += 1;
                    } else {
                        fail = true;
                    }
                }
                Op::NotWordBound { ascii } => {
                    let bad = if s == 0 {
                        (s as i64) < range && self.is_word_at(s, *ascii)
                    } else if (s as i64) == range {
                        self.is_word_at(s - 1, *ascii)
                    } else {
                        self.is_word_at(s, *ascii) != self.is_word_at(s - 1, *ascii)
                    };
                    if bad {
                        fail = true;
                    } else {
                        pc += 1;
                    }
                }
                Op::Keep => {
                    pkeep = s;
                    pc += 1;
                }
                Op::MemStart { mem, push } => {
                    if *push {
                        self.push_mem_start(*mem, s);
                    } else {
                        // OP_MEMORY_START: assigned in place, not restored
                        // by backtracking.
                        self.mem_start[*mem] = Slot::Pos(s);
                        self.mem_end[*mem] = Slot::Invalid;
                    }
                    pc += 1;
                }
                Op::MemEnd { mem, push, rec } => {
                    if *rec {
                        // The start that pairs with this end by recursion
                        // level; a called group's start is always pushed.
                        let start_index = self.matching_start(*mem);
                        if *push {
                            self.push_mem_end(*mem, s);
                        } else {
                            self.mem_end[*mem] = Slot::Pos(s);
                            self.stack.push(Entry::MemEndMark { mem: *mem });
                        }
                        if let Some(start_index) = start_index {
                            self.mem_start[*mem] = Slot::Index(start_index);
                        }
                    } else if *push {
                        self.push_mem_end(*mem, s);
                    } else {
                        self.mem_end[*mem] = Slot::Pos(s);
                    }
                    pc += 1;
                }
                Op::BackRef { mems, ignore_case } => {
                    let mut matched = false;
                    for mem in mems {
                        if *mem > program.num_mem {
                            continue;
                        }
                        let Some((start, finish)) = self.capture_span(*mem) else {
                            continue;
                        };
                        let captured = &self.text[start..finish];
                        if (s + captured.len()) as i64 > range {
                            continue;
                        }
                        let after = if *ignore_case {
                            self.compare_folded_lockstep(captured, s, range)
                        } else if self.text[s..s + captured.len()] == *captured {
                            Some(s + captured.len())
                        } else {
                            None
                        };
                        if let Some(after) = after {
                            s = after;
                            matched = true;
                            break;
                        }
                    }
                    if matched {
                        pc += 1;
                    } else {
                        fail = true;
                    }
                }
                Op::BackRefLevel {
                    mems,
                    level,
                    ignore_case,
                } => match self.backref_at_level(mems, *level, *ignore_case, s, range) {
                    Some(after) => {
                        s = after;
                        pc += 1;
                    }
                    None => fail = true,
                },
                Op::NullCheckStart(id) => {
                    self.stack.push(Entry::NullCheckStart { id: *id, s });
                    pc += 1;
                }
                Op::NullCheckEnd { id, kind } => {
                    let is_null = self.null_check(*id, s, *kind);
                    if is_null == -1 {
                        fail = true;
                    } else if is_null == 1 {
                        // An empty iteration: skip the loop's next step.
                        pc += 2;
                    } else {
                        if *kind == EmptyInfo::Recursion {
                            self.stack.push(Entry::NullCheckEnd { id: *id });
                        }
                        pc += 1;
                    }
                }
                Op::Jump(addr) => pc = *addr,
                Op::Push(addr) => {
                    self.stack.push(Entry::Alt {
                        pc: *addr,
                        s,
                        pkeep,
                    });
                    pc += 1;
                }
                Op::Pop => {
                    self.stack.pop();
                    pc += 1;
                }
                Op::Repeat { id, greedy, after } => {
                    let (lower, _) = program.repeat_ranges[*id];
                    self.repeat_stk[*id] = self.stack.len();
                    self.stack.push(Entry::Repeat {
                        id: *id,
                        pc: pc + 1,
                        count: 0,
                    });
                    if lower == 0 {
                        if *greedy {
                            self.stack.push(Entry::Alt {
                                pc: *after,
                                s,
                                pkeep,
                            });
                            pc += 1;
                        } else {
                            self.stack.push(Entry::Alt {
                                pc: pc + 1,
                                s,
                                pkeep,
                            });
                            pc = *after;
                        }
                    } else {
                        pc += 1;
                    }
                }
                Op::RepeatInc {
                    id,
                    greedy,
                    by_stack,
                } => {
                    let (lower, upper) = program.repeat_ranges[*id];
                    let index = if *by_stack {
                        self.repeat_entry_by_stack(*id)
                    } else {
                        self.repeat_stk[*id]
                    };
                    let (body_pc, count) = match &mut self.stack[index] {
                        Entry::Repeat {
                            pc: body_pc, count, ..
                        } => {
                            *count += 1;
                            (*body_pc, *count)
                        }
                        _ => unreachable!("a repeat entry"),
                    };
                    let at_upper = upper.is_some_and(|upper| count >= upper);
                    if *greedy {
                        if at_upper {
                            pc += 1;
                        } else if count >= lower {
                            self.stack.push(Entry::Alt {
                                pc: pc + 1,
                                s,
                                pkeep,
                            });
                            pc = body_pc;
                        } else {
                            pc = body_pc;
                        }
                        self.stack.push(Entry::RepeatInc { index });
                    } else if upper.is_none_or(|upper| count < upper) {
                        if count >= lower {
                            self.stack.push(Entry::RepeatInc { index });
                            self.stack.push(Entry::Alt {
                                pc: body_pc,
                                s,
                                pkeep,
                            });
                            pc += 1;
                        } else {
                            pc = body_pc;
                            self.stack.push(Entry::RepeatInc { index });
                        }
                    } else {
                        self.stack.push(Entry::RepeatInc { index });
                        pc += 1;
                    }
                }
                Op::PushPos => {
                    self.stack.push(Entry::Pos { s });
                    pc += 1;
                }
                Op::PopPos => {
                    let mut index = self.stack.len();
                    let mut saved = s;
                    while index > 0 {
                        index -= 1;
                        if let Entry::Pos { s: mark_s } = self.stack[index] {
                            saved = mark_s;
                            break;
                        }
                    }
                    self.void_until(|entry| matches!(entry, Entry::Pos { .. }));
                    s = saved;
                    pc += 1;
                }
                Op::PushPosNot(addr) => {
                    self.stack.push(Entry::PosNot {
                        pc: *addr,
                        s,
                        pkeep,
                    });
                    pc += 1;
                }
                Op::FailPos => {
                    self.pop_until(|entry| matches!(entry, Entry::PosNot { .. }));
                    fail = true;
                }
                Op::PushStopBt => {
                    self.stack.push(Entry::StopBt);
                    pc += 1;
                }
                Op::PopStopBt => {
                    self.void_until(|entry| matches!(entry, Entry::StopBt));
                    pc += 1;
                }
                Op::LookBehind(len) => {
                    if s < *len {
                        fail = true;
                    } else {
                        s -= len;
                        pc += 1;
                    }
                }
                Op::PushLookBehindNot { addr, len } => {
                    if s < *len {
                        // Too short to hold the body: the negative lookbehind
                        // succeeds.
                        pc = *addr;
                    } else {
                        self.stack.push(Entry::LookBehindNot {
                            pc: *addr,
                            s,
                            pkeep,
                        });
                        s -= len;
                        pc += 1;
                    }
                }
                Op::FailLookBehindNot => {
                    self.pop_until(|entry| matches!(entry, Entry::LookBehindNot { .. }));
                    fail = true;
                }
                Op::PushAbsentPos => {
                    self.stack.push(Entry::AbsentPos { abs_s: s, range });
                    pc += 1;
                }
                Op::Absent(addr) => {
                    let aend = range;
                    let Some(Entry::AbsentPos {
                        abs_s: absent,
                        range: saved_range,
                    }) = self.stack.pop()
                    else {
                        unreachable!("an absent position")
                    };
                    range = saved_range;
                    let s_signed = s as i64;
                    let absent_signed = absent as i64;
                    if absent_signed > aend && s > absent {
                        // An empty match inside (?~...) at its start: never.
                        self.pop();
                        fail = true;
                    } else if s_signed >= aend && s > absent {
                        if s_signed > aend {
                            fail = true;
                        } else {
                            pc = *addr;
                        }
                    } else if (s as i64) == range {
                        // At the end of the string: just match.
                        pc = *addr;
                    } else {
                        self.stack.push(Entry::Alt {
                            pc: *addr,
                            s,
                            pkeep,
                        });
                        self.stack.push(Entry::AbsentPos {
                            abs_s: absent,
                            range,
                        });
                        self.stack.push(Entry::Alt {
                            pc,
                            s: s + 1,
                            pkeep,
                        });
                        self.stack.push(Entry::Absent);
                        range = aend;
                        pc += 1;
                    }
                }
                Op::AbsentEnd => {
                    if sprev < range {
                        range = sprev;
                    }
                    self.pop_until(|entry| matches!(entry, Entry::Absent));
                    fail = true;
                }
                Op::Call(addr) => {
                    self.stack.push(Entry::CallFrame { ret_pc: pc + 1 });
                    pc = *addr;
                }
                Op::Return => {
                    let ret = self.return_address();
                    self.stack.push(Entry::Return);
                    pc = ret;
                }
                Op::Condition { mem, addr } => {
                    if *mem > program.num_mem
                        || self.mem_end[*mem] == Slot::Invalid
                        || self.mem_start[*mem] == Slot::Invalid
                    {
                        pc = *addr;
                    } else {
                        pc += 1;
                    }
                }
                Op::Fail => fail = true,
            }
            if fail {
                match self.pop() {
                    Some(Entry::Alt {
                        pc: resume_pc,
                        s: resume_s,
                        pkeep: resume_keep,
                    })
                    | Some(Entry::PosNot {
                        pc: resume_pc,
                        s: resume_s,
                        pkeep: resume_keep,
                    })
                    | Some(Entry::LookBehindNot {
                        pc: resume_pc,
                        s: resume_s,
                        pkeep: resume_keep,
                    }) => {
                        pc = resume_pc;
                        s = resume_s;
                        pkeep = resume_keep;
                    }
                    _ => return Ok(None),
                }
            }
        }
    }
}
