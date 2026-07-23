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
- **Logical operators** — `&&` `||` (short-circuiting) and `!`, strict
  booleans — plus the word forms `and` `or` `not`, **dead-identical** to the
  sigils (ADR 0007): same precedence (`x = nil or 7` binds the `or` first,
  unlike Ruby), same semantics. `||`/`or` is *typed*: booleans get logical
  or; a maybe gets unwrap-or-else (below).
- **Optionals, the runtime half** (ADRs 0005–0010) — `nil` is a keyword
  literal: no methods (`nil.upcase` panics "handle the nil case first"),
  not falsy, `puts nil` refuses. `nil?`/`some?` work on every value — the
  one dispatch a maybe allows. Partial operations return maybes instead of
  panicking: `[].first`/`last`/`min`/`max`, out-of-range array and string
  indexing, missing hash keys. The unwrap toolkit: `x or default` (lazy),
  the or-guard divergers (`x = f() or return` / `break` / `next`, and
  `or panic "why"` — the language's only crash spelling, also a builtin at
  statement position), and safe navigation `&.` (absent receiver
  short-circuits; arguments never run). The wrapper's nested case:
  `some(x)` is identity on plain values and a real box only around
  nil/Some, lookups lift found values through it — so `[nil].first` ≠
  `[].first`, and a stored hash nil beats the or-guard default (fetch's
  rule). The *static* half (flow narrowing, unhandled-maybe compile
  errors, `Boolean?` never-guess, dead right sides) is structurally out of
  a tree-walker's reach — those panic at runtime here and refuse at
  compile time in real Portland.
- **Variables** — bare assignment `x = 1`, reassignment allowed, compound
  assignment (`+= -= *= /= %=`). No declarations.
- **Control** — `if` / `elsif` / `else` / `end` (an *expression*, per the
  expression-orientation principle), `unless`, `case/when` (equality matching,
  aligned `when x then y` one-liners), `while ... end`, and postfix guards
  (`return 0 if n < 0`, `puts(x) unless quiet`). Conditions are strict
  booleans: no truthiness, since there is no nil to be falsy.
- **`case/in` pattern matching** (ADR 0013, runtime half) — literal and
  `nil` patterns, captures (bind and persist, no-shadow-fenced), `|`
  alternatives, pin `^variable`, guards (`in x if x > 10`, bind-first),
  array patterns with a trailing splat (`in [first, *rest]`), and
  keyword-only struct patterns (`in Token(kind: "integer", text:)` — the
  `field:` shorthand binds). One-line forms: `expr in pattern` (boolean,
  binds on a hit) and `expr => pattern` (rightward destructuring, panics
  on mismatch). No match and no `else` panics — the runtime preview of
  compile-checked exhaustiveness. Hash patterns and the find pattern are
  deliberately unbuilt.
- **Methods** — `def name(a, b) ... end`, implicit return of the last
  expression, arity-checked calls, default parameter values (trailing only;
  defaults may reference earlier parameters), and **keyword parameters**
  (ADR 0014): `label:` required, `label: default` optional, strictly
  separate from positionals, labels accepted on paren and command calls,
  missing/unknown labels are named errors. Method bodies get a fresh
  scope: no outer locals (Ruby's rule, kept).
- **Paren-less calls, the Portland way** — *command calls* at statement
  position (`puts "hello"`, `shout word, other`) and *bare zero-argument
  calls* (`ready?`, `pdx`) anywhere. Two rules replace Ruby's guessing:
  - **No shadowing.** A name is a local or a method, never both — assigning
    `greet = 1` where a method `greet` exists is an error. Bare names are
    therefore always unambiguous.
  - **Never guess.** Forms Ruby resolves by whitespace heuristics are clean
    errors instead: `puts -1`, `puts [1]`, `puts (1)` each say
    *"ambiguous without parens"* and show both readings. `foo - 1` stays
    subtraction. Blocks don't attach to command calls yet.
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
  the enclosing method, unwinding through loops *and blocks*; `break` and
  `next` control the enclosing `while` or block iteration. A call broken out
  of produces nil (ADR 0012).
- **Blocks** — `do |item| ... end` on `each` (arrays, and hashes with
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
- **`require_relative "lexer"`** — multi-file programs, Ruby-style: resolved
  against the requiring file's directory, `.pdx` implied, loads once
  (returns false on a repeat).
- **Comments** — `#` to end of line.
- **Names** — `snake_case`, `?`/`!` suffixes are part of the name.

## Out (deliberately, for now)

- Heredocs — remaining Prism-textbook lexer work (interpolation, `%w[]`, and
  single-quoted strings are in).
- Symbols, floats, ranges.
- The static half of optionals (narrowing, exhaustiveness, compile-time
  maybe tracking) — the tree-walker previews those errors as panics.
- Classes/objects, modules, constants; methods inside `struct` bodies.
- Splats (`*args`, `**kwargs`) — deferred, ADR 0014.
- `together` / concurrency (#11), macros (#14).
- Mutating methods (`push`, `upcase!`) — mutable-values semantics are
  #10; the seed stays read-only rather than prejudging them.
- Hash patterns and the find pattern (`in [*, x, *]`) — deferred (ADR 0013).
- Command calls nested in expressions (`x = foo bar`) — statement position only.
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

Ruby returns `nil` from: `if` with no taken branch, `puts`, out-of-range
index, `first`/`last` on empty. All but `puts` now genuinely return
Portland's `nil` — lookups by ADR 0010, branchless `if`/finished
`while`/broken-out calls by ADR 0012. `puts` alone still produces
*nothing*: it could never have had an answer, so using its result stays an
error (ADR 0012's dividing rule). The ledger this section tracked is
closed.
