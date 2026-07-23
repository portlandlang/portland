# 0009 — The presence word is `some` / `some?`

- **Status:** Accepted
- **Date:** 2026-07-22
- **Issue:** [#4](https://github.com/portlandlang/portland/issues/4) — closes its last open task

## Context

ADR 0006 picked `nil`/`nil?` for absence and left the partner open, with a
hard filter from the polyfill test: the presence word must be unclaimed
across Ruby core, Rails, and major gems, so a gem can define it in Ruby
without changing existing behavior.

Ecosystem scan (2026-07-22, verified against sources, not memory):

- **Ruby core** — unclaimed. The Enumerable quantifier family is
  `any?`/`all?`/`one?`/`none?`; `some?` is a hole that was never filled.
- **Rails** — unclaimed. (`present?` is claimed *and* means the
  nil/blank conflation — disqualified in ADR 0006.)
- **Hanami** — unclaimed; the framework layers on dry-monads' Result side
  (`Success`/`Failure`), never `Some`/`None`.
- **dry-monads** — claimed, **with exactly our meaning**:
  `def some? = is_a?(Some)` on its `Maybe`, constructed `Some(x)`/`None()`.
  Precedent, not conflict — the one place a Rubyist has met the word, it
  meant this.
- Cross-language footnote: JavaScript's `array.some` means "any element
  matches"; Portland keeps `any?` for that job, so the misreading
  self-corrects at the first error message.

## Decision

The presence partner to `nil`/`nil?` is **`some`** / **`some?`**.

- **`some?`** — the predicate on a maybe: `if user.some?` (narrows, per
  ADR 0008).
- **`some(x)`** — the wrap form. Auto-wrapping (ADR 0005) makes it almost
  never appear: you write it only when constructing or matching the rare
  *nested* case explicitly, where a bare `nil` is genuinely ambiguous —
  `{"digest" => some(nil)}` (key present, value absent) vs
  `{"digest" => nil}`, and `in some(nil)` vs `in nil` in patterns. A bare
  `nil` where the type is doubly-optional is a never-guess error naming
  both spellings.

## Consequences

- The polyfill gem ships `Object#some?` (`!nil?`) — dry-monads' own
  `Maybe#some?` takes precedence on its objects and agrees in meaning, so
  the two coexist.
- Both #4 words are now decided (`nil`/ADR 0006, `some`/here); what
  remains of #4 is the pattern-matching spec detail and anything the
  `[].first` decision surfaces.
- Nothing implemented yet.
