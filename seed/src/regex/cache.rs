//! The match cache: the memoization Ruby's `regexec.c` added in 3.3, which
//! turns a backtracking blowup into linear work for the patterns it covers.
//! A "cache opcode" is a branching instruction: a `Push`, a `Repeat` that
//! may run zero times, a `RepeatInc` with room to run again. A "cache point"
//! is one such opcode at one iteration count of the repeat around it. A
//! "match cache point" is a cache point at one text position. Once a search
//! has failed often enough (the text's length times the number of cache
//! opcodes, counted across its attempts), every match cache point that
//! backtracking passes over is recorded in a bit array, and reaching one
//! again fails at once. Inside a lookaround or an atomic group a point takes
//! two bits, the second recording that the body went on to succeed from
//! there, so a second visit can skip to the body's end instead.
//!
//! Patterns the cache cannot cover run without it, as they do in Ruby: a
//! backreference, a subexpression call, a conditional, the absence
//! operator, a counted repeat inside another, a capture inside a
//! lookaround, a lookaround inside an atomic group or the other way round.

use super::compile::Op;

/// One cacheable instruction (Onigmo's `OnigCacheOpcode`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CacheOpcode {
    pub addr: usize,
    /// Its first cache point; relative to the repeat around it, if any.
    pub cache_point: usize,
    pub outer_repeat: Option<usize>,
    pub num_cache_points_at_outer_repeat: usize,
    pub num_cache_points_in_outer_repeat: usize,
    /// 0 outside any lookaround or atomic group; the depth of lookarounds
    /// around it; -1 inside an atomic group.
    pub lookaround_nesting: i32,
    /// Where the lookaround or atomic group around it ends.
    pub match_addr: Option<usize>,
}

/// The cacheable instructions of a program and how many cache points they
/// make between them; `None` for a program the cache cannot cover.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CacheOpcodes {
    pub opcodes: Vec<CacheOpcode>,
    pub num_cache_points: usize,
}

struct Impossible;

struct Walk<'a> {
    ops: &'a [Op],
    repeat_ranges: &'a [(u32, Option<u32>)],
    opcodes: Vec<CacheOpcode>,
}

/// How many cache points an opcode takes: two under a lookaround or an
/// atomic group, one otherwise.
fn width(lookaround_nesting: i32) -> usize {
    if lookaround_nesting == 0 { 1 } else { 2 }
}

