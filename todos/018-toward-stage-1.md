# Toward Stage 1: what the subset still needs to write a compiler in

The Stage 1 test (from [009](009-stage-0-subset.md)): could the seed compiler
itself be rewritten in the Stage 0 subset? Gap analysis as of 2026-07-15:

## Blockers

- [x] **File IO** — crude `read_file`/`write_file` builtins landed
  (placeholder names; the real `File` API needs the object model).
- [x] **ARGV** — `argv()` builtin landed.
- [ ] **Structs or something** — tokens/AST nodes need a shape. Hashes may be
  enough for a crude first pass (`{"kind" => "integer", "text" => "42"}`), but
  it'll be joyless; a minimal record/struct is probably worth designing early
  since it feeds the real object model anyway.
- [ ] **Recursion depth** — the interpreter recurses on the Rust stack; a
  self-parse of a large file may blow it. Measure, then decide.

## Wanted (joy and practicality)

- [ ] `return`/`break`/`next` inside blocks (currently an honest panic)
- [ ] Paren-less method calls (`puts "hi"`) — the lexer-feedback dance;
  deserves its own focused session
- [ ] `each_with_index` / `map` with index
- [ ] String slicing beyond one char (`s[1, 3]` or ranges)
- [ ] Heredocs (last big Prism-textbook item)

## Non-goals for the seed

Types, optionals, `together`, macros, GPU anything — those are the *real*
compiler's features, designed at the language level, not snuck into the
disposable interpreter.
