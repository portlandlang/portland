# History

_For: anyone digging into how Portland got here._

**Nothing in this folder is current, and nothing in it is a source of truth.** Every file here is dated writing, frozen the day it was published, and never revised. Where one of these disagrees with an [ADR](../adr/), the ADR wins. Where one describes what is built, the [language](../language.md) and [architecture](../architecture.md) docs win. Where one describes what is planned, [ROADMAP](../../ROADMAP.md) and the [issues](https://github.com/portlandlang/portland/issues) win.

That is the whole contract, and it is what makes these files safe to keep: they can never be wrong, only old. Read them for reasoning, context, and the shape of an argument at a moment in time — never for status.

Status notes live in the index below rather than in the files themselves, so the writing stays frozen.

## The record

- [2026-06-28 — the first brainstorm](2026-06-28-first-brainstorm.md) The session that started Portland: the seed question, joy-first, "Ruby, the good parts", the bootstrap ladder, and the branding bank. Its living argument now lives in the [README](../../README.md) and [architecture](../architecture.md); the decisions it bikeshedded (the mutability keyword, the absence word, the task sigil) are all settled in ADRs 0001–0021.

- [2026-07-22 — the decisions ahead](2026-07-22-open-decisions.md) A map of every open decision at the close of the optionals arc, with options, recommendations, and a suggested order. Its first half has shipped: #22 (ADR 0012), #20 (0013), keyword arguments (0014), methods in struct bodies, and #10 (0015). Its second half — error handling, `together` semantics, the full object model, type inference, the compile pipeline — is still live, and tracked on the ROADMAP and the issues rather than here.

- [2026-07-23 — symbols, first pass](2026-07-23-symbols-first-pass.md) The audit that tabled symbols: what jobs Ruby's symbols do, which of them Portland's existing ADRs had already reassigned, and why the residue is enum-shaped. The core question has since been decided — `:foo` exists as a general type, checked for membership where a closed vocabulary is declared — but the ADR still waits on the enum shape, so the corpus questions listed here remain the homework.
