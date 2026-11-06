# Numbers: the tower

**Summary:** A number is an Integer, a Float, a Rational, or a Complex — four builtin values with Ruby's spellings and Ruby's mixing rules — and BigDecimal is a default tile; there is no `Numeric` superclass to inherit from and no coercion protocol yet.

**Status:** Integer and Float built ([ADR 0018](../adr/0018-2026-07-23-numbers-ruby-division.md), [ADR 0033](../adr/0033-2026-08-11-exponent-is-starstar-and-pow.md), [ADR 0070](../adr/0070-2026-11-03-integers-are-64-bit-for-now.md)); Rational and Complex decided ([ADR 0085](../adr/0085-2026-11-03-rational-complex-and-bigdecimal.md), 2026-11-03, out of [#124](https://github.com/portlandlang/portland/issues/124)) and **built** on both implementations 2026-11-06 under [#198](https://github.com/portlandlang/portland/issues/198), with `core/rational/` and `core/complex/` in the manifest; BigDecimal decided as a tile, waiting on the tile system — [#199](https://github.com/portlandlang/portland/issues/199). Coercion is open ([#155](https://github.com/portlandlang/portland/issues/155)).

## Ruby

`Numeric` is the superclass of `Integer`, `Float`, `Rational`, and `Complex`, and `BigDecimal` joins the tower from a bundled gem. Mixed arithmetic follows a fixed contagion — integer with rational is rational, rational with float is float, anything with complex is complex — and a user's numeric type joins by defining `coerce`. `Rational(1, 3)`, `3r`, `Complex(1, 2)`, `2i`, `to_r`, `to_c`, `rationalize`; `(3/2)` and `(1+2i)` are the inspect forms.

## Portland

```ruby
Rational(1, 3) + 1         # (4/3)
Rational(1, 3) + 1.0       # 1.3333333333333333
Rational(6, 4)             # (3/2)
3r / 2                     # (3/2)
Complex(1, 2) * 2          # (2+4i)
(-8.0) ** (1.0 / 3)        # a Complex, as in Ruby
0.75.to_r                  # (3/4)
```

- **Four values, Ruby's spellings, Ruby's mixing.** `Rational(a, b)` and `Complex(a, b)`, the suffixes `3r`, `1.5r`, `2i`, `Complex.polar`, `Complex.rectangular`, `Complex::I`, and `to_r`, `to_c`, `rationalize`, `numerator`, `denominator`, `real`, `imaginary`, `abs`, `abs2`, `arg`, `polar`, `rectangular`, `conjugate`, `i` on every number; a rational is always reduced, a complex division's whole parts come back as integers, `2 ** -1` is `(1/2)` and `1.quo(2)` its spelling; `inspect` is Ruby's, `(1/3)` and `(1-(1/2)*i)`, and `to_s` the same without the parentheses. A complex argument to either constructor reads as Ruby's does: `Rational(Complex(1, 0))` is `(1/1)`, `Complex(Complex(3, 4), Complex(5, 6))` is `a + b * i`, and `Rational(1, Complex(2, 1))` is the quotient, a complex. The formulas are CRuby's step for step, so the digits are Ruby's — `rationalize` by its continued fractions, a complex power by its polar form.
- **The checker knows the four.** An integer with a rational types as a rational, anything with a complex as a complex; `<` on a complex refuses at build, as Ruby's raises, while `<=>` answers nil.
- **Rational parts are 64-bit** ([integers.md](integers.md)): a numerator or denominator past the edge refuses with `… overflows the 64-bit integers`. NaN and the infinities have no rational: `to_r` and `rationalize` refuse in Ruby's one word, `NaN` or `Infinity`, while `numerator` answers the float itself and `denominator` 1, as Ruby's do.
- **`Rational(1, 0)` refuses as `1 / 0` does**, `divided by 0`.
- **No `Numeric`.** There is no superclass to subclass or to mock ([mixins-and-inheritance.md](mixins-and-inheritance.md)); a number is one of the four, each with its own spec directory, and the manifest's `core/numeric/` rows that drive a mock subclass stay upstream.
- **No coercion protocol yet.** A user's type does not join the arithmetic by defining `coerce`; what a struct does in `1 + money` is [#155](https://github.com/portlandlang/portland/issues/155)'s question, deferred.
- **BigDecimal is a tile**, loaded by `require "bigdecimal"` as in Ruby ([standard-library.md](standard-library.md)), written in Portland with its own digits; until the tile system exists the require refuses pointing at its build.

## Migration

- Rational and complex arithmetic, literals, and conversions port verbatim. _Free._
- `round(2)` and `floor(2)` on a rational, and the `half:` modes, wait for a pull; the bare rounders are Ruby's. _Locked._
- `class Money < Numeric` and `def coerce` have no home; a struct with its own operators is the spelling, and mixed arithmetic with builtins waits on #155. _Rewrite, loud._
- `require "bigdecimal"` and `BigDecimal("1.1")` port verbatim once the tile exists. _Locked until #199._
