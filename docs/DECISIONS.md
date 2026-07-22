# Decisions

Language decisions that are settled but not necessarily built yet.
`docs/STAGE0.md` records what exists; this records what's been decided.
Everything here is revisable pre-1.0 — but the burden is on the challenger.

## Mutability: `mutable` (2026-07-20, #2)

The full word, deliberately verbose — mutability is the rare, marked case.

```ruby
foo = "abc"          # immutable, the default
mutable bar = "xyz"  # rebindable, the marked exception
bar += "123"
```

- Declares a **rebindable name**, fused to its first assignment. No
  standalone/uninitialized form. Declared once, reassigned freely after.
- Gates **rebinding only** — in-place mutable *values* (`push!`-style) are a
  separate, possibly-never decision (see `<<` below).
- Closure rules for `name = x` inside a block:
  - outer `mutable name` → rebinds the outer (the accumulator pattern)
  - outer immutable `name` → error: *"`name` is immutable — declare it
    `mutable name = …` if the block needs to update it"*
  - no outer `name` → fresh block-local, dies at `end`
- Parallel blocks capturing a mutable binding will be a compile error —
  this keyword is the marker frozen-when-shared needs.
- Rejected: `let` (opposite meanings across languages), `var`
  (contrast-dependent), `mut` (abbreviation). Precedent: OCaml.

## Concurrency task sigil: `~` (2026-07-20, #3)

```ruby
together do
  ~ user = fetch_user(id)
  ~ orders = recent_orders(id)
  ~ news = latest_news
end
```

- Clean to Ruby hands: unary bitwise NOT is rare, `=~` is fading, `Regexp#~`
  is a cut perlism; the warm associations (`<<~`, `~>`) are gentle.
- Unambiguous in every position in Portland — no positional rules needed.
- Still open on #3: the word form (`spawn` is an unconfirmed placeholder)
  and whether the terse positional register earns its existence.

## Bitwise operators: probably out (2026-07-20, #3)

`& | ^ ~ << >>` as *bit* operators are probably not in the grammar: rare in
application code, expensive in ASCII budget (freeing `~` above), and
`&`-vs-`&&` is a classic footgun. Capability preserved as named methods
(`flags.bit_and(mask)`) that inline identically under AOT. Not final.

**Explicitly TBD: `<<` as append** (`Array#<<` / `String#<<`) — common and
beloved in Ruby, and a *separate* question from bit-shift. Append implies
in-place mutation, so it must be decided together with mutable values.

## Earlier decisions, recorded where built

- Structs (`struct ... end`, kwargs-only `new`, immutable, `with`) — STAGE0.md
- Paren-less calls: command calls at statement position, bare zero-arg calls,
  the no-shadow rule, never-guess errors — STAGE0.md
- Strict booleans, no ambient nil, panics-where-Ruby-nils — STAGE0.md
- macOS 26+ only; `.pdx`; MIT license
