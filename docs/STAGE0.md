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
- **Variables** — bare assignment `x = 1`, reassignment allowed, compound
  assignment (`+= -= *= /= %=`). No declarations.
- **Control** — `if` / `elsif` / `else` / `end` (an *expression*, per the
  expression-orientation principle), `unless`, `case/when` (equality matching,
  aligned `when x then y` one-liners), `while ... end`, and postfix guards
  (`return 0 if n < 0`, `puts(x) unless quiet`). Conditions are strict
  booleans: no truthiness, since there is no nil to be falsy.
- **Methods** — `def name(a, b) ... end`, implicit return of the last
  expression, arity-checked calls with parens, default parameter values
  (trailing only; defaults may reference earlier parameters). Method bodies get
  a fresh scope: no outer locals (Ruby's rule, kept).
- **Structs** — immutable named records, the seed of the object model:
  ```ruby
  struct Token
    kind
    text
  end

  token = Token.new(kind: "integer", text: "42")   # kwargs only, all fields required
  token.kind                                       # field access
  token.with(text: "43")                           # updated copy; nothing mutates
  ```
  Value equality, definition-ordered fields, capitalized names. No methods in
  struct bodies yet — that arrives with the real object model.
- **`return` / `break` / `next`** — `return` (with or without a value) exits
  the enclosing method, unwinding through loops; `break` and `next` control the
  enclosing `while`. None work inside blocks yet.
- **Blocks** — `do |x| ... end` on `each` (arrays, and hashes with
  `|key, value|`), `each_with_index`, `map`, `select`, `reject`,
  `reduce(initial)`, `times`, `upto`, `downto`. Blocks are closures over the
  enclosing scope; parameters are block-local.
- **Value methods** (read-only) — strings: `length upcase downcase reverse empty?
  chars split include? start_with? end_with? to_i` plus `[index]`; integers:
  `abs zero? positive? negative? even? odd?`; arrays: `length first last empty?
  join include? sum min max` plus `[index]` with negative indices; hashes:
  `length empty? key? keys values` plus `[key]`; everything: `to_s`.
  `slice(start, length)` and `sort` (integers) round out arrays/strings. `*`
  repeats strings and arrays; `%w[rose city]` builds word arrays. Method
  chains may continue across newlines with a leading dot.
- **IO** — `puts(...)`, one line per argument; `puts` produces *no value* —
  using its result is an error (seed-level preview of "no ambient nil").
  `p(...)` prints `inspect` renderings (strings keep quotes) and returns its
  argument, like Ruby. The REPL shows results via `inspect`.
  Crude file IO: `argv()`, `read_file(path)`, `write_file(path, content)` —
  placeholder names until the real object model exists.
- **Comments** — `#` to end of line.
- **Names** — `snake_case`, `?`/`!` suffixes are part of the name.

## Out (deliberately, for now)

- Heredocs — remaining Prism-textbook lexer work (interpolation, `%w[]`, and
  single-quoted strings are in).
- Symbols, floats, ranges.
- Optionals and the absence story — *the* headline feature, designed at the
  language level (todo 005), not snuck into the seed.
- Classes/objects, modules, constants; methods inside `struct` bodies.
- Keyword arguments on regular methods (`new`/`with` only so far).
- `together` / concurrency (todos 004, 012), macros (todo 015).
- Mutating methods (`push`, `upcase!`) — mutability semantics are todo 011;
  the seed stays read-only rather than prejudging them.
- `case/in` pattern matching; `return`/`break`/`next` inside blocks.
- Paren-less method calls (`puts "hi"`) — needs the lexer-feedback dance;
  parens required in Stage 0.
- Types — the seed is dynamically checked at runtime; inference is the real
  compiler's job.

## Depth limits (measured 2026-07-19)

On the default 8 MB main stack the seed died — as a silent macOS *hang*, not a
crash — at ~1,200 nested parens, ~1,500-term `1 + 1 + …` chains, and ~900
Portland call frames. (Even a trivial Rust `fn f() { f() }` hangs on overflow
under macOS 26, so the OS gives no clean failure.) Two fixes, both in:

- The interpreter runs on a **512 MB-stack thread**, moving real limits ~64×
  out (5,000-deep recursion and nesting are in the test suite).
- **Explicit depth guards** fail as clean Portland errors long before the Rust
  stack is at risk: expression nesting > 10,000 (parse), expression evaluation
  > 100,000, call stack > 10,000 frames.

## Panics are the error story

The seed panics on every error (parse, type, arity, missing `end`, index out of
range, `first` on empty). Real diagnostics are a headline joy feature of the
actual compiler, not the seed.

## Where nil would have been

Ruby returns `nil` from: `if` with no taken branch, `puts`, out-of-range index,
`first`/`last` on empty. The seed returns *nothing* (an expression that
produces no value) for the first two and panics for the rest. Portland proper
answers all of these with optionals.
