//! What `onig_compile` does between the parse and the emit: the whole
//! pattern becomes group 0 when it is called; unnamed groups stop capturing
//! when named ones exist; subexpression calls resolve to their groups and
//! recursion is marked and checked; lookbehinds are checked for a fixed
//! length; backreferences and conditions are checked against the group
//! count; and each quantifier learns whether its body can match empty.

use super::ast::{AnchorKind, EmptyInfo, EncloseKind, Node, QuantifierNode};
use super::casefold;
use super::error::{self, RegexError};
use super::parser::Parsed;

/// The tree after setup, with what the compiler needs beside it.
#[derive(Debug)]
pub struct Prepared {
    pub root: Node,
    pub num_mem: usize,
    pub names: Vec<(String, Vec<usize>)>,
    pub num_call: usize,
    /// Per group, whether its start is kept on the stack and restored by
    /// backtracking; otherwise it is assigned in place and a failed path's
    /// value survives, which Ruby's answers depend on.
    pub bt_mem_start: Vec<bool>,
    /// Per group, whether its end is kept on the stack.
    pub bt_mem_end: Vec<bool>,
}

pub fn setup(parsed: Parsed) -> Result<Prepared, RegexError> {
    let Parsed {
        mut root,
        mut num_mem,
        num_named,
        num_call,
        mut names,
    } = parsed;

    if num_call > 0 {
        root = Node::enclose(
            EncloseKind::Memory {
                regnum: 0,
                named: false,
            },
            root,
        );
    }

    // Mixed named and unnamed groups: the unnamed ones stop capturing.
    if num_named > 0 {
        if num_named != num_mem {
            let mut map: Vec<usize> = vec![0; num_mem + 1];
            let mut counter = 0;
            root = disable_unnamed(root, &mut map, &mut counter);
            renumber(&mut root, &map)?;
            for (_, groups) in names.iter_mut() {
                for group in groups.iter_mut() {
                    *group = map[*group];
                }
            }
            num_mem = num_named;
        } else {
            numbered_ref_check(&root)?;
        }
    }

    let mut bt_mem_start = vec![false; num_mem + 1];
    let mut bt_mem_end = vec![false; num_mem + 1];
    {
        let mut state = SetupState {
            num_mem,
            num_named,
            names: &names,
            groups: Vec::new(),
            bt_mem_start: vec![false; num_mem + 1],
            bt_mem_end: vec![false; num_mem + 1],
        };
        if num_call > 0 {
            resolve_calls(&mut root, &mut state)?;
            // A called group keeps its start on the stack.
            mark_called_starts(&root, &mut state.bt_mem_start);
            // The bodies kept for reachability must hold the resolved calls.
            let mut resolved = Vec::new();
            collect_groups(&root, &mut resolved);
            state.groups = resolved;
            // Recursion: a called group whose body can reach a call to itself.
            let mut recursive = Vec::new();
            mark_recursion(&root, &mut recursive, &state);
            apply_recursion_marks(&mut root, &recursive);
            // The bodies kept for length questions must carry the marks too.
            let mut groups = Vec::new();
            collect_groups(&root, &mut groups);
            state.groups = groups;
            infinite_recursion_check(&root, &state)?;
        }
        setup_tree(&mut root, &mut state, Context::default())?;
        bt_mem_start = state.bt_mem_start;
        bt_mem_end = state.bt_mem_end;
    }
    Ok(Prepared {
        root,
        num_mem,
        names,
        num_call,
        bt_mem_start,
        bt_mem_end,
    })
}

/// `setup_subexp_call`'s side effect: every called group keeps its start
/// on the stack.
fn mark_called_starts(node: &Node, bt_mem_start: &mut [bool]) {
    match node {
        Node::List(parts) | Node::Alt(parts) => {
            for part in parts {
                mark_called_starts(part, bt_mem_start);
            }
        }
        Node::Quantifier(quantifier) => mark_called_starts(&quantifier.target, bt_mem_start),
        Node::Enclose(enclose) => {
            if let EncloseKind::Memory { regnum, .. } = enclose.kind
                && enclose.call_target
            {
                bt_mem_start[regnum] = true;
            }
            mark_called_starts(&enclose.target, bt_mem_start);
        }
        Node::Anchor(anchor) => {
            if let Some(target) = anchor.target.as_ref() {
                mark_called_starts(target, bt_mem_start);
            }
        }
        _ => {}
    }
}

// --- unnamed groups ----------------------------------------------------

