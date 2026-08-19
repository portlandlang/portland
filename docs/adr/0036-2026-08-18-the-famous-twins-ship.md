# 0036 — The famous twins ship

- **Status:** Accepted (built in both oracles, 2026-08-18)
- **Date:** 2026-08-18
- **Issue:** [#79](https://github.com/portlandlang/portland/issues/79)

## Context

The ruby/spec import kept meeting the same question at different types: does Portland ship Ruby's alias families — `size` beside `length`, `inject` beside `reduce`? The question stalled behind a philosophy error: "one name per job" had been written down as the house lean, and it never was one. Principle 3 now records the actual rule — **one spelling never has two behaviors; one behavior may have many spellings** — synonyms being one of Ruby's good parts, kept for joy and migration both. `succ`/`pred`/`next` shipped on exactly that argument.

That left scope: which families? The menu had narrowed to shipping the famous pairs outright versus corpus-gating each family through [ruby_research](https://github.com/portlandlang/ruby_research). Corpus-gating was rejected for this call — it produces an uneven surface nobody can recite and re-litigates every future twin — though the corpus stays the tool for *individual* contested names later.

## Decision

**The six families the import met ship as plain twin builtins**, no `alias` machinery:

| job | survivor | twins shipped |
| --- | --- | --- |
| how many | `length` | `size` (String, Array, Hash, Range) |
| is this key there | `key?` | `has_key?`, `include?`, `member?` (Hash) |
| first block hit | `find` | `detect` |
| transform each | `map` | `collect` |
| fold | `reduce` | `inject` — takes the initial argument `reduce` takes; no first-element-as-seed form |
| membership | `include?` | `member?` (Array) |

`count` on a Range joins `size` there (both answer the element count), since the import had noted both against this decision.

**Twins cannot drift, by construction.** In the seed each twin is an or-pattern on the survivor's own match arm (`"map" | "collect"`) — one body, two names. In the compiler each twin rides the survivor's host builtin, or the host twin where the survivor depends on the receiver's type (`member?`). There is no second implementation anywhere for behavior to hide in.

**Beyond the six: an unshipped alias refuses by naming the survivor.** The wording shape is `detect is spelled find here` — the refusal is the documentation. New twins arrive one ruling at a time, corpus-in-hand where contested; the famous six needed no report because the reflex is the evidence.

**Separate and deliberately later:** the wish that Ruby's names weren't abbreviations (`succ`, `pred`). Unabbreviated spellings are a possible stdlib expansion someday; nothing here blocks or starts it.

## Consequences

- Migration reflexes (`size`, `inject`, `collect`) stop being refusals — zero-cost compat for the most-typed aliases in Ruby.
- The spec exclusion notes at Hash, Range, and the block library flip into examples; each twin is pinned equal to its survivor, not just to its own answer.
- Ledger: [aliases.md](../ruby/aliases.md) — mostly a non-difference (the six ship with Ruby's meanings); the difference is the closed edge, where Ruby's remaining aliases refuse by name.
- `inject` inherits `reduce`'s Portland shape (explicit initial), so Ruby's `inject(:+)` symbol form and seedless form stay refusals — they ride the proc question ([#77](https://github.com/portlandlang/portland/issues/77)) and are not part of this decision.
