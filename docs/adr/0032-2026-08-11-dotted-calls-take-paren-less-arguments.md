# 0032 — Dotted calls take paren-less arguments

- **Status:** Accepted (settling [#68](https://github.com/portlandlang/portland/issues/68); built on both oracles the same day, differentially pinned, specced)
- **Date:** 2026-08-11

## Context

Portland's paren-less command calls stopped at the dot: `puts "hello"` parsed, `expect(x).to eq(y)` did not. The gap was found by asking whether Portland could ever have rspec, and the surprise in the answer is that rspec's readability is mostly a *parsing* feature, not a metaprogramming one: `expect(this).to eq(that)` reads as a sentence because the chain is the grammar — `.to` is doing a preposition's job, the same instinct that put `.of` and `.from` on constructor entry points (ADR 0031). `assert_equal x, y` reads badly for exactly the missing reason: verb-object-object, no connective.

The pull is this repo's own language spec — 271 assertions writing `expect(x).to(eq(y))` with parens that exist only because the parse was missing — and it wants settling before any larger ruby/spec import, so the imported corpus can be written in the sentence form once.

## Decision

**`receiver.method argument` parses, as the mirror of the bare command call one dot deeper** — the same test for what starts an argument, the same argument loop (multiple arguments, keywords), the same refusals where two readings exist. Every acceptance is in previously-refused space, so no existing program changes meaning. The rules, each verified against Ruby 4.0.6 before adoption:

- **Nesting follows Ruby: adjacency nests, commas make siblings.** `a.to b.c d` gives `d` to the innermost call; `a.to x, y` gives both to `to`. One reading each.
- **A postfix chain binds to the argument**: `expect(x).to eq(1).and(y)` is `to(eq(1).and(y))` — Ruby's parse, one reading. Chaining off the outer call takes parens on the outer call.
- **A `do` block past the arguments belongs to the outermost call** — the rule bare commands already lean on. `[1, 2].reduce 10 do ... end` hands the block to `reduce`; inside another command's arguments, the inner dotted call leaves the `do` alone.
- **The never-guess shapes refuse, with the bare command's wordings**: `a.b -1` (Ruby itself warns "ambiguous first argument" here — the warning becomes a refusal), `a.b [x]` (argument or index), `a.b (x)` (argument list or parenthesized argument), and a bare `{` after an argument gets ADR 0016's whose-block menu. The rewrites spell the bare method name — `slice(-1)`, never `"x".slice(-1)` — because a receiver is an expression neither oracle can faithfully re-print.
- **Deliberately not included: bare commands as arguments.** `expect(x).to eq y` — the fully paren-less form — needs `eq y` to parse as a command in expression position, a wider change to the grammar of bare names that nothing yet pulls for. The matcher keeps its parens, which is rspec's own dominant style; the line waits for a real file to want crossing.

## Consequences

- The spec suite drops the wrapper parens — `expect(config).to have_key(:name)` — which was the point.
- **Mostly a non-difference**: the grammar adopted is Ruby's own parse for these forms. The divergences are the refusals — where Ruby warns or silently guesses, Portland errors with the menu — and two acceptance edges: `a.b (x)` and a space-then-`[` after a dot call, which Ruby accepts by guesswork and Portland now refuses. [The parentheses ledger](../ruby/parentheses.md) records both directions.
- One deliberate asymmetry stands until pulled for: nested *dotted* calls parse (`a.to b.c d`), nested *bare* commands do not (`a.to eq y`). Recorded here so its later removal is a decision, not a drift.
- The `command_arguments` discipline extends: the seed guards the `do` attach with its flag, the trio truncates the argument token list at the opener — different mechanisms, same rule, byte-identical outcomes, pinned differentially.
