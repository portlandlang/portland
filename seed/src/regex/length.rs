//! `compile_length_tree`: how many bytes of Onigmo bytecode a node would
//! compile to. The number decides how a quantifier is emitted: a counted
//! or open repeat over a body short enough is unrolled into pushes rather
//! than a counting repeat (`QUANTIFIER_EXPAND_LIMIT_SIZE`), and the choice
//! shows in what the match cache can cover and when it turns on. The
//! program here is not Onigmo's bytes, so the sizes are computed from the
//! tree as `regcomp.c` would have them: `SIZE_OPCODE` 1, addresses and
//! lengths 4, group numbers 2, a bitset 32.

use super::ast::{AnchorKind, EmptyInfo, EncloseKind, Node, QuantifierNode};

const SIZE_OPCODE: usize = 1;
const SIZE_RELADDR: usize = 4;
const SIZE_ABSADDR: usize = 4;
const SIZE_LENGTH: usize = 4;
const SIZE_MEMNUM: usize = 2;
const SIZE_OPTION: usize = 4;
const SIZE_BITSET: usize = 32;
/// A code range buffer: its count, then two code points per range.
const SIZE_CODE_POINT: usize = 4;

const SIZE_OP_ANYCHAR_STAR: usize = SIZE_OPCODE;
const SIZE_OP_JUMP: usize = SIZE_OPCODE + SIZE_RELADDR;
pub const SIZE_OP_PUSH: usize = SIZE_OPCODE + SIZE_RELADDR;
const SIZE_OP_REPEAT_INC: usize = SIZE_OPCODE + SIZE_MEMNUM;
const SIZE_OP_PUSH_POS: usize = SIZE_OPCODE;
const SIZE_OP_PUSH_POS_NOT: usize = SIZE_OPCODE + SIZE_RELADDR;
const SIZE_OP_POP_POS: usize = SIZE_OPCODE;
const SIZE_OP_FAIL_POS: usize = SIZE_OPCODE;
const SIZE_OP_MEMORY_START: usize = SIZE_OPCODE + SIZE_MEMNUM;
const SIZE_OP_MEMORY_END: usize = SIZE_OPCODE + SIZE_MEMNUM;
const SIZE_OP_PUSH_STOP_BT: usize = SIZE_OPCODE;
const SIZE_OP_POP_STOP_BT: usize = SIZE_OPCODE;
const SIZE_OP_NULL_CHECK_START: usize = SIZE_OPCODE + SIZE_MEMNUM;
const SIZE_OP_NULL_CHECK_END: usize = SIZE_OPCODE + SIZE_MEMNUM;
const SIZE_OP_LOOK_BEHIND: usize = SIZE_OPCODE + SIZE_LENGTH;
const SIZE_OP_PUSH_LOOK_BEHIND_NOT: usize = SIZE_OPCODE + SIZE_RELADDR + SIZE_LENGTH;
const SIZE_OP_FAIL_LOOK_BEHIND_NOT: usize = SIZE_OPCODE;
const SIZE_OP_CALL: usize = SIZE_OPCODE + SIZE_ABSADDR;
const SIZE_OP_RETURN: usize = SIZE_OPCODE;
const SIZE_OP_CONDITION: usize = SIZE_OPCODE + SIZE_MEMNUM + SIZE_RELADDR;
const SIZE_OP_PUSH_ABSENT_POS: usize = SIZE_OPCODE;
const SIZE_OP_ABSENT: usize = SIZE_OPCODE + SIZE_RELADDR;
const SIZE_OP_ABSENT_END: usize = SIZE_OPCODE;

pub const QUANTIFIER_EXPAND_LIMIT_SIZE: usize = 50;

/// `IS_EXPAND_LIMIT_OK`: `tlen * n` fits the limit, without overflowing.
pub fn is_expand_limit_ok(tlen: usize, n: u32) -> bool {
    n == 0 || tlen <= QUANTIFIER_EXPAND_LIMIT_SIZE / n as usize
}

/// `\X` is, in Ruby's Onigmo, a tree of alternations and lookaheads built
/// at parse time, far past any expansion limit; here it is one instruction,
/// so it is given a size past the limit.
const GRAPHEME_CLUSTER_LENGTH: usize = 1000;

