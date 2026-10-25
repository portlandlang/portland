# Bitwise operators

**Summary:** Out of the grammar; named methods instead — `bit_and`, `bit_or`, `bit_xor`, `bit_not`, `shift_left`, `shift_right`, and Ruby's own `allbits?`, `anybits?`, `nobits?`, `bit_length`, `bit_count`, and `n[i]`.

**Status:** decided ([ADR 0003](../adr/0003-2026-07-20-bitwise-operators-out.md), locked 2026-10-25 out of [#157](https://github.com/portlandlang/portland/issues/157)); built in the seed and the compiler, differentially pinned. `<<` is the append operator ([ADR 0015](../adr/0015-2026-07-23-values-never-mutate.md)).

## Ruby

Six operators for the bitwise family: `& | ^ ~ << >>`. Rare in application code, yet they cost real grammar: `&`-vs-`&&` precedence bugs, and `<<` is a three-way pileup (shift, append, heredoc opener) that complicates Ruby's lexer permanently.

## Portland

Bitwise **operators** are probably not in the grammar. The capability stays, as named methods that inline to identical machine instructions under AOT compilation — syntax is being declined, not capability:

```ruby
flags.bit_and(mask)       # flags & mask
flags.bit_or(mask)        # flags | mask
flags.bit_xor(mask)       # flags ^ mask
flags.bit_not             # ~flags
value.shift_left(3)       # value << 3   — a negative count shifts right
value.shift_right(3)      # value >> 3
flags.allbits?(mask)      # Ruby's own names, unchanged
flags.anybits?(mask)
flags.nobits?(mask)
value.bit_length
value.bit_count
value[3]                  # the bit at position 3, two's complement
```

A left shift past 64 bits refuses — integers are 64-bit here ([#156](https://github.com/portlandlang/portland/issues/156)) — and `bit_count` on a negative refuses, as Ruby's does.

What the freed characters buy: `~` marks `together` tasks, `|` stays purely block parameters, `<<` is reserved for heredocs (and possibly append).

## Migration

- Application-shaped Ruby almost never contains bit math; most codebases migrate with zero contact.
- Where it exists, `x & y` → `x.bit_and(y)` is a mechanical linter autocorrect, and the refusal names it: `'&' is a bitwise operator, which is out of the grammar (ADR 0003) — write 'bit_and', or 'intersection' on arrays`; `^` and `>>` say `bit_xor` and `shift_right` the same way. Loud errors otherwise, never silent reinterpretation.
- Array's set operators go the same way, to names Ruby already has: `a | b` is `a.union(b)` and `a & b` is `a.intersection(b)`. `a - b` is not bitwise and stays, as does its twin `a.difference(b)`.
- `array << item` awaits the mutable-values decision — not ruled out, not promised.
