//! Regex (ADR 0086): Ruby's literals over Onigmo's pattern language, and a
//! match that is a value. The engine sits behind this one module — today
//! oniai, a pure-Rust Onigmo (#212) — so that a Portland program never
//! sees which code runs its pattern, and the choice can change.
//!
//! Positions a match answers count graphemes (ADR 0038), translated here
//! from the engine's byte offsets; inside the pattern the engine keeps
//! Onigmo's characters, so `.` is one codepoint and `\X` one grapheme.
//!
//! The engine's public API has no search-from-a-position and no list of
//! capture names, so `match(text, position)`, `index(regex, from)`, and
//! `rindex` search the text cut at that position — `\A`, `^`, `\b`, and a
//! lookbehind read the cut — and the names are read off the pattern's own
//! text here. Both are recorded in docs/ruby/regex.md and on #212.

use crate::value::Value;
use std::cell::OnceCell;
use std::fmt;
use std::rc::Rc;
use unicode_segmentation::UnicodeSegmentation;

/// A compiled pattern: its source as written, Ruby's three flags, and the
/// engine's compiled form. `anchored` is the same pattern pinned to the
/// start, built on first use for `start_with?` and `rindex`.
pub struct Pattern {
    pub source: String,
    pub ignore_case: bool,
    pub multiline: bool,
    pub extended: bool,
    compiled: oniai::Regex,
    anchored: OnceCell<oniai::Regex>,
    /// The capture names in group order; empty when the groups are numbered.
    pub names: Vec<String>,
    /// How many capture groups the pattern has.
    pub group_count: usize,
}

impl fmt::Debug for Pattern {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.inspect())
    }
}

/// One match: the pattern, the whole text it ran over, and the byte span
/// of the match and of each group (`None` for a group that took no part).
pub struct Found {
    pub pattern: Rc<Pattern>,
    pub text: String,
    pub spans: Vec<Option<(usize, usize)>>,
}

impl fmt::Debug for Found {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.inspect())
    }
}

/// Ruby's option bits.
const IGNORE_CASE: i64 = 1;
const EXTENDED: i64 = 2;
const MULTILINE: i64 = 4;

impl Pattern {
    /// Compile a source with its flags, refusing in the engine's words
    /// followed by the pattern, as Ruby's `RegexpError` reads.
    pub fn new(source: &str, ignore_case: bool, multiline: bool, extended: bool) -> Rc<Pattern> {
        let mut inline_flags = String::new();
        if ignore_case {
            inline_flags.push('i');
        }
        if multiline {
            inline_flags.push('m');
        }
        if extended {
            inline_flags.push('x');
        }
        let engine_source = if inline_flags.is_empty() {
            source.to_string()
        } else {
            format!("(?{inline_flags}){source}")
        };
        let compiled = match oniai::Regex::new(&engine_source) {
            Ok(compiled) => compiled,
            Err(error) => {
                crate::interpreter::refusal("arguments");
                let message = error.to_string();
                let message = message
                    .strip_prefix("parse error: ")
                    .or_else(|| message.strip_prefix("compile error: "))
                    .unwrap_or(&message);
                panic!("{message}: /{source}/")
            }
        };
        let (names, group_count) = capture_names(source);
        Rc::new(Pattern {
            source: source.to_string(),
            ignore_case,
            multiline,
            extended,
            compiled,
            anchored: OnceCell::new(),
            names,
            group_count,
        })
    }

    /// `Regexp.new(text, options)`: an integer of Ruby's bits, a string of
    /// the flag letters, or `true` for ignore-case.
    pub fn from_options(source: &str, options: Option<&Value>) -> Rc<Pattern> {
        let (ignore_case, multiline, extended) = match options {
            None | Some(Value::Nil) | Some(Value::Boolean(false)) => (false, false, false),
            Some(Value::Boolean(true)) => (true, false, false),
            Some(Value::Integer(bits)) => (
                bits & IGNORE_CASE != 0,
                bits & MULTILINE != 0,
                bits & EXTENDED != 0,
            ),
            Some(Value::String(letters)) => {
                for letter in letters.chars() {
                    if !matches!(letter, 'i' | 'm' | 'x') {
                        crate::interpreter::refusal("arguments");
                        panic!("unknown regexp option: {letters}");
                    }
                }
                (
                    letters.contains('i'),
                    letters.contains('m'),
                    letters.contains('x'),
                )
            }
            Some(other) => {
                crate::interpreter::refusal("arguments");
                panic!(
                    "Regexp.new takes an integer, a string, or true as its options, got {}",
                    other.shown()
                )
            }
        };
        Pattern::new(source, ignore_case, multiline, extended)
    }

