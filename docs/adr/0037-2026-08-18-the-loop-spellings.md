# 0037 — The loop spellings: `until` in, postfix loops in, `loop` in after all

- **Status:** Accepted (built in both oracles, 2026-08-18; §3 reversed the same day — see its revision note)
- **Date:** 2026-08-18
- **Issue:** [#73](https://github.com/portlandlang/portland/issues/73) — three of its five spellings; `||=` and subjectless `case` stay open there, deliberately unruled

## Context

The ruby/spec import probed five spellings Portland had never ruled on ([#73](https://github.com/portlandlang/portland/issues/73)), all refusing as unknown syntax. Three are loop-shaped, and they triage together under principle 3's ratified direction — one behavior may have many spellings — plus the `unless` precedent: Portland already kept `if`'s negated twin.

## Decision

**1. `until` ships, as `while`'s negated twin.** `until done? … end` parses as a `while` whose condition is wrapped in `not` — one desugar in each parser, no new node, no evaluator work. The symmetry argument decided it: keeping `unless` while refusing `until` is an asymmetry migrating fingers trip on, and `until done?` reads forward where `while not done?` stutters. Ruby's one genuine `until` wart — `begin … end until` running the body before the first check — cannot import, because `begin` blocks do not exist.

**2. The postfix loop modifiers ship, both words.** `i += 1 while i < 3` and `attempt until connected?` parse as the block-form loop wrapping the one statement, joining postfix `if`/`unless` so the modifier family is whole. Same wart-shield as above: with no `begin`, a postfix loop is always a plain pre-checked loop — Ruby's hidden do-while mode has nothing to attach to. (This was the batch's weakest call, accepted for family completeness; it is one grammar production per parser and reverts cheaply if real Portland style never uses it.)

**3. `loop do … end` ships — as a keyword form desugaring to `while true`.** *Revised the same day it was decided: a first ruling declined it toward the rewrite (`loop is spelled while true here`), and the deciding user reversed within hours — the word is genuinely useful Ruby, liked, and wanted. The decline's own framing supplied the build: the rewrite it named is now the desugar.*

`loop` is a keyword, not a method (a departure from Ruby, where it is `Kernel#loop`), and `loop do … end` parses in both oracles to the `while` node with a `true` condition. That shape makes the sameness structural rather than promised: `break`, `next`, the per-iteration scope rule, and a `yield` in the body reaching the enclosing method's block are all `while`'s behaviors because it *is* a `while` — no new dispatch shape, no evaluator work, nothing to drift. Three consequences of the keyword choice, named:

- `def loop` is impossible — the word is reserved, like every loop-control word (`while`, `break`, `next`).
- `loop` is statement-position like `while`, so `x = loop do … end` refuses. Ruby's answers the `break` value; Portland's `break` carries none, so nothing real is lost.
- Only the `do` block shape exists; bare `loop` refuses with the wording both oracles share: `` `loop` takes a `do` block — write loop do ... end `` (stated here per ADR 0034 §1, pinned by the wording test).

Ruby's `loop` also quietly rescues `StopIteration`; that attachment rides the enumerator question ([#80](https://github.com/portlandlang/portland/issues/80)) and does not import.

## Consequences

- The import's probes for all three flip from refusals into examples.
- `until` and postfix loops are non-differences from Ruby (same meaning, verbatim); `loop`'s differences are the edges of the keyword choice — no `def loop`, no assigning the loop, no blockless form — recorded in [removed-syntax.md](../ruby/removed-syntax.md).
- Checker: nothing new — both spellings desugar to the `While` node the walker already visits.
- `||=` and subjectless `case` remain open in #73 on purpose: skipped, not decided, so the issue stays open holding exactly those two.
