# 0016 — Brace blocks, with the whose-block error

- **Status:** Accepted (not yet built)
- **Date:** 2026-07-23

## Context

Demand arrived on schedule: the trio's own

```ruby
def sexp_list(nodes)
  nodes.map do |node|
    node.sexp
  end.join(" ")
end
```

is a one-liner trapped in five lines. `do/end` is Portland's only block
form; Ruby writes this `nodes.map { |node| node.sexp }.join(" ")`.

Adopting braces verbatim imports Ruby's binding heuristic: after a
paren-less command call, `render config { ... }` gives the block to the
*nearest* call (`config`) while `do/end` gives it to the *farthest*
(`render`) — a precedence guess readers get bitten by, and exactly the
kind the never-guess rule exists to kill. Three candidate rules were
considered: never-guess error at the ambiguous spot; Ruby's precedence
verbatim (a silent guess); braces-never-blocks after command calls (a
silent divergence from Ruby, which ADR 0006's promise forbids).

## Decision

**Brace blocks enter the grammar. `do/end` is unchanged. The one
ambiguous position is a compile error that names each reading with its
exact rewrite.**

A bare `{` immediately after a paren-less command call's arguments is
the only spot where readings genuinely collide, and there are at most
three of them:

```ruby
render config { "a" => 1 }
# error: `{` after a command call could be three things — parenthesize the one you mean:
#   a hash argument to config:  render config({ "a" => 1 })
#   a block for config:         render(config { "a" => 1 })
#   a block for render:         render(config) { "a" => 1 }
```

The parser peeks past the `{` to trim the menu — never to pick a
winner:

- `{ |item| ...` cannot be a hash, so the error offers only the two
  block owners.
- `{ expression }` with no `=>` cannot be a hash either — same two-way
  error.
- `{ key => value }` and `{}` keep all three readings (a hash literal is
  also a valid zero-parameter block body, since one-line `=>` match
  assertion is a statement per ADR 0013) — the error offers all three.

Whichever rewrite the author copies, the parens answer hash-vs-block
and whose-block in the same stroke; the error is one round, never a
dialogue.

Everywhere else, braces mean what they look like, with no error:

```ruby
nodes.map { |node| node.sexp }.join(" ")   # dot call: block
render(config) { |item| item.name }        # parens close the arguments: render's block
config = { "theme" => "teal" }             # assignment position: hash
apply({ "theme" => "teal" })               # parenthesized argument: hash
```

There is **no semantic difference** between `{ ... }` and `do ... end`
in Portland — no precedence split, no binding split. Where Ruby's two
forms would disagree, Portland has already refused to compile the
ambiguous spelling.

Deliberately not decided here: the `it` implicit parameter (Ruby 3.4)
and `&:symbol` blocks (waits on symbols). Both are separate pulls.

## Consequences

- Peeking is not guessing: the parser already looks ahead (two-character
  operators, `newline_then_dot?`, `or panic`); never-guess constrains
  what the *reader* must disambiguate, not what the parser may read.
- Implementation slots into the existing `ambiguous_command?` /
  `ambiguity_message` machinery — one new trigger position, a one-token
  peek to pick the message; the substantive work is parsing brace blocks
  in the unambiguous positions, seed and trio.
- Migration: Ruby code that follows community style (braces for
  one-liners on dot calls, `do/end` for command-position blocks) compiles
  verbatim with identical meaning. Code leaning on braces-bind-tight
  becomes a loud error with its rewrite attached — never a silent
  rebinding.
- RuboCop's default styles (`Style/BlockDelimiters`, brace-vs-do-end)
  already steer Ruby away from the ambiguous form, so the polyfill tiers
  cleanly: the never-guess error is a free lint pre-flip.
