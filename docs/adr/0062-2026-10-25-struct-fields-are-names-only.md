# 0062 — A struct's fields are names only

- **Status:** Accepted (ruled by the deciding user 2026-10-25 on the recommendation in [#163](https://github.com/portlandlang/portland/issues/163); the refusals built in the seed and the trio, differentially pinned, 2026-10-25)
- **Date:** 2026-10-25
- **Issue:** [#163](https://github.com/portlandlang/portland/issues/163)

## Context

Ruby's `Struct` reads a field three ways: `car.make`; `car[:make]` or `car["make"]`; and `car[0]` by definition position. `dig`, `values_at`, and `deconstruct_keys` ride the same lookups. A lookup by a *runtime* name — `car[field]` with `field` a variable — is the reflection the metaprogramming cut removes ([metaprogramming.md](../ruby/metaprogramming.md)); with a literal it is a longer spelling of `car.make`. Positional access treats a named record as a tuple. The rest of Ruby's Struct surface — `to_h`, `to_a`, `members`, `size`, `each`, `each_pair`, `select` — is built ([#114](https://github.com/portlandlang/portland/issues/114)).

## Decision

1. **A struct is a named record, read by its field names.** `car.make` is the one spelling, and the checker knows every field.
1. **`[]`, `dig`, `values_at`, and `deconstruct_keys` refuse on a struct, naming the field spelling**: `a Car is read by its field names, not indexed — write '.make', or 'to_h[name]' for a name decided at runtime`; by position, `not by position — write the field, or 'to_a[0]'`; and `'dig' is not a Car's — its fields are read by their bare names; for a name decided at runtime, write 'to_h.dig'`.
1. **`to_h` carries the dynamic case.** `car.to_h[name]` answers a maybe, the honest shape for a name the checker cannot see through; `car.to_a[0]` the positional one.
1. **Patterns stay keyword-only** ([pattern-matching.md](../ruby/pattern-matching.md)): fields are the pattern surface, and there is no `deconstruct_keys` to call.

## Consequences

- ruby/spec's core/struct `element_reference`, `dig`, `values_at`, and `deconstruct_keys` are recorded as decided out.
- Recorded in [classes.md](../ruby/classes.md).