    fn anchored(&self) -> &oniai::Regex {
        self.anchored.get_or_init(|| {
            let mut inline_flags = String::new();
            if self.ignore_case {
                inline_flags.push('i');
            }
            if self.multiline {
                inline_flags.push('m');
            }
            if self.extended {
                inline_flags.push('x');
            }
            let flagged = if inline_flags.is_empty() {
                String::new()
            } else {
                format!("(?{inline_flags})")
            };
            oniai::Regex::new(&format!("{flagged}\\A(?:{})", self.source))
                .expect("the pattern compiled once already")
        })
    }

    /// Ruby's option bits: `i` 1, `x` 2, `m` 4.
    pub fn options(&self) -> i64 {
        let mut bits = 0;
        if self.ignore_case {
            bits |= IGNORE_CASE;
        }
        if self.extended {
            bits |= EXTENDED;
        }
        if self.multiline {
            bits |= MULTILINE;
        }
        bits
    }

    /// The flag letters as Ruby orders them, `m` then `i` then `x`.
    fn flag_letters(&self) -> String {
        let mut letters = String::new();
        if self.multiline {
            letters.push('m');
        }
        if self.ignore_case {
            letters.push('i');
        }
        if self.extended {
            letters.push('x');
        }
        letters
    }

    /// `/source/flags`, an unescaped slash in the source written `\/`.
    pub fn inspect(&self) -> String {
        let mut body = String::new();
        let mut characters = self.source.chars();
        while let Some(character) = characters.next() {
            match character {
                '\\' => {
                    body.push('\\');
                    if let Some(escaped) = characters.next() {
                        body.push(escaped);
                    }
                }
                '/' => body.push_str("\\/"),
                other => body.push(other),
            }
        }
        format!("/{body}/{}", self.flag_letters())
    }

    /// `(?mix-mix:source)`: the flags on, a dash, the flags off, as Ruby
    /// spells a pattern for embedding in another.
    pub fn to_s(&self) -> String {
        let on = self.flag_letters();
        let mut off = String::new();
        if !self.multiline {
            off.push('m');
        }
        if !self.ignore_case {
            off.push('i');
        }
        if !self.extended {
            off.push('x');
        }
        if off.is_empty() {
            format!("(?{on}:{})", self.source)
        } else {
            format!("(?{on}-{off}:{})", self.source)
        }
    }

    /// The first match in `text`, as a value.
    pub fn search(self: &Rc<Self>, text: &str) -> Option<Rc<Found>> {
        self.compiled
            .captures(text)
            .map(|captures| self.found(text, 0, &captures))
    }

    /// The first match in `text` at or after a byte offset — the text cut
    /// there, as the module comment records.
    pub fn search_from(self: &Rc<Self>, text: &str, from: usize) -> Option<Rc<Found>> {
        self.compiled
            .captures(&text[from..])
            .map(|captures| self.found(text, from, &captures))
    }

    /// Whether the pattern matches at the very start of `text`.
    pub fn matches_start(&self, text: &str) -> bool {
        self.anchored().is_match(text)
    }

    /// Every match in `text`, non-overlapping, an empty match advancing one
    /// character — Ruby's `scan` positions.
    pub fn all(self: &Rc<Self>, text: &str) -> Vec<Rc<Found>> {
        self.compiled
            .captures_iter(text)
            .map(|captures| self.found(text, 0, &captures))
            .collect()
    }

