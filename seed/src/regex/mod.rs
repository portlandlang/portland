//! Regex: a port of Onigmo, the engine Ruby runs, to the seed (ADR 0086,
//! #212). Ruby syntax, UTF-8 text, every construct Onigmo's `doc/RE` lists —
//! lookaround, atomic and possessive groups, backreferences with recursion
//! levels, subexpression calls, conditionals, the absence operator, Unicode
//! properties, full case folding, `\X` and `\R`. The answers are pinned byte
//! for byte against Ruby 4.0.9 over Onigmo's own test suite
//! (`seed/tests/fixtures/onigmo/`).
//!
//! The layers follow Onigmo's own: `preprocess` is CRuby's `re.c` pass that
//! rewrites `\u`, `\x`, and the control and meta escapes before the engine
//! sees the pattern; `parser` is `regparse.c`, pattern to tree; `setup` and
//! `compile` are `regcomp.c`, tree to a program; `vm` is `regexec.c`, a
//! backtracking matcher over the program; `unicode` and `casefold` are
//! `enc/unicode.c` over tables generated from Ruby's own copies.

pub mod ast;
pub mod casefold;
pub mod cclass;
pub mod compile;
pub mod error;
pub mod parser;
pub mod preprocess;
pub mod setup;
pub mod unicode;
pub mod unicode_tables;
pub mod vm;

pub use error::RegexError;

/// The compile-time options a Ruby literal's flags set: `i`, `x`, `m`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Options {
    /// `i`: case-insensitive, with full Unicode case folding.
    pub ignore_case: bool,
    /// `x`: extended form, whitespace and `#` comments ignored.
    pub extend: bool,
    /// `m`: `.` also matches a newline (Ruby's meaning of multiline).
    pub multiline: bool,
}

/// A compiled pattern.
#[derive(Debug)]
pub struct Regex {
    source: String,
    options: Options,
    program: compile::Program,
    /// Group names in definition order, each with its group numbers.
    names: Vec<(String, Vec<usize>)>,
}

/// Where a match and each of its groups landed, as byte offsets into the
/// text; group 0 is the whole match, and a group that took no part is `None`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Captures {
    pub groups: Vec<Option<(usize, usize)>>,
}

impl Regex {
    /// Compiles `pattern` as Ruby would `Regexp.new(pattern)`.
    pub fn new(pattern: &str) -> Result<Regex, RegexError> {
        Regex::with_options(pattern, Options::default())
    }

    /// Compiles `pattern` under the flags of a literal.
    pub fn with_options(pattern: &str, options: Options) -> Result<Regex, RegexError> {
        let unescaped = preprocess::preprocess(pattern, options.extend)?;
        let parse_options =
            parser::ParseOptions::ruby(options.ignore_case, options.extend, options.multiline);
        let parsed = parser::parse(&unescaped, parse_options)?;
        let prepared = setup::setup(parsed)?;
        let program = compile::compile(&prepared)?;
        Ok(Regex {
            source: pattern.to_string(),
            options,
            program,
            names: prepared.names,
        })
    }

    /// The pattern as written.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The options the pattern was compiled under.
    pub fn options(&self) -> Options {
        self.options
    }

    /// How many capture groups the pattern has.
    pub fn group_count(&self) -> usize {
        self.program.num_mem
    }

    /// The capture names in definition order, each with its group numbers;
    /// empty when the groups are numbered.
    pub fn names(&self) -> &[(String, Vec<usize>)] {
        &self.names
    }

    /// Searches `text` from its start for the leftmost match, as
    /// `Regexp#match` does.
    pub fn search(&self, text: &str) -> Option<Captures> {
        self.search_from(text, 0)
    }

    /// Searches `text` for the leftmost match starting at or after the byte
    /// offset `start`, as `Regexp#match(text, position)` does; the whole
    /// text stays in view, so `\A`, `^`, `\b`, and a lookbehind read what
    /// comes before `start`.
    pub fn search_from(&self, text: &str, start: usize) -> Option<Captures> {
        self.search_bounded(text, start, u64::MAX)
            .expect("an unbounded search cannot run out of budget")
    }

    /// `search_from` with a ceiling on the matcher's steps, refusing with
    /// Onigmo's `match-stack limit over` when it is reached; what a test
    /// harness uses so a pathological case cannot run away.
    pub fn search_bounded(
        &self,
        text: &str,
        start: usize,
        budget: u64,
    ) -> Result<Option<Captures>, RegexError> {
        let chars: Vec<char> = text.chars().collect();
        let mut byte_offsets: Vec<usize> = Vec::with_capacity(chars.len() + 1);
        let mut offset = 0;
        for character in &chars {
            byte_offsets.push(offset);
            offset += character.len_utf8();
        }
        byte_offsets.push(offset);
        let start_char = byte_offsets
            .iter()
            .position(|candidate| *candidate >= start)
            .unwrap_or(chars.len());
        let mut matcher = vm::Matcher::new(&self.program, &chars, text, &byte_offsets, budget);
        let mut attempt = start_char;
        while attempt <= chars.len() {
            // A pattern pinned to the text's start has one place to try;
            // one opening with a literal can jump to its next occurrence.
            if self.program.anchored && attempt > 0 {
                return Ok(None);
            }
            if let Some(prefix) = &self.program.prefix {
                let from = byte_offsets[attempt];
                let Some(found) = text[from..].find(prefix.as_str()) else {
                    return Ok(None);
                };
                let target = from + found;
                attempt = byte_offsets
                    .binary_search(&target)
                    .expect("a match starts on a character boundary");
            }
            if let Some(groups) = matcher.match_at(attempt, start_char)? {
                let spans = groups
                    .into_iter()
                    .map(|group| group.map(|(from, to)| (byte_offsets[from], byte_offsets[to])))
                    .collect();
                return Ok(Some(Captures { groups: spans }));
            }
            attempt += 1;
        }
        Ok(None)
    }
}
