# 0001 — The mutability keyword is `mutable`

- **Status:** Accepted
- **Date:** 2026-07-20
- **Issue:** [#2](https://github.com/portlandlang/portland/issues/2)

## Context

Bare bindings are immutable by default; mutability is the rare, marked case,
and the design bar says ceremony goes on the rare thing. Candidates were
`let`, `var`, `mut` — all carrying baggage: `let` means opposite things
across Swift/Rust/JS (a never-guess violation baked into grammar), `var` is
contrast-dependent, `mut` is an abbreviation (the repo's own naming standard
— intention-revealing words, never abbreviations — applies to the language).

## Decision

The full word: **`mutable`**, fused to the binding's first assignment.

```ruby
foo = "abc"          # immutable, the default
mutable bar = "xyz"  # rebindable, the marked exception
bar += "123"
```

- Declares a **rebindable name**. No standalone/uninitialized form (no
  nil-shaped holes). Declared once, reassigned freely after.
- Gates **rebinding only** — in-place mutable *values* (`push!`-style) are a
  separate, possibly-never decision (see ADR 0003's `<<` question).
- Closure rules for `name = x` inside a block:
  - outer `mutable name` → rebinds the outer (the accumulator pattern)
  - outer immutable `name` → error: *"`name` is immutable — declare it
    `mutable name = …` if the block needs to update it"*
  - no outer `name` → fresh block-local, dies at `end`

## Consequences

- Kills Ruby's accidental-clobber closure footgun; capture-and-write is
  opt-in at the binding site.
- The declaration is exactly the marker frozen-when-shared needs: parallel
  blocks capturing a `mutable` binding become a compile error later.
- `grep mutable` audits a codebase's whole mutation surface.
- Not yet implemented in the seed or the Portland trio.

Precedent for the word: OCaml (`mutable` record fields).
