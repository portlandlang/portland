# 0066 — Pattern arguments take the `when` rule

- **Status:** Accepted (ruled by the deciding user 2026-10-25 on the recommendation in [#171](https://github.com/portlandlang/portland/issues/171); built in the seed and the trio, differentially pinned, 2026-10-25)
- **Date:** 2026-10-25
- **Issue:** [#171](https://github.com/portlandlang/portland/issues/171)

## Context

Ruby's Enumerable takes a pattern where Portland took only a block: `all?(1..3)`, `any?(2)`, `none?(pattern)`, `one?(pattern)`, `grep(3..7)`, `grep_v(pattern)`, `slice_before(pattern)`, `slice_after(pattern)`. Each asks `pattern === element`. Here case equality asks `==` ([ADR 0054](0054-2026-09-25-ordering.md)) and `when 3..7` covers a range, so the `when` rule already existed; only the method forms were missing, and ruby/spec's core/enumerable carried their rows.

## Decision

1. **A pattern argument matches the way a `when` clause would**: a range covers a number, anything else asks `==`. The eight methods take one on arrays, hashes (over their pairs), ranges, and Enumerable structs.
1. **`grep(pattern)` is `select` under that rule, `grep(pattern) { }` is `filter_map`, and `grep_v` the complement.** `slice_before(pattern)` and `slice_after(pattern)` cut where the block forms would.
1. **A pattern beside a block refuses**: `'all?' takes a pattern or a block, not both`, rather than letting one silently win, as Ruby's warning-then-pattern does.
1. **A type name is not a pattern argument.** `all?(Integer)` has no value to pass, since a type is not a value here; `all? { it in Integer }` is the spelling. A regex pattern waits on [#74](https://github.com/portlandlang/portland/issues/74).

## Consequences

- ruby/spec's grep and grep_v files port; the pattern rows of all, any, none, one, slice_before, and slice_after join their files.
- Recorded in [pattern-matching.md](../ruby/pattern-matching.md).
