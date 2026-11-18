# 0089 — ADRs are decision records; the reference docs win

<!-- generated: do not edit by hand — script/docs/generate -->

> **A decision record, not a reference.** It says why a ruling was made; a later ruling may have changed it without marking this file. See also: [AGENT.md: Read in this order](../../AGENT.md#read-in-this-order).

<!-- /generated -->

- **Status:** Accepted (ruled by the deciding user 2026-11-18 on [#288](https://github.com/portlandlang/portland/issues/288))
- **Date:** 2026-11-18
- **Issue:** [#288](https://github.com/portlandlang/portland/issues/288)

## Context

The decision log was meant to be read as a whole: a later ADR overrides an earlier one, and the latest ruling is the truth. In practice a single ADR gets cited as the reference for how something works, and a partial override never marks the older ADR, so its Status still reads Accepted. ADRs 0002, 0004, and 0011 said `together` was unbuilt for four months after ADR 0029 built it, and nothing pointed a reader from them to what was true.

AGENT.md made this worse: "ADRs beat everything except a newer ADR" sent a reader to a single ADR as an authority.

## Decision

1. **The reference docs hold what is true, each fact in one:** [docs/language.md](../language.md) for what Portland speaks, [docs/architecture.md](../architecture.md) for how it is built, [docs/ruby/](../ruby/) for what each difference costs a Rubyist, and [docs/principles.md](../principles.md) for how arguments are settled. [AGENT.md](../../AGENT.md) holds the working conventions. When one of these and an ADR disagree, the reference doc wins.
1. **ADRs are the record of why**, in the way the CHANGELOG is the record of what. Their decision text is never rewritten; a ruling that changes gets a new ADR.
1. **A new ADR lands with its edit to the reference doc** in the same change, as a Ruby-divergent ADR already lands with its ledger page.
1. **Every ADR opens with a generated banner** saying it is a decision record, not a reference, and linking each reference section that cites it. `script/docs/generate` writes the banners from those citations, and `script/docs/check generated` fails when one is out of date or when no reference doc cites an ADR at all.
1. **Time-relative words stay out of the banner**, as they will out of the docs generally: it says "See also", not "today" or "current".

This is a non-difference from Ruby: it governs the project's documents, not the language.

## Consequences

- AGENT.md's reading order puts the reference docs above the ADRs, and drops "ADRs beat everything except a newer ADR".
- A reader who lands on an ADR from a search sees, before its text, where the reference answer lives.
- An ADR that no reference doc cites cannot pass the docs check, so a decision cannot be recorded only in the log.
- The banner links whole ledger pages and the sections of language.md, architecture.md, principles.md, and AGENT.md, since those are long and the ledger pages are one topic each.
