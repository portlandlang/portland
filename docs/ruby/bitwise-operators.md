# Bitwise operators

**Summary:** Out of the grammar; named methods instead.

**Status:** tentative — leaning out ([ADR 0003](../adr/0003-2026-07-20-bitwise-operators-out.md)); `<<` as _append_ is explicitly undecided and travels with the mutable-values question.

## Ruby

Six operators for the bitwise family: `& | ^ ~ << >>`. Rare in application code, yet they cost real grammar: `&`-vs-`&&` precedence bugs, and `<<` is a three-way pileup (shift, append, heredoc opener) that complicates Ruby's lexer permanently.

## Portland

Bitwise **operators** are probably not in the grammar. The capability stays, as named methods that inline to identical machine instructions under AOT compilation — syntax is being declined, not capability:

```ruby
flags.bit_and(mask)
value.shift_left(3)
```

What the freed characters buy: `~` marks `together` tasks, `|` stays purely block parameters, `<<` is reserved for heredocs (and possibly append).

## Migration

- Application-shaped Ruby almost never contains bit math; most codebases migrate with zero contact.
- Where it exists, `x & y` → `x.bit_and(y)` is a mechanical linter autocorrect. Loud errors otherwise, never silent reinterpretation.
- Array's set operators go the same way, to names Ruby already has: `a | b` is `a.union(b)` and `a & b` is `a.intersection(b)`. `a - b` is not bitwise and stays, as does its twin `a.difference(b)`.
- `array << item` awaits the mutable-values decision — not ruled out, not promised.