/// `noname_disable_map`: unnamed capturing groups become their bodies, and
/// named ones are renumbered in order.
fn disable_unnamed(node: Node, map: &mut Vec<usize>, counter: &mut usize) -> Node {
    match node {
        Node::List(parts) => Node::List(
            parts
                .into_iter()
                .map(|part| disable_unnamed(part, map, counter))
                .collect(),
        ),
        Node::Alt(branches) => Node::Alt(
            branches
                .into_iter()
                .map(|branch| disable_unnamed(branch, map, counter))
                .collect(),
        ),
        Node::Quantifier(mut quantifier) => {
            let inner_target = std::mem::replace(&mut *quantifier.target, Node::Empty);
            let target = disable_unnamed(inner_target, map, counter);
            if let Node::Quantifier(inner) = target {
                // The group between two quantifiers is gone: reduce them.
                return super::parser::reduce_quantifiers(quantifier, inner);
            }
            quantifier.target = Box::new(target);
            Node::Quantifier(quantifier)
        }
        Node::Enclose(mut enclose) => {
            if let EncloseKind::Memory { regnum, named } = enclose.kind {
                if named {
                    *counter += 1;
                    map[regnum] = *counter;
                    enclose.kind = EncloseKind::Memory {
                        regnum: *counter,
                        named: true,
                    };
                } else if regnum != 0 {
                    return disable_unnamed(*enclose.target, map, counter);
                }
            }
            enclose.target = Box::new(disable_unnamed(*enclose.target, map, counter));
            Node::Enclose(enclose)
        }
        Node::Anchor(mut anchor) => {
            if let Some(target) = anchor.target.take() {
                anchor.target = Some(Box::new(disable_unnamed(*target, map, counter)));
            }
            Node::Anchor(anchor)
        }
        other => other,
    }
}

