# 0033 — Exponent is `**` and `pow`, and a negated base needs parens

- **Status:** Accepted (built on both oracles the same day, differentially pinned, specced)
- **Date:** 2026-08-11

## Context

The ruby/spec import ([#23](https://github.com/portlandlang/portland/issues/23)) reached `core/integer`'s exponent specs and found `**` did not parse. The open question was whether ADR 0003's precedent — bitwise operators became named methods — applied here too. It does not: that decision was about collision and category (`&` and `|` shadowing the boolean world, `<<` fighting append, bit-twiddling rare in application code), and none of it indicts exponent. `**` is ordinary arithmetic in `*`'s own family, collides with nothing, and Ruby itself ships both spellings — `Integer#pow` is real Ruby — so both-exist is a non-difference, not a compromise.

One genuine design question hides inside: **`-2 ** 2`**. The survey, run rather than recalled where the tools were at hand:

| answer | who |
| ------------------ | --------------------------------------------------------------------------- |
| `-4` — math's `-x²` | Fortran (which invented `**`), Python, Ruby, Perl (verified); Lua, Julia, PHP, Haskell's `^` (recalled); **macOS Spotlight** |
| `+4` | **every spreadsheet**: Excel, Google Sheets, Apple Numbers |
| refuses | **JavaScript** — ES2016 made the parenthesis mandatory |

Apple ships both answers, depending on whether you typed into Spotlight or Numbers. That split is not language nerds versus civilians — it is the two most-used casual-math surfaces on earth contradicting each other, which is TC39's own stated reason for refusing. The newest design, looking at all the evidence, chose never-guess.

Ruby answers `-4`, and tie-goes-to-Ruby (principle 2) sits above never-guess (principle 3) — but the house already resolved this exact shape once: `puts -1` is Ruby-legal and Portland refuses it. A sign Ruby resolves by guessing is the established exception. And there is a sharper local reason: Portland's negative-literal rule (`-5.abs` is 5) gives `-5.abs ** 2` the answer `25` where Ruby's precedence gives `-25` — the same spelling meaning different things, which principle 5 forbids to compile at all.

## Decision

- **`**` exists**: right-associative (`2 ** 3 ** 2` is 512 — the tower reading, mathematics' own convention, so it is recitable) and binding above `*`. `pow` is its named twin, one arity; the modular second argument waits to be pulled for. `**=` is not included; nothing pulls for it.
- **A negated base under `**` refuses**: `-2 ** 2`, `-x ** 2`, and `-5.abs ** 2` all say `a negated base under ** is ambiguous — write (-2) ** 2 or -(2 ** 2)`. The scope is the base only — `2 ** -1` has one reading everywhere (ES2016 agrees) and stays legal.
- **Two integer edges refuse where Ruby answers**, each naming its rewrite: a negative integer exponent is a `Rational` in Ruby and Portland has no rationals — `2 ** -1 is a fraction, and integers have none — write 2.0 ** -1 for the float` — and a past-i64 result refuses like a past-i64 literal does: `2 ** 100 overflows the 64-bit integers`. Magnitude-one bases (`0`, `1`, `-1`) answer Ruby's values at any tower height rather than tripping the width check.
- **Floats ride `powf`**: `9 ** 0.5` is `3.0`, `2.0 ** -1` is `0.5`, mixed operands promote as all arithmetic does (ADR 0018).

## Consequences

- A non-difference for every unsigned spelling, in both notations. The divergences are three refusals — the negated base, the rational, the overflow — each loud with its rewrite, recorded in [the parentheses ledger](../ruby/parentheses.md).
- The trio pays nothing: its `**` and `pow` ride the host operator whole, so the seed's edge wordings reach hosted programs automatically — pinned by an `evaluator_exponents` differential and three wording cases.
- The Integer import's exclusion note flips into examples; `core/integer/pow_spec.rb` and `exponent_spec.rb` are importable.