pub fn tree_length(node: &Node) -> usize {
    match node {
        Node::Empty => 0,
        Node::List(parts) => parts.iter().map(tree_length).sum(),
        Node::Alt(branches) => {
            let total: usize = branches.iter().map(tree_length).sum();
            total + (SIZE_OP_PUSH + SIZE_OP_JUMP) * (branches.len() - 1)
        }
        Node::Str {
            chars, ignore_case, ..
        } => string_length(chars, *ignore_case),
        Node::Class(class) => {
            let Some(ranges) = &class.mbuf else {
                return SIZE_OPCODE + SIZE_BITSET;
            };
            let head = if class.bitset_is_empty() {
                SIZE_OPCODE
            } else {
                SIZE_OPCODE + SIZE_BITSET
            };
            head + SIZE_LENGTH + SIZE_CODE_POINT + 2 * SIZE_CODE_POINT * ranges.len()
        }
        Node::Word { .. } | Node::AnyChar { .. } => SIZE_OPCODE,
        Node::GraphemeCluster => GRAPHEME_CLUSTER_LENGTH,
        Node::BackRef(backref) => {
            if backref.level.is_some() {
                SIZE_OPCODE
                    + SIZE_OPTION
                    + SIZE_LENGTH
                    + SIZE_LENGTH
                    + SIZE_MEMNUM * backref.groups.len()
            } else if backref.groups.len() == 1 {
                if !backref.ignore_case && backref.groups[0] <= 2 {
                    SIZE_OPCODE
                } else {
                    SIZE_OPCODE + SIZE_MEMNUM
                }
            } else {
                SIZE_OPCODE + SIZE_LENGTH + SIZE_MEMNUM * backref.groups.len()
            }
        }
        Node::Call(_) => SIZE_OP_CALL,
        Node::Quantifier(quantifier) => quantifier_length(quantifier),
        Node::Enclose(enclose) => {
            let tlen = tree_length(&enclose.target);
            match enclose.kind {
                EncloseKind::Option => tlen,
                EncloseKind::Memory { .. } => {
                    let mut length = SIZE_OP_MEMORY_START + tlen + SIZE_OP_MEMORY_END;
                    if enclose.call_target {
                        length += SIZE_OP_CALL + SIZE_OP_JUMP + SIZE_OP_RETURN;
                    }
                    length
                }
                EncloseKind::StopBacktrack => SIZE_OP_PUSH_STOP_BT + tlen + SIZE_OP_POP_STOP_BT,
                EncloseKind::Condition { .. } => {
                    let Node::Alt(branches) = enclose.target.as_ref() else {
                        return SIZE_OP_CONDITION + tlen + SIZE_OP_JUMP;
                    };
                    let yes = tree_length(&branches[0]);
                    let no = branches.get(1).map(tree_length).unwrap_or(0);
                    SIZE_OP_CONDITION + yes + SIZE_OP_JUMP + no
                }
                EncloseKind::Absent => {
                    SIZE_OP_PUSH_ABSENT_POS + SIZE_OP_ABSENT + tlen + SIZE_OP_ABSENT_END
                }
            }
        }
        Node::Anchor(anchor) => {
            let tlen = anchor
                .target
                .as_ref()
                .map(|target| tree_length(target))
                .unwrap_or(0);
            match anchor.kind {
                AnchorKind::PrecRead => SIZE_OP_PUSH_POS + tlen + SIZE_OP_POP_POS,
                AnchorKind::PrecReadNot => SIZE_OP_PUSH_POS_NOT + tlen + SIZE_OP_FAIL_POS,
                AnchorKind::LookBehind => SIZE_OP_LOOK_BEHIND + tlen,
                AnchorKind::LookBehindNot => {
                    SIZE_OP_PUSH_LOOK_BEHIND_NOT + tlen + SIZE_OP_FAIL_LOOK_BEHIND_NOT
                }
                _ => SIZE_OPCODE,
            }
        }
    }
}

