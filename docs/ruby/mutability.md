# Mutability

**Status:** fully decided — the keyword and rebinding rules
([ADR 0001](../adr/0001-2026-07-20-mutable-keyword.md)) and the values
question ([ADR 0015](../adr/0015-2026-07-23-values-never-mutate.md):
values never mutate; names do). Implementation in progress; `!`-suffix
semantics deferred.

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

And the values question is answered (ADR 0015): **values never mutate;
names do.** No `push`, no `map!`, no aliased buffers. `<<` and index
assignment survive as *rebinding* sugar in the `+=` family:

```ruby
mutable line = ""
line << word            # ≡ line = line + word — aliases can't be spooked
mutable counts = {}
counts["portland"] = 1  # ≡ a functional update of counts
```

Why: aliased mutation is Ruby's action-at-a-distance bug class; immutable
values are what let tier-1 parallelism spread `.map` across cores; and
immutable values can't form reference cycles, which is what makes plain
reference counting exact — no GC pauses, ever. Performance is the
runtime's job (refcount-1 buffer reuse), not the semantics'.

## Migration

- A large fraction of existing Ruby never rebinds or mutates — it is
  already correct Portland, verbatim.
- Rebinding without `mutable` is a loud error with a one-word fix at the
  binding site. `grep mutable` then audits a codebase's entire mutation
  surface.
- Every `<<` / `+=` / `[]=` site gates on that `mutable` declaration —
  which is the loudness mechanism for the one real semantic change:
  Ruby's `<<` mutates through aliases, Portland's rebinds one name. The
  compile error brings a human to every such site; the linter explains
  the aliasing difference there.
- Bang mutators (`upcase!`, `merge!`) are loud unknown-method errors;
  the rewrite is rebinding (`word = word.upcase`). Whether `!` returns
  as sugar for exactly that is deferred (ADR 0015 §5).
- `freeze` / `frozen?` / `dup` / `clone` /
  `# frozen_string_literal: true` are meaningless in a
  values-never-mutate language: the first four are loud errors, the
  magic comment is ignored-not-an-error (it asks for what is already
  true).
