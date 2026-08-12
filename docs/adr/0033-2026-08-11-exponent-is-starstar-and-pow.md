# 0033 — Exponent is `**` and `pow`, and a negated base needs parens

- **Status:** Accepted (built on both oracles the same day, differentially pinned, specced)
- **Date:** 2026-08-11

## Context

The ruby/spec import ([#23](https://github.com/portlandlang/portland/issues/23)) reached `core/integer`'s exponent specs and found `**` did not parse. The open question was whether ADR 0003's precedent — bitwise operators became named methods — applied here too. It does not: that decision was about collision and category (`&` and `|` shadowing the boolean world, `<<` fighting append, bit-twiddling rare in application code), and none of it indicts exponent. `**` is ordinary arithmetic in `*`'s own family, collides with nothing, and Ruby itself ships both spellings — `Integer#pow` is real Ruby — so both-exist is a non-difference, not a compromise.

One genuine design question hides inside: **`-2 ** 2`**. The survey, run rather than recalled where the tools were at hand:

| answer              | who                                                                                                                          |
| ------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| `-4` — math's `-x²` | Fortran (which invented `**`), Python, Ruby, Perl (verified); Lua, Julia, PHP, Haskell's `^` (recalled); **macOS Spotlight** |
| `+4`                | **every spreadsheet**: Excel, Google Sheets, Apple Numbers                                                                   |
| refuses             | **JavaScript** — ES2016 made the parenthesis mandatory                                                                       |

Apple ships both answers, depending on whether you typed into Spotlight or Numbers. ES2016 read that split as grounds to require the parenthesis, and a first draft of this ADR followed it — refusing every negated base. Review rejected that draft, on an argument worth keeping: **a declared precedence is one grammar reading, not two.** Never-guess (principle 3) is about spellings the grammar could take two ways; `1 + 2 * 3` is also misread by someone somewhere and nobody refuses it. Tie-goes-to-Ruby (principle 2) outranks it besides, and Ruby, Python, Fortran, and mathematics all agree what `-x²` means.

One corner survives the review, because there Ruby disagrees *with itself*. Probed, not recalled: Ruby's negative-literal fusion gives `-5.abs` the answer `5`, and its `**` precedence gives `-5 ** 2` the answer `-25` — but `-5.abs ** 2` answers `25`, the fusion winning where the plain form let `**` win. Add `.abs` to a base and the whole expression's sign flips. Following that faithfully imports a rule nobody can recite; following the clean precedence instead would make the same spelling answer differently here than in Ruby, silently — principle 5's exact shape. Where upstream is incoherent, the only honest move is the menu.

## Decision

- **`**` exists**: right-associative (`2 ** 3 ** 2` is 512 — the tower reading, mathematics' own convention, so it is recitable) and binding above `*`. `pow` is its named twin, one arity; the modular second argument waits to be pulled for. `**=` is not included; nothing pulls for it.
- **A minus before `**` applies last** — one recitable rule, Ruby's and mathematics' own: `-2 ** 2` is `-4`, `-x ** 2` is `-(x ** 2)`, and `(-2) ** 2` stays the spelling for raising the negative whole. The literal fusion (`-5.abs` is 5) yields to a following `**`.
- **Only the chained negative literal refuses** — `-5.abs ** 2`, the corner where Ruby's fusion and Ruby's precedence contradict each other — with the menu: `a chained negative literal under ** is ambiguous — write ((-2).abs) ** 2 or -(2.abs ** 2)`.
- **Two integer edges refuse where Ruby answers**, each naming its rewrite: a negative integer exponent is a `Rational` in Ruby and Portland has no rationals — `2 ** -1 is a fraction, and integers have none — write 2.0 ** -1 for the float` — and a past-i64 result refuses like a past-i64 literal does: `2 ** 100 overflows the 64-bit integers`. Magnitude-one bases (`0`, `1`, `-1`) answer Ruby's values at any tower height rather than tripping the width check.
- **Floats ride `powf`**: `9 ** 0.5` is `3.0`, `2.0 ** -1` is `0.5`, mixed operands promote as all arithmetic does (ADR 0018).

## Consequences

- A non-difference for every spelling but three: `-2 ** 2` and kin mean exactly what Ruby means. The divergences are the refusals — the chained negative literal, the rational, the overflow — each loud with its rewrite, recorded in [the parentheses ledger](../ruby/parentheses.md). Retired on the way: the trio never fused negative *float* literals, so `-2.5.abs` silently answered `-2.5` hosted and `2.5` direct — the `**` work made the gap loud, and both oracles fuse now.
- The trio pays nothing at runtime: its `**` and `pow` ride the host operator whole, so the seed's edge wordings reach hosted programs automatically — pinned by an `evaluator_exponents` differential and three wording cases.
- The Integer import's exclusion note flips into examples; `core/integer/pow_spec.rb` and `exponent_spec.rb` are importable.
- **A ruby_research pass should size the refused corner** — how often real gems write a negated base under `**` at all, and the chained-literal form in particular — so the refusal's cost is a number rather than a guess. Queued with the other corpus questions on [#23](https://github.com/portlandlang/portland/issues/23)'s arc.
