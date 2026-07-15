# Immutability semantics: frozen at the sharing boundary

Immutable when shared, mutable when local. Mutate freely in your own scope; a value freezes the moment it crosses a boundary where it could race. Compiler-enforced.

## Tasks

- [ ] Define "sharing boundary" precisely (passed to parallel `.map`, crosses `together`, stored where another task can see it — full list)
- [ ] Freeze mechanics: deep vs shallow, copy vs transfer of ownership
- [ ] How this composes with tier-1 implicit parallelism (immutability is _why_ auto-parallel is safe)
- [ ] Diagnostics: what does the compiler say when you mutate a shared value? Sell it as deleting races, not adding rules.