    fn found(self: &Rc<Self>, text: &str, offset: usize, captures: &oniai::Captures) -> Rc<Found> {
        let spans = (0..=self.group_count)
            .map(|index| {
                captures
                    .get(index)
                    .map(|group| (group.start() + offset, group.end() + offset))
            })
            .collect();
        Rc::new(Found {
            pattern: Rc::clone(self),
            text: text.to_string(),
            spans,
        })
    }
}

impl Found {
    /// The byte span of a group, by number.
    fn span(&self, group: usize) -> Option<(usize, usize)> {
        self.spans.get(group).copied().flatten()
    }

    /// The text of a group, or nil where it took no part.
    pub fn group_value(&self, group: usize) -> Value {
        match self.span(group) {
            Some((start, end)) => Value::String(self.text[start..end].to_string()),
            None => Value::Nil,
        }
    }

    /// The whole match's text.
    pub fn matched(&self) -> &str {
        let (start, end) = self.span(0).expect("a match has a whole");
        &self.text[start..end]
    }

    pub fn pre_match(&self) -> &str {
        let (start, _) = self.span(0).expect("a match has a whole");
        &self.text[..start]
    }

    pub fn post_match(&self) -> &str {
        let (_, end) = self.span(0).expect("a match has a whole");
        &self.text[end..]
    }

    /// The group a name refers to, or none.
    pub fn group_named(&self, name: &str) -> Option<usize> {
        self.pattern
            .names
            .iter()
            .position(|candidate| candidate == name)
            .map(|position| position + 1)
    }

    /// A group by an index value: a number, counted from the end when
    /// negative; a name as a string or a symbol.
    pub fn group_of(&self, index: &Value) -> Result<Option<usize>, String> {
        match index {
            Value::Integer(number) => {
                let count = self.spans.len() as i64;
                let position = if *number < 0 { count + number } else { *number };
                if position < 0 || position >= count {
                    return Ok(None);
                }
                Ok(Some(position as usize))
            }
            Value::String(name) | Value::Symbol(name) => match self.group_named(name) {
                Some(group) => Ok(Some(group)),
                None => Err(format!("undefined group name reference: {name}")),
            },
            other => Err(format!(
                "no implicit conversion of {} into Integer",
                other.type_name()
            )),
        }
    }

    /// Where a byte offset falls, counted in graphemes.
    fn position(&self, byte: usize) -> i64 {
        self.text[..byte].graphemes(true).count() as i64
    }

    pub fn begin_of(&self, group: usize) -> Value {
        match self.span(group) {
            Some((start, _)) => Value::Integer(self.position(start)),
            None => Value::Nil,
        }
    }

    pub fn end_of(&self, group: usize) -> Value {
        match self.span(group) {
            Some((_, end)) => Value::Integer(self.position(end)),
            None => Value::Nil,
        }
    }

    pub fn captures(&self) -> Vec<Value> {
        (1..self.spans.len())
            .map(|group| self.group_value(group))
            .collect()
    }

    /// `#<MatchData "whole" 1:"a" 2:nil>`, names in place of numbers.
    pub fn inspect(&self) -> String {
        let mut rendered = format!(
            "#<MatchData {}",
            Value::String(self.matched().to_string()).inspect()
        );
        for group in 1..self.spans.len() {
            let label = match self.pattern.names.get(group - 1) {
                Some(name) => name.clone(),
                None => group.to_string(),
            };
            rendered.push_str(&format!(" {label}:{}", self.group_value(group).inspect()));
        }
        rendered.push('>');
        rendered
    }
}

