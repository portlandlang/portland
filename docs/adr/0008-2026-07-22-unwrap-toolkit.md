# 0008 — The unwrap toolkit: narrowing, or-guard, `&.`, `case/in` — and nothing else

- **Status:** Accepted
- **Date:** 2026-07-22
- **Issue:** [#4](https://github.com/portlandlang/portland/issues/4)

## Context

ADR 0005 forbids the optional from being ceremonial; something still has
to get values *out*. The design bar: one tool per intent, zero new
grammar, every spelling either valid Ruby today or a loud error — never a
silent shift (ADR 0006's migration property).

## Decision

Four tools, one per intent:

- **Flow narrowing** — the invisible workhorse. Conditions and guards are
  proofs the compiler listens to: below `return if user.nil?`, or inside
  `if user.some?`, `user` is a plain value. `&&` short-circuit narrows its
  right side; postfix guards narrow their statement. Narrowing applies to
  **locals only** (call results must land in a local first), and the
  no-shadow rule is what makes it sound — a proven name cannot be rebound
  to something else mid-scope by a shadowing trick.
- **The or-guard** (semantics in ADR 0007) — bind-or-bail and the escape
  hatch: `user = find_user(id) or return`,
  `row = lookup(key) or panic "row #{key} must exist"`.
- **Safe navigation `&.`** — kept verbatim from Ruby, flattening as it
  chains (ADR 0005): `user&.nickname&.upcase or "FRIEND"`. On a receiver
  that can never be absent, `&.` is a compile error (dead safe-nav — the
  ADR 0007 dead-code rule applied here).
- **`case/in`** — when both branches deserve real code. Patterns match
  the payload directly or `nil` (ADR 0005); both are valid Ruby pattern
  grammar today.

Two deliberate absences:

- **No bind-and-test conditional** (Swift's `if let`). Its three jobs are
  covered — test-in-place (narrowing), bind-or-bail (or-guard), both
  branches (`case/in`) — and `if user = find_user(id)` assignment-in-
  condition is a Ruby footgun that stays dead.
- **No force-unwrap operator** (Swift `!`, Rust `.unwrap()`). Rust shows
  an escape-hatch-less system gets fought; Swift shows a one-character
  hatch gets typed reflexively. `or panic "why"` is the hatch, with the
  message mandatory in spirit: the beautiful assertion carries its reason.

## Consequences

- Every tool is polyfill tier 1 or 2: `&.`, `|| default`, `f or return`,
  and `in nil` all run in Ruby today; the linter half fakes narrowing
  warnings pre-flip.
- The `[].first` / `hash[key]` / indexing story (what actually *returns*
  a maybe) is the next decision, built on these tools.
- Nothing implemented yet.
