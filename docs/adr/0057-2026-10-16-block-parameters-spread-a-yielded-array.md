# 0057 — Block parameters spread a yielded array

- **Status:** Accepted (ruled by the deciding user 2026-10-16 on the recommendation in [#158](https://github.com/portlandlang/portland/issues/158); built in the seed and the trio, differentially pinned, 2026-10-16)
- **Date:** 2026-10-16
- **Issue:** [#158](https://github.com/portlandlang/portland/issues/158)

## Context

Ruby spreads a single yielded Array across a block's parameters when the block takes two or more: `[[1, 2], [3, 4]].each { |a, b| }`, `pairs.map { |key, value| }`, `list.zip(other).each { |left, right| }`, `hash.to_a.sort_by { |k, v| v }`. It is one of the commonest shapes in Ruby code, and Portland refused every one of them with `block expects 2 arguments, got 1`. A hash's own `each { |key, value| }` worked only because a hash yields two values.

The ledger had no decision on it. [ADR 0014](0014-2026-07-22-keyword-arguments.md) deferred splats, and [pattern-matching.md](../ruby/pattern-matching.md) gave `pair => [a, b]` as the explicit destructure; neither spoke to a block's parameter list. The ruby/spec port ([#104](https://github.com/portlandlang/portland/issues/104), [#105](https://github.com/portlandlang/portland/issues/105), [#110](https://github.com/portlandlang/portland/issues/110)) leans on the spread throughout.

Ruby's rule has one silent corner: a parameter the array cannot fill becomes nil. `[[1], :a].each { |x, y| }` gives `y = nil` for the first element and `x = :a, y = nil` for the second. That is the ambient nil [ADR 0006](0006-2026-07-22-absence-word-nil.md) keeps out, arriving through a side door.

## Decision

1. **The spread is Ruby's.** A block declaring two or more positional parameters that is handed exactly one argument, and that argument an array, binds its parameters from the array's elements in order. Extra elements are dropped, as Ruby drops them. Tie goes to Ruby (principle 2), and the rule reads off the block's own parameter list at the call site, so it costs a reader nothing new.
1. **The nil corner refuses.** Where Ruby would fill a parameter with nil — an array with fewer elements than parameters, or a single argument that is not an array — Portland refuses, naming the count and, for a short array, the array: `block expects 2 arguments — [1] has 1 element to spread`. Principle 5 prefers loud to silent, and working code, where pairs are pairs, is unaffected.
1. **Nested destructuring waits.** `|(a, b), c|` is not built here; it waits for a real pull, as splats do.
1. **A single parameter never spreads.** `|pair|` and `it` receive the array whole, as in Ruby.

## Consequences

- `each`, `map`, `select`, `sort_by`, `to_h`, `sum`, `flat_map`, and every other block-taking method that yields one value spread on both implementations; `reduce`, `each_with_index`, and a hash's `each`, which yield two, are unchanged.
- The checker types the parameters of a spreading block from the element's own element type, so `left + right` over `[[1, 2]]` reads as integer arithmetic rather than refusing as Array against Integer.
- Migration: `|key, value|` over an array of pairs compiles verbatim. A block relying on nil-filling is refused at the call, with the array shown.
- Recorded in [pattern-matching.md](../ruby/pattern-matching.md) beside the explicit destructure.
