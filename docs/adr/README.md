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

Decisions made before this log exists as-built documentation in
`docs/STAGE0.md` (paren-less rules, no-shadow, strict booleans, structs, …)
and may be backfilled as ADRs when they're next touched.
