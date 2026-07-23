# 0017 — `it` is a normal binding under no-shadow

- **Status:** Accepted (not yet built)
- **Date:** 2026-07-23

## Context

ADR 0016 brought brace blocks in and deliberately left the `it` implicit
block parameter (Ruby 3.4) undecided. `it` is wanted — one-expression
blocks are the whole point of braces — but Ruby ships it as a *soft
keyword*: if a local, method, or parameter named `it` is in scope, `it`
silently means that instead of the block parameter. Context-dependent
meaning is exactly what the no-shadow rule exists to kill.

Ruby also ships numbered parameters (`_1`–`_9`), the line-noise
predecessor `it` was added to replace.

## Decision

**`it` is in — as an ordinary binding that participates in the no-shadow
rule. Not reserved, not a soft keyword.** A block that mentions `it`
declares an implicit parameter named `it`; from there, every existing
rule applies unchanged:

```ruby
nodes.map { it.sexp }              # the canonical use

it = 5                             # fine — nothing contests the name
puts it + 1

it = 5
nodes.each { puts it }             # error: `it` is a local here and a
                                   # block parameter there — rename one
```

Everything else falls out of "it's a binding":

- **Nesting is shadowing, so it's an error** (strict). `it` in a block
  whose enclosing block also uses `it` is one binding shadowing another:

  ```ruby
  nodes.each { it.children.each { puts it } }
  # error: `it` is already the outer block's parameter — name your parameters
  nodes.each { |node| node.children.each { puts it } }   # fine
  ```

  The error fires only when both blocks actually use `it`; an outer
  block with named parameters puts no `it` in scope.

- **No mixing**: `it` inside a block that declares `|parameters|` is an
  error (Ruby agrees).

- **`def it` is legal but contested by every implicit block in reach** —
  the collision rule makes it extinct in practice without a special
  case.

- **A zero-parameter block's bare `it` can never be a silent reference
  to an outer local** — that situation *is* the collision, so wherever
  `it` compiles it has exactly one possible meaning.

**Numbered parameters `_1`–`_9` are out**, joining the perlisms. Named
parameters and `it` cover the space.

`&:symbol` blocks stay undecided (waits on the symbols session, per
ADR 0016).

## Consequences

- No new machinery: the seed's existing bidirectional shadow checks
  ("local shadows method — rename one") gain one more participating
  name. Seed and trio, at brace-block build time.
- Error quality: the collision message should eventually cite both
  sites' line numbers ("a local at line 12, a block parameter at
  line 61") — waits on the seed growing line tracking; the rule doesn't.
- Migration: Ruby 3.4 `it` blocks compile verbatim, same meaning, when
  the name is uncontested — and where Ruby would have silently preferred
  a local named `it`, Portland errors loudly instead (promise 1 held).
- Polyfill: `_1 → it` is a free-tier autocorrect (both already valid
  Ruby); multi-parameter `_2` sites autocorrect to named parameters.
  The no-shadow collision is lintable in plain Ruby pre-flip.
