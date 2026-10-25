# Equality

**Summary:** `==` and `eql?` migrate unchanged; `hash` and `equal?` do not exist.

**Status:** decided ([ADR 0059](../adr/0059-2026-10-25-eql-on-every-value-and-no-hash.md), 2026-10-25, out of [#154](https://github.com/portlandlang/portland/issues/154)). Built in the seed and the compiler, differentially pinned.

## Ruby

Four equalities: `==` (value equality, with `1 == 1.0`), `eql?` (hash-key equality: same kind, no numeric promotion), `equal?` (identity), and `===` (case equality). Beside them, `hash` answers an integer digest that `eql?` values share, reseeded every process.

## Portland

- **`==` is Ruby's.** `1 == 1.0`, and a collection asks `==` of what it holds, so `[1] == [1.0]`.
- **`eql?` is Ruby's**, on every value: `1.eql?(1.0)` is false, `0.0.eql?(-0.0)` is true, NaN is never `eql?` to anything, strings compare by canonical text, and arrays, hashes, ranges, and structs compare element by element through `eql?`. A hash's order is not its identity.
- **`===` asks `==`** ([ADR 0054](../adr/0054-2026-09-25-ordering.md)); a type test is an `in` pattern.
- **`hash` refuses**: `'hash' has no Portland meaning — a value is a hash key by 'eql?', and its digest is never exposed`. The runtime keys its hashes without telling you the number.
- **`equal?` refuses** as an unknown method: values never mutate ([mutability](mutability.md)), so identity is not observable.

## Migration

- `==` and `eql?` port verbatim, including as hash keys: `{1 => :x}` and `{1.0 => :x}` are two different hashes in both languages.
- `value.hash` — rewrite to `eql?` where it stood in for equality, or to a content digest of your own where a stable number was wanted; Ruby's was never stable across processes either.
- `equal?` — there is no identity to compare; `==` is the question.
