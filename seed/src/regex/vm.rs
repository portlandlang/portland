//! The backtracking matcher: Onigmo's `match_at` over a program, as Ruby's
//! copy of `regexec.c` has it. Positions are byte offsets into the UTF-8
//! text, always on a character boundary, so a capture is already what
//! Ruby's `MatchData#byteoffset` answers and nothing is translated. The
//! stack is Onigmo's: besides the alternatives to resume, it records
//! capture starts and ends, repeat counts, null-check marks, call frames,
//! and the atomic and lookaround marks, so that backtracking restores all
//! of them in one walk.
//!
//! Two positions are signed: the text's end, which Ruby's build lets the
//! absence operator narrow for every read and end anchor alike, and the
//! position before the current one, which is -1 at the start of the text;
//! Onigmo has a null pointer there, and the absence operator's arithmetic
//! leans on it.

use super::ast::EmptyInfo;
use super::cache;
use super::compile::{Op, Program};
use super::error::{self, RegexError};
use super::unicode::{self, CTYPE_WORD};

/// The groups of a match as byte spans, group 0 first; a group that took
/// no part is `None`.
pub type Spans = Vec<Option<(usize, usize)>>;

fn bit_at(bitset: &[u64; 4], code: u32) -> bool {
    code < 256 && (bitset[(code / 64) as usize] >> (code % 64)) & 1 == 1
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
    /// A match cache point passed on the way down, memoized when
    /// backtracking pops it: the path from here failed.
    MatchCachePoint {
        index: usize,
        mask: u8,
    },
    /// The same inside an atomic group that has since closed: popping it
    /// memoizes that the group succeeded from here and what followed failed.
    AtomicMatchCachePoint {
        index: usize,
        mask: u8,
    },
}

/// `STACK_POP_TIL_*`: which mark a pop runs back to, and what the entries
/// on the way mean for the match cache and the failure count.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PopTo {
    /// A negative lookahead whose body matched: the alternatives inside
    /// count as failures, and the cache points memoize that the body
    /// succeeds from them.
    PosNot,
    LookBehindNot,
    Absent,
}

/// `STACK_POS_END` and `STACK_STOP_BT_END`: which mark the alternatives
/// are voided back to.
#[derive(Clone, Copy, PartialEq, Eq)]
enum VoidTo {
    /// A positive lookahead whose body matched: its cache points memoize
    /// that the body succeeds from them.
    Pos,
    /// An atomic group that matched: its cache points turn atomic.
    StopBt,
}

/// What a match cache check at a branching instruction decides.
enum CacheVerdict {
    /// Not memoized, or the cache is off: carry on.
    Proceed,
    /// Memoized as failing from here.
    Fail,
    /// Memoized as succeeding from here inside a lookaround: skip to its end.
    Jump(usize),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CacheStatus {
    /// Not yet failed often enough to be worth it.
    Waiting,
    /// On: the bit array is live.
    Enabled,
    /// The program cannot be cached.
    Disabled,
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
    text: &'a str,
    bytes: &'a [u8],
    stack: Vec<Entry>,
    mem_start: Vec<Slot>,
    mem_end: Vec<Slot>,
    repeat_stk: Vec<usize>,
    budget: u64,
    /// `REGEX_TRACE` in the environment prints every step to stderr.
    trace: bool,
    cache_status: CacheStatus,
    /// Failures so far, across the attempts of one search: the match cache
    /// turns on when they reach the text's length times the cache opcodes.
    num_fails: u64,
    /// The match cache's bit array, one search long.
    match_cache: Vec<u8>,
}

impl<'a> Matcher<'a> {
    pub fn new(program: &'a Program, text: &'a str, budget: u64) -> Matcher<'a> {
        let cache_status = match &program.cache {
            Some(cache) if !cache.opcodes.is_empty() => CacheStatus::Waiting,
            _ => CacheStatus::Disabled,
        };
        Matcher {
            program,
            text,
            bytes: text.as_bytes(),
            stack: Vec::new(),
            mem_start: vec![Slot::Invalid; program.num_mem + 1],
            mem_end: vec![Slot::Invalid; program.num_mem + 1],
            repeat_stk: vec![0; program.repeat_ranges.len()],
            budget,
            trace: std::env::var_os("REGEX_TRACE").is_some(),
            cache_status,
            num_fails: 0,
            match_cache: Vec::new(),
        }
    }

