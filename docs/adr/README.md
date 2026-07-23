# Architecture Decision Records

One decision per file, named `NNNN-YYYY-MM-DD-slug.md`, never renumbered.
Each records Context → Decision → Consequences, with a Status of
**Accepted**, **Tentative** (leaning, not locked), or **Superseded by NNNN**.

`docs/STAGE0.md` records what's _built_.
ADRs record what's _decided_.
Everything is revisable pre-1.0, but the burden is on the challenger,
and revisions get a new ADR that supersedes the old one.

## Index

- [0001](0001-2026-07-20-mutable-keyword.md) — the mutability keyword is `mutable`
- [0002](0002-2026-07-20-together-task-sigil.md) — the `together` task sigil is `~`
- [0003](0003-2026-07-20-bitwise-operators-out.md) — bitwise operators out of the grammar (tentative: `<<` append TBD)
- [0004](0004-2026-07-20-together-meanwhile.md) — concurrency vocabulary: `together` + `meanwhile` + `~` (tentative)
- [0005](0005-2026-07-22-optionals-wrapper-model.md) — optionals are a wrapper, with a collapsed-feeling surface
- [0006](0006-2026-07-22-absence-word-nil.md) — the absence word is `nil` (partner word: 0009)
- [0007](0007-2026-07-22-or-and-not-dead-identical.md) — `or`/`||`, `and`/`&&`, `not`/`!` dead-identical; `or` is typed
- [0008](0008-2026-07-22-unwrap-toolkit.md) — the unwrap toolkit: narrowing, or-guard, `&.`, `case/in` — and nothing else
- [0009](0009-2026-07-22-presence-word-some.md) — the presence word is `some`/`some?`

Decisions made before this log exists as-built documentation in
`docs/STAGE0.md` (paren-less rules, no-shadow, strict booleans, structs, …)
and may be backfilled as ADRs when they're next touched.