impl Walk<'_> {
    fn record(
        &mut self,
        addr: usize,
        outer_repeat: Option<usize>,
        lookaround_nesting: i32,
        cache_point: &mut usize,
    ) {
        self.opcodes.push(CacheOpcode {
            addr,
            cache_point: *cache_point,
            outer_repeat,
            num_cache_points_at_outer_repeat: 0,
            num_cache_points_in_outer_repeat: 0,
            lookaround_nesting,
            match_addr: None,
        });
        *cache_point += width(lookaround_nesting);
    }

    /// `count_num_cache_opcodes_inner` and `init_cache_opcodes_inner` as
    /// one pass: walks from `pc` to the end of the enclosing construct,
    /// leaving `pc` just past its closing instruction.
    fn walk(
        &mut self,
        pc: &mut usize,
        current_repeat: Option<usize>,
        lookaround_nesting: i32,
        cache_point: &mut usize,
    ) -> Result<(), Impossible> {
        while *pc < self.ops.len() {
            let addr = *pc;
            *pc += 1;
            match &self.ops[addr] {
                Op::Push(_)
                | Op::PushOrJumpExact1 { .. }
                | Op::PushIfPeekNext { .. }
                | Op::AnyCharStar { .. } => {
                    self.record(addr, current_repeat, lookaround_nesting, cache_point)
                }
                Op::Repeat { id, .. } => {
                    if current_repeat.is_some() {
                        // A counted repeat inside another.
                        return Err(Impossible);
                    }
                    let (lower, upper) = self.repeat_ranges[*id];
                    if lower == 0 && upper == Some(0) {
                        // A body that never runs: walked for its checks,
                        // recorded nowhere.
                        let mark = self.opcodes.len();
                        let mut unused = 0;
                        self.walk(pc, Some(*id), lookaround_nesting, &mut unused)?;
                        self.opcodes.truncate(mark);
                        continue;
                    }
                    if lower == 0 {
                        self.record(addr, current_repeat, lookaround_nesting, cache_point);
                    }
                    let at_repeat = *cache_point;
                    let first = self.opcodes.len();
                    let mut in_repeat = 0;
                    self.walk(pc, Some(*id), lookaround_nesting, &mut in_repeat)?;
                    if upper.is_none_or(|upper| lower < upper) {
                        // Ruby records the repeat's end under the repeat's
                        // own address (`pbegin` is still the `OP_REPEAT`
                        // when `INC_CACHE_OPCODES` runs), so the end is
                        // never found by the lookup and never memoizes; its
                        // points stay reserved. Kept as Ruby has it: the
                        // end's point would ignore the iteration count, and
                        // memoizing it would turn a lazy `b{0,4}?` wrong.
                        self.record(addr, current_repeat, lookaround_nesting, cache_point);
                        *cache_point -= width(lookaround_nesting);
                    }
                    let bounds = match upper {
                        None => 1,
                        Some(upper) => (upper - lower) as usize,
                    };
                    *cache_point += in_repeat * lower as usize
                        + (in_repeat + width(lookaround_nesting)) * bounds;
                    for opcode in &mut self.opcodes[first..] {
                        opcode.num_cache_points_at_outer_repeat = at_repeat;
                        opcode.num_cache_points_in_outer_repeat = in_repeat;
                    }
                }
                Op::RepeatInc { id, by_stack, .. } => {
                    if *by_stack || Some(*id) != current_repeat {
                        // A repeat found by the stack, or a lone end.
                        return Err(Impossible);
                    }
                    return Ok(());
                }
                Op::PushPos | Op::PushPosNot(_) | Op::PushLookBehindNot { .. } => {
                    if lookaround_nesting < 0 {
                        // A lookaround inside an atomic group.
                        return Err(Impossible);
                    }
                    self.enclosed(pc, current_repeat, lookaround_nesting + 1, cache_point)?;
                }
                Op::PushStopBt => {
                    if lookaround_nesting != 0 {
                        // An atomic group inside a lookaround or another.
                        return Err(Impossible);
                    }
                    self.enclosed(pc, current_repeat, -1, cache_point)?;
                }
                Op::PopPos | Op::FailPos | Op::FailLookBehindNot | Op::PopStopBt => {
                    return Ok(());
                }
                Op::MemStart { .. } | Op::MemEnd { .. } => {
                    if lookaround_nesting != 0 {
                        // A capture inside a lookaround.
                        return Err(Impossible);
                    }
                }
                Op::BackRef { .. }
                | Op::BackRefLevel { .. }
                | Op::PushAbsentPos
                | Op::Absent(_)
                | Op::AbsentEnd
                | Op::Call(_)
                | Op::Return
                | Op::Condition { .. } => return Err(Impossible),
                _ => {}
            }
        }
        Ok(())
    }

    /// A lookaround's or an atomic group's body: walked at its nesting,
    /// and the opcodes inside learn where the body ends.
    fn enclosed(
        &mut self,
        pc: &mut usize,
        current_repeat: Option<usize>,
        lookaround_nesting: i32,
        cache_point: &mut usize,
    ) -> Result<(), Impossible> {
        let first = self.opcodes.len();
        self.walk(pc, current_repeat, lookaround_nesting, cache_point)?;
        let match_addr = *pc - 1;
        for opcode in &mut self.opcodes[first..] {
            if opcode.match_addr.is_none() {
                opcode.match_addr = Some(match_addr);
            }
        }
        Ok(())
    }
}

/// `count_num_cache_opcodes` and `init_cache_opcodes`: the cacheable
/// instructions of `ops`, or `None` when the cache cannot cover them.
pub fn collect(ops: &[Op], repeat_ranges: &[(u32, Option<u32>)]) -> Option<CacheOpcodes> {
    let mut walk = Walk {
        ops,
        repeat_ranges,
        opcodes: Vec::new(),
    };
    let mut pc = 0;
    let mut cache_point = 0;
    walk.walk(&mut pc, None, 0, &mut cache_point).ok()?;
    if pc != ops.len() {
        return None;
    }
    Some(CacheOpcodes {
        opcodes: walk.opcodes,
        num_cache_points: cache_point,
    })
}