    /// `INC_NUM_FAILS`: an alternative thrown away by a lookaround or an
    /// atomic group that matched counts as a failure.
    fn count_fail(&mut self) {
        self.num_fails += 1;
    }

    /// The failure count's threshold check, run after every pop on failure:
    /// `num_fails` reaching the text's length times the cache opcodes turns
    /// the match cache on for the rest of the search.
    fn count_fail_and_check(&mut self) {
        self.num_fails += 1;
        if self.cache_status != CacheStatus::Waiting {
            return;
        }
        let cache = self.program.cache.as_ref().expect("a cacheable program");
        let threshold = self.bytes.len() as u64 * cache.opcodes.len() as u64;
        if self.num_fails < threshold {
            return;
        }
        self.cache_status = CacheStatus::Enabled;
        self.match_cache = cache::new_buffer(cache.num_cache_points, self.bytes.len());
    }

    /// `CHECK_MATCH_CACHE` at the branching instruction `addr`, about to
    /// try from `s`; `is_inc` says it is a repeat's end whose count was
    /// just raised.
    fn check_match_cache(&mut self, addr: usize, s: usize, is_inc: bool) -> CacheVerdict {
        if self.cache_status != CacheStatus::Enabled {
            return CacheVerdict::Proceed;
        }
        let program = self.program;
        let cache = program.cache.as_ref().expect("a cacheable program");
        let count_of = |id: usize| match self.stack[self.repeat_stk[id]] {
            Entry::Repeat { count, .. } => count,
            _ => unreachable!("a repeat entry"),
        };
        let Some((cache_point, opcode)) =
            cache.find_cache_point(&program.repeat_ranges, addr, is_inc, count_of)
        else {
            return CacheVerdict::Proceed;
        };
        let (index, mask) = cache::match_cache_point(cache.num_cache_points, s, cache_point);
        if cache::is_memoized(&self.match_cache, index, mask) {
            let extended = cache::is_memoized_extended(&self.match_cache, index, mask);
            if opcode.lookaround_nesting == 0 {
                return CacheVerdict::Fail;
            }
            if opcode.lookaround_nesting < 0 {
                if extended {
                    self.stop_bt_fail();
                }
                return CacheVerdict::Fail;
            }
            if extended {
                return CacheVerdict::Jump(opcode.match_addr.expect("a lookaround's end"));
            }
            return CacheVerdict::Fail;
        }
        self.stack.push(Entry::MatchCachePoint { index, mask });
        CacheVerdict::Proceed
    }

    /// `STACK_STOP_BT_FAIL`: an atomic group known to succeed and then
    /// fail from here is left wholesale, its cache points memoized as such,
    /// restoring nothing on the way: the cache covers no capture or repeat
    /// inside one.
    fn stop_bt_fail(&mut self) {
        while let Some(entry) = self.stack.pop() {
            match entry {
                Entry::StopBt => return,
                Entry::MatchCachePoint { index, mask } => {
                    cache::memoize_extended(&mut self.match_cache, index, mask);
                }
                _ => {}
            }
        }
    }

    /// The character at byte `s` and its length; `s` is on a boundary.
    fn char_at(&self, s: usize) -> (char, usize) {
        let lead = self.bytes[s];
        if lead < 0x80 {
            (lead as char, 1)
        } else {
            let character = self.text[s..].chars().next().expect("a boundary");
            (character, character.len_utf8())
        }
    }

