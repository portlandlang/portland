# 0061 — `to_i` by Ruby's rule: leading digits count, garbage is 0

- **Status:** Accepted (ruled by the deciding user 2026-10-25 on the recommendation in [#162](https://github.com/portlandlang/portland/issues/162); built in the seed and the trio, differentially pinned, 2026-10-25)
- **Date:** 2026-10-25
- **Issue:** [#162](https://github.com/portlandlang/portland/issues/162)

## Context

Ruby's `String#to_i` never fails: it skips leading whitespace, reads a sign, an optional base prefix, and digits with single underscores between them, and answers 0 when nothing parses — `"12abc".to_i` is 12, `"abc".to_i` is 0. `Integer("abc")` is the strict form, which raises. The seed's `to_i` refused anything Rust's parser could not read whole, a choice made in the first lexer and never written down, while its `to_f` already followed Ruby's lenient rule ([#106](https://github.com/portlandlang/portland/issues/106)); the two conversions disagreed with each other.

## Decision

1. **`to_i` reads by Ruby's rule, with `to_i(base)`.** Leading whitespace, one sign, the prefix of the base asked for (`0b`, `0o`, `0d`, `0x`; with base 0, any of them, or a bare `0` for octal), then digits of the base with single underscores between digits; the first thing that is none of these ends the number. Nothing parsed is 0. Tie goes to Ruby (principle 2): migrating code leans on `"8080".to_i` and `"3 apples".to_i` both, and 0 for garbage is the one Ruby behavior everyone already guards.
1. **A base outside 2 to 36 refuses in Ruby's words**, `invalid radix 1`; base 0 asks the prefix.
1. **Past 64 bits refuses.** Ruby reaches for a bignum; integers are 64-bit here ([#156](https://github.com/portlandlang/portland/issues/156)), so a number that will not fit refuses rather than answering a wrong one.
1. **The strict forms wait.** `Integer(text)` and `Float(text)`, the type functions that refuse garbage, arrive in [ADR 0058](0058-2026-10-16-builtin-types-carry-type-functions.md)'s shape when a real port pulls for them.

## Consequences

- `to_i` and `to_f` agree with each other and with Ruby; `hex` and `oct` already read this way.
- Recorded in [conversions.md](../ruby/conversions.md).
