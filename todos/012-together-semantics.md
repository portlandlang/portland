# `together` block semantics

Tier 2 concurrency: declare independence in one word. Needs language support — a normal block returns only its last value, so each marked line becoming a task is the one bit of magic.

## Tasks

- [ ] Spec named-at-site register (`user = • fetch_user`) — scoping rules for names escaping the block
- [ ] Spec terse positional register (`a, b = together do … end`)
- [ ] Interleaving plain lines with task lines — ordering/visibility guarantees
- [ ] Join semantics: the block end is the join boundary; what happens on task failure?
- [ ] Tier 3 hooks: cancellation, timeouts, racing — rare but must have a home

Depends on [004](004-bikeshed-concurrency-sigil.md) for the sigil/word decision.
