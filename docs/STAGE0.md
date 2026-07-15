# Stage 0 — the seed subset

What the disposable Rust seed (`seed/`) actually speaks, as built. This is the
language the Stage 1 compiler will be written in, so everything here earns its
place by answering one question: *is it needed to write a compiler?*

Reference semantics live in the tree-walking interpreter (`seed/src/interpreter.rs`);
the tests are the spec until a real one exists.

## In

- **Literals** — integers (`i64`), double-quoted strings with `\n` `\t` `\"` `\\`
  `\#` escapes **and `#{...}` interpolation** (auto-`to_s`, nesting handled),
  `true`/`false`, arrays (`[1, "two", [3]]`), hashes
  (`{"key" => value}`, insertion-ordered, any-type keys; missing key panics).
- **Arithmetic** — `+ - * / %`, unary minus, parens. `+` concatenates strings
  and arrays. Division/modulo currently truncate (Rust semantics, not Ruby's
  floor — flagged to revisit).
- **Comparisons** — `== != < <= > >=`. Equality works across all types (mixed
  types are unequal); ordering is integers-only.
- **Logical operators** — `&&` `||` (short-circuiting) and `!`, strict booleans.
- **Variables** — bare assignment `x = 1`, reassignment allowed. No declarations.
- **Control** — `if` / `elsif` / `else` / `end` (an *expression*, per the
  expression-orientation principle), `unless`, `while ... end`, and postfix
  guards (`return 0 if n < 0`, `puts(x) unless quiet`). Conditions are strict
  booleans: no truthiness, since there is no nil to be falsy.
- **Methods** — `def name(a, b) ... end`, implicit return of the last
  expression, arity-checked calls with parens. Method bodies get a fresh scope:
  no outer locals (Ruby's rule, kept).
- **`return` / `break`** — `return` (with or without a value) exits the
  enclosing method, unwinding through loops; `break` exits the enclosing
  `while`. Neither works inside blocks yet.
- **Blocks** — `do |x| ... end` on `each`, `map`, `times`. Blocks are closures
  over the enclosing scope; parameters are block-local.
- **Value methods** (read-only) — strings: `length upcase downcase reverse empty?
  chars split include? start_with? end_with?` plus `[index]`; integers: `abs
  zero? positive? negative? even? odd?`; arrays: `length first last empty? join
  include? sum min max` plus `[index]` with negative indices; hashes: `length
  empty? key? keys values` plus `[key]`; everything: `to_s`.
- **IO** — `puts(...)`, one line per argument. `puts` produces *no value*;
  using its result is an error (seed-level preview of "no ambient nil").
- **Comments** — `#` to end of line.
- **Names** — `snake_case`, `?`/`!` suffixes are part of the name.

## Out (deliberately, for now)

- Heredocs, `%w[]`, single-quoted strings — remaining Prism-textbook lexer work.
- Symbols, floats, ranges.
- Optionals and the absence story — *the* headline feature, designed at the
  language level (todo 005), not snuck into the seed.
- Classes/objects, modules, constants.
- `together` / concurrency (todos 004, 012), macros (todo 015).
- Mutating methods (`push`, `upcase!`) — mutability semantics are todo 011;
  the seed stays read-only rather than prejudging them.
- `next`, `case/in`; `return`/`break` inside blocks.
- Paren-less method calls (`puts "hi"`) — needs the lexer-feedback dance;
  parens required in Stage 0.
- Types — the seed is dynamically checked at runtime; inference is the real
  compiler's job.

## Panics are the error story

The seed panics on every error (parse, type, arity, missing `end`, index out of
range, `first` on empty). Real diagnostics are a headline joy feature of the
actual compiler, not the seed.

## Where nil would have been

Ruby returns `nil` from: `if` with no taken branch, `puts`, out-of-range index,
`first`/`last` on empty. The seed returns *nothing* (an expression that
produces no value) for the first two and panics for the rest. Portland proper
answers all of these with optionals.
