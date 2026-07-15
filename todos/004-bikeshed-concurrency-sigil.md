# Bikeshed: the concurrency sigil

`together` blocks mark each concurrent line with a sigil (working placeholder: `•`) plus a word form (`spawn`).

## Constraints

- Word form and symbol form must be **dead-identical** in semantics (don't repeat the `lambda`/`proc` `return` footgun)
- Symbol must be one easy keystroke on Apple keyboards (`•` is ⌥8 — acceptable? decide)
- A marked line is a task; an unmarked line is ordinary code — no other magic

## Tasks

- [ ] Pick the symbol
- [ ] Pick the word (`spawn`? something warmer?)
- [ ] Spec the two registers: terse positional (`a, b = together do … end`) and named-at-site
