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
//! sees the pattern; `parser` is `regparse.c`, pattern to tree; `compile` is
//! `regcomp.c`, tree to a program; `vm` is `regexec.c`, a backtracking matcher
//! over the program; `unicode` is `enc/unicode.c` over tables generated from
//! Ruby's own copies.

pub mod error;
pub mod unicode;
pub mod unicode_tables;

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
        let _ = (pattern, options);
        Err(RegexError::new("regex is not built yet"))
    }

    /// The pattern as written.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The options the pattern was compiled under.
    pub fn options(&self) -> Options {
        self.options
    }

    /// Searches `text` from its start for the leftmost match, as
    /// `Regexp#match` does.
    pub fn search(&self, text: &str) -> Option<Captures> {
        self.search_from(text, 0)
    }

    /// Searches `text` for the leftmost match starting at or after the byte
    /// offset `start`, as `Regexp#match(text, position)` does.
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
        let _ = (text, start, budget);
        Ok(None)
    }
}
