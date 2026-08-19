# 0037 — The loop spellings: `until` in, postfix loops in, `loop` declined

- **Status:** Accepted (built in both oracles, 2026-08-18)
- **Date:** 2026-08-18
- **Issue:** [#73](https://github.com/portlandlang/portland/issues/73) — three of its five spellings; `||=` and subjectless `case` stay open there, deliberately unruled

## Context

The ruby/spec import probed five spellings Portland had never ruled on ([#73](https://github.com/portlandlang/portland/issues/73)), all refusing as unknown syntax. Three are loop-shaped, and they triage together under principle 3's ratified direction — one behavior may have many spellings — plus the `unless` precedent: Portland already kept `if`'s negated twin.

## Decision

**1. `until` ships, as `while`'s negated twin.** `until done? … end` parses as a `while` whose condition is wrapped in `not` — one desugar in each parser, no new node, no evaluator work. The symmetry argument decided it: keeping `unless` while refusing `until` is an asymmetry migrating fingers trip on, and `until done?` reads forward where `while not done?` stutters. Ruby's one genuine `until` wart — `begin … end until` running the body before the first check — cannot import, because `begin` blocks do not exist.

**2. The postfix loop modifiers ship, both words.** `i += 1 while i < 3` and `attempt until connected?` parse as the block-form loop wrapping the one statement, joining postfix `if`/`unless` so the modifier family is whole. Same wart-shield as above: with no `begin`, a postfix loop is always a plain pre-checked loop — Ruby's hidden do-while mode has nothing to attach to. (This was the batch's weakest call, accepted for family completeness; it is one grammar production per parser and reverts cheaply if real Portland style never uses it.)

**3. `loop do … end` is declined for now, with a survivor-naming refusal:** `loop is spelled while true here`. The word buys a synonym for a spelling that is already clear, at the cost of a new dispatch shape (the first block-only free function). And it is the one spelling in the batch user code can simply define when it wants it — `def loop` + `while true` + `yield` works today, `break` already unwinds correctly through a yielding method (ADR 0025) — so its natural home is the stdlib story ([#78](https://github.com/portlandlang/portland/issues/78)), not the grammar. Ruby's `loop` also quietly rescues `StopIteration`; that attachment rides the enumerator question ([#80](https://github.com/portlandlang/portland/issues/80)) and does not import regardless.

The refusal wording is stated here because no oracle produces it (ADR 0034 §1) — both oracles must say it byte-for-byte, and the wording test pins it.

## Consequences

- The import's probes for `until` and the postfix modifiers flip from refusals into examples; `loop`'s flips into a pinned refusal.
- `until` and postfix loops are non-differences from Ruby (same meaning, verbatim); `loop`'s decline is the difference, recorded in [removed-syntax.md](../ruby/removed-syntax.md) with its rewrite.
- Checker: nothing new — both spellings desugar to the `While` node the walker already visits.
- `||=` and subjectless `case` remain open in #73 on purpose: skipped, not decided, so the issue stays open holding exactly those two.
