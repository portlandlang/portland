# Architecture Decision Records

**For:** anyone asking why Portland works the way it does.

One decision per file, named `NNNN-YYYY-MM-DD-slug.md`, never renumbered. Each records Context → Decision → Consequences, with a Status of **Accepted**, **Tentative** (leaning, not locked), or **Superseded by NNNN**.

[`docs/language.md`](../language.md) records what's _built_. ADRs record what's _decided_. Everything is revisable pre-1.0, but the burden is on the challenger, and revisions get a new ADR that supersedes the old one.

## Index

Decisions made before this log exist as-built documentation in [`docs/language.md`](../language.md) (paren-less rules, no-shadow, strict booleans, structs, …) and may be backfilled as ADRs when they're next touched.

<!-- generated: do not edit by hand — script/docs/generate -->

- [0001](0001-2026-07-20-mutable-keyword.md) — The mutability keyword is `mutable`
- [0002](0002-2026-07-20-together-task-sigil.md) — The `together` task sigil is `~`
- [0003](0003-2026-07-20-bitwise-operators-out.md) — Bitwise operators out of the grammar _(tentative)_
- [0004](0004-2026-07-20-together-meanwhile.md) — Concurrency vocabulary: `together` + `meanwhile` + `~` _(tentative)_
- [0005](0005-2026-07-22-optionals-wrapper-model.md) — Optionals are a wrapper, with a collapsed-feeling surface
- [0006](0006-2026-07-22-absence-word-nil.md) — The absence word is `nil`
- [0007](0007-2026-07-22-or-and-not-dead-identical.md) — `or`/`||`, `and`/`&&`, `not`/`!` are dead-identical; `or` is typed
- [0008](0008-2026-07-22-unwrap-toolkit.md) — The unwrap toolkit: narrowing, or-guard, `&.`, `case/in` — and nothing else
- [0009](0009-2026-07-22-presence-word-some.md) — The presence word is `some` / `some?`
- [0010](0010-2026-07-22-partial-operations-return-maybes.md) — Partial operations return maybes; the only crash is one you typed; `fetch` retires
- [0011](0011-2026-07-22-together-single-register.md) — `together` has one register: named-at-site
- [0012](0012-2026-07-22-branchless-if-is-nil.md) — A branch that doesn't happen produces nil
- [0013](0013-2026-07-22-case-in-spec.md) — The `case/in` spec: exhaustive, static, Ruby-shaped
- [0014](0014-2026-07-22-keyword-arguments.md) — Keyword arguments are Ruby 3's; splats stay out
- [0015](0015-2026-07-23-values-never-mutate.md) — Values never mutate; names do
- [0016](0016-2026-07-23-brace-blocks-never-guess-owner.md) — Brace blocks, with the whose-block error
- [0017](0017-2026-07-23-it-under-no-shadow.md) — `it` is a normal binding under no-shadow; `_1`–`_9` stay out
- [0018](0018-2026-07-23-numbers-ruby-division.md) — Numbers: Ruby's division, floats without ceremony
- [0019](0019-2026-07-23-ranges.md) — Ranges: total coverage, slices are collections, never-guess ends
- [0020](0020-2026-07-23-heredocs-squiggly-only.md) — Heredocs: squiggly only, SCREAMING_CAPS terminators
- [0021](0021-2026-07-24-namespaces-and-modules.md) — Namespaces: `module`, `::` for names, always qualified
- [0022](0022-2026-07-25-enums-with-payloads.md) — Enums: closed vocabularies of symbol cases, with keyword payloads
- [0023](0023-2026-07-25-symbols.md) — Symbols: a general type, checked against declared vocabularies

<!-- /generated -->

Each line is that ADR's own H1, and the tentative marker is read from its Status, so neither can drift from the file. Adding an ADR means writing it — `script/docs/generate` picks it up.