/// `renumber_by_map`: named references follow the groups; numbered ones
/// are refused.
fn renumber(node: &mut Node, map: &[usize]) -> Result<(), RegexError> {
    match node {
        Node::List(parts) | Node::Alt(parts) => {
            for part in parts {
                renumber(part, map)?;
            }
        }
        Node::Quantifier(quantifier) => renumber(&mut quantifier.target, map)?,
        Node::Enclose(enclose) => {
            if let EncloseKind::Condition { regnum, by_name } = enclose.kind {
                enclose.kind = EncloseKind::Condition {
                    regnum: map[regnum],
                    by_name,
                };
            }
            renumber(&mut enclose.target, map)?;
        }
        Node::BackRef(backref) => {
            if !backref.by_name {
                return Err(RegexError::new(error::NUMBERED_BACKREF_OR_CALL_NOT_ALLOWED));
            }
            for group in backref.groups.iter_mut() {
                *group = map[*group];
            }
        }
        Node::Anchor(anchor) => {
            if let Some(target) = anchor.target.as_mut() {
                renumber(target, map)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// `numbered_ref_check`.
fn numbered_ref_check(node: &Node) -> Result<(), RegexError> {
    match node {
        Node::List(parts) | Node::Alt(parts) => {
            for part in parts {
                numbered_ref_check(part)?;
            }
        }
        Node::Quantifier(quantifier) => numbered_ref_check(&quantifier.target)?,
        Node::Enclose(enclose) => numbered_ref_check(&enclose.target)?,
        Node::BackRef(backref) => {
            if !backref.by_name {
                return Err(RegexError::new(error::NUMBERED_BACKREF_OR_CALL_NOT_ALLOWED));
            }
        }
        Node::Anchor(anchor) => {
            if let Some(target) = anchor.target.as_ref() {
                numbered_ref_check(target)?;
            }
        }
        _ => {}
    }
    Ok(())
}

// --- calls ---------------------------------------------------------------

struct SetupState<'a> {
    num_mem: usize,
    num_named: usize,
    names: &'a [(String, Vec<usize>)],
    /// Group number to a clone of the group's body, for length and
    /// recursion questions about calls.
    groups: Vec<(usize, Node)>,
    /// Per group, whether its start is kept on the stack (`bt_mem_start`).
    bt_mem_start: Vec<bool>,
    /// Per group, whether its end is kept on the stack (`bt_mem_end`).
    bt_mem_end: Vec<bool>,
}

impl SetupState<'_> {
    fn group_body(&self, regnum: usize) -> Option<&Node> {
        self.groups
            .iter()
            .find(|(number, _)| *number == regnum)
            .map(|(_, body)| body)
    }
}

fn collect_groups(node: &Node, groups: &mut Vec<(usize, Node)>) {
    match node {
        Node::List(parts) | Node::Alt(parts) => {
            for part in parts {
                collect_groups(part, groups);
            }
        }
        Node::Quantifier(quantifier) => collect_groups(&quantifier.target, groups),
        Node::Enclose(enclose) => {
            if let EncloseKind::Memory { regnum, .. } = enclose.kind {
                groups.push((regnum, (*enclose.target).clone()));
            }
            collect_groups(&enclose.target, groups);
        }
        Node::Anchor(anchor) => {
            if let Some(target) = anchor.target.as_ref() {
                collect_groups(target, groups);
            }
        }
        _ => {}
    }
}

/// `setup_subexp_call`: every call gets its group number; the group is
/// marked as a call target.
fn resolve_calls(root: &mut Node, state: &mut SetupState) -> Result<(), RegexError> {
    let mut groups = Vec::new();
    collect_groups(root, &mut groups);
    state.groups = groups;
    let mut targets: Vec<usize> = Vec::new();
    resolve_calls_in(root, state, &mut targets)?;
    mark_called(root, &targets);
    Ok(())
}

fn resolve_calls_in(
    node: &mut Node,
    state: &SetupState,
    targets: &mut Vec<usize>,
) -> Result<(), RegexError> {
    match node {
        Node::List(parts) | Node::Alt(parts) => {
            for part in parts {
                resolve_calls_in(part, state, targets)?;
            }
        }
        Node::Quantifier(quantifier) => resolve_calls_in(&mut quantifier.target, state, targets)?,
        Node::Enclose(enclose) => resolve_calls_in(&mut enclose.target, state, targets)?,
        Node::Anchor(anchor) => {
            if let Some(target) = anchor.target.as_mut() {
                resolve_calls_in(target, state, targets)?;
            }
        }
        Node::Call(call) => {
            if call.by_number {
                // `\g<0>` is the whole pattern and is always allowed.
                if call.group != 0 && state.num_named > 0 {
                    return Err(RegexError::new(error::NUMBERED_BACKREF_OR_CALL_NOT_ALLOWED));
                }
                if call.group > state.num_mem {
                    return Err(RegexError::named(
                        error::UNDEFINED_GROUP_REFERENCE,
                        &call.name,
                    ));
                }
            } else {
                let found = state
                    .names
                    .iter()
                    .find(|(name, _)| *name == call.name)
                    .map(|(_, groups)| groups.clone());
                match found {
                    None => {
                        return Err(RegexError::named(
                            error::UNDEFINED_NAME_REFERENCE,
                            &call.name,
                        ));
                    }
                    Some(groups) if groups.len() > 1 => {
                        return Err(RegexError::named(
                            error::MULTIPLEX_DEFINITION_NAME_CALL,
                            &call.name,
                        ));
                    }
                    Some(groups) => call.group = groups[0],
                }
            }
            if state.group_body(call.group).is_none() {
                return Err(RegexError::named(
                    error::UNDEFINED_NAME_REFERENCE,
                    &call.name,
                ));
            }
            targets.push(call.group);
        }
        _ => {}
    }
    Ok(())
}

fn mark_called(node: &mut Node, targets: &[usize]) {
    match node {
        Node::List(parts) | Node::Alt(parts) => {
            for part in parts {
                mark_called(part, targets);
            }
        }
        Node::Quantifier(quantifier) => mark_called(&mut quantifier.target, targets),
        Node::Enclose(enclose) => {
            if let EncloseKind::Memory { regnum, .. } = enclose.kind
                && targets.contains(&regnum)
            {
                enclose.call_target = true;
            }
            mark_called(&mut enclose.target, targets);
        }
        Node::Anchor(anchor) => {
            if let Some(target) = anchor.target.as_mut() {
                mark_called(target, targets);
            }
        }
        _ => {}
    }
}

/// `subexp_recursive_check_trav` and `subexp_recursive_check`: a called
/// group is recursive when its body reaches a call back to it.
fn mark_recursion(node: &Node, recursive: &mut Vec<usize>, state: &SetupState) {
    match node {
        Node::List(parts) | Node::Alt(parts) => {
            for part in parts {
                mark_recursion(part, recursive, state);
            }
        }
        Node::Quantifier(quantifier) => mark_recursion(&quantifier.target, recursive, state),
        Node::Enclose(enclose) => {
            if let EncloseKind::Memory { regnum, .. } = enclose.kind
                && enclose.call_target
                && !recursive.contains(&regnum)
            {
                let mut seen = vec![regnum];
                if reaches_call_to(&enclose.target, regnum, &mut seen, state) {
                    recursive.push(regnum);
                }
            }
            mark_recursion(&enclose.target, recursive, state);
        }
        Node::Anchor(anchor) => {
            if let Some(target) = anchor.target.as_ref() {
                mark_recursion(target, recursive, state);
            }
        }
        _ => {}
    }
}

fn reaches_call_to(node: &Node, wanted: usize, seen: &mut Vec<usize>, state: &SetupState) -> bool {
    match node {
        Node::List(parts) | Node::Alt(parts) => parts
            .iter()
            .any(|part| reaches_call_to(part, wanted, seen, state)),
        Node::Quantifier(quantifier) => reaches_call_to(&quantifier.target, wanted, seen, state),
        Node::Enclose(enclose) => reaches_call_to(&enclose.target, wanted, seen, state),
        Node::Anchor(anchor) => anchor
            .target
            .as_ref()
            .is_some_and(|target| reaches_call_to(target, wanted, seen, state)),
        Node::Call(call) => {
            if call.group == wanted {
                return true;
            }
            if seen.contains(&call.group) {
                return false;
            }
            seen.push(call.group);
            match state.group_body(call.group) {
                Some(body) => reaches_call_to(body, wanted, seen, state),
                None => false,
            }
        }
        _ => false,
    }
}

fn apply_recursion_marks(node: &mut Node, recursive: &[usize]) {
    match node {
        Node::List(parts) | Node::Alt(parts) => {
            for part in parts {
                apply_recursion_marks(part, recursive);
            }
        }
        Node::Quantifier(quantifier) => apply_recursion_marks(&mut quantifier.target, recursive),
        Node::Enclose(enclose) => {
            if let EncloseKind::Memory { regnum, .. } = enclose.kind
                && recursive.contains(&regnum)
            {
                enclose.recursive = true;
            }
            apply_recursion_marks(&mut enclose.target, recursive);
        }
        Node::Anchor(anchor) => {
            if let Some(target) = anchor.target.as_mut() {
                apply_recursion_marks(target, recursive);
            }
        }
        _ => {}
    }
}

/// `subexp_inf_recursive_check_trav`: a recursive group whose body can
/// call itself again before consuming anything never ends.
fn infinite_recursion_check(node: &Node, state: &SetupState) -> Result<(), RegexError> {
    match node {
        Node::List(parts) | Node::Alt(parts) => {
            for part in parts {
                infinite_recursion_check(part, state)?;
            }
        }
        Node::Quantifier(quantifier) => infinite_recursion_check(&quantifier.target, state)?,
        Node::Enclose(enclose) => {
            if let EncloseKind::Memory { regnum, .. } = enclose.kind
                && enclose.recursive
            {
                // Onigmo refuses when the recursion is unconditional:
                // reached at the head, or reached on every path.
                let mut seen = Vec::new();
                if infinite_check(&enclose.target, regnum, true, &mut seen, state)
                    != Recursion::None
                {
                    return Err(RegexError::new(error::NEVER_ENDING_RECURSION));
                }
            }
            infinite_recursion_check(&enclose.target, state)?;
        }
        Node::Anchor(anchor) => {
            if let Some(target) = anchor.target.as_ref() {
                infinite_recursion_check(target, state)?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Recursion {
    None,
    Exists,
    Infinite,
}

/// `subexp_inf_recursive_check`: walking the body with `head` true while
/// nothing has had to be consumed yet.
fn infinite_check(
    node: &Node,
    wanted: usize,
    head: bool,
    seen: &mut Vec<usize>,
    state: &SetupState,
) -> Recursion {
    match node {
        Node::List(parts) => {
            let mut result = Recursion::None;
            let mut head = head;
            for part in parts {
                let answer = infinite_check(part, wanted, head, seen, state);
                if answer == Recursion::Infinite {
                    return answer;
                }
                if answer == Recursion::Exists {
                    result = Recursion::Exists;
                }
                if head && min_match_length(part, state, &mut Vec::new()) != 0 {
                    head = false;
                }
            }
            result
        }
        Node::Alt(branches) => {
            let mut all_exist = true;
            for branch in branches {
                let answer = infinite_check(branch, wanted, head, seen, state);
                if answer == Recursion::Infinite {
                    return answer;
                }
                if answer != Recursion::Exists {
                    all_exist = false;
                }
            }
            if all_exist {
                Recursion::Exists
            } else {
                Recursion::None
            }
        }
        Node::Quantifier(quantifier) => {
            let answer = infinite_check(&quantifier.target, wanted, head, seen, state);
            if answer == Recursion::Exists && quantifier.lower == 0 {
                Recursion::None
            } else {
                answer
            }
        }
        Node::Anchor(anchor) => match anchor.target.as_ref() {
            Some(target) => infinite_check(target, wanted, head, seen, state),
            None => Recursion::None,
        },
        Node::Call(call) => {
            if call.group == wanted {
                return if head {
                    Recursion::Infinite
                } else {
                    Recursion::Exists
                };
            }
            if seen.contains(&call.group) {
                return Recursion::None;
            }
            seen.push(call.group);
            let answer = match state.group_body(call.group) {
                Some(body) => infinite_check(body, wanted, head, seen, state),
                None => Recursion::None,
            };
            seen.pop();
            answer
        }
        Node::Enclose(enclose) => infinite_check(&enclose.target, wanted, head, seen, state),
        _ => Recursion::None,
    }
}

// --- lengths -------------------------------------------------------------

/// `get_min_match_length`, in characters.
fn min_match_length(node: &Node, state: &SetupState, visiting: &mut Vec<usize>) -> usize {
    match node {
        Node::Empty | Node::Anchor(_) | Node::BackRef(_) => 0,
        Node::Str { chars, .. } => chars.len(),
        Node::Class(_) | Node::Word { .. } | Node::AnyChar { .. } => 1,
        Node::List(parts) => parts
            .iter()
            .map(|part| min_match_length(part, state, visiting))
            .sum(),
        Node::Alt(branches) => branches
            .iter()
            .map(|branch| min_match_length(branch, state, visiting))
            .min()
            .unwrap_or(0),
        Node::Quantifier(quantifier) => {
            quantifier.lower as usize * min_match_length(&quantifier.target, state, visiting)
        }
        Node::Enclose(enclose) => match enclose.kind {
            EncloseKind::Absent => 0,
            EncloseKind::Memory { regnum, .. } => {
                if visiting.contains(&regnum) {
                    return 0;
                }
                visiting.push(regnum);
                let length = min_match_length(&enclose.target, state, visiting);
                visiting.pop();
                length
            }
            _ => min_match_length(&enclose.target, state, visiting),
        },
        Node::Call(call) => {
            if visiting.contains(&call.group) {
                return 0;
            }
            visiting.push(call.group);
            let length = match state.group_body(call.group) {
                Some(body) => min_match_length(body, state, visiting),
                None => 0,
            };
            visiting.pop();
            length
        }
    }
}

/// `get_char_length_tree`: a fixed length in characters, or `None`; the
/// top-level alternation may differ in length, answered separately.
#[derive(Debug, PartialEq, Eq)]
enum CharLength {
    Fixed(usize),
    Variable,
    TopAltVariable,
}

fn char_length(node: &Node, state: &SetupState, level: usize) -> CharLength {
    match node {
        Node::Empty | Node::Anchor(_) => CharLength::Fixed(0),
        Node::Str {
            chars, ignore_case, ..
        } => {
            // A string under `i` is folded in place before its length is
            // read, so `ŉ` counts as its fold `ʼn`, two characters.
            if *ignore_case {
                CharLength::Fixed(
                    chars
                        .iter()
                        .map(|character| super::unicode::fold_sequence(*character as u32).len())
                        .sum(),
                )
            } else {
                CharLength::Fixed(chars.len())
            }
        }
        Node::Class(_) | Node::Word { .. } | Node::AnyChar { .. } => CharLength::Fixed(1),
        Node::List(parts) => {
            let mut total = 0;
            for part in parts {
                match char_length(part, state, level + 1) {
                    CharLength::Fixed(length) => total += length,
                    other => return other,
                }
            }
            CharLength::Fixed(total)
        }
        Node::Alt(branches) => {
            let mut first: Option<usize> = None;
            let mut varies = false;
            for branch in branches {
                match char_length(branch, state, level + 1) {
                    CharLength::Fixed(length) => {
                        if let Some(known) = first {
                            if known != length {
                                varies = true;
                            }
                        } else {
                            first = Some(length);
                        }
                    }
                    other => return other,
                }
            }
            if varies {
                if level == 0 {
                    CharLength::TopAltVariable
                } else {
                    CharLength::Variable
                }
            } else {
                CharLength::Fixed(first.unwrap_or(0))
            }
        }
        Node::Quantifier(quantifier) => {
            if Some(quantifier.lower) == quantifier.upper {
                match char_length(&quantifier.target, state, level + 1) {
                    CharLength::Fixed(length) => {
                        CharLength::Fixed(length * quantifier.lower as usize)
                    }
                    other => other,
                }
            } else {
                CharLength::Variable
            }
        }
        Node::Call(call) => match state.group_body(call.group) {
            Some(body) => {
                if is_recursive_call(call.group, state) {
                    CharLength::Variable
                } else {
                    char_length(body, state, level + 1)
                }
            }
            None => CharLength::Variable,
        },
        Node::Enclose(enclose) => match enclose.kind {
            EncloseKind::Memory { .. }
            | EncloseKind::StopBacktrack
            | EncloseKind::Condition { .. } => char_length(&enclose.target, state, level + 1),
            EncloseKind::Option => char_length(&enclose.target, state, level + 1),
            EncloseKind::Absent => CharLength::Fixed(0),
        },
        Node::BackRef(_) => CharLength::Variable,
    }
}

fn is_recursive_call(regnum: usize, state: &SetupState) -> bool {
    let mut seen = vec![regnum];
    match state.group_body(regnum) {
        Some(body) => reaches_call_to(body, regnum, &mut seen, state),
        None => false,
    }
}

// --- the tree walk ----------------------------------------------------

#[derive(Clone, Copy, Default)]
struct Context {
    in_lookbehind: bool,
    /// Inside a group a call can reach again.
    in_recursive: bool,
    /// Inside a quantifier's body (Onigmo's `IN_REPEAT`).
    in_repeat: bool,
    /// Inside an alternation branch (`IN_ALT`).
    in_alt: bool,
    /// Inside a negative lookaround (`IN_NOT`).
    in_not: bool,
    /// Inside a quantifier whose bounds differ (`IN_VAR_REPEAT`).
    in_var_repeat: bool,
    /// Inside a group that is called (`IN_CALL`).
    in_call: bool,
}

/// `setup_tree`.
fn setup_tree(node: &mut Node, state: &mut SetupState, context: Context) -> Result<(), RegexError> {
    match node {
        Node::List(parts) => {
            for index in 0..parts.len() {
                setup_tree(&mut parts[index], state, context)?;
                if index > 0 {
                    let (before, after) = parts.split_at_mut(index);
                    next_setup(&mut before[index - 1], &after[0]);
                }
            }
        }
        Node::Alt(parts) => {
            for part in parts {
                setup_tree(
                    part,
                    state,
                    Context {
                        in_alt: true,
                        ..context
                    },
                )?;
            }
        }
        Node::BackRef(backref) => {
            for group in &backref.groups {
                if *group > state.num_mem {
                    return Err(RegexError::new(error::INVALID_BACKREF));
                }
                // A referred group keeps its start on the stack, and its
                // end too when the reference carries a recursion level.
                state.bt_mem_start[*group] = true;
                if backref.level.is_some() {
                    state.bt_mem_end[*group] = true;
                }
            }
        }
        Node::Quantifier(quantifier) => {
            let mut info = EmptyInfo::Never;
            if quantifier.upper.is_none_or(|upper| upper >= 1)
                && min_match_length(&quantifier.target, state, &mut Vec::new()) == 0
            {
                info = EmptyInfo::Plain;
                match memory_info(&quantifier.target, state, &mut Vec::new()) {
                    EmptyInfo::Captures => info = EmptyInfo::Captures,
                    EmptyInfo::Recursion => info = EmptyInfo::Recursion,
                    _ => {}
                }
            }
            quantifier.empty = info;
            quantifier.referred =
                quantifier.upper == Some(0) && holds_called_group(&quantifier.target);
            quantifier.in_repeat = context.in_repeat;
            setup_tree(
                &mut quantifier.target,
                state,
                Context {
                    in_repeat: true,
                    in_var_repeat: context.in_var_repeat
                        || Some(quantifier.lower) != quantifier.upper,
                    ..context
                },
            )?;
            if let Some(expanded) = expand_repeated_string(quantifier) {
                *node = expanded;
            } else if quantifier.greedy && quantifier.empty != EmptyInfo::Never {
                // `USE_OP_PUSH_OR_JUMP_EXACT`: a greedy loop over a body
                // that may be empty learns the literal its body opens
                // with, taking a nested loop's own.
                quantifier.head_exact = match quantifier.target.as_mut() {
                    Node::Quantifier(inner) => inner.head_exact.take(),
                    target => head_value_node(target, true).cloned().map(Box::new),
                };
            }
        }
        Node::Enclose(enclose) => {
            if let EncloseKind::Condition { regnum, by_name } = enclose.kind {
                if !by_name && state.num_named > 0 {
                    return Err(RegexError::new(error::NUMBERED_BACKREF_OR_CALL_NOT_ALLOWED));
                }
                if regnum > state.num_mem {
                    return Err(RegexError::new(error::INVALID_BACKREF));
                }
                if let Node::Alt(branches) = enclose.target.as_ref()
                    && branches.len() > 2
                {
                    return Err(RegexError::new(error::INVALID_CONDITION_PATTERN));
                }
            }
            let mut inner = context;
            if let EncloseKind::Memory { regnum, .. } = enclose.kind {
                // A group that backtracking may reach keeps its start on
                // the stack; the rest assign it directly, and the value
                // outlives a failed path, as Onigmo's does.
                if context.in_alt || context.in_not || context.in_var_repeat || context.in_call {
                    state.bt_mem_start[regnum] = true;
                }
                if enclose.call_target {
                    inner.in_call = true;
                }
                // A group inside a recursive group records its end by
                // recursion level too, so an outer instance wins.
                if enclose.recursive {
                    inner.in_recursive = true;
                } else if context.in_recursive {
                    enclose.recursive = true;
                }
            }
            setup_tree(&mut enclose.target, state, inner)?;
        }
        Node::Anchor(anchor) => match anchor.kind {
            AnchorKind::PrecRead | AnchorKind::PrecReadNot => {
                let negative = anchor.kind == AnchorKind::PrecReadNot;
                if let Some(target) = anchor.target.as_mut() {
                    setup_tree(
                        target,
                        state,
                        Context {
                            in_not: context.in_not || negative,
                            ..context
                        },
                    )?;
                }
            }
            AnchorKind::LookBehind | AnchorKind::LookBehindNot => {
                let negative = anchor.kind == AnchorKind::LookBehindNot;
                let target = anchor.target.as_mut().expect("a lookbehind has a body");
                if !allowed_in_lookbehind(target, negative) {
                    return Err(RegexError::new(error::INVALID_LOOK_BEHIND_PATTERN));
                }
                setup_tree(
                    target,
                    state,
                    Context {
                        in_lookbehind: true,
                        in_not: context.in_not || negative,
                        ..context
                    },
                )?;
                match char_length(target, state, 0) {
                    CharLength::Fixed(length) => anchor.char_len = length,
                    CharLength::Variable => {
                        return Err(RegexError::new(error::INVALID_LOOK_BEHIND_PATTERN));
                    }
                    CharLength::TopAltVariable => {
                        // (?<=A|B) becomes (?<=A)|(?<=B); (?<!A|B) becomes
                        // (?<!A)(?<!B).
                        let kind = anchor.kind;
                        let Node::Alt(branches) = (**target).clone() else {
                            unreachable!("a top-level variable length is an alternation");
                        };
                        let mut pieces = Vec::new();
                        for branch in branches {
                            let length = match char_length(&branch, state, 1) {
                                CharLength::Fixed(length) => length,
                                _ => {
                                    return Err(RegexError::new(
                                        error::INVALID_LOOK_BEHIND_PATTERN,
                                    ));
                                }
                            };
                            pieces.push(Node::Anchor(super::ast::AnchorNode {
                                kind,
                                ascii_range: false,
                                target: Some(Box::new(branch)),
                                char_len: length,
                            }));
                        }
                        *node = if negative {
                            Node::List(pieces)
                        } else {
                            Node::Alt(pieces)
                        };
                    }
                }
            }
            _ => {}
        },
        // A string under `i` becomes its case-fold expansion; one already
        // folded (a class's multi-character spelling) stays.
        Node::Str {
            chars,
            ignore_case: true,
            folded: false,
        } => {
            let chars = std::mem::take(chars);
            *node = casefold::expand_string(&chars, context.in_lookbehind);
        }
        _ => {}
    }
    Ok(())
}

/// `check_type_tree` with the lookbehind masks: captures are allowed in a
/// positive lookbehind only; nothing else beyond strings, classes,
/// quantifiers, calls, and the simple anchors.
fn allowed_in_lookbehind(node: &Node, negative: bool) -> bool {
    match node {
        Node::List(parts) | Node::Alt(parts) => parts
            .iter()
            .all(|part| allowed_in_lookbehind(part, negative)),
        Node::Quantifier(quantifier) => allowed_in_lookbehind(&quantifier.target, negative),
        Node::Enclose(enclose) => match enclose.kind {
            EncloseKind::Memory { .. } => {
                !negative && allowed_in_lookbehind(&enclose.target, negative)
            }
            EncloseKind::Option => allowed_in_lookbehind(&enclose.target, negative),
            _ => false,
        },
        Node::Anchor(anchor) => match anchor.kind {
            AnchorKind::PrecRead
            | AnchorKind::PrecReadNot
            | AnchorKind::EndBuf
            | AnchorKind::SemiEndBuf => false,
            AnchorKind::LookBehind | AnchorKind::LookBehindNot => anchor
                .target
                .as_ref()
                .is_some_and(|target| allowed_in_lookbehind(target, negative)),
            _ => true,
        },
        Node::BackRef(_) => false,
        _ => true,
    }
}

/// `quantifiers_memory_node_info`.
fn memory_info(node: &Node, state: &SetupState, visiting: &mut Vec<usize>) -> EmptyInfo {
    match node {
        Node::List(parts) | Node::Alt(parts) => {
            let mut result = EmptyInfo::Never;
            for part in parts {
                let answer = memory_info(part, state, visiting);
                if rank(answer) > rank(result) {
                    result = answer;
                }
            }
            result
        }
        Node::Call(call) => {
            if visiting.contains(&call.group) || is_recursive_call(call.group, state) {
                return EmptyInfo::Recursion;
            }
            visiting.push(call.group);
            let answer = match state.group_body(call.group) {
                Some(body) => memory_info(body, state, visiting),
                None => EmptyInfo::Never,
            };
            visiting.pop();
            answer
        }
        Node::Quantifier(quantifier) => {
            if quantifier.upper != Some(0) {
                memory_info(&quantifier.target, state, visiting)
            } else {
                EmptyInfo::Never
            }
        }
        Node::Enclose(enclose) => match enclose.kind {
            EncloseKind::Memory { .. } => EmptyInfo::Captures,
            _ => memory_info(&enclose.target, state, visiting),
        },
        _ => EmptyInfo::Never,
    }
}

fn rank(info: EmptyInfo) -> u8 {
    match info {
        EmptyInfo::Never => 0,
        EmptyInfo::Plain => 1,
        EmptyInfo::Captures => 2,
        EmptyInfo::Recursion => 3,
    }
}

fn holds_called_group(node: &Node) -> bool {
    match node {
        Node::List(parts) | Node::Alt(parts) => parts.iter().any(holds_called_group),
        Node::Quantifier(quantifier) => holds_called_group(&quantifier.target),
        Node::Enclose(enclose) => enclose.call_target || holds_called_group(&enclose.target),
        Node::Anchor(anchor) => anchor
            .target
            .as_ref()
            .is_some_and(|target| holds_called_group(target)),
        _ => false,
    }
}

/// `next_setup`: what a node learns from the one after it in a sequence.
/// A greedy open loop followed by a literal records the literal's first
/// byte, so it pushes an alternative only where that byte is next; and
/// `a*b` becomes `(?>a*)b` when the loop's body and what follows can
/// never open with the same character, since backtracking into the loop
/// could then never help (Onigmo's automatic possessivation). A capturing
/// group that is not called is looked through.
fn next_setup(node: &mut Node, next: &Node) {
    match node {
        Node::Quantifier(quantifier) => {
            if !quantifier.greedy || quantifier.upper.is_some() {
                return;
            }
            if let Some(Node::Str { chars, .. }) = head_value_node(next, true) {
                let first = chars[0].len_utf8();
                let mut bytes = [0u8; 4];
                chars[0].encode_utf8(&mut bytes);
                if first > 0 && bytes[0] != 0 {
                    quantifier.next_head_byte = Some(bytes[0]);
                }
            }
            if quantifier.lower > 1 || !is_simple(&quantifier.target) {
                return;
            }
            let Some(body_head) = head_value_node(&quantifier.target, false) else {
                return;
            };
            let Some(next_head) = head_value_node(next, false) else {
                return;
            };
            if is_not_included(body_head, next_head) {
                let inner = std::mem::replace(node, Node::Empty);
                *node = Node::Enclose(super::ast::EncloseNode {
                    kind: EncloseKind::StopBacktrack,
                    target: Box::new(inner),
                    call_target: false,
                    recursive: false,
                    char_len: None,
                });
            }
        }
        Node::Enclose(enclose)
            if matches!(enclose.kind, EncloseKind::Memory { .. }) && !enclose.call_target =>
        {
            next_setup(&mut enclose.target, next);
        }
        _ => {}
    }
}

/// `IS_NODE_TYPE_SIMPLE`.
fn is_simple(node: &Node) -> bool {
    matches!(
        node,
        Node::Str { .. }
            | Node::Class(_)
            | Node::Word { .. }
            | Node::AnyChar { .. }
            | Node::BackRef(_)
    )
}

/// `get_head_value_node`: the node a match of `node` must open with — a
/// string, or with `exact` off a class or `\w` too — when the tree says
/// so; a case-insensitive string is not exact.
fn head_value_node(node: &Node, exact: bool) -> Option<&Node> {
    match node {
        Node::Word { .. } | Node::Class(_) => {
            if exact {
                None
            } else {
                Some(node)
            }
        }
        Node::List(parts) => parts
            .first()
            .and_then(|first| head_value_node(first, exact)),
        Node::Str {
            chars, ignore_case, ..
        } => {
            if chars.is_empty() {
                None
            } else if !exact || !*ignore_case {
                Some(node)
            } else {
                None
            }
        }
        Node::Quantifier(quantifier) => {
            if quantifier.lower == 0 {
                return None;
            }
            match &quantifier.head_exact {
                Some(head) => Some(head),
                None => head_value_node(&quantifier.target, exact),
            }
        }
        Node::Enclose(enclose) => match enclose.kind {
            EncloseKind::Absent => None,
            _ => head_value_node(&enclose.target, exact),
        },
        Node::Anchor(anchor) if anchor.kind == AnchorKind::PrecRead => anchor
            .target
            .as_ref()
            .and_then(|target| head_value_node(target, exact)),
        _ => None,
    }
}

fn is_word_code(code: u32, ascii_range: bool) -> bool {
    if ascii_range && code >= 0x80 {
        return false;
    }
    super::unicode::ctype_contains(super::unicode::CTYPE_WORD, code)
}

/// `is_not_included`: whether no character can open both `x` and `y`,
/// read off the two heads as Onigmo reads them: a byte at a time over the
/// bitsets, so a class with ranges beyond them is never certain.
fn is_not_included(x: &Node, y: &Node) -> bool {
    match (x, y) {
        (
            Node::Word {
                not: x_not,
                ascii_range: x_ascii,
            },
            Node::Word {
                not: y_not,
                ascii_range: y_ascii,
            },
        ) => x_not != y_not && x_ascii == y_ascii,
        (Node::Word { .. }, Node::Class(_)) | (Node::Word { .. }, Node::Str { .. }) => {
            is_not_included(y, x)
        }
        (Node::Class(class), Node::Word { not, ascii_range }) => {
            if !*not {
                if class.mbuf.is_some() || class.not {
                    return false;
                }
                (0..256u32).all(|byte| !class.bit(byte) || !is_word_code(byte, *ascii_range))
            } else {
                if class.mbuf.is_some() {
                    return false;
                }
                (0..256u32)
                    .all(|byte| is_word_code(byte, *ascii_range) || class.bit(byte) == class.not)
            }
        }
        (Node::Class(x_class), Node::Class(y_class)) => {
            for byte in 0..256u32 {
                if x_class.bit(byte) != x_class.not && y_class.bit(byte) != y_class.not {
                    return false;
                }
            }
            (x_class.mbuf.is_none() && !x_class.not) || (y_class.mbuf.is_none() && !y_class.not)
        }
        (Node::Class(_), Node::Str { .. }) => is_not_included(y, x),
        (Node::Str { chars, .. }, Node::Word { not, ascii_range }) => {
            if chars.is_empty() {
                return false;
            }
            let word = is_word_code(chars[0] as u32, *ascii_range);
            if word { *not } else { !*not }
        }
        (Node::Str { chars, .. }, Node::Class(class)) => {
            if chars.is_empty() {
                return false;
            }
            !class.matches(chars[0] as u32)
        }
        (
            Node::Str {
                chars: x_chars,
                ignore_case: x_ignore_case,
                ..
            },
            Node::Str {
                chars: y_chars,
                ignore_case: y_ignore_case,
                ..
            },
        ) => {
            if x_chars.is_empty() || *x_ignore_case || *y_ignore_case {
                return false;
            }
            let x_bytes: String = x_chars.iter().collect();
            let y_bytes: String = y_chars.iter().collect();
            let length = x_bytes.len().min(y_bytes.len());
            x_bytes.as_bytes()[..length] != y_bytes.as_bytes()[..length]
        }
        _ => false,
    }
}

/// The string expansion of `setup_tree`: `(?:abc){3,5}` becomes
/// `abcabcabc(?:abc){0,2}`, up to a hundred bytes. Answers the node that
/// replaces the quantifier, if any.
fn expand_repeated_string(quantifier: &QuantifierNode) -> Option<Node> {
    const EXPAND_STRING_MAX_LENGTH: usize = 100;
    let Node::Str {
        chars,
        ignore_case,
        folded,
    } = quantifier.target.as_ref()
    else {
        return None;
    };
    if quantifier.lower <= 1 {
        return None;
    }
    let length: usize = chars.iter().map(|character| character.len_utf8()).sum();
    let mut repeated = chars.clone();
    let mut copies = 1u32;
    while copies < quantifier.lower && (copies as usize + 1) * length <= EXPAND_STRING_MAX_LENGTH {
        repeated.extend(chars.iter().copied());
        copies += 1;
    }
    let repeated = Node::Str {
        chars: repeated,
        ignore_case: *ignore_case,
        folded: *folded,
    };
    let more = match quantifier.upper {
        None => true,
        Some(upper) => copies < upper,
    };
    if more {
        let mut rest = quantifier.clone();
        rest.lower -= copies;
        if let Some(upper) = rest.upper.as_mut() {
            *upper -= copies;
        }
        Some(Node::List(vec![repeated, Node::Quantifier(rest)]))
    } else {
        Some(repeated)
    }
}