impl CacheOpcodes {
    /// `bsearch_cache_opcodes`, step for step: a repeat with room to run
    /// zero times has two entries under one address, and which one the
    /// search lands on is part of the behaviour.
    fn opcode_at(&self, addr: usize) -> Option<&CacheOpcode> {
        let mut low: i64 = 0;
        let mut high: i64 = self.opcodes.len() as i64 - 1;
        let mut middle: i64 = 0;
        while low <= high {
            middle = (low + high) / 2;
            let candidate = self.opcodes[middle as usize].addr;
            if candidate == addr {
                break;
            }
            if candidate < addr {
                low = middle + 1;
            } else {
                high = middle - 1;
            }
        }
        let opcode = self.opcodes.get(middle as usize)?;
        (opcode.addr == addr).then_some(opcode)
    }

    /// `find_cache_point`: the cache point of the instruction at `addr`
    /// under the current iteration count of the repeat around it, which
    /// `count_of` answers; `is_inc` says the instruction is the repeat's own
    /// end, whose count was just raised.
    pub fn find_cache_point(
        &self,
        repeat_ranges: &[(u32, Option<u32>)],
        addr: usize,
        is_inc: bool,
        count_of: impl Fn(usize) -> u32,
    ) -> Option<(usize, &CacheOpcode)> {
        let opcode = self.opcode_at(addr)?;
        let cache_point = opcode.cache_point as i64;
        let Some(outer) = opcode.outer_repeat else {
            return Some((opcode.cache_point, opcode));
        };
        let at_outer = opcode.num_cache_points_at_outer_repeat as i64;
        let in_outer = opcode.num_cache_points_in_outer_repeat as i64;
        let (lower, upper) = repeat_ranges[outer];
        let lower = i64::from(lower);
        let count = i64::from(count_of(outer)) - if is_inc { 1 } else { 0 };
        let point = if count < lower {
            at_outer + in_outer * count + cache_point
        } else if upper.is_none() {
            let is_inc_one = if is_inc { 1 } else { 0 };
            at_outer + in_outer * (lower - is_inc_one) + (1 - is_inc_one) + cache_point
        } else {
            at_outer + in_outer * (lower - 1) + (in_outer + 1) * (count - lower + 1) + cache_point
        };
        Some((point as usize, opcode))
    }
}

/// The byte and bit of a match cache point in the bit array.
pub fn match_cache_point(
    num_cache_points: usize,
    position: usize,
    cache_point: usize,
) -> (usize, u8) {
    let point = num_cache_points * position + cache_point;
    (point >> 3, 1 << (point & 7))
}

/// The bit array for a text of `length` bytes.
pub fn new_buffer(num_cache_points: usize, length: usize) -> Vec<u8> {
    let points = num_cache_points * (length + 1);
    let bytes = (points >> 3) + if points & 7 != 0 { 1 } else { 0 } + 1;
    vec![0; bytes]
}

pub fn is_memoized(buffer: &[u8], index: usize, mask: u8) -> bool {
    buffer[index] & mask != 0
}

/// `check_extended_match_cache_point`: the second bit, the one after
/// `mask`, which may sit in the next byte.
pub fn is_memoized_extended(buffer: &[u8], index: usize, mask: u8) -> bool {
    if mask & 0x80 != 0 {
        buffer[index + 1] & 0x01 != 0
    } else {
        buffer[index] & (mask << 1) != 0
    }
}

pub fn memoize(buffer: &mut [u8], index: usize, mask: u8) {
    buffer[index] |= mask;
}

