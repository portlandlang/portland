# Integers: 64-bit, refusing past the edge

**Summary:** An integer is a signed 64-bit value; an answer that would leave −2⁶³ … 2⁶³ − 1 refuses with `… overflows the 64-bit integers` where Ruby quietly reaches for a Bignum — and a bignum stays possible, since every refusal could later become an answer.

**Status:** decided ([ADR 0070](../adr/0070-2026-11-03-integers-are-64-bit-for-now.md), 2026-11-03, out of [#156](https://github.com/portlandlang/portland/issues/156)); the behavior has been both implementations' since the seed, and this page is the record the manifest's "no bignums" reasons link.

## Ruby

Integer has arbitrary precision. `2 ** 100` is `1267650600228229401496703205376`; `9223372036854775807 + 1` is `9223372036854775808`, the type silently widening past the machine word (Fixnum and Bignum were unified into one Integer in 2.4). `Integer.sqrt(10 ** 400)`, `(10 ** 70).floor(-60)`, `25.downto(1).reduce(:*)` all answer exactly. The cost is a check on every arithmetic operation and a heap allocation past 62 bits.

## Portland

An integer is `i64`, what the register holds. Arithmetic that fits answers as Ruby's does — `+ - * / % **`, floored division, `abs`, `gcd`, `pow` with a modulus. Arithmetic that would not fit refuses, on both implementations, in one sentence:

```text
9223372036854775807 + 1 overflows the 64-bit integers
2 ** 64 overflows the 64-bit integers
```

The same sentence covers a literal past the edge, unary minus of −2⁶³, `abs` and `succ` at the ends, `sum` of an array that crosses, `floor` and `ceil` with a precision that would, a left shift past 64 bits ([bitwise-operators.md](bitwise-operators.md)), and `to_i` of a string with too many digits ([conversions.md](conversions.md)). Nothing wraps, nothing widens, nothing becomes a float. [ADR 0010](../adr/0010-2026-07-22-partial-operations-return-maybes.md)'s rule holds: the only crash is one you typed, and an overflow is a refusal of the program, not a sad path a caller handles — a `+` that may or may not fit is not a partial operation the way a lookup is.

Where an approximate answer is fine, the integer becomes a float first: `2.0 ** 100` is `1.2676506002282294e+30`.

**Why 64-bit, and why now.** An ahead-of-time compiler emits one instruction for `i64` addition and a function call with an allocation for a bignum, and Portland's thesis is that the beautiful line is also the fast one (principle 1). Nothing in the compiler, the spec suite, or the gem census has yet pulled for an exact integer past 2⁶³ (principle 6). And waiting is free: because today's answer past the edge is always a refusal, an arbitrary-precision integer added later would turn refusals into answers and change no working program. The record exists so that, if it comes, it comes as an ADR that supersedes [ADR 0070](../adr/0070-2026-11-03-integers-are-64-bit-for-now.md) rather than as a drift.

## Migration

- Everyday arithmetic — counters, indices, sizes, money in cents, timestamps in nanoseconds (2⁶³ ns is the year 2262) — ports verbatim, same answers. _Free._
- `2 ** 100`, factorials past 20, `Integer.sqrt(10 ** 400)`, hashing and CRC arithmetic written to rely on wraparound-free widening — refuse loudly at the operation; rewrite with `to_f` where approximation serves, or wait for the pull. _Locked until a bignum ADR._
- `x.bit_length`, `Integer::MAX`-style probes: `fixnum_max` is 2⁶³ − 1, the one edge there is.
- `Fixnum`/`Bignum` as names never existed here; the manifest's `core/integer/constants` row says so.