/// The capture names in a pattern's own text, in group order, and the
/// number of groups: every `(?<name>` and `(?'name'`, or, when the pattern
/// names none, every plain `(` — Ruby's rule that a pattern with named
/// groups numbers only those.
fn capture_names(source: &str) -> (Vec<String>, usize) {
    let characters: Vec<char> = source.chars().collect();
    let mut names = Vec::new();
    let mut plain = 0usize;
    let mut index = 0;
    let mut in_class = 0usize;
    while index < characters.len() {
        match characters[index] {
            '\\' => index += 1,
            '[' => in_class += 1,
            ']' if in_class > 0 => in_class -= 1,
            '(' if in_class == 0 => {
                if characters.get(index + 1) != Some(&'?') {
                    plain += 1;
                } else if characters.get(index + 2) == Some(&'(') {
                    // A conditional, `(?(1)yes|no)`: the condition's own
                    // parentheses are not a group.
                    index += 3;
                    while index < characters.len() && characters[index] != ')' {
                        index += 1;
                    }
                } else if let Some(opener @ ('<' | '\'')) = characters.get(index + 2)
                    && !matches!(characters.get(index + 3), Some('=' | '!'))
                {
                    let closer = if *opener == '<' { '>' } else { '\'' };
                    let mut name = String::new();
                    let mut cursor = index + 3;
                    while cursor < characters.len() && characters[cursor] != closer {
                        name.push(characters[cursor]);
                        cursor += 1;
                    }
                    names.push(name);
                }
            }
            _ => {}
        }
        index += 1;
    }
    if names.is_empty() {
        (names, plain)
    } else {
        let count = names.len();
        (names, count)
    }
}

/// `Regexp.escape`: Ruby's set, the characters a pattern reads specially
/// plus space and the whitespace escapes.
pub fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '.' | '*' | '?' | '+' | '^' | '$' | '|' | '(' | ')' | '[' | ']' | '{' | '}' | '\\'
            | '-' | '#' => {
                escaped.push('\\');
                escaped.push(character);
            }
            ' ' => escaped.push_str("\\ "),
            '\n' => escaped.push_str("\\n"),
            '\t' => escaped.push_str("\\t"),
            '\r' => escaped.push_str("\\r"),
            '\x0c' => escaped.push_str("\\f"),
            '\x0b' => escaped.push_str("\\v"),
            other => escaped.push(other),
        }
    }
    escaped
}

/// `Regexp.union`: none is a pattern that never matches, one string is
/// that text escaped, one pattern is itself, and several are joined by
/// `|`, each pattern embedded in its `(?-mix:…)` form.
pub fn union(arguments: &[Value]) -> Rc<Pattern> {
    let members: Vec<&Value> = match arguments {
        [Value::Array(elements)] => elements.iter().collect(),
        _ => arguments.iter().collect(),
    };
    match members.as_slice() {
        [] => Pattern::new("(?!)", false, false, false),
        [Value::Regex(pattern)] => Rc::clone(pattern),
        _ => {
            let parts: Vec<String> = members
                .iter()
                .map(|member| match member {
                    Value::String(text) => escape(text),
                    Value::Symbol(name) => escape(name),
                    Value::Regex(pattern) => pattern.to_s(),
                    other => {
                        crate::interpreter::refusal("arguments");
                        panic!(
                            "no implicit conversion of {} into String",
                            other.type_name()
                        )
                    }
                })
                .collect();
            Pattern::new(&parts.join("|"), false, false, false)
        }
    }
}

/// Ruby's replacement escapes: `\0` and `\&` the match, `\1`–`\9` a group,
/// `\k<name>` a named group, `` \` `` and `\'` the text before and after,
/// `\\` a backslash, and any other `\x` as written.
pub fn expand_replacement(replacement: &str, found: &Found) -> String {
    let mut expanded = String::with_capacity(replacement.len());
    let mut characters = replacement.chars().peekable();
    while let Some(character) = characters.next() {
        if character != '\\' {
            expanded.push(character);
            continue;
        }
        match characters.next() {
            Some(digit @ '0'..='9') => {
                let group = digit as usize - '0' as usize;
                if let Value::String(text) = found.group_value(group) {
                    expanded.push_str(&text);
                }
            }
            Some('&') => expanded.push_str(found.matched()),
            Some('`') => expanded.push_str(found.pre_match()),
            Some('\'') => expanded.push_str(found.post_match()),
            Some('\\') => expanded.push('\\'),
            Some('k') if characters.peek() == Some(&'<') => {
                characters.next();
                let mut name = String::new();
                for next in characters.by_ref() {
                    if next == '>' {
                        break;
                    }
                    name.push(next);
                }
                match found.group_named(&name) {
                    Some(group) => {
                        if let Value::String(text) = found.group_value(group) {
                            expanded.push_str(&text);
                        }
                    }
                    None => {
                        crate::interpreter::refusal("arguments");
                        panic!("undefined group name reference: {name}")
                    }
                }
            }
            Some(other) => {
                expanded.push('\\');
                expanded.push(other);
            }
            None => expanded.push('\\'),
        }
    }
    expanded
}

