# 0073 — A struct with no fields is a value with one instance

- **Status:** Accepted (ruled by the deciding user 2026-11-03, accepting the recommendation in [#140](https://github.com/portlandlang/portland/issues/140)). Already built: the seed stopped refusing a fieldless struct on 2026-09-27, when the compiler was found to accept one, and `a_struct_may_have_no_fields` in `seed/tests/run_pdx_files.rs` pins both implementations; #140's census had run against the seed before that fix. This ADR records the ruling the fix anticipated; [#186](https://github.com/portlandlang/portland/issues/186) closed as already done.
- **Date:** 2026-11-03
- **Issue:** [#140](https://github.com/portlandlang/portland/issues/140)

## Context

`class Formatter` with only methods and no fields refused on the seed until 2026-09-27: `struct Formatter needs at least one field`. ruby_research's level-0 census ([ruby_research#9](https://github.com/portlandlang/ruby_research/issues/9)) finds that refusal in **14,086 gems**, and as the *only* refusal in 1,281 of them. These are Ruby's classes used as a bundle of behavior (`class Formatter; def format(x) …`), as a namespace with `def self.` functions, or as an error type (`class NotFound < StandardError; end`, which also inherits). The one-field minimum came in with structs as a guard against writing a module as a struct; it was never a principle.

## Decision

1. **A struct — and so a `class` ([ADR 0056](0056-2026-09-27-class-is-a-spelling-of-struct.md)) — may declare no fields.** Its body holds methods, type functions, `include`s, and nested types as any struct's does.
1. **It is a value with exactly one instance.** `Formatter.new` builds it; two of them are `==` and `eql?`, since values compare by their fields and there are none ([ADR 0015](0015-2026-07-23-values-never-mutate.md), [ADR 0059](0059-2026-10-25-eql-on-every-value-and-no-hash.md)); `inspect` is `Formatter()`; `with` takes no keywords and answers the same value; `to_h` is `{}`.
1. **The checker's shape checks apply unchanged.** A method call on it resolves against its declared methods; a field read refuses as it would on any struct that lacks the field.
1. **Rejected:** pointing these at `module` with `def self.` — a rewrite for 14,086 gems that buys nothing, since a struct with methods and no state already *is* a module with a value to pass around.

## Consequences

- The `needs at least one field` refusal is gone from both implementations; the seed's `Struct` representation and the compiler's tagged `["__struct__", name, []]` carry an empty field list.
- The error-type half of the census (`class NotFound < StandardError`) still refuses on the inheritance, which is [ADR 0028](0028-2026-07-27-object-model-structs-and-traits.md)'s and [errors.md](../ruby/errors.md)'s; this ADR takes the field count out of the way so that refusal is the only one left to name.
- Ledger: [classes.md](../ruby/classes.md).
