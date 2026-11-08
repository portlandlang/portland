# `Math`

**Summary:** `Math.sqrt`, `Math.log`, `Math::PI`, and the rest of Ruby's module functions are builtins spelled as Ruby spells them; a domain error is a failure in Ruby's own sentence, and `include Math` refuses.

**Status:** decided ([ADR 0083](../adr/0083-2026-11-03-math-is-rubys.md), 2026-11-03, out of [#126](https://github.com/portlandlang/portland/issues/126)) and built the same day on both implementations under [#196](https://github.com/portlandlang/portland/issues/196): every function in `core/math/`, ported whole into `spec/core/math/`, one file per upstream file, against Ruby 4.0.9's answers. `Integer.sqrt` was already built ([ADR 0058](../adr/0058-2026-10-16-builtin-types-carry-type-functions.md)). Two of Ruby's edges kept on purpose: `Math.log(x, 1)` answers NaN as Ruby's does, and `gamma` answers the exact factorial for the integers 1 through 23 as Ruby's table does.

## Ruby

`Math` is a module of module functions over floats — `sqrt`, `cbrt`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `sinh`, `cosh`, `tanh`, `asinh`, `acosh`, `atanh`, `exp`, `log`, `log2`, `log10`, `hypot`, `erf`, `erfc`, `gamma`, `lgamma`, `ldexp`, `frexp` — with `PI` and `E`. `Math.sqrt(-1)` raises `Math::DomainError: Numerical argument is out of domain - "sqrt"`; `Math.log(0)` answers `-Infinity`. `include Math` lets a class write `sqrt(x)` bare.

## Portland

```ruby
Math.sqrt(2)               # 1.4142135623730951
Math.hypot(3, 4)           # 5.0
Math::PI                   # 3.141592653589793
Math.sqrt(-1)              # failure("Numerical argument is out of domain - sqrt")
Math.log(0)                # -Infinity
```

- **Every function and both constants, Ruby's spellings, Ruby's answers.** Integers and floats go in, floats come out.
- **A domain error is a failure** ([errors.md](errors.md)), with Ruby's sentence, so a caller unwraps it as any failure: `Math.sqrt(x) or 0.0`.
- **`include Math` refuses** — a namespace is not a mixin ([namespaces.md](namespaces.md)): `Math is a namespace, not a trait — namespaces are never injected; write Math.method(...)`. `Math.sqrt` is the one spelling, which is Ruby's own convention made law.
- **A non-number argument refuses at build time**, where Ruby raises `TypeError` at the call.

## Migration

- `Math.name(...)` and `Math::PI` port verbatim. _Free._
- A `rescue Math::DomainError` becomes an unwrap of the failure. _Rewrite, loud._
- `include Math` with bare `sqrt(x)` becomes `Math.sqrt(x)`. _Mechanical._