/// A plain-string pattern's match, for `sub`/`gsub` with a string: the
/// match and no groups, so `\0` and `\&` read and `\1` is empty.
pub fn literal_found(text: &str, start: usize, end: usize) -> Found {
    Found {
        pattern: Pattern::new("", false, false, false),
        text: text.to_string(),
        spans: vec![Some((start, end))],
    }
}

/// `sub`/`gsub` with a replacement string.
pub fn replace(text: &str, pattern: &Rc<Pattern>, replacement: &str, all: bool) -> String {
    let mut result = String::new();
    let mut copied_to = 0;
    let matches = if all {
        pattern.all(text)
    } else {
        pattern.search(text).into_iter().collect()
    };
    for found in matches {
        let (start, end) = found.span(0).expect("a match has a whole");
        result.push_str(&text[copied_to..start]);
        result.push_str(&expand_replacement(replacement, &found));
        copied_to = end;
    }
    result.push_str(&text[copied_to..]);
    result
}

/// `scan`: each match's text, or an array of its groups when it has any.
pub fn scan_values(text: &str, pattern: &Rc<Pattern>) -> Vec<Value> {
    pattern
        .all(text)
        .iter()
        .map(|found| scan_item(found))
        .collect()
}

pub fn scan_item(found: &Found) -> Value {
    if found.spans.len() == 1 {
        Value::String(found.matched().to_string())
    } else {
        Value::array(found.captures())
    }
}

/// `split` by a pattern, Ruby's rules: an empty match right where a piece
/// begins splits nothing, each match's groups join the pieces, a positive
/// limit caps the pieces with the rest in the last, a negative one keeps
/// the trailing empties, and none drops them.
pub fn split(text: &str, pattern: &Rc<Pattern>, limit: i64) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }
    if limit == 1 {
        return vec![Value::String(text.to_string())];
    }
    let mut pieces: Vec<Value> = Vec::new();
    let mut piece_count = 0i64;
    let mut begin = 0usize;
    for found in pattern.all(text) {
        let (start, end) = found.span(0).expect("a match has a whole");
        if start == end && start == begin {
            continue;
        }
        pieces.push(Value::String(text[begin..start].to_string()));
        piece_count += 1;
        begin = end;
        for group in 1..found.spans.len() {
            if let Value::String(group_text) = found.group_value(group) {
                pieces.push(Value::String(group_text));
            }
        }
        if limit > 0 && piece_count >= limit - 1 {
            break;
        }
    }
    pieces.push(Value::String(text[begin..].to_string()));
    if limit == 0 {
        while matches!(pieces.last(), Some(Value::String(last)) if last.is_empty()) {
            pieces.pop();
        }
    }
    pieces
}

/// The byte offset where a character position starts, or none past the end.
fn byte_at(text: &str, position: i64) -> Option<usize> {
    let count = text.graphemes(true).count() as i64;
    let position = if position < 0 {
        count + position
    } else {
        position
    };
    if position < 0 || position > count {
        return None;
    }
    if position == count {
        return Some(text.len());
    }
    text.grapheme_indices(true)
        .nth(position as usize)
        .map(|(at, _)| at)
}

/// `index(pattern, from)`: the position of the first match at or after
/// `from`, as a maybe.
pub fn index_of(text: &str, pattern: &Rc<Pattern>, from: Option<i64>) -> Value {
    let start = match from {
        None => 0,
        Some(position) => match byte_at(text, position) {
            Some(byte) => byte,
            None => return Value::Nil,
        },
    };
    match pattern.search_from(text, start) {
        Some(found) => found.begin_of(0),
        None => Value::Nil,
    }
}

