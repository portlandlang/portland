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
//! `compile` are `regcomp.c`, tree to a program, and `optimize` its
//! reading of what a search can skip; `search` is `onig_search`, which
//! picks the positions to try; `vm` is `regexec.c`'s `match_at`, a
//! backtracking matcher over the program, and `cache` its match cache, the
//! memoization that keeps Ruby's backtracking linear; `unicode` and
//! `casefold` are `enc/unicode.c` over tables generated from Ruby's own
//! copies.

pub mod ast;
pub mod cache;
pub mod casefold;
pub mod cclass;
pub mod compile;
pub mod error;
pub mod length;
pub mod optimize;
pub mod parser;
pub mod preprocess;
pub mod search;
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

    /// What Ruby's `Regexp.linear_time?` answers: whether the match cache
    /// covers the pattern, so a search runs in time linear in the text.
    pub fn linear_time(&self) -> bool {
        self.program.cache.is_some()
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
        // A start inside a character moves up to the next boundary.
        let mut start = start.min(text.len());
        while !text.is_char_boundary(start) {
            start += 1;
        }
        search::search(&self.program, text, start, budget)
    }
}
