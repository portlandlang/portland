# 0030 — `%w[]` contents take Ruby's rules: escapes and balanced nesting

- **Status:** Accepted (settling the bug half of [#29](https://github.com/portlandlang/portland/issues/29); the rest of the `%` zoo stays open there; built on both oracles, 2026-07-27)
- **Date:** 2026-07-27

## Context

`%w[]` was Portland's one member of Ruby's `%` literal family, and it carried a live bug: the lexer scanned to the first `]`, so `%w[] ) } ,]` lexed as an **empty** word array plus four stray operator tokens — a silent misparse, the exact shape principle 5 exists to forbid. The compiler felt it first: `parser.pdx` wrote its delimiter lists as `["]", ")", "}", ","]` at eight sites where `%w[]` would read better, which is a real file pulling for the fix (principle 6).

The bug is separable from #29's zoo ruling — which members survive, which delimiters, what the corpus says — because the broken member is already *in*. Bugs don't wait for strategy.

## Decision

**Inside `%w[...]`, the content rules are Ruby's**, verified against Ruby 4.0.6 rather than remembered (principle 8):

- A backslash before `]`, `[`, `\`, or whitespace escapes it: the backslash drops, the character joins the current word, and escaped whitespace joins instead of splitting — `%w[a \] b]` is `["a", "]", "b"]`, `%w[rose\ city]` is `["rose city"]`.
- A backslash before anything else stays literal: `%w[a\nb]` is the five-character `"a\nb"` with its backslash, exactly Ruby's single-quoted-style behavior.
- Unescaped brackets balance: `%w[a [b] c]` is `["a", "[b]", "c"]`, the outer literal closing only at depth zero.
- Words split on runs of whitespace — space, tab, newline — and never produce empty strings; a multi-line `%w[]` therefore reads as a list.

The token carries the raw source; unescaping happens where the words are built (the seed's parser, the trio's evaluator), which is the same borrow-the-source discipline the heredoc pass established.

`[` stays the only delimiter, and the rest of the family — `%i`, `%q`, `%r`, other delimiters — remains #29's corpus question. This ADR settles only what the existing member means.

## Consequences

- A **deliberate non-difference** from Ruby: every `%w[...]` both languages accept means the same array, so migrating code and muscle memory carry over whole (principle 2).
- `parser.pdx`'s eight workaround sites become `%w[]` again — the compiler is the first beneficiary of its own fix.
- One recorded oracle edge joins the existing class: the seed asks Unicode what whitespace is, the trio asks the separators a Portland string can spell (space, tab, newline — carriage return has no escape, on purpose). No literally-written program has met the difference.
- Retired on the way: the trio split words on single spaces where the seed split on whitespace runs, a silent divergence for `%w[a  b]` that had never met a fixture. Both oracles now agree, pinned differentially.
