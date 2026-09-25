# 0051 — The `%` literal family: `%w` and `%i` with three delimiters, the rest declined

- **Status:** Accepted (ruled member by member and built in both oracles, 2026-09-24; `%r` deferred to #74)
- **Date:** 2026-09-24
- **Issue:** [#29](https://github.com/portlandlang/portland/issues/29)

## Context

Portland had `%w[]` and nothing else from Ruby's `%` family, by accident rather than by decision. [ADR 0030](0030-2026-07-27-word-array-contents.md) settled what `%w[]` means and left the rest — which members, which delimiters — to a corpus question. The corpus was asked on 2026-09-24: `ruby_research`'s `percent-literals` report over every gem on RubyGems.org (195,390 gems, a July 2026 snapshot), counting members, delimiters, escaped and nested delimiters, and whether the string members' bodies hold a quote. The per-gem figures, in the ledger's [percent-literals.md](../ruby/percent-literals.md), drove each ruling below. The gem corpus does not see Rails applications; the deciding user's knowledge of them was weighed where it mattered.

## Decision

1. **`%i[]` is in.** The one member still growing — 0.7% of `%`-using gems before 2015, 38.8% after 2020 — and the modern spelling of a symbol list; Rails applications, outside the corpus, write it constantly. It is `%w[]`'s lexer path with symbols out: ADR 0030's content rules, each word a plain symbol (`%i[odd\ name]` is `[:"odd name"]`). Symbols do not interpolate (ADR 0023 §2), so there is nothing an `%I` could add.
1. **Three delimiters: `[]`, `()`, `{}`.** Nearly half of all `%w` in the wild is `%w(...)`, steady across eras and RuboCop's default; `{}` is a further tenth; everything else is under two percent combined. Three bracket pairs for one literal are spellings with identical behavior, which the house welcomes; a pair with a different behavior would be refused. The pair the literal opened with is the one that balances and the one a backslash escapes — inside `%w(...)`, `\)` drops its backslash and `\]` keeps it, Ruby's own rule. Any other delimiter refuses: `'%w|' is not a Portland delimiter — write %w[], %w(), or %w{}`.
1. **`%q`, `%Q`, and bare `%` are out.** Pure synonyms for `'...'` and `"..."`, both of which Portland has, shrinking in every cohort (`%Q` from 16% of `%`-using gems to 5%). Their one purchase is a body full of quotes without backslashes, and two thirds of their bodies in the corpus hold no quote at all; the quote-heavy HTML and JSON fragments already have a home in `<<~HTML`. The `%q`/`%Q` case-sensitive pair is exactly what "Ruby, the good parts" leaves behind. Refusal: `'%q(' is not a Portland literal — write a quoted string or a heredoc`. A bare `%` is read as this form only where Ruby reads it so — after a space and glued to its opener, `a %(b)` — so `a % (b)` and `a%(b)` stay modulo.
1. **`%s` is out.** Forty-four gems in the corpus, flat across eras; `:name` and `:"odd name"` (ADR 0023) already spell everything it can. Refusal: `'%s(' is not a Portland literal — write :name, or :"odd name" for a name with spaces`.
1. **`%W` and `%I` are out.** A capital letter is their whole difference from `%w`/`%i`, the same case-sensitive pair as `%q`/`%Q`; 1.3% and 0.1% of gems, shrinking. `["#{a}", "b"]` is the array that interpolates. Refusals: `'%W[' is not a Portland literal — write %w[] when no word interpolates, or ["#{a}", "b"] when one does`; `'%I[' is not a Portland literal — write %i[]; a symbol does not interpolate`.
1. **`%r` is deferred to [#74](https://github.com/portlandlang/portland/issues/74).** Big and growing (8.2% of gems, 29.6% of `%`-using gems after 2020), but it spells a regex, and Portland has no regex decision — no literal, no engine, no answer for a match. `%r{}` is the delimiter 61% of its users reach for, which #74 now knows. Until then it refuses plainly: `'%r{' is not a Portland literal — there is no regex yet`.
1. **`%x` is out of the family.** It is a process spawn wearing literal syntax, not a literal, and belongs with the OS-surface question ([#78](https://github.com/portlandlang/portland/issues/78)) alongside backticks — neither is in the grammar, and neither is decided here. Refusal: `'%x(' is not a Portland literal — there is no shell execution`.

Every refusal is one sentence in ADR 0047's voice — the spelling as written in single quotes, a dash, the Portland form — said by both lexers to the byte. A declined letter is recognised only when a delimiter character follows it, so `a %x` at a line's end is still modulo by a name.

## Consequences

- The seed gains a `SymbolArray` token beside `WordArray`; the trio a `SymbolArrayNode` beside `WordArrayNode`; the word-splitting helper on each side takes the delimiter pair. The type of `%i[]` is `array of symbol`.
- ADR 0030's "`[` stays the only delimiter" is superseded on that one point; its content rules stand, now keyed to the pair in use.
- Pinned by seed lexer tests for each form and each refusal, a two-oracle evaluator test, a two-oracle refusal test over all ten sentences, `spec/array/word_array_spec.pdx` (delimiters) and `spec/array/symbol_array_spec.pdx`.
- Ledger: [percent-literals.md](../ruby/percent-literals.md) carries the corpus table and the migration rewrite for each declined member. The migration linter ([#36](https://github.com/portlandlang/portland/issues/36)) can apply every rewrite mechanically, since each refusal names it.
- The corpus tool itself — `script/report percent-literals` in `ruby_research` — is reusable; the full run takes about twenty minutes and no network.
