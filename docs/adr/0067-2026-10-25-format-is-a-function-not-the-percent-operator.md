# 0067 — Formatting is `format(template, values)`, not `template % values`

- **Status:** Accepted, not yet built (ruled by the deciding user 2026-10-25 on the recommendation in [#172](https://github.com/portlandlang/portland/issues/172), choosing its option 2; the build waits for a pull, and [#172](https://github.com/portlandlang/portland/issues/172) stays open as the buildable ticket)
- **Date:** 2026-10-25
- **Issue:** [#172](https://github.com/portlandlang/portland/issues/172)

## Context

Ruby's `"%05.2f" % value` and `format("%-10s|", name)` share one mini-language — flags, width, precision, `%{name}` and `%<name>d` references, `*` widths from the arguments — spelled two ways: as the `%` operator on a string, and as the functions `format` and `sprintf`. ruby/spec's core/string/modulo_spec.rb carries 170 rows of the operator form. Today `"%d" % 1` refuses as a wrong operand to `%`.

The directives are worth keeping: tie goes to Ruby (principle 2), and migrating Ruby leans on them for every aligned table and padded number — `rjust`, `round`, and interpolation do not cover fixed precision, signs, exponents, and column alignment in one template. The operator is the question. `total % 3` is remainder on a number and `"%d" % total` is formatting on a string, decided by the left operand's type — the operand-typed overloading the promotion rule of [ADR 0018](0018-2026-07-23-numbers-ruby-division.md) otherwise keeps out of the arithmetic operators.

## Decision

1. **Formatting is a function, `format(template, values...)`**, carrying Ruby's directives whole: `%d %i %f %e %g %s %x %o %b %c %%`, the flags `-`, `+`, a space, `0`, and `#`, width and precision, `*` from the arguments, and the named references `%{name}` and `%<name>d` from keyword arguments. `sprintf` is its twin (principle 3). What Ruby's `format` answers, this answers.
1. **`%` stays arithmetic.** `"%d" % x` refuses naming the rewrite: `'%' on a string is formatting, which is spelled 'format("%d", x)' here`.
1. **Not yet built.** This ADR records the ruling; the build arrives when pulled for, with core/string/modulo_spec.rb's rows ported against `format` and a two-oracle pin.

## Consequences

- `format(...)` and `sprintf(...)` port verbatim; `template % values` is a mechanical rewrite rubocop-portland can make, since `template % values` with a string left side has no other reading.
- The `%<name>d` hash form becomes keyword arguments, Ruby 3's own spelling for `format`.
- Recorded in [conversions.md](../ruby/conversions.md).
