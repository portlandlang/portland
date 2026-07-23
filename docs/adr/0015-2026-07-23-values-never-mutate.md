# 0015 — Values never mutate; names do

- **Status:** Accepted (`!`-suffix semantics explicitly deferred)
- **Date:** 2026-07-23
- **Issue:** [#10](https://github.com/portlandlang/portland/issues/10)

## Context

ADR 0001 decided `mutable` gates *rebinding* and deferred mutable
*values* (`push`, `upcase!`, `<<`, `[]=`). Two findings since sharpened
the stakes:

- **The acyclicity dividend (#12):** immutable values cannot form
  reference cycles — a cycle needs a back-edge written after creation —
  which is what makes plain reference counting *exact*: no tracing GC,
  no pauses, no weak/unowned ceremony. In-place mutable values would
  break this permanently (one self-containing array and Portland needs a
  cycle collector forever). This is a memory-model decision wearing
  syntax clothes.
- **Lived experience:** the seed and trio are already mutation-free and
  thrived — `Rc` sharing made self-parse 50× faster *because* nothing
  mutates, and the accumulator idiom (`total += n`) is rebinding.

## Decision

**One principle: values never mutate; names do.** The package:

1. **In-place mutators never exist** — no `push`, no `map!`, no aliased
   buffers, no action at a distance. Ruby's aliased-mutation bug class
   dies at birth.
2. **`<<` returns as a rebinding operator** in the compound-assignment
   family, and **index assignment rides along**:

   ```ruby
   mutable line = ""
   line << word            # ≡ line = line + word
   mutable counts = {}
   counts["portland"] = 1  # ≡ a functional update of counts
   ```

   Both live at statement/assignment position only (Ruby's `<<` lexer
   pileup never fully returns) and both are rebinding — so they require
   a `mutable` binding, exactly like `+=`.
3. **The loudness mechanism is sequencing: `mutable` (ADR 0001) is
   implemented first.** Ruby's `<<` mutates through aliases; Portland's
   doesn't. That divergence would be silent — except every migrated
   `<<`/`+=`/`[]=` site fails to compile until its binding says
   `mutable`, so a human visits every site, and the linter explains the
   aliasing change right there. The gate makes it loud.
4. **The share-boundary question dissolves.** With no mutable values
   there is nothing to freeze or copy at a `together` boundary; only
   `mutable` bindings interact with parallelism, and ADR 0001 already
   promises that compile error.
5. **Bang methods are out; `!`-suffix semantics are deferred.** Leading
   candidate (user, 2026-07-23): bang as rebinding sugar —
   `word.upcase!` ≡ `word = word.upcase`, `mutable`-gated. Competing
   candidate: `!` = "may panic" (pairs with `?` = "answers a question").
   Decided later, alongside the error-handling session; neither may
   claim `!` without revisiting this note.

Performance is the runtime's job, not the semantics': refcount-1 reuse
(mutate in place when nobody else holds the value) makes rebuild-append
O(1) amortized, invisibly. The seed's `Rc` machinery is already shaped
for it.

## Consequences

- RC-exactness (#12) is secured at the language level; "no GC pause"
  stops being aspirational.
- Ruby's `freeze`/`frozen?`/`dup`/`clone` and
  `# frozen_string_literal:` become meaningless — ledger entries, not
  features.
- Implementation order: `mutable` in the seed (breaking — done on a
  branch until green, our own code takes the migration medicine), then
  `<<`/`[]=` sugar, then the trio's guest-hash story can finally use
  `[]=`.
- Migration: mutation-free Ruby compiles verbatim; every mutating call
  site is a loud compile error with a mechanical rewrite; aliased
  mutation is the one *semantic* change, gated behind the `mutable`
  declaration every such site must add.
