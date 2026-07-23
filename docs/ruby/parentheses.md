# Parentheses and ambiguity

**Status:** decided and built — this is in the seed and the Portland trio
today (see [STAGE0](../STAGE0.md)).

## Ruby

Paren-less calls resolved by whitespace heuristics: `puts -1`, `foo [1]`,
`puts (1)` each parse by guesswork, sometimes with a warning, sometimes
silently as the wrong thing. Locals and methods share a namespace and
shadow each other, so `foo` means different things at different distances
from an assignment.

## Portland

Paren-less survives — it's load-bearing Ruby joy — but two rules replace
the guessing:

- **Command calls** at statement position (`puts "hello"`,
  `shout word, other`) and **bare zero-argument calls** (`ready?`, `pdx`)
  anywhere. Same reading Ruby's style guides already prefer.
- **No shadowing.** A name is a local or a method, never both — assigning
  `greet = 1` where a method `greet` exists is a compile error. A bare
  name is therefore always unambiguous. (This same rule is what makes
  flow narrowing on locals sound.)
- **Never guess.** The forms Ruby resolves heuristically are clean
  errors instead: `puts -1`, `puts [1]`, `puts (1)` each say *"ambiguous
  without parens"* and show both readings. `foo - 1` stays subtraction.

The never-guess principle generalizes past parens: wherever one spelling
has two genuine readings ([`Boolean?` with `or`](word-operators.md), for
instance), Portland errors and asks, rather than picking for you.

## Brace blocks (ADR 0016 — decided, not yet built)

Ruby gives `{ ... }` and `do ... end` different binding strength: after
`render config`, a brace block belongs to `config` (nearest call) while a
`do/end` block belongs to `render` (farthest). That's a precedence guess
the reader has to know.

Portland takes brace blocks with no precedence split: the two forms mean
exactly the same thing, and the one position where readings collide — a
bare `{` right after a paren-less command call — is a compile error
naming each reading with its rewrite:

```ruby
render config { "a" => 1 }
# error: `{` after a command call could be three things — parenthesize the one you mean:
#   a hash argument to config:  render config({ "a" => 1 })
#   a block for config:         render(config { "a" => 1 })
#   a block for render:         render(config) { "a" => 1 }
```

(The parser peeks to shrink the menu — `{ |item| ...` can't be a hash, so
only the two owners are offered — but never to pick a winner.)

## `it` (ADR 0017 — decided, not yet built)

Ruby 3.4's `it` is a soft keyword: a local or method named `it` silently
wins over the block parameter. Portland makes `it` an ordinary binding
under the no-shadow rule — `nodes.map { it.sexp }` just works; naming
anything else `it` in the same reach is a rename-one compile error; `it`
nested under a block that also uses `it` is shadowing, so it errors
("name your parameters"); `it` alongside declared `|parameters|` errors
(Ruby agrees). Wherever `it` compiles, it has exactly one possible
meaning.

Numbered parameters (`_1`–`_9`) are out — see
[removed syntax](removed-syntax.md). The polyfill autocorrects `_1 → it`
for free (both are valid Ruby today).

## Migration (brace blocks)

- Community style already avoids the ambiguous form (RuboCop's
  `Style/BlockDelimiters`: braces for one-line dot-call blocks, `do/end`
  for command position) — that code compiles verbatim, same meaning.
- Code that leans on braces-bind-tight gets the loud three-way error, one
  paren from compiling. Nothing silently rebinds.

## Migration

- Code following Ruby community style (parens except for DSL-ish command
  calls) is nearly all already unambiguous — compiles verbatim.
- The heuristic forms become loud errors with both readings shown; the
  fix is always "add the parens you meant."
- Shadowing collisions (a local named like a method) are loud errors,
  fixed by a rename.
