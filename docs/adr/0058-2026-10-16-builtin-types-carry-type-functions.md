# 0058 — The builtin types carry Ruby's type functions

- **Status:** Accepted (ruled by the deciding user 2026-10-16 on the recommendation in [#159](https://github.com/portlandlang/portland/issues/159); `Array.new` and `Integer.sqrt` built in the seed and the trio, differentially pinned, 2026-10-16)
- **Date:** 2026-10-16
- **Issue:** [#159](https://github.com/portlandlang/portland/issues/159)

## Context

Some of Ruby's core API lives on the class rather than the value: `Array.new(3) { |index| index * 2 }` and `Array.new(3, 0)`, the commonest way to build a sized array; `Integer.sqrt(24)`; `Array[1, 2]`; `Hash.new(0)`. Portland has type functions for structs — `Token.of(...)`, registered under the struct's name ([ADR 0031](0031-2026-07-31-smart-constructors-definable-new.md)) — but the builtin types had none, so `Array.new` refused as an undefined name. The type names themselves already exist as names: `when Integer` in case equality, `in String` in a pattern.

A builtin type is not a class object here ([metaprogramming.md](../ruby/metaprogramming.md)): there is nothing to `allocate` from, no ancestors to ask about. A type function does not need one. It is a function that happens to live under the type's name, which is what Portland's struct type functions already are.

## Decision

1. **The builtin types answer Ruby's type functions, spelled as Ruby spells them**, starting with the two a real port pulled for: `Array.new` in every arity Ruby gives it — bare, by count (nil-filled, as Ruby's), by count and fill, by count and a block handed each index, or copied from an array — and `Integer.sqrt`, exact where a float would round past 2^53. Tie goes to Ruby (principle 2).
1. **The name is not a value.** `Array` and `Integer` on their own still refuse; only `Array.new(...)` and `Integer.sqrt(...)` have meaning. A struct or local wearing the name wins, as everywhere.
1. **The refusals are Ruby's words where Ruby has them** — `negative array size`, `Numerical argument is out of domain - "isqrt"` — and name the argument otherwise: `Array.new takes a count or an array, got "a"`, `Integer.sqrt takes an Integer, got 2.5`. A fill beside a block refuses rather than letting one silently win, as Ruby's warning-then-block does.
1. **Not decided here.** `Array[1, 2]` is a synonym for a literal, welcome under principle 3 but unpulled. `Hash.new`'s default value or block changes what a lookup answers, which [lookups.md](../ruby/lookups.md) retired with `fetch`; it is [#105](https://github.com/portlandlang/portland/issues/105)'s to decide. Further type functions arrive one real pull at a time.

## Consequences

- `Array.new(3, [])`'s famous trap — three names for one mutable array — cannot bite, since values never mutate ([ADR 0015](0015-2026-07-23-values-never-mutate.md)).
- The checker types `Array.new(...)` as an array and `Integer.sqrt(...)` as an integer, so a chain off either reads as a chain off the value.
- Migration: `Array.new(n) { }` and `Array.new(n, fill)` compile verbatim; `Array.new(n, fill) { }` refuses at the call.
- Recorded in [metaprogramming.md](../ruby/metaprogramming.md) beside the class-object differences.
