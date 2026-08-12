# History

**For:** anyone digging into how Portland got here.

**Nothing in this folder is current, and nothing in it is a source of truth.** Every file here is dated writing, frozen the day it was published, and never revised. Where one of these disagrees with an [ADR](../adr/), the ADR wins. Where one describes what is built, the [language](../language.md) and [architecture](../architecture.md) docs win. Where one describes what is planned, [ROADMAP](../../ROADMAP.md) and the [issues](https://github.com/portlandlang/portland/issues) win.

That is the whole contract, and it is what makes these files safe to keep: they can never be wrong, only old. Read them for reasoning, context, and the shape of an argument at a moment in time — never for status.

Nothing here records what has happened since it was written — not in the files, and not in the index. That is the same rule stated twice: a note like "half of this has shipped" would have to change as more ships, and there is no place in a frozen folder for a line that changes.

## The record

<!-- generated: do not edit by hand — script/docs/generate -->

- [Portland — design notes](2026-06-28-first-brainstorm.md) — The session that started Portland: the seed question, joy-first, "Ruby, the good parts", the bootstrap ladder, and the branding bank.
- [Report: the decisions ahead](2026-07-22-open-decisions.md) — A map of every open decision at the close of the optionals arc, with options, tradeoffs, and a recommended order and answer for each.
- [Symbols: first pass](2026-07-23-symbols-first-pass.md) — The audit that tabled symbols: which of their jobs Portland's ADRs had already reassigned, and why the residue is enum-shaped.
- [pliron / MLIR hybrid — a backend architecture sketch](2026-07-24-pliron-mlir-hybrid.md) — A candidate #5 architecture: pliron hosts the Portland-specific middle IR in native Rust, real MLIR runs below a textual seam as pinned stock binaries and owns the hardware arrows.

<!-- /generated -->

Each line is that file's own summary. They describe what a document _is_, and deliberately not what has happened since — a frozen file cannot host a line like "half of this has shipped" without becoming a file that changes. For what is current, the contract above already points you at ROADMAP and the issues.
