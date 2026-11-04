# 0080 — Hash patterns, `unless` guards, `=> name`, `_` wildcards, and left-to-right pins

- **Status:** Accepted (ruled by the deciding user 2026-11-03, accepting the recommendation in [#168](https://github.com/portlandlang/portland/issues/168); not built — the build is [#193](https://github.com/portlandlang/portland/issues/193))
- **Date:** 2026-11-03
- **Issue:** [#168](https://github.com/portlandlang/portland/issues/168)

## Context

Porting ruby/spec `language/pattern_matching_spec.rb` against [pattern-matching.md](../ruby/pattern-matching.md) found five Ruby forms that both implementations refuse and the page does not rule on. The find pattern, `[*, x, *]`, is recorded as out for now; these were not:

1. Hash patterns on a hash: `in {a: 0, b:}`, `in {a:}`, `in {}`, `in {a: 0, **rest}`, `in {a: 0, **nil}` — `unsupported pattern starting with LeftBrace` (14 upstream rows).
1. `unless` guards: `in 0 unless done?` (2 rows).
1. The AS pattern: `in Integer => number`, `in [1, Array => inner]` (3 rows).
1. `_` and `_name` reused in one pattern: `in [0, _, _]`, `in [0, _x, _x]` — refused as a rebind of an immutable name (2 rows).
1. Pinning a name bound earlier in the same pattern: `in [n, ^n]` — `undefined variable n in a ^ pin` (1 row).

## Decision

All five, each as Ruby reads it, with the no-shadow rule unchanged:

1. **A hash pattern binds its keys as a struct pattern binds its fields.** `in {a: 0, b:}` matches a hash whose `a` is 0 and binds `b`; a hash pattern matches partially, so extra keys are fine; `in {}` matches only the empty hash, as Ruby's does; `**rest` binds the remaining pairs as a hash; `**nil` demands no remaining pairs. Keys are symbols, the one hash-key spelling.
1. **`unless` is `if not`**: `in 0 unless done?` is `in 0 if not done?`.
1. **`pattern => name` binds the value the pattern matched**, at any depth: `in Integer => number`, `in [1, Array => inner]`.
1. **A `_`-prefixed name in a pattern never binds**, so it may appear any number of times. Ruby's reading; `_` was being bound as a normal name and the second use refused as a rebind.
1. **A pin reads names bound to its left in the same pattern**: `in [n, ^n]` matches a two-element array whose elements are equal.

The find pattern stays out as recorded, until a real file pulls for it.

## Consequences

- Both pattern compilers grow the five forms; the exhaustiveness checker learns that a hash pattern with no value constraints covers a hash of those keys, and that a wildcard name adds nothing to a binding set.
- The twenty-two manifest rows move from `partial_planned` to the build.
- Ledger: [pattern-matching.md](../ruby/pattern-matching.md).
