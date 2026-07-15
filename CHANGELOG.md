# Changelog

## Unreleased

- Design docs, todos, and namespace squats (GitHub orgs `portlandlang` + `pdxlang`, crates.io `portland` v0.0.0).
- Cargo workspace: `crate/` (the published placeholder, eventually the real compiler) + `seed/` (Stage 0, never published), with `script/test` (fmt + clippy + tests).
- Seed lexer: integer literals, identifiers with `?`/`!` suffixes, double-quoted strings (no escapes/interpolation yet), newline tokens, space/tab skipping.
- Seed lexer: `def`/`do`/`end` keywords (lookalikes like `def?`/`ending` stay identifiers) and single-character punctuation (`(` `)` `,` `.` `=` `+`).
- Seed AST + recursive descent parser: integer and string literals, left-associative `+`, parenthesized grouping. `1 + 2` now means something.
