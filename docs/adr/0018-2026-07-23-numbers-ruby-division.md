# 0018 — Numbers: Ruby's division, floats without ceremony

- **Status:** Accepted (floats not yet built; division fix lands now)
- **Date:** 2026-07-23

## Context

Floats were waiting on exactly one genuine question: what does `7 / 2`
return once `3.5` is expressible? The candidates: Ruby's answer (`3` —
integer division stays integer), Python 3's answer (`3.5` — `/` always
real division), or a never-guess error on integer `/`.

Python's answer reuses Ruby's spelling with a quietly different value —
a straight violation of migration promise 1 (loud, never silent). The
never-guess error is loud but breaks working Ruby that means exactly
what it says, for an operation too common to tax. Tie goes to Ruby.

## Decision

**Ruby's rule, permanently: integer `/` integer is integer division,
and it is Ruby's *floored* division, not Rust's truncation.**

```ruby
7 / 2      # 3
-7 / 2     # -4  (floor, like Ruby — not -3)
7 % 2      # 1
-7 % 2     # 1   (sign of the divisor, like Ruby — not -1)
```

The seed's truncating `/` and `%` were flagged crude-on-purpose from day
one; this ADR retires the flag by fixing them to Ruby's semantics.

The rest of the floats package rides along, Ruby-match throughout, no
open questions:

- IEEE 754 doubles; `3.14` literals.
- Ruby's printing: a float always shows its point (`1.0`, not `1`).
- Mixed arithmetic promotes: `7 / 2.0` is `3.5`; comparison works across
  the two numeric types the way Ruby's does.
- `fdiv` / `div` arrive on demand, Ruby-named, when a real file pulls.

## Consequences

- The division/modulo fix is a seed change only — the trio's arithmetic
  is delegated to host operators, so the hosted evaluator inherits the
  fix; a fixture pins the negative cases differentially.
- Floats themselves remain a build day (lexer `3.14`, a float value
  type, promotion rules) — decided here, built when pulled for.
- Migration: `7 / 2 == 3` muscle memory holds verbatim, including the
  classic trap — which is Ruby's trap, kept on purpose; the trade was
  weighed against promise 1 and promise 1 won.
- STAGE0's "flagged to revisit" arithmetic note closes.
