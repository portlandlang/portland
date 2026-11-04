# 0085 — Rational and Complex are core; BigDecimal is a default tile

- **Status:** Accepted (ruled by the deciding user 2026-11-03, reversing the recommendation in [#124](https://github.com/portlandlang/portland/issues/124) to defer — "build now"; not built — the builds are [#198](https://github.com/portlandlang/portland/issues/198) for Rational and Complex and [#199](https://github.com/portlandlang/portland/issues/199) for BigDecimal). The shape below is the author's reading of "build now" and is open to amendment.
- **Date:** 2026-11-03
- **Issue:** [#124](https://github.com/portlandlang/portland/issues/124)

## Context

Portland has `Integer` and `Float` ([ADR 0018](0018-2026-07-23-numbers-ruby-division.md), [ADR 0033](0033-2026-08-11-exponent-is-starstar-and-pow.md)). Ruby's numeric tower also has `Rational` and `Complex` in core, and `BigDecimal`, a bundled gem since Ruby 3.4 loaded by `require "bigdecimal"`. ruby/spec has `core/rational/` (155 examples), `core/complex/` (168), and `library/bigdecimal/` (291); a dozen manifest reasons already say "waits on #124". The recommendation was to defer until the corpus showed a pull; the deciding user chose to build.

## Decision

1. **`Rational` and `Complex` are builtin values in core**, with Ruby's spellings: `Rational(1, 3)`, `Complex(1, 2)`, the literal suffixes `3r`, `1.5r`, `2i`, and `to_r`, `to_c`, `rationalize`, `numerator`, `denominator`, `real`, `imaginary`, `abs`, `arg`, `polar`, `rectangular`, `conjugate`, and the arithmetic. The ruby/spec files are the specification; tie goes to Ruby.
1. **Arithmetic mixes as Ruby's does**: an integer with a rational is a rational, a rational with a float is a float, anything with a complex is a complex, and a rational is always reduced (`Rational(6, 4)` is `(3/2)`). `inspect` is Ruby's, `(3/2)` and `(1+2i)`.
1. **A rational's parts are 64-bit integers** ([ADR 0070](0070-2026-11-03-integers-are-64-bit-for-now.md)): a numerator or denominator past the edge refuses with `… overflows the 64-bit integers`, and the bignum rows stay upstream as they do for `Integer`.
1. **`Rational(1, 0)` refuses as `1 / 0` does today**, in Ruby's words, `divided by 0`; a float to a fractional power with a negative base answers a `Complex`, as Ruby's does, which is what `core/float/exponent_spec.rb` has been waiting on.
1. **`BigDecimal` is a default tile**, not core, by [ADR 0064](0064-2026-10-25-the-standard-library-core-default-tiles-external-tiles.md)'s rule — it is a bundled gem in Ruby, loaded by `require "bigdecimal"` — written in Portland with its own decimal digits, since the 64-bit integer cannot carry it. It is the tile system's first real customer after the four named in ADR 0064, and its build waits on the tile system ([#174](https://github.com/portlandlang/portland/issues/174)) for how it is loaded, not for whether. Until the tile system exists, `require "bigdecimal"` refuses pointing at [#199](https://github.com/portlandlang/portland/issues/199).

## Consequences

- Both implementations gain two numeric values, the two constructor functions, the three literal suffixes, and the mixed-arithmetic table; the checker learns the four-way number types.
- The manifest's dozen "waits on #124" reasons move to the two build issues; `core/rational/`, `core/complex/`, and the rational and complex rows of `core/numeric/`, `core/integer/`, `core/float/`, `core/random/`, and `core/enumerable/sum_spec.rb` port under [#198](https://github.com/portlandlang/portland/issues/198), `library/bigdecimal/` under [#199](https://github.com/portlandlang/portland/issues/199).
- Ledger: a new [numbers.md](../ruby/numbers.md), the page [#155](https://github.com/portlandlang/portland/issues/155) was going to add; its coercion question stays open there.
