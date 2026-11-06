# 0082 — `Set` is a builtin value, as in Ruby 4

- **Status:** Accepted (ruled by the deciding user 2026-11-03, accepting the recommendation in [#123](https://github.com/portlandlang/portland/issues/123); built on both implementations 2026-11-05 under [#195](https://github.com/portlandlang/portland/issues/195) — the algebra under its word names, `+`, and `-`, since [ADR 0003](0003-2026-07-20-bitwise-operators-out.md) took `|`, `&`, and `^` out of the grammar; [#207](https://github.com/portlandlang/portland/issues/207) holds that edge)
- **Date:** 2026-11-03
- **Issue:** [#123](https://github.com/portlandlang/portland/issues/123)

## Context

Ruby 4 moved `Set` into core, with no `require`; ruby/spec has `core/set/` (55 files, 189 examples, plus `set/enumerable` and `set/sortedset`), and gems use a set for membership everywhere. [ADR 0064](0064-2026-10-25-the-standard-library-core-default-tiles-external-tiles.md) named `Set` among the things Ruby made core that Portland takes as each is decided. Today there is no `Set`, and `core/enumerable/to_set_spec.rb` is skipped for want of one.

## Decision

1. **`Set` is a builtin immutable value**, built the way `Array` and `Hash` are, with value equality from being a value ([ADR 0015](0015-2026-07-23-values-never-mutate.md), [ADR 0059](0059-2026-10-25-eql-on-every-value-and-no-hash.md)).
1. **No literal.** `Set.new(array)`, `Set[1, 2, 3]`, and `to_set` on any collection build one ([ADR 0058](0058-2026-10-16-builtin-types-carry-type-functions.md) for the type functions).
1. **`add` and `delete` answer a new set.** The bang forms, `add!`-style mutation, and `<<` follow [#103](https://github.com/portlandlang/portland/issues/103)'s answer on mutable state; until then they refuse naming the non-bang spelling, as `push` does on an array ([ADR 0015](0015-2026-07-23-values-never-mutate.md)).
1. **Ruby's surface otherwise**: `include?`, `member?`, `===`, `size`, `empty?`, `|`/`union`, `&`/`intersection`, `-`/`difference`, `^`, `subset?`/`<=`, `superset?`/`>=`, `proper_subset?`/`<`, `proper_superset?`/`>`, `disjoint?`, `intersect?`, `each`, and the Enumerable surface answering what it answers on an array; `inspect` is Ruby 4's `Set[1, 2, 3]`. The ruby/spec files are the specification; tie goes to Ruby everywhere the ledger does not speak.
1. **Insertion order is kept**, as Ruby's is, so `to_a` and `each` are deterministic.
1. **`SortedSet` is gone from Ruby and stays gone.**

## Consequences

- Both implementations gain a `Set` value with the type functions `new` and `[]`, and `to_set` on arrays, hashes, ranges, and sets; the checker's builtin table gains the type and its methods.
- `core/set/` and `core/enumerable/to_set_spec.rb` port under the build issue.
- Ledger: a new [sets.md](../ruby/sets.md).
