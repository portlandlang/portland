# 0005 — Optionals are a wrapper, with a collapsed-feeling surface

- **Status:** Accepted
- **Date:** 2026-07-22
- **Issues:** [#4](https://github.com/portlandlang/portland/issues/4),
  evidence file on [#9](https://github.com/portlandlang/portland/issues/9)

## Context

No ambient nil is a locked decision; absence is one explicit case of an
optional (`T?`). The open question was the optional's *shape*:

- **Wrapper** (Rust/Swift): `T?` is its own thing holding zero or one `T`.
  Nests — `T??` exists and means something.
- **Collapsed nilable** (Kotlin): `T?` means "`T` or absent"; `T??` = `T?`.

Evidence from real Portland code (the #9 evidence file): the trio kept
convergently inventing the wrapper as a zero-or-one slot — `ReturnBareNode`
alongside `ReturnNode`, `MethodCallNode.block` as a zero-or-one array, the
evaluator's `Outcome.value` as `[]`/`[v]`. And the collapsed model's cost is
a live bug class the user knows from Ruby: `config["key"]` cannot say
whether the key was missing or the value was absent, forcing a `key?`
sidecar API forever (Kotlin's `containsKey`, Ruby's `key?`).

Two migration criteria (2026-07-22) sharpened the choice:

1. **Smooth Ruby→Portland migration** is a standing design goal.
2. **The polyfill test:** a hypothetical gem + RuboCop-style autocorrector
   should be able to teach Portland idioms inside Ruby before the flip.
   Ruby's runtime is *natively collapsed* (`nil` cannot nest), so only the
   unnested surface is polyfillable — and the unnested surface is where the
   two models are spelling-identical.

## Decision

**The wrapper — but it must never be ceremonial.** `T?` is a distinct
zero-or-one container; nesting is real and distinguishable where it is
load-bearing (double absence: a lookup that can miss, of a value that can
be absent). The surface is designed so unnested code never touches the
wrapper:

- Auto-wrap at boundaries: a maybe-returning method writes `return user`,
  not a wrapped form. Only absence is spelled (`return nil`-shaped; word
  per ADR 0006).
- Patterns match the payload directly, or absence — no wrapper word in the
  common case (`in User(name:)` / `in nil`), both valid Ruby patterns today.
- Chaining sugar flattens as it goes, so chains stay flat in both models.

Rust's ever-present `Some(x)` ceremony is explicitly rejected.

## Consequences

- `[nil].first` and `[].first` stay distinguishable; hashes with optional
  values answer "missing key or absent value?" in the value's shape, no
  sidecar API required.
- The polyfill gem covers the whole unnested surface (Ruby already behaves
  that way at runtime); the linter flags nested-optional sites as
  fix-at-flip-time.
- The absence word is ADR 0006. Unwrap ergonomics (narrowing, the
  or-guard, safe navigation, whether a bind-and-test form is needed) are a
  future decision — sketched in session, not locked.
- Nothing implemented yet; the seed's panic-where-Ruby-nils sites and the
  trio's slot pattern are the future call sites.