/// `memoize_extended_match_cache_point`: both bits.
pub fn memoize_extended(buffer: &mut [u8], index: usize, mask: u8) {
    buffer[index] |= mask;
    if mask & 0x80 != 0 {
        buffer[index + 1] |= 0x01;
    } else {
        buffer[index] |= mask << 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::regex::Regex;

    fn opcodes_of(pattern: &str) -> Option<CacheOpcodes> {
        let regex = Regex::new(pattern).unwrap();
        collect(&regex.program.ops, &regex.program.repeat_ranges)
    }

    #[test]
    fn a_plain_alternation_has_one_cache_opcode_per_push() {
        let opcodes = opcodes_of("a|b|c").unwrap();
        assert_eq!(opcodes.opcodes.len(), 2);
        assert_eq!(opcodes.num_cache_points, 2);
    }

    #[test]
    fn a_counted_repeat_multiplies_its_body_points_by_its_bounds() {
        // `(?:a|b){2,4}`: one point inside, lower 2, bounds 2, and the
        // repeat's own end: 1 * 2 + (1 + 1) * 2 = 6.
        let opcodes = opcodes_of("(?:a|b){2,4}").unwrap();
        assert_eq!(opcodes.num_cache_points, 6);
    }

    #[test]
    fn a_repeats_end_is_recorded_under_the_repeats_address_as_ruby_has_it() {
        let opcodes = opcodes_of("b{0,4}?").unwrap();
        let addresses: Vec<usize> = opcodes.opcodes.iter().map(|opcode| opcode.addr).collect();
        assert_eq!(addresses, vec![addresses[0], addresses[0]]);
        assert_eq!(opcodes.num_cache_points, 5);
    }

    #[test]
    fn the_uncoverable_constructs_are_refused() {
        assert!(opcodes_of(r"(a)\1").is_none());
        assert!(opcodes_of(r"(?<n>a)\g<n>").is_none());
        assert!(opcodes_of(r"(?~abc)").is_none());
        assert!(opcodes_of(r"(?:(?:a|b){2,3}){2,3}").is_none());
        assert!(opcodes_of(r"(?=(a))").is_none());
        assert!(opcodes_of(r"(?>(?=a))").is_none());
        assert!(opcodes_of(r"(?=(?>a))").is_none());
    }

    #[test]
    fn the_extended_bit_crosses_into_the_next_byte() {
        let mut buffer = vec![0u8; 3];
        memoize_extended(&mut buffer, 0, 0x80);
        assert!(is_memoized(&buffer, 0, 0x80));
        assert!(is_memoized_extended(&buffer, 0, 0x80));
        assert_eq!(buffer, vec![0x80, 0x01, 0]);
    }

    #[test]
    fn linear_time_follows_rubys_tree_shapes() {
        // `a*b` is possessivized into `(?>a*)b`, which the cache covers;
        // inside an atomic group that nests one atomic in another, which
        // it does not. A loop split off a string's end (`bx*`) stays in
        // its own list, so it is never compared with what follows.
        assert!(Regex::new("a*b").unwrap().linear_time());
        assert!(!Regex::new("(?>a*b)").unwrap().linear_time());
        assert!(Regex::new("(?>bx*ß)").unwrap().linear_time());
        assert!(Regex::new("(?>(?:a\\d*)b)").unwrap().linear_time());
        assert!(!Regex::new("(?>(?:a)\\d*b)").unwrap().linear_time());
        assert!(!Regex::new(r"\X").unwrap().linear_time());
        assert!(!Regex::new(r"(a)\1").unwrap().linear_time());
    }

    #[test]
    fn a_catastrophic_pattern_finishes_in_linear_steps() {
        // Without the cache, `(a+)+$` on thirty a's and a b backtracks
        // through two to the thirty paths; with it, well under a million.
        let regex = Regex::new(r"(a+)+$").unwrap();
        let text = format!("{}b", "a".repeat(30));
        assert_eq!(regex.search_bounded(&text, 0, 1_000_000), Ok(None));
    }

    #[test]
    fn a_lazy_counted_repeat_keeps_its_count_out_of_the_cache() {
        // A fuzz case: the repeat's end memoized without its count would
        // answer 44-46 here, having failed from 42 under a higher count.
        let regex = Regex::new(r"b{0,4}?\Z").unwrap();
        let text = "baaabbbbbaaabcabaaaaaaaababbbaaaaabacaaacbbbbb";
        let found = regex.search(text).unwrap();
        assert_eq!(found.groups[0], Some((42, 46)));
    }
}