/// `compile_length_string_node`: a string compiles to runs of characters
/// of one byte length each, an opcode per run, with a length field for a
/// run past the fixed-size opcodes; a folded string is one run compared
/// case-insensitively.
fn string_length(chars: &[char], folded: bool) -> usize {
    if chars.is_empty() {
        return 0;
    }
    let mut total = 0;
    let mut run_char_len = chars[0].len_utf8();
    let mut run_bytes = 0;
    for character in chars {
        let char_len = character.len_utf8();
        if char_len == run_char_len || folded {
            run_bytes += char_len;
        } else {
            total += run_length(run_char_len, run_bytes, folded);
            run_char_len = char_len;
            run_bytes = char_len;
        }
    }
    total + run_length(run_char_len, run_bytes, folded)
}

/// `add_compile_string_length` with `select_str_opcode`.
fn run_length(mb_len: usize, byte_len: usize, ignore_case: bool) -> usize {
    let str_len = byte_len.div_ceil(mb_len);
    let needs_length = if ignore_case {
        str_len != 1
    } else {
        match mb_len {
            1 => str_len > 5,
            2 => str_len > 3,
            _ => true,
        }
    };
    let mut length = SIZE_OPCODE;
    if !ignore_case && mb_len > 3 {
        // OP_EXACTMBN carries the character width too.
        length += SIZE_LENGTH;
    }
    if needs_length {
        length += SIZE_LENGTH;
    }
    length + byte_len
}

/// `compile_length_quantifier_node`.
fn quantifier_length(quantifier: &QuantifierNode) -> usize {
    let target = quantifier.target.as_ref();
    let tlen = tree_length(target);
    let infinite = quantifier.upper.is_none();
    let lower = quantifier.lower;
    if matches!(target, Node::AnyChar { .. }) && quantifier.greedy && infinite {
        return SIZE_OP_ANYCHAR_STAR + tlen * lower as usize;
    }
    let mod_tlen = if quantifier.empty != EmptyInfo::Never {
        tlen + SIZE_OP_NULL_CHECK_START + SIZE_OP_NULL_CHECK_END
    } else {
        tlen
    };
    if infinite && (lower <= 1 || is_expand_limit_ok(tlen, lower)) {
        let mut length = if lower == 1 && tlen > QUANTIFIER_EXPAND_LIMIT_SIZE {
            SIZE_OP_JUMP
        } else {
            tlen * lower as usize
        };
        if quantifier.greedy {
            length += SIZE_OP_PUSH + mod_tlen + SIZE_OP_JUMP;
        } else {
            length += SIZE_OP_JUMP + mod_tlen + SIZE_OP_PUSH;
        }
        return length;
    }
    let upper = quantifier.upper.unwrap_or(u32::MAX);
    if upper == 0 && quantifier.referred {
        return SIZE_OP_JUMP + tlen;
    }
    if !infinite
        && quantifier.greedy
        && (upper == 1 || is_expand_limit_ok(tlen + SIZE_OP_PUSH, upper))
    {
        return tlen * lower as usize + (SIZE_OP_PUSH + tlen) * (upper - lower) as usize;
    }
    if !quantifier.greedy && upper == 1 && lower == 0 {
        return SIZE_OP_PUSH + SIZE_OP_JUMP + tlen;
    }
    SIZE_OP_REPEAT_INC + mod_tlen + SIZE_OPCODE + SIZE_RELADDR + SIZE_MEMNUM
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_byte_runs_take_the_fixed_opcodes_up_to_five() {
        assert_eq!(string_length(&['a'], false), 2);
        assert_eq!(string_length(&['a'; 5], false), 6);
        assert_eq!(string_length(&['a'; 6], false), 11);
    }

    #[test]
    fn a_run_breaks_where_the_character_width_changes() {
        // `aあ`: EXACT1 (2) then EXACTMB3N (1 + 4 + 3).
        assert_eq!(string_length(&['a', 'あ'], false), 10);
    }

    #[test]
    fn a_folded_string_is_one_case_insensitive_run() {
        assert_eq!(string_length(&['a'], true), 2);
        assert_eq!(string_length(&['a', 'b'], true), 7);
        assert_eq!(string_length(&['a', 'あ'], true), 9);
    }
}
