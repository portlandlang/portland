# Mutability

**Status:** the keyword and rebinding rules are decided
([ADR 0001](../adr/0001-2026-07-20-mutable-keyword.md)); mutable *values*
(`push!`-style, `<<` append) are deliberately undecided. Not yet
implemented.

## Ruby

Everything is mutable, always, everywhere — every string, array, and
object can be changed by anyone holding a reference. Joyous locally,
race-prone the moment code goes parallel, and the reason `freeze` and
`# frozen_string_literal: true` exist as bolt-on discipline.

## Portland

Immutable by default; mutability is the marked, rare case — ceremony goes
on the rare thing:

```ruby
foo = "abc"          # immutable, the default
mutable bar = "xyz"  # rebindable, the marked exception
bar += "123"
```

`mutable` declares a rebindable *name*, fused to its first assignment (no
standalone declaration, no uninitialized holes). Rebinding an unmarked
name is a compile error.

The real line is **immutable when shared, mutable when local**: mutate
freely in your own scope; a value is frozen the moment it crosses a
boundary where it could race (handed to a parallel `.map`, sent across a
`together` boundary). The compiler enforces it — a parallel block
capturing a `mutable` binding is a compile error.

Closure rules kill Ruby's accidental-clobber footgun. Assigning `name = x`
inside a block:

- outer `mutable name` → rebinds the outer (the accumulator pattern, licensed)
- outer immutable `name` → error, with the fix named
- no outer `name` → fresh block-local, dies at `end`

## Migration

- A large fraction of existing Ruby never rebinds or mutates — it is
  already correct Portland, verbatim.
- Rebinding without `mutable` is a loud error with a one-word fix at the
  binding site. `grep mutable` then audits a codebase's entire mutation
  surface.
- In-place mutation (`push`, `upcase!`, `<<` append) awaits the
  mutable-values decision — the seed deliberately ships no mutating
  methods rather than prejudge it.
