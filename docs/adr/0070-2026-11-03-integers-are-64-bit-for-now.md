# 0070 — Integers are 64-bit for now; past the edge refuses, and bignums stay possible

- **Status:** Accepted (ruled by the deciding user 2026-11-03, accepting the recommendation in [#156](https://github.com/portlandlang/portland/issues/156); recorded, not built — the behavior described is what both implementations already do)
- **Date:** 2026-11-03
- **Issue:** [#156](https://github.com/portlandlang/portland/issues/156)

## Context

Ruby's Integer has arbitrary precision: `2 ** 100` answers, and `9223372036854775807 + 1` quietly becomes a Bignum. Portland's integers are `i64`, and have been since the seed: a literal past the edge, `**` ([ADR 0033](0033-2026-08-11-exponent-is-starstar-and-pow.md)), `+ - *`, unary minus, `abs`, `succ`, `sum`, `floor`/`ceil` with a precision, a left shift ([ADR 0003](0003-2026-07-20-bitwise-operators-out.md)), and `to_i` ([ADR 0061](0061-2026-10-25-to-i-by-rubys-rule.md)) all refuse past it with `… overflows the 64-bit integers`. Some fifty entries in `spec/ruby_spec.yml` give "no bignums" as a reason with no ledger page to link, because no page said it. This ADR says it.

## Decision

1. **Integers are 64-bit, signed.** The range is −2⁶³ to 2⁶³ − 1, what the Apple silicon register holds and what an ahead-of-time compiler emits one instruction for (principle 1: the beautiful line is also the fast line, and here it is the same line).
1. **Past the edge refuses, never wraps and never widens.** Every arithmetic answer that would leave the range is a refusal in one sentence — `9223372036854775807 + 1 overflows the 64-bit integers` — on both implementations. No silent wraparound (C's), no silent promotion to a float (JavaScript's), no silent bignum (Ruby's). Principle 5: where Ruby's answer differs, the program fails to compile or run, loudly.
1. **A bignum stays an open possibility, and waiting costs nothing.** Nothing has pulled for arbitrary precision (principle 6), and because every past-`i64` answer today is a refusal, a later arbitrary-precision integer would turn refusals into answers and change no working program. The decision is reversible by construction; it is recorded so the reversal, if it comes, is a new ADR superseding this one rather than a quiet drift.
1. **Where approximation is fine, the rewrite is `to_f`.** `2.0 ** 100` answers; `factorial(25)` does not, and a program that needs it exactly is the pull this ADR waits for.

## Consequences

- Ledger: [integers.md](../ruby/integers.md), the page the manifest's fifty "no bignums" reasons can now link. The entries that said "wait on #156" point at it.
- The seed's `format` ([ADR 0067](0067-2026-10-25-format-is-a-function-not-the-percent-operator.md)) refuses a `%d` of a string or float past the edge with `integer … is past 64 bits`; aligning that sentence with `overflows the 64-bit integers` is a two-line change owed to the house wording, not made here since this ADR records and does not build. (Made later the same day: `format("%d", 1e19)` refuses with `1.0e+19 overflows the 64-bit integers`.)
- Sizing the cost is [ruby_research](https://github.com/portlandlang/ruby_research)'s: a corpus pass counting literals past `i64` and the patterns that overflow in practice (`**` with large exponents, factorials, hash and CRC arithmetic) would turn "nothing has pulled" into a number. Not run yet.
- [#156](https://github.com/portlandlang/portland/issues/156) closes; the reopening signal is a real file that needs an exact integer past 2⁶³.
