# 0006 — The absence word is `nil`

- **Status:** Accepted (partner word still open)
- **Date:** 2026-07-22
- **Issue:** [#4](https://github.com/portlandlang/portland/issues/4)

## Context

Candidates were `nil`, `none`, `empty`. Two criteria decided it
(both 2026-07-22): smooth Ruby/Rails migration, and the polyfill test —
can a gem teach the idiom inside valid Ruby before the flip?

- **`empty`** — collides with `Array#empty?` / `String#empty?`; empty is a
  *present* value with nothing in it, the exact concept Portland keeps
  separate from absence.
- **`none`** — squeezed from both grammatical directions: `Model.none` in
  Rails is a present, empty relation (the empty/absent cross-wire again),
  and `Enumerable#none?` already means "no elements match." Fatally: in a
  Ruby pattern, `in none` is a *capture* — it silently matches anything and
  binds a local named `none`. The polyfill cannot have it.
- **`nil`** — the baggage was never the word; it was the *ambient-ness*
  (every value secretly haunted). Kill the ambient-ness, keep the word, and
  Ruby's good nil hygiene compiles verbatim: `return if x.nil?`, `x == nil`,
  and — decisively — `in nil`, the only absence *literal* Ruby's pattern
  grammar accepts.

## Decision

The absence word is **`nil`**; the predicate is **`nil?`**. Portland's
`nil` is the empty case of an optional (ADR 0005), not Ruby's creature: it
has no methods, is not falsy (strict booleans), belongs to no `NilClass`,
and exists only where the type admits absence.

The migration property that makes word-reuse safe: everywhere Ruby-nil and
Portland-nil agree, code compiles and means the same thing; everywhere they
differ (`if user` truthiness, `nil.to_s`, `nil?` on a never-absent value),
the code **fails to compile with a suggested rewrite** — loud divergence,
never silent.

## Consequences

- `return if bar.nil?` narrows: below the guard, `bar` is a plain `T`.
- `if user` / `unless user` doing nil-work becomes a compile error; the
  rewrites are `user.some?`-style predicates (word pending), the or-guard
  (`user = find_user(id) or return`), or `case/in`.
- **Open: the partner word** (the "some" side — the pattern/predicate for
  presence, and the wrap word if construction ever needs one). Hard filter
  inherited from the polyfill test: the name must be unclaimed across Ruby
  core, Rails, and major gems, so the gem can define it without changing
  existing behavior (`present?` is disqualified; `some?` currently passes).
- Nothing implemented yet; the seed still panics where Ruby nils.