/// `rindex(pattern, from)`: the position of the last match starting at
/// or before `from`, as a maybe.
pub fn rindex_of(text: &str, pattern: &Rc<Pattern>, from: Option<i64>) -> Value {
    let limit = match from {
        None => text.len(),
        Some(position) => match byte_at(text, position) {
            Some(byte) => byte,
            None => return Value::Nil,
        },
    };
    let mut starts: Vec<usize> = text
        .grapheme_indices(true)
        .map(|(at, _)| at)
        .filter(|at| *at <= limit)
        .collect();
    if limit == text.len() {
        starts.push(text.len());
    }
    for start in starts.into_iter().rev() {
        if pattern.matches_start(&text[start..]) {
            return Value::Integer(text[..start].graphemes(true).count() as i64);
        }
    }
    Value::Nil
}

/// The last match starting at or before the text's end, for `rpartition`.
fn last_found(text: &str, pattern: &Rc<Pattern>) -> Option<Rc<Found>> {
    let mut starts: Vec<usize> = text.grapheme_indices(true).map(|(at, _)| at).collect();
    starts.push(text.len());
    for start in starts.into_iter().rev() {
        if pattern.matches_start(&text[start..]) {
            return pattern.search_from(text, start);
        }
    }
    None
}

/// The methods a pattern, a match, or a string given a pattern answers
/// without a block; `None` where the receiver and arguments are not this
/// module's, so the general arms and the table's refusal follow.
pub fn method(receiver: &Value, name: &str, arguments: &[Value]) -> Option<Value> {
    match (receiver, name, arguments) {
        (Value::Regex(pattern), "source", []) => Some(Value::String(pattern.source.clone())),
        (Value::Regex(pattern), "options", []) => Some(Value::Integer(pattern.options())),
        (Value::Regex(pattern), "casefold?", []) => Some(Value::Boolean(pattern.ignore_case)),
        (Value::Regex(pattern), "names", []) => Some(Value::array(
            pattern.names.iter().cloned().map(Value::String).collect(),
        )),
        (Value::Regex(pattern), "to_s", []) => Some(Value::String(pattern.to_s())),
        (Value::Regex(pattern), "inspect", []) => Some(Value::String(pattern.inspect())),
        (Value::Regex(pattern), "match" | "=~", [subject]) => {
            Some(match_value(pattern, subject, None))
        }
        (Value::Regex(pattern), "match", [subject, Value::Integer(from)]) => {
            Some(match_value(pattern, subject, Some(*from)))
        }
        (Value::Regex(pattern), "match?", [subject]) => Some(Value::Boolean(!matches!(
            match_value(pattern, subject, None),
            Value::Nil
        ))),
        (Value::Regex(pattern), "match?", [subject, Value::Integer(from)]) => Some(Value::Boolean(
            !matches!(match_value(pattern, subject, Some(*from)), Value::Nil),
        )),
        (Value::Regex(pattern), "!~", [subject]) => Some(Value::Boolean(matches!(
            match_value(pattern, subject, None),
            Value::Nil
        ))),

        (Value::MatchData(found), "captures", []) => Some(Value::array(found.captures())),
        (Value::MatchData(found), "named_captures", []) => Some(Value::hash(
            found
                .pattern
                .names
                .iter()
                .enumerate()
                .map(|(index, name)| (Value::String(name.clone()), found.group_value(index + 1)))
                .collect(),
        )),
        (Value::MatchData(found), "names", []) => Some(Value::array(
            found
                .pattern
                .names
                .iter()
                .cloned()
                .map(Value::String)
                .collect(),
        )),
        (Value::MatchData(found), "pre_match", []) => {
            Some(Value::String(found.pre_match().to_string()))
        }
        (Value::MatchData(found), "post_match", []) => {
            Some(Value::String(found.post_match().to_string()))
        }
        (Value::MatchData(found), "to_s", []) => Some(Value::String(found.matched().to_string())),
        (Value::MatchData(found), "inspect", []) => Some(Value::String(found.inspect())),
        (Value::MatchData(found), "string", []) => Some(Value::String(found.text.clone())),
        (Value::MatchData(found), "regexp", []) => Some(Value::Regex(Rc::clone(&found.pattern))),
        (Value::MatchData(found), "size" | "length", []) => {
            Some(Value::Integer(found.spans.len() as i64))
        }
        (Value::MatchData(found), "to_a", []) => Some(Value::array(
            (0..found.spans.len())
                .map(|group| found.group_value(group))
                .collect(),
        )),
        (Value::MatchData(found), "values_at", indices) => Some(Value::array(
            indices
                .iter()
                .map(|index| match group_or_refuse(found, index) {
                    Some(group) => found.group_value(group),
                    None => Value::Nil,
                })
                .collect(),
        )),
        (Value::MatchData(found), "begin", [index]) => Some(match group_or_refuse(found, index) {
            Some(group) => found.begin_of(group),
            None => {
                crate::interpreter::refusal("arguments");
                panic!("index {} out of matches", index.shown())
            }
        }),
        (Value::MatchData(found), "end", [index]) => Some(match group_or_refuse(found, index) {
            Some(group) => found.end_of(group),
            None => {
                crate::interpreter::refusal("arguments");
                panic!("index {} out of matches", index.shown())
            }
        }),
        (Value::MatchData(found), "offset", [index]) => Some(match group_or_refuse(found, index) {
            Some(group) => Value::array(vec![found.begin_of(group), found.end_of(group)]),
            None => {
                crate::interpreter::refusal("arguments");
                panic!("index {} out of matches", index.shown())
            }
        }),
        (Value::MatchData(found), "[]", [index]) => Some(match group_or_refuse(found, index) {
            Some(group) => found.group_value(group),
            None => Value::Nil,
        }),

        (Value::String(text), "=~", [Value::Regex(pattern)]) => {
            Some(found_value(pattern.search(text)))
        }
        (Value::String(_), "=~", [Value::String(_)]) => {
            crate::interpreter::refusal("arguments");
            panic!("type mismatch: String given")
        }
        (Value::String(text), "!~", [Value::Regex(pattern)]) => {
            Some(Value::Boolean(pattern.search(text).is_none()))
        }
        (Value::String(text), "match", [Value::Regex(pattern)]) => {
            Some(found_value(pattern.search(text)))
        }
        (Value::String(text), "match", [Value::Regex(pattern), Value::Integer(from)]) => {
            Some(match byte_at(text, *from) {
                Some(byte) => found_value(pattern.search_from(text, byte)),
                None => Value::Nil,
            })
        }
        (Value::String(text), "match", [Value::String(source)]) => {
            let pattern = Pattern::new(source, false, false, false);
            Some(found_value(pattern.search(text)))
        }
        (Value::String(text), "match?", [Value::Regex(pattern)]) => {
            Some(Value::Boolean(pattern.search(text).is_some()))
        }
        (Value::String(text), "match?", [Value::Regex(pattern), Value::Integer(from)]) => {
            Some(Value::Boolean(match byte_at(text, *from) {
                Some(byte) => pattern.search_from(text, byte).is_some(),
                None => false,
            }))
        }
        (Value::String(text), "match?", [Value::String(source)]) => {
            let pattern = Pattern::new(source, false, false, false);
            Some(Value::Boolean(pattern.search(text).is_some()))
        }
        (Value::String(text), "scan", [Value::Regex(pattern)]) => {
            Some(Value::array(scan_values(text, pattern)))
        }
        (Value::String(text), "split", [Value::Regex(pattern)]) => {
            Some(Value::array(split(text, pattern, 0)))
        }
        (Value::String(text), "split", [Value::Regex(pattern), Value::Integer(limit)]) => {
            Some(Value::array(split(text, pattern, *limit)))
        }
        (
            Value::String(text),
            "sub" | "gsub",
            [Value::Regex(pattern), Value::String(replacement)],
        ) => Some(Value::String(replace(
            text,
            pattern,
            replacement,
            name == "gsub",
        ))),
        (Value::String(text), "index", [Value::Regex(pattern)]) => {
            Some(index_of(text, pattern, None))
        }
        (Value::String(text), "index", [Value::Regex(pattern), Value::Integer(from)]) => {
            Some(index_of(text, pattern, Some(*from)))
        }
        (Value::String(text), "rindex", [Value::Regex(pattern)]) => {
            Some(rindex_of(text, pattern, None))
        }
        (Value::String(text), "rindex", [Value::Regex(pattern), Value::Integer(from)]) => {
            Some(rindex_of(text, pattern, Some(*from)))
        }
        (Value::String(text), "slice" | "[]", [Value::Regex(pattern)]) => {
            Some(match pattern.search(text) {
                Some(found) => Value::String(found.matched().to_string()),
                None => Value::Nil,
            })
        }
        (Value::String(text), "slice" | "[]", [Value::Regex(pattern), group]) => {
            Some(match pattern.search(text) {
                Some(found) => match group_or_refuse(&found, group) {
                    Some(group) => found.group_value(group),
                    None => Value::Nil,
                },
                None => Value::Nil,
            })
        }
        (Value::String(text), "start_with?", prefixes)
            if prefixes
                .iter()
                .any(|prefix| matches!(prefix, Value::Regex(_))) =>
        {
            Some(Value::Boolean(prefixes.iter().any(|prefix| match prefix {
                Value::Regex(pattern) => pattern.matches_start(text),
                Value::String(prefix) => text.starts_with(prefix.as_str()),
                other => {
                    crate::interpreter::refusal("arguments");
                    panic!(
                        "no implicit conversion of {} into String",
                        other.type_name()
                    )
                }
            })))
        }
        (Value::String(text), "partition", [Value::Regex(pattern)]) => {
            Some(match pattern.search(text) {
                Some(found) => Value::array(vec![
                    Value::String(found.pre_match().to_string()),
                    Value::String(found.matched().to_string()),
                    Value::String(found.post_match().to_string()),
                ]),
                None => Value::array(vec![
                    Value::String(text.clone()),
                    Value::String(String::new()),
                    Value::String(String::new()),
                ]),
            })
        }
        (Value::String(text), "rpartition", [Value::Regex(pattern)]) => {
            Some(match last_found(text, pattern) {
                Some(found) => Value::array(vec![
                    Value::String(found.pre_match().to_string()),
                    Value::String(found.matched().to_string()),
                    Value::String(found.post_match().to_string()),
                ]),
                None => Value::array(vec![
                    Value::String(String::new()),
                    Value::String(String::new()),
                    Value::String(text.clone()),
                ]),
            })
        }
        // A symbol answers the string questions by its name, as Ruby's does.
        (Value::Symbol(symbol_name), "match" | "=~" | "match?" | "!~", [Value::Regex(_)]) => {
            method(&Value::String(symbol_name.clone()), name, arguments)
        }
        _ => None,
    }
}

