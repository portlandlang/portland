# Changelog

## Unreleased

- Design docs, todos, and namespace squats (GitHub orgs `portlandlang` + `pdxlang`, crates.io `portland` v0.0.0).
- Cargo workspace: `crate/` (the published placeholder, eventually the real compiler) + `seed/` (Stage 0, never published), with `script/test` (fmt + clippy + tests).
- Seed lexer: integer literals, identifiers with `?`/`!` suffixes, double-quoted strings (no escapes/interpolation yet), newline tokens, space/tab skipping.
- Seed lexer: `def`/`do`/`end` keywords (lookalikes like `def?`/`ending` stay identifiers) and single-character punctuation (`(` `)` `,` `.` `=` `+`).
- Seed AST + recursive descent parser: integer and string literals, left-associative `+`, parenthesized grouping. `1 + 2` now means something.
- Seed parser, statement level: newline-separated programs, variable references, assignment, method calls with parenthesized arguments, and `def ... end` with parameters and body.
- Seed interpreter (tree-walking reference semantics): literals, arithmetic (`+ - * / %`, unary minus), string concatenation, comparisons, strict-boolean `if`/`elsif`/`else` expressions, `while` loops, assignment, and user-defined methods with fresh scopes.
- `puts` builtin with pluggable output; builtins produce no value (a seed-level preview of "no ambient nil").
- `pdx` binary: runs `.pdx` files (fixture-tested end to end, fizzbuzz included) and opens a REPL when run bare — multi-line definitions buffer, errors report and continue.
- Comments (`#` to end of line).
- String escape sequences (`\n` `\t` `\"` `\\`), decoded in the parser.
- Dot method calls, chainable, with read-only builtin value methods: `length`, `upcase`, `downcase`, `reverse`, `empty?` on strings; `abs`, `zero?`, `positive?`, `negative?` on integers; `to_s` on everything. `-5` is a negative literal, so `-5.abs == 5`.
