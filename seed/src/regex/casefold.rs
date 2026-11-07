//! Case-insensitive matching's two expansions, from `regparse.c` and
//! `enc/unicode.c`: a class under `i` gains every code point that folds to
//! one of its members (`cclass_case_fold`), and a string under `i` offers,
//! at each position, the alternatives a fold could spell
//! (`get_case_fold_codes_by_str`).

use super::ast::{ClassNode, Node};
use super::unicode;
use super::unicode_tables::{CASE_UNFOLD_ONE, CASE_UNFOLD_THREE, CASE_UNFOLD_TWO};

/// `cclass_case_fold`: the class with its fold equivalents added, and, when
/// a member folds to several code points, an alternation with those spelled
/// out as strings compared folded.
pub fn class_case_fold(mut class: ClassNode) -> Node {
    let mut alternatives: Vec<Node> = Vec::new();
    apply_all_case_fold(&mut |from, to| apply(&mut class, &mut alternatives, from, to));
    if alternatives.is_empty() {
        Node::Class(class)
    } else {
        let mut branches = vec![Node::Class(class)];
        branches.extend(alternatives);
        Node::Alt(branches)
    }
}

/// `i_apply_case_fold`.
fn apply(class: &mut ClassNode, alternatives: &mut Vec<Node>, from: u32, to: &[u32]) {
    let add_allowed = match class.ascii_twin.as_ref() {
        None => false,
        Some(twin) => {
            if is_ascii(from) == is_ascii(to[0]) {
                true
            } else {
                twin.set.contains(from)
            }
        }
    };
    if to.len() == 1 {
        // In the raw set, before negation: a negated class gains the fold
        // too, so `[^a]` under `i` refuses `A` as well.
        if class.set.contains(from) && add_allowed {
            class.set.add_code(to[0]);
        }
    } else if class.matches(from) && !class.not {
        let chars: Vec<char> = to.iter().filter_map(|code| char::from_u32(*code)).collect();
        alternatives.push(Node::Str {
            chars,
            ignore_case: true,
        });
    }
}

fn is_ascii(code: u32) -> bool {
    code < 0x80
}

/// `onigenc_unicode_apply_all_case_fold`: every (code, fold) pair the
/// tables define, in Onigmo's order, which matters because the class grows
/// while the pairs are applied.
pub fn apply_all_case_fold(apply: &mut dyn FnMut(u32, &[u32])) {
    for (from, to) in CASE_UNFOLD_ONE {
        for (index, code) in to.iter().enumerate() {
            apply(*code, &[*from]);
            apply(*from, &[*code]);
            for earlier in &to[..index] {
                apply(*code, &[*earlier]);
                apply(*earlier, &[*code]);
            }
        }
    }
    for (key, to) in CASE_UNFOLD_TWO {
        for (index, code) in to.iter().enumerate() {
            apply(*code, key);
            for (other_index, other) in to.iter().enumerate() {
                if other_index != index {
                    apply(*code, &[*other]);
                }
            }
        }
    }
    for (key, to) in CASE_UNFOLD_THREE {
        for (index, code) in to.iter().enumerate() {
            apply(*code, key);
            for (other_index, other) in to.iter().enumerate() {
                if other_index != index {
                    apply(*code, &[*other]);
                }
            }
        }
    }
}

/// One way the text could spell the pattern at a position: `char_count`
/// pattern characters are covered by the text codes `codes`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoldItem {
    pub char_count: usize,
    pub codes: Vec<u32>,
}

