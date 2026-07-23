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

## Migration

- Code following Ruby community style (parens except for DSL-ish command
  calls) is nearly all already unambiguous — compiles verbatim.
- The heuristic forms become loud errors with both readings shown; the
  fix is always "add the parens you meant."
- Shadowing collisions (a local named like a method) are loud errors,
  fixed by a rename.
