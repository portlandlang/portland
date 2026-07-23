# 0010 — Partial operations return maybes; the only crash is one you typed

- **Status:** Accepted
- **Date:** 2026-07-22
- **Issue:** [#4](https://github.com/portlandlang/portland/issues/4) (the panic frontier)

## Context

Ruby's partial operations — lookups and aggregates that can't always
produce an answer (`[].first`, `array[99]`, `hash[missing]`, `[].max`,
seedless `reduce` on empty) — return nil, except `fetch`, which raises.
The seed panics on all of them as an explicit placeholder.

Candidates, tournament-style: uniform maybe, uniform panic, Ruby's
doubled soft/hard method families, Swift's per-container split
(`dict[key]` optional, `array[i]` traps). Uniform panic makes normal
data (a key absent from a config file — a runtime fact no compiler can
check) into a manual-discipline crash: Ruby's nil problem with the
polarity flipped. Swift's split has no rule you can say in a sentence —
`first` forgives, `[0]` crashes — a per-method table, which is guessing,
and never-guess already rejected that move. Doubled methods are the
winner wearing redundant merch: with the or-guard, `fetch` is a second
spelling of a sentence the language already says.

## Decision

One rule: **partial operations return maybes. Asserting certainty is
always spelled at the call site — `or panic "why"` — and the language
never panics implicitly.** The only crash in a Portland program is one
the programmer typed; `grep panic` audits every crash site, the way
`grep mutable` audits mutation.

`fetch` retires. All three of its arities are the or-guard, and the
eager-default gotcha (`fetch(key, default)` evaluates `default` even on
a hit — the reason Ruby needed the block arity at all) never exists,
because `or` is born lazy:

```ruby
h.fetch(:key)                       # ⇒ h[:key] or panic 'key not found: :key'
h.fetch(:key, :default)             # ⇒ h[:key] or :default   (lazy, for free)
h.fetch(:key) { |key| load(key) }   # ⇒ h[:key] or load(:key)
```

The wrapper model (ADR 0005) preserves `fetch`'s stored-nil semantics
exactly: a stored nil comes back `some(nil)`, the or-guard sees
"present" and hands over the inner nil rather than the default.

## Consequences

- `[].first`, `array[out_of_range]`, `hash[missing]`, `[].min`/`max`,
  seedless `reduce` on empty: all nil, typed as maybes, handled with the
  ADR 0008 toolkit. The seed's panic placeholders now have their answer.
- Negative indices stay (`array[-1]` — beloved Ruby); the maybe return
  makes them safe (`array[-99]` is nil, not a crash).
- Pre-flip, the polyfill linter rewrites `fetch` mechanically — marked
  **unsafe autocorrect** in RuboCop's vocabulary, because collapsed Ruby
  can't tell a stored nil from a missing key (`-A`, not `-a`).
- Dense-numerics indexing (`matrix[i][j] or panic` noise) is real and
  deferred: idiomatic Portland iterates (which never indexes out of
  range, and which tier-1 parallelism wants anyway), the compiler may
  delete checks it can prove, and the true answer belongs to the
  hardware/SME story, not to `Array#[]` semantics.
- Not covered here: the value of a branchless `if` and of a
  broken-out-of call (STAGE0's "produces no value" cases) — a separate
  future decision.
- Nothing implemented yet; the seed still panics.
