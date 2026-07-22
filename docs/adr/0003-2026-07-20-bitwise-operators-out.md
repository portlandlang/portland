# 0003 — Bitwise operators out of the grammar

- **Status:** Tentative (leaning out; not locked)
- **Date:** 2026-07-20
- **Issue:** [#3](https://github.com/portlandlang/portland/issues/3) (decided alongside the sigil)

## Context

Ruby spends six operators on the bitwise family: `& | ^ ~ << >>`. They are
rare in application-shaped code, the ASCII budget is precious (the sigil
hunt in ADR 0002 made that concrete), and `&`-vs-`&&` precedence bugs are a
classic footgun of the kind the design doc already cuts. Ruby's `<<` is a
three-way pileup (shift, append, heredoc opener) that complicates its lexer
permanently.

## Decision

Bitwise **operators** are probably not in the grammar. The capability is
preserved as named methods — `flags.bit_and(mask)`, `value.shift_left(3)` —
which inline to identical machine instructions under AOT compilation.
Syntax is being declined, not capability.

This frees `~` for `together` tasks (ADR 0002), keeps `|` purely for block
parameters, and reserves `<<` for heredocs.

## Explicitly TBD: `<<` as append

`Array#<<` / `String#<<` *append* is common and beloved in Ruby, and is a
separate question from bit-shift. It is **not ruled out**. Append implies
in-place mutation, so it must be decided together with the mutable-values
question deferred in ADR 0001 — the two travel as one future decision.

## Consequences

- Portland's lexer never inherits Ruby's `<<` ambiguity.
- If systems-level work later demands operator-form bit math, this ADR gets
  superseded — with the `~` sigil (0002) as a known constraint by then.
