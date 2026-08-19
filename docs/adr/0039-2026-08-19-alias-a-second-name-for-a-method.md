# 0039 — `alias`: a second name for a method

- **Status:** Accepted (built in both oracles, 2026-08-19)
- **Date:** 2026-08-19
- **Issue:** [#81](https://github.com/portlandlang/portland/issues/81)

## Context

[ADR 0036](0036-2026-08-18-the-famous-twins-ship.md) shipped the builtin twins and declined the *machinery* — but a review caught that "alias went out with the metaprogramming" lumped two different things together. `alias_method` (computed names at runtime) is metaprogramming and stays gone. `alias` the keyword is a static declaration whose Ruby hazards were all redefinition-shaped: Ruby's `alias` *snapshots* the method at that point in class-body execution — alias it, redefine over it, and the alias answers the old body — which is exactly what made alias-method-chain monkeypatching work. Portland has no redefinition and no reopening, so the snapshot/name distinction vanishes, and the deciding user wanted the word itself, not just the `def fu = foo` workaround.

## Decision

**`alias new_name old_name` — Ruby's spelling, Portland's semantics.**

- **Syntax is Ruby's:** the keyword, bare words, new name first. The symbol-taking form was `alias_method`'s and does not exist. `?` names alias like any method name; `!` refuses with the binding rule's own words (a name ending in `!` could never be reached, ADR 0027).
- **Semantics are simpler than Ruby's, necessarily:** with no redefinition there is nothing to snapshot, so the alias and the original are the same method forever. In the seed this is the memory layout — the same `Rc<Method>` inserted under a second qualified name; in the compiler, the same slot value bound twice. "One body, two names" is structure, not promise, the same construction as ADR 0036's twins.
- **The target must already exist.** Programs read top to bottom, and an alias of nothing is a typo, not a plan: `alias points at nothing — no method foo defined yet`, both oracles, pinned. (Stated here per ADR 0034 §1 — this is a build-time-shaped refusal the interpreter merely previews.)
- **Where it works today:** top level and module bodies (qualified like any `def`). Struct and trait bodies wait for a pull — the natural refusal stands in the meantime. Aliasing a *builtin* (`alias total sum`) also waits: builtins have no method entry to share, and the want, if real, arrives with the stdlib story ([#78](https://github.com/portlandlang/portland/issues/78)).

## Consequences

- The ledger's [aliases.md](../ruby/aliases.md) gains its user-facing half: twins for the stdlib, `alias` for user code, `alias_method` gone — three fates, each stated.
- Ruby code using `alias` for its overwhelmingly common purpose (a friendlier second name) migrates verbatim. Code using it for alias-method-chain gets the no-redefinition refusal at the *redefinition*, which was already true before this ADR.
- The checker has nothing to check: both names are static, existence is enforced at the definition site, and unknown nodes pass silently (principle 4) — though a future increment could hoist the exists-check to build time for dead-code aliases, the same move ADR 0034 made for enum construction.