    /// The length in bytes of the character at `s`.
    pub fn char_len_at(&self, s: usize) -> usize {
        let lead = self.bytes[s];
        if lead < 0x80 {
            1
        } else if lead >> 5 == 0b110 {
            2
        } else if lead >> 4 == 0b1110 {
            3
        } else {
            4
        }
    }

    /// The start of the character before byte `s`.
    fn prev_head(&self, s: usize) -> Option<usize> {
        if s == 0 {
            return None;
        }
        let mut position = s - 1;
        while !self.text.is_char_boundary(position) {
            position -= 1;
        }
        Some(position)
    }

    /// `ONIGENC_STEP_BACK`: `count` characters back from `s`.
    fn step_back(&self, s: usize, count: usize) -> Option<usize> {
        let mut position = s;
        for _ in 0..count {
            position = self.prev_head(position)?;
        }
        Some(position)
    }

    fn is_word(&self, character: char, ascii: bool) -> bool {
        if ascii {
            character.is_ascii_alphanumeric() || character == '_'
        } else {
            unicode::ctype_contains(CTYPE_WORD, character as u32)
        }
    }

    fn is_word_at(&self, s: usize, ascii: bool) -> bool {
        self.is_word(self.char_at(s).0, ascii)
    }

    fn is_word_before(&self, s: usize, ascii: bool) -> bool {
        match self.prev_head(s) {
            Some(head) => self.is_word_at(head, ascii),
            None => false,
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
            let (character, length) = self.char_at(position);
            let mut scratch = [0u32; 1];
            let text_fold = unicode::fold_slice(character as u32, &mut scratch);
            if folded.len() < index + text_fold.len()
                || folded[index..index + text_fold.len()] != *text_fold
            {
                return None;
            }
            index += text_fold.len();
            position += length;
        }
        Some(position)
    }

    /// `string_cmp_ic`: `captured` against the text at `s`, each character
    /// folded, in lockstep. Answers the position after the text consumed.
    fn compare_folded_lockstep(&self, captured: &str, s: usize, range: i64) -> Option<usize> {
        let mut position = s;
        for character in captured.chars() {
            if (position as i64) >= range {
                return None;
            }
            let (text_character, length) = self.char_at(position);
            let mut scratch_captured = [0u32; 1];
            let mut scratch_text = [0u32; 1];
            if unicode::fold_slice(character as u32, &mut scratch_captured)
                != unicode::fold_slice(text_character as u32, &mut scratch_text)
            {
                return None;
            }
            position += length;
        }
        Some(position)
    }

