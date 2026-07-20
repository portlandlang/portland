# Changelog

## Unreleased

- parser.pdx rung 3 (#18): statements — assignment, compound assignment (desugared), return/break/next, and error recovery that skips to the next line instead of cascading.

- parser.pdx rung 2 (#18): postfix — dot calls, indexing, paren call arguments, leading-dot chains across newlines, and negative literals binding Ruby-style (-5.abs).

- parser.pdx rung 1 (#18): the full expression precedence ladder — logicals, comparisons, arithmetic, unary, parens — with identifiers, strings, and booleans as operands.

- parser.pdx rung 0 (#18): Step-threaded recursive descent skeleton, sexp printer, parse.pdx driver — integers parse, unknown tokens become error nodes.

- `require_relative`: multi-file Portland programs, Ruby-style (resolved against the requiring file, `.pdx` implied, loads once). `compiler/lexer.pdx` is now a library; `compiler/tokenize.pdx` is the command-line driver.
- Paren-less calls, the Portland way: command calls at statement position (`puts "hello"`) and bare zero-argument calls (`ready?`) — powered by two new rules instead of Ruby's whitespace guessing: shadowing a method with a local is an error, and ambiguous forms (`puts -1`, `puts [1]`, `puts (1)`) are clean parse errors that show both readings. The lexer now records leading whitespace to detect them.
- `return`/`break`/`next` now work inside blocks with Ruby semantics: `next` skips the iteration, `break` stops it (the call produces no value), `return` unwinds through the block to the enclosing method — guard-search idioms like `return number if number.even?` inside `each` work.
- Single-character variable names renamed away everywhere (`character` not `c`, `index` not `i`), per style.
- MIT license (`LICENSE.md`); the crate's dual-license placeholder becomes plain MIT on its next publish.
- `compiler/lexer.pdx`: Portland's lexer, written in Portland — tokenizes the full token set (strings with escapes and interpolation, `%w[]`, two-character operators, `?`/`!` names) and lexes its own source with zero error tokens. Step one of Stage 1.
- Recursion depths measured (and the macOS-26 hang-on-overflow discovered): the seed now runs on a 512 MB-stack thread with explicit depth guards that fail as clean Portland errors instead of hanging.
- Structs: `struct Name ... end` immutable records with kwargs-only construction (`Token.new(kind: ...)`), field access, `.with(...)` updated copies, and value equality. First user: `mini_lexer.pdx`, now hash-free.

- Design docs, todos, and namespace squats (GitHub orgs `portlandlang` + `pdxlang`, crates.io `portland` v0.0.0).
- Todos migrated to GitHub issues #1–#17; `todos/` now holds only the mapping.
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
- Arrays: literals, indexing (negative indices; out of range panics — no nil), `+` concatenation, `length`/`first`/`last`/`empty?`/`join`.
- Blocks: `do |x| ... end` on `each`, `map`, and `times`. Blocks are closures over the enclosing scope; parameters are block-local.
- `return` (exits the enclosing method, unwinding through loops) and `break` (exits the enclosing `while`); misuse panics, unsupported-in-blocks stated honestly.
- `docs/STAGE0.md`: the seed subset documented as built, including what's deliberately out.
- Hashes: `{"key" => value}` literals (insertion-ordered, duplicate keys last-wins), lookup by any value (missing key panics — no nil), `length`/`empty?`/`key?`/`keys`/`values`.
- Stdlib breadth: string `chars`/`split`/`include?`/`start_with?`/`end_with?` and `[index]`; integer `even?`/`odd?`; array `include?`/`sum`/`min`/`max`.
- `unless` (block and postfix) and postfix `if` — guard clauses (`return 0 if n < 0`) work.
- Short-circuiting `&&`/`||` and `!`, strict booleans.
- String interpolation `"#{...}"`, desugared to concatenation with auto-`to_s`; lexer keeps token boundaries honest through nested strings and braces.
- Compound assignment (`+= -= *= /= %=`), `next` in `while` loops, `hash.each do |key, value|`.
- `case/when` with equality matching, multiple values per `when`, and aligned `when x then y` one-liners.
- `tour.pdx` fixture: the full Stage 0 surface through the real binary.
- `p` builtin and `inspect` rendering (strings keep quotes; the REPL uses it, like irb).
- Single-quoted literal strings, `%w[]` word arrays, `*` repetition for strings and arrays.
- Default parameter values (trailing only, bound left to right so defaults can reference earlier parameters).
- `select`/`reject`/`reduce(initial)` blocks and `String#to_i`.
- Crude IO builtins (`argv()`, `read_file`, `write_file`) — names are placeholders; unblocks real programs. `word_count.pdx` fixture is a working `wc`.
- `each_with_index`, `sort` (integer arrays), `slice(start, length)` on strings and arrays; bare `puts()` prints a blank line.
- `mini_lexer.pdx` fixture: a lexer written in Portland, tokenizing Portland-ish source — first compiler work in the language itself.
- `each_with_index`, `upto`/`downto`, leading-dot method chains across newlines, duplicate-parameter rejection, REPL buffers multi-line strings.
- `script/bootstrap` and `script/cibuild`; `todos/018` gap analysis toward Stage 1 self-hosting.
