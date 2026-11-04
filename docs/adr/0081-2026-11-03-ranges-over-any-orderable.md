# 0081 — Ranges over any orderable value

- **Status:** Accepted (ruled by the deciding user 2026-11-03, accepting the recommendation in [#149](https://github.com/portlandlang/portland/issues/149); not built — the build is [#194](https://github.com/portlandlang/portland/issues/194)). Amends ADR 0019's integer-only rule.
- **Date:** 2026-11-03
- **Issue:** [#149](https://github.com/portlandlang/portland/issues/149)

## Context

[ADR 0019](0019-2026-07-23-ranges.md) made ranges integer-only. Ruby's hold anything `<=>` orders: `"a".."z"`, `1.0..2.5`, ranges of a user's `Comparable` values. [#101](https://github.com/portlandlang/portland/issues/101) built `clamp(range)` for numbers by integer ranges, and what is left of ruby/spec's `core/comparable/clamp_spec.rb` clamps structs by a range of structs; `core/range/` ([#111](https://github.com/portlandlang/portland/issues/111)) meets the same question file after file. [ADR 0054](0054-2026-09-25-ordering.md) made "what `<=>` orders" a precise set: numbers, strings, arrays, and `Comparable` structs.

## Decision

1. **A range's ends may be any two values `<=>` orders**: two numbers, two strings, two arrays, two structs of one type that includes `Comparable`. Beginless and endless ranges keep their one end's type.
1. **Membership and bounds work over any such range**: `include?`, `member?`, `cover?`, `===` and so `case/when`, `clamp`, `min`, `max`, `minmax`, `first`, `last`, `begin`, `end`, `==`, `eql?`, `inspect`, `to_s`.
1. **Iteration stays with integer ranges**: `each`, `to_a`, `map`, `step`, `size`, `count`, `reverse_each`, and the Enumerable surface. Iterating a float or struct range refuses as Ruby's does, in Ruby's words: `can't iterate from Float`. Iterating a string range, Ruby's `succ` walk (`("a".."e").to_a`), is not taken until a real file pulls for it; the refusal names the fact.
1. **Ends that `<=>` cannot order refuse at build time.** `1.."a"` is `bad value for range` in Ruby at runtime; here both ends' types are known, so the build refuses the range where it is written.
1. **Exhaustiveness over range patterns stays integer.** The gap-free proof in [ranges.md](../ruby/ranges.md) sorts integer ends; a `case/in` over float or string ranges needs an `else`, as `when` does when coverage cannot be proven.

## Consequences

- Both implementations widen the range value to carry any orderable ends and route membership through `<=>`; the iteration paths keep their integer check and gain the refusal sentence.
- `core/comparable/clamp_spec.rb`'s struct rows and the float-range rows of `core/random/rand_spec.rb` become portable; `core/range/` ports file by file under [#111](https://github.com/portlandlang/portland/issues/111).
- Ledger: [ranges.md](../ruby/ranges.md).