    /// A captured span against the text at `s`: `STRING_CMP` or its
    /// case-insensitive twin. Answers the position after the text consumed.
    fn compare_captured(
        &self,
        start: usize,
        finish: usize,
        ignore_case: bool,
        s: usize,
        range: i64,
    ) -> Option<usize> {
        let captured = &self.text[start..finish];
        if ignore_case {
            return self.compare_folded_lockstep(captured, s, range);
        }
        let length = captured.len();
        if (s + length) as i64 > range {
            return None;
        }
        if self.bytes[s..s + length] == *captured.as_bytes() {
            Some(s + length)
        } else {
            None
        }
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
            // `MEMOIZE_MATCH_CACHE_POINT`: a path backtracked over failed.
            Entry::MatchCachePoint { index, mask } => {
                cache::memoize(&mut self.match_cache, index, mask);
            }
            Entry::AtomicMatchCachePoint { index, mask } => {
                cache::memoize_extended(&mut self.match_cache, index, mask);
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

    /// `STACK_POP_TIL_*`: pop with restoring until the mark `to` names
    /// (inclusive).
    fn pop_until(&mut self, to: PopTo) {
        while let Some(entry) = self.stack.pop() {
            let is_mark = match to {
                PopTo::PosNot => matches!(entry, Entry::PosNot { .. }),
                PopTo::LookBehindNot => matches!(entry, Entry::LookBehindNot { .. }),
                PopTo::Absent => matches!(entry, Entry::Absent),
            };
            if is_mark {
                return;
            }
            if to == PopTo::PosNot {
                match entry {
                    Entry::MatchCachePoint { index, mask } => {
                        cache::memoize_extended(&mut self.match_cache, index, mask);
                        continue;
                    }
                    Entry::AtomicMatchCachePoint { .. } => continue,
                    _ if entry.resumes() => self.count_fail(),
                    _ => {}
                }
            }
            self.restore(entry);
        }
    }

    /// `STACK_POS_END` and `STACK_STOP_BT_END`: void the resumable entries
    /// back to the mark, and the mark itself; each voided alternative
    /// counts as a failure.
    fn void_until(&mut self, to: VoidTo) {
        let mut index = self.stack.len();
        while index > 0 {
            index -= 1;
            let entry = self.stack[index];
            let is_mark = match to {
                VoidTo::Pos => matches!(entry, Entry::Pos { .. }),
                VoidTo::StopBt => matches!(entry, Entry::StopBt),
            };
            if is_mark {
                self.stack[index] = Entry::Void;
                return;
            }
            match entry {
                _ if entry.resumes() => {
                    self.count_fail();
                    self.stack[index] = Entry::Void;
                }
                Entry::MatchCachePoint {
                    index: point_index,
                    mask,
                } => match to {
                    VoidTo::Pos => {
                        cache::memoize_extended(&mut self.match_cache, point_index, mask);
                    }
                    VoidTo::StopBt => {
                        self.stack[index] = Entry::AtomicMatchCachePoint {
                            index: point_index,
                            mask,
                        };
                    }
                },
                _ => {}
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
                        return self.compare_captured(start_pos, end_pos, ignore_case, s, range);
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

    /// `match_at`: one attempt from `sstart`; `gpos` is where the search
    /// began, for `\G`. Answers the groups as byte spans.
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
        let mut range: i64 = self.bytes.len() as i64;
        loop {
            if self.budget == 0 {
                return Err(RegexError::new(error::MATCH_STACK_LIMIT_OVER));
            }
            self.budget -= 1;
            if pc == usize::MAX {
                return Ok(None);
            }
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
                    // A path that ends before its start — a lookbehind
                    // whose folded body consumed fewer characters than it
                    // stepped back over — is no match, and Onigmo's `n >
                    // best_len` test ends the attempt there, alternatives
                    // untried.
                    if s < sstart {
                        return Ok(None);
                    }
                    let mut groups = Vec::with_capacity(program.num_mem + 1);
                    groups.push(Some((pkeep.min(s), s)));
                    for mem in 1..=program.num_mem {
                        groups.push(self.capture_span(mem));
                    }
                    return Ok(Some(groups));
                }
                Op::Exact(literal) => {
                    let length = literal.len();
                    if (s + length) as i64 > range
                        || self.bytes[s..s + length] != *literal.as_bytes()
                    {
                        fail = true;
                    } else {
                        s += length;
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
                        let (character, length) = self.char_at(s);
                        let code = character as u32;
                        // The six OP_CCLASS* behaviours: without ranges the
                        // bitset is read at the character's lead byte; with
                        // ranges a multibyte character consults them and a
                        // single-byte one the bitset, or passes a negated
                        // class outright when the bitset is empty.
                        let found = match mbuf {
                            None => bit_at(bitset, u32::from(self.bytes[s])),
                            Some(ranges) => {
                                if code >= 0x80 {
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
                            s += length;
                            pc += 1;
                        } else {
                            fail = true;
                        }
                    }
                }
                Op::Word { not, ascii } => {
                    if (s as i64) < range {
                        let (character, length) = self.char_at(s);
                        if self.is_word(character, *ascii) != *not {
                            s += length;
                            pc += 1;
                        } else {
                            fail = true;
                        }
                    } else {
                        fail = true;
                    }
                }
                Op::AnyChar => {
                    if (s as i64) < range && self.bytes[s] != b'\n' {
                        s += self.char_len_at(s);
                        pc += 1;
                    } else {
                        fail = true;
                    }
                }
                Op::AnyCharMl => {
                    if (s as i64) < range {
                        s += self.char_len_at(s);
                        pc += 1;
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
                        || (s < self.bytes.len()
                            && self.bytes[s] == b'\n'
                            && (s as i64) + 1 == range)
                    {
                        pc += 1;
                    } else {
                        fail = true;
                    }
                }
                Op::BeginLine => {
                    if s == 0 || (self.bytes[s - 1] == b'\n' && (s as i64) != range) {
                        pc += 1;
                    } else {
                        fail = true;
                    }
                }
                Op::EndLine => {
                    if (s as i64) == range || (s < self.bytes.len() && self.bytes[s] == b'\n') {
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
                        self.is_word_before(s, *ascii)
                    } else {
                        self.is_word_at(s, *ascii) != self.is_word_before(s, *ascii)
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
                        self.is_word_before(s, *ascii)
                    } else {
                        self.is_word_at(s, *ascii) != self.is_word_before(s, *ascii)
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
                        if let Some(after) =
                            self.compare_captured(start, finish, *ignore_case, s, range)
                        {
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
                Op::Push(addr) => match self.check_match_cache(pc, s, false) {
                    CacheVerdict::Proceed => {
                        self.stack.push(Entry::Alt {
                            pc: *addr,
                            s,
                            pkeep,
                        });
                        pc += 1;
                    }
                    CacheVerdict::Fail => fail = true,
                    CacheVerdict::Jump(addr) => pc = addr,
                },
                Op::PushOrJumpExact1 { addr, byte } => {
                    if s < self.bytes.len() && self.bytes[s] == *byte && (s as i64) < range {
                        match self.check_match_cache(pc, s, false) {
                            CacheVerdict::Proceed => {
                                self.stack.push(Entry::Alt {
                                    pc: *addr,
                                    s,
                                    pkeep,
                                });
                                pc += 1;
                            }
                            CacheVerdict::Fail => fail = true,
                            CacheVerdict::Jump(addr) => pc = addr,
                        }
                    } else {
                        pc = *addr;
                    }
                }
                Op::PushIfPeekNext { addr, byte } => match self.check_match_cache(pc, s, false) {
                    CacheVerdict::Proceed => {
                        if s < self.bytes.len() && self.bytes[s] == *byte {
                            self.stack.push(Entry::Alt {
                                pc: *addr,
                                s,
                                pkeep,
                            });
                        } else {
                            // No alternative to come back to: as good as a
                            // failure for the match cache's count.
                            self.count_fail();
                        }
                        pc += 1;
                    }
                    CacheVerdict::Fail => fail = true,
                    CacheVerdict::Jump(addr) => pc = addr,
                },
                Op::AnyCharStar { multiline, peek } => {
                    let mut jumped = false;
                    while (s as i64) < range {
                        match self.check_match_cache(pc, s, false) {
                            CacheVerdict::Proceed => {}
                            CacheVerdict::Fail => {
                                fail = true;
                                break;
                            }
                            CacheVerdict::Jump(addr) => {
                                pc = addr;
                                jumped = true;
                                break;
                            }
                        }
                        match peek {
                            Some(byte) if self.bytes[s] != *byte => self.count_fail(),
                            _ => self.stack.push(Entry::Alt {
                                pc: pc + 1,
                                s,
                                pkeep,
                            }),
                        }
                        let length = self.char_len_at(s);
                        if (s + length) as i64 > range || (!multiline && self.bytes[s] == b'\n') {
                            fail = true;
                            break;
                        }
                        s += length;
                    }
                    if !fail && !jumped {
                        pc += 1;
                    }
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
                        match self.check_match_cache(pc, s, false) {
                            CacheVerdict::Proceed => {
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
                            }
                            CacheVerdict::Fail => fail = true,
                            CacheVerdict::Jump(addr) => pc = addr,
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
                            self.stack.push(Entry::RepeatInc { index });
                        } else if count >= lower {
                            match self.check_match_cache(pc, s, true) {
                                CacheVerdict::Proceed => {
                                    self.stack.push(Entry::Alt {
                                        pc: pc + 1,
                                        s,
                                        pkeep,
                                    });
                                    pc = body_pc;
                                    self.stack.push(Entry::RepeatInc { index });
                                }
                                verdict => {
                                    // `MATCH_CACHE_HIT`: the count goes
                                    // back, since no entry will restore it.
                                    if let Entry::Repeat { count, .. } = &mut self.stack[index] {
                                        *count -= 1;
                                    }
                                    match verdict {
                                        CacheVerdict::Jump(addr) => pc = addr,
                                        _ => fail = true,
                                    }
                                }
                            }
                        } else {
                            pc = body_pc;
                            self.stack.push(Entry::RepeatInc { index });
                        }
                    } else if upper.is_none_or(|upper| count < upper) {
                        if count >= lower {
                            self.stack.push(Entry::RepeatInc { index });
                            match self.check_match_cache(pc, s, true) {
                                CacheVerdict::Proceed => {
                                    self.stack.push(Entry::Alt {
                                        pc: body_pc,
                                        s,
                                        pkeep,
                                    });
                                    pc += 1;
                                }
                                CacheVerdict::Fail => fail = true,
                                CacheVerdict::Jump(addr) => pc = addr,
                            }
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
                    self.void_until(VoidTo::Pos);
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
                    self.pop_until(PopTo::PosNot);
                    fail = true;
                }
                Op::PushStopBt => {
                    self.stack.push(Entry::StopBt);
                    pc += 1;
                }
                Op::PopStopBt => {
                    self.void_until(VoidTo::StopBt);
                    pc += 1;
                }
                Op::LookBehind(len) => match self.step_back(s, *len) {
                    Some(back) => {
                        s = back;
                        pc += 1;
                    }
                    None => fail = true,
                },
                Op::PushLookBehindNot { addr, len } => match self.step_back(s, *len) {
                    // Too short to hold the body: the negative lookbehind
                    // succeeds.
                    None => pc = *addr,
                    Some(back) => {
                        self.stack.push(Entry::LookBehindNot {
                            pc: *addr,
                            s,
                            pkeep,
                        });
                        s = back;
                        pc += 1;
                    }
                },
                Op::FailLookBehindNot => {
                    self.pop_until(PopTo::LookBehindNot);
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
                    } else if s_signed == range {
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
                        if s < self.bytes.len() {
                            // The next iteration, one character on; past
                            // the text it could only fail.
                            let next = s + self.char_len_at(s);
                            self.stack.push(Entry::Alt { pc, s: next, pkeep });
                        }
                        self.stack.push(Entry::Absent);
                        range = aend;
                        pc += 1;
                    }
                }
                Op::AbsentEnd => {
                    let sprev: i64 = match self.prev_head(s) {
                        Some(head) => head as i64,
                        None => -1,
                    };
                    if sprev < range {
                        range = sprev;
                    }
                    self.pop_until(PopTo::Absent);
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
                        self.count_fail_and_check();
                    }
                    _ => return Ok(None),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::regex::Regex;

    #[test]
    fn a_lookbehind_body_ending_before_the_start_is_no_match() {
        // `ﬁ` folds to two characters, so the lookbehind steps back two,
        // and its body then consumes the one `ﬁ`, ending a character
        // short of where the attempt began: Ruby answers nil.
        let regex = Regex::new(r"(?i)(?<=ﬁ)").unwrap();
        assert_eq!(regex.search("Θﬁµ"), None);
        let regex = Regex::new(r"(?i)(?<=ss)x").unwrap();
        assert_eq!(regex.search("ßx"), None);
    }
}
