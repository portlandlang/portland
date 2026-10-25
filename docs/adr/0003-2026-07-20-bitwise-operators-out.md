# 0003 — Bitwise operators out of the grammar

- **Status:** Accepted (locked 2026-10-25 by the deciding user on the recommendation in [#157](https://github.com/portlandlang/portland/issues/157); the named methods built in the seed and the trio, differentially pinned, 2026-10-25)
- **Date:** 2026-07-20, locked 2026-10-25
- **Issue:** [#3](https://github.com/portlandlang/portland/issues/3) (decided alongside the sigil); [#157](https://github.com/portlandlang/portland/issues/157) (the names)

## Context

Ruby spends six operators on the bitwise family: `& | ^ ~ << >>`. They are rare in application-shaped code, the ASCII budget is precious (the sigil hunt in ADR 0002 made that concrete), and `&`-vs-`&&` precedence bugs are a classic footgun of the kind the design doc already cuts. Ruby's `<<` is a three-way pileup (shift, append, heredoc opener) that complicates its lexer permanently.

## Decision

Bitwise **operators** are not in the grammar. The capability is preserved as named methods — `flags.bit_and(mask)`, `value.shift_left(3)` — which inline to identical machine instructions under AOT compilation. Syntax is being declined, not capability.

This frees `~` for `together` tasks (ADR 0002), keeps `|` purely for block parameters, and reserves `<<` for heredocs.

### The names (locked 2026-10-25, #157)

1. **The six operators keep their Ruby method names where Ruby has a name, and take the ADR's own where it has only a glyph**: `&` is `bit_and`, `|` is `bit_or`, `^` is `bit_xor`, `~` is `bit_not`, `<<` is `shift_left`, `>>` is `shift_right`. One family, so rubocop-portland can autocorrect `x & y` to `x.bit_and(y)` mechanically. A shift by a negative count shifts the other way, as Ruby's does; a left shift past 64 bits refuses, since integers are 64-bit here (#156).
1. **Ruby's own named bit methods arrive as Ruby spells them**: `allbits?`, `anybits?`, `nobits?`, `bit_length`, and `bit_count` (Ruby 4.1's; a negative receiver refuses).
1. **`Integer#[]` reads a bit**, two's complement, as Ruby's does: `0b101[2]` is 1, `(-1)[100]` is 1, `5[-1]` is 0. The two-argument and range forms stay unbuilt until pulled for.
1. **Each operator refuses with its rewrite named** where the glyph still lexes as nothing else: `'&' is a bitwise operator, which is out of the grammar (ADR 0003) — write 'bit_and', or 'intersection' on arrays`, and the same for `^` and `>>`. `|` is a block parameter's wall, `~` a task's marker, `<<` the append, so those three say what they are.

## Explicitly TBD: `<<` as append

`Array#<<` / `String#<<` _append_ is common and beloved in Ruby, and is a separate question from bit-shift. It is **not ruled out**. Append implies in-place mutation, so it must be decided together with the mutable-values question deferred in ADR 0001 — the two travel as one future decision.

## Consequences

- Portland's lexer never inherits Ruby's `<<` ambiguity.
- If systems-level work later demands operator-form bit math, this ADR gets superseded — with the `~` sigil (0002) as a known constraint by then.
