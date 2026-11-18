# 0089 — ADRs are history; the current-truth docs win

<!-- generated: do not edit by hand — script/docs/generate -->

> **History, not current truth.** This records why a ruling was made when it was; a later one may have changed it without marking this file. What holds today: [AGENT.md: Read in this order](../../AGENT.md#read-in-this-order).

<!-- /generated -->

- **Status:** Accepted (ruled by the deciding user 2026-11-18 on [#288](https://github.com/portlandlang/portland/issues/288))
- **Date:** 2026-11-18
- **Issue:** [#288](https://github.com/portlandlang/portland/issues/288)

## Context

The decision log was meant to be read as a whole: a later ADR overrides an earlier one, and the latest ruling is the truth. In practice a single ADR gets cited as the reference for how something works, and a partial override never marks the older ADR, so its Status still reads Accepted. ADRs 0002, 0004, and 0011 said `together` was unbuilt for four months after ADR 0029 built it, and nothing pointed a reader from them to what was true.

AGENT.md made this worse: "ADRs beat everything except a newer ADR" sent a reader to a single ADR as an authority.

## Decision

1. **The current truth lives in four places, each fact in one:** [docs/language.md](../language.md) for what Portland speaks, [docs/architecture.md](../architecture.md) for how it is built, [docs/ruby/](../ruby/) for what each difference costs a Rubyist, and [docs/principles.md](../principles.md) for how arguments are settled. [AGENT.md](../../AGENT.md) holds the working conventions. When one of these and an ADR disagree, the current-truth doc wins and the ADR is history.
1. **ADRs are the record of why**, in the way the CHANGELOG is the record of what. Their decision text is never rewritten; a ruling that changes gets a new ADR.
1. **A new ADR lands with its edit to the current-truth doc** in the same change, as a Ruby-divergent ADR already lands with its ledger page.
1. **Every ADR opens with a generated banner** saying it is history and linking each current-truth section that cites it. `script/docs/generate` writes the banners from those citations, and `script/docs/check generated` fails when one is out of date or when no current-truth doc cites an ADR at all.

This is a non-difference from Ruby: it governs the project's documents, not the language.

## Consequences

- AGENT.md's reading order puts the current-truth docs above the ADRs, and drops "ADRs beat everything except a newer ADR".
- A reader who lands on an old ADR from a search sees, before its text, where the current answer lives.
- An ADR that no current-truth doc cites cannot pass the docs check, so a decision cannot be recorded only as history.
- The banner links whole ledger pages and the sections of language.md, architecture.md, and principles.md, since those three are long and the ledger pages are one topic each.
