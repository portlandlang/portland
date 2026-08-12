# Enumerators

**Summary:** There are none; a method Ruby answers with a lazy enumerator answers the finished collection here, and `.to_a` on a collection is the identity.

**Status:** decided in practice (the [#23](https://github.com/portlandlang/portland/issues/23) import, 2026-08-12); no ADR — laziness is a capability nothing has pulled for (principle 6), and the eager answers are pinned by spec so a future lazy tier must preserve them.

## Ruby

A block-taking method called without its block answers an `Enumerator` — a lazy, chainable, externally-steppable iteration object. `(1..9).step(3)` is an `ArithmeticSequence`, `[1, 2, 3, 4].each_slice(2)` an `Enumerator`, and both need `.to_a` (or a chained call) to become data. Enumerators also power `.lazy`, infinite sequences, and external iteration with `.next`.

## Portland

No enumerator type exists. The methods that would answer one answer the **finished collection** outright:

```ruby
(1..9).step(3)              # [1, 4, 7] — an array, immediately
[1, 2, 3, 4].each_slice(2)  # [[1, 2], [3, 4]]
```

Because migrating Ruby spells these with a trailing `.to_a`, `Array#to_a` exists as the identity — the harmless end of Ruby's rule — so `(1..9).step(3).to_a` means the same thing in both languages.

Eagerness is observable only where laziness was load-bearing: infinite sequences and `.lazy` chains have no translation, and external iteration (`.next` on an enumerator) does not exist. A beginless or endless range refuses to be walked at all, with the same wording `each` uses.

## Migration

- `enumerator.to_a` chains migrate verbatim, same answer — _free_.
- Bare enumerator values that were passed around, `.lazy` chains, and `Enumerator.new` fail loudly (no such methods/type), never silently — the rewrite is to materialize the collection or restructure the loop.
- If a lazy tier is ever pulled for, the eager answers pinned in `spec/` are the semantics it must preserve; only the timing could change.