/// `onigenc_unicode_get_case_fold_codes_by_str`: the alternatives at
/// `index` of `chars`, multi-character folds included.
pub fn case_fold_items(chars: &[char], index: usize) -> Vec<FoldItem> {
    let mut items: Vec<FoldItem> = Vec::new();
    let code = chars[index] as u32;
    let mut multi_char = true;
    match unicode::fold(code) {
        Some(folded) if folded.len() == 1 => {
            let target = folded[0];
            items.push(FoldItem {
                char_count: 1,
                codes: vec![target],
            });
            if let Some(unfolded) = unicode::unfold_one(target) {
                for candidate in unfolded {
                    if *candidate != code {
                        items.push(FoldItem {
                            char_count: 1,
                            codes: vec![*candidate],
                        });
                    }
                }
            }
        }
        Some(folded) => {
            // A multi-character fold: every spelling of each folded code,
            // and the other code points with the same fold.
            let mut spellings: Vec<Vec<u32>> = Vec::new();
            for folded_code in folded {
                let mut choices = vec![*folded_code];
                if let Some(unfolded) = unicode::unfold_one(*folded_code) {
                    choices.extend(unfolded.iter().copied());
                }
                spellings.push(choices);
            }
            if folded.len() == 2 {
                for first in &spellings[0] {
                    for second in &spellings[1] {
                        items.push(FoldItem {
                            char_count: 1,
                            codes: vec![*first, *second],
                        });
                    }
                }
                if let Some(others) = unicode::unfold_two([folded[0], folded[1]]) {
                    for other in others {
                        if *other != code {
                            items.push(FoldItem {
                                char_count: 1,
                                codes: vec![*other],
                            });
                        }
                    }
                }
            } else {
                for first in &spellings[0] {
                    for second in &spellings[1] {
                        for third in &spellings[2] {
                            items.push(FoldItem {
                                char_count: 1,
                                codes: vec![*first, *second, *third],
                            });
                        }
                    }
                }
                if let Some(others) = unicode::unfold_three([folded[0], folded[1], folded[2]]) {
                    for other in others {
                        if *other != code {
                            items.push(FoldItem {
                                char_count: 1,
                                codes: vec![*other],
                            });
                        }
                    }
                }
            }
            // A multi-char folded code is not the head of another.
            multi_char = false;
        }
        None => {
            if let Some(unfolded) = unicode::unfold_one(code) {
                for candidate in unfolded {
                    items.push(FoldItem {
                        char_count: 1,
                        codes: vec![*candidate],
                    });
                }
            }
        }
    }
    if multi_char && index + 1 < chars.len() {
        let mut codes = [code, 0, 0];
        codes[1] = single_fold(chars[index + 1] as u32);
        if let Some(others) = unicode::unfold_two([codes[0], codes[1]]) {
            for other in others {
                items.push(FoldItem {
                    char_count: 2,
                    codes: vec![*other],
                });
            }
        }
        if index + 2 < chars.len() {
            codes[2] = single_fold(chars[index + 2] as u32);
            if let Some(others) = unicode::unfold_three(codes) {
                for other in others {
                    items.push(FoldItem {
                        char_count: 3,
                        codes: vec![*other],
                    });
                }
            }
        }
    }
    items
}

/// The single-code fold of a code point, or itself.
fn single_fold(code: u32) -> u32 {
    match unicode::fold(code) {
        Some(folded) if folded.len() == 1 => folded[0],
        _ => code,
    }
}

#[cfg(test)]
mod tests {
    use super::super::ast::CharSet;
    use super::*;

    #[test]
    fn a_class_gains_its_fold_equivalents() {
        let mut class = ClassNode::new();
        class.set.add_code('a' as u32);
        class.ascii_twin = Some(Box::new(ClassNode {
            set: CharSet::from_ranges(&[('a' as u32, 'a' as u32)]),
            not: false,
            ascii_twin: None,
        }));
        let Node::Class(folded) = class_case_fold(class) else {
            panic!("a plain class stays a class");
        };
        assert!(folded.matches('A' as u32));
        assert!(!folded.matches('b' as u32));
    }

    #[test]
    fn a_sharp_s_spells_ss() {
        let items = case_fold_items(&['ß'], 0);
        assert!(items.contains(&FoldItem {
            char_count: 1,
            codes: vec![0x73, 0x73]
        }));
        let items = case_fold_items(&['s', 's'], 0);
        assert!(items.contains(&FoldItem {
            char_count: 2,
            codes: vec![0xdf]
        }));
    }
}