fn group_or_refuse(found: &Found, index: &Value) -> Option<usize> {
    match found.group_of(index) {
        Ok(group) => group,
        Err(message) => {
            crate::interpreter::refusal("arguments");
            panic!("{message}")
        }
    }
}

fn found_value(found: Option<Rc<Found>>) -> Value {
    match found {
        Some(found) => Value::MatchData(found),
        None => Value::Nil,
    }
}

/// A pattern's `match`, `=~`, and `match?` take a string or a symbol.
fn match_value(pattern: &Rc<Pattern>, subject: &Value, from: Option<i64>) -> Value {
    let text = match subject {
        Value::String(text) | Value::Symbol(text) => text,
        Value::Nil => return Value::Nil,
        other => {
            crate::interpreter::refusal("arguments");
            panic!(
                "no implicit conversion of {} into String",
                other.type_name()
            )
        }
    };
    match from {
        None => found_value(pattern.search(text)),
        Some(position) => match byte_at(text, position) {
            Some(byte) => found_value(pattern.search_from(text, byte)),
            None => Value::Nil,
        },
    }
}

/// `pattern === subject`: a match on a string or a symbol, false on
/// anything else, as Ruby's.
pub fn case_matches(pattern: &Rc<Pattern>, subject: &Value) -> bool {
    match subject {
        Value::String(text) | Value::Symbol(text) => pattern.search(text).is_some(),
        _ => false,
    }
}
