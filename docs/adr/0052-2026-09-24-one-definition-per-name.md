# 0052 — One definition per name: redefinition refuses, and the builtins are not for taking

- **Status:** Accepted (recommended and built in both oracles, 2026-09-24, under the deciding user's standing "make the case, recommend, build; course-correct after")
- **Date:** 2026-09-24
- **Issue:** [#70](https://github.com/portlandlang/portland/issues/70)

## Context

A second definition of a top-level name silently replaced the first on both oracles — a def, a struct, an enum, a trait — and no ADR said so. Sharpest of all, the builtins were redefinable: `def panic(reason)` then `panic("fatal")` printed and carried on, which unenforced ADR 0010's "the only crash is one you typed" and would have let `portland report panic` (#30) read a clean audit off a program with no crash left in it. It nearly shipped a lying spec suite once (#69), when six spec files' `struct Token`s shared a scope and whichever ran last went green against the wrong one.

The issue set out three options: refuse redefinition outright; refuse it only for builtins and bless last-wins for user names; or bless last-wins whole with an ADR.

## Decision

**Refuse redefinition outright — a name is defined once — and a builtin's name is never taken.**

- **The case for.** Portland dropped runtime metaprogramming (ADR 0004): open classes, `define_method`, monkeypatching. In Ruby, redefinition is load order doing something deliberate; with the machinery gone, a second `def apply` is a typo or a paste, and last-wins turns it into a silent choice. The no-shadow rule (ADR 0001) already refuses a local over a method, a `mutable` redeclared, and `duplicate method X in struct Y`; method-over-method at the top level was the one shadowing nobody had fenced. The corpus cost was zero: nothing in the compiler, the fixtures, or the spec suite defines a name twice.
- **The case against, and the answer.** Half a fence — builtins only — protects ADR 0010 but leaves the general hole. Blessing last-wins records the status quo but blesses the least defensible thing: `require_relative` already loads once, so what last-wins serves is mostly accidents. Neither survives the corpus finding that no real program needed it.
- **Scope.** A top-level def, struct, enum, trait, alias's new name, or constant (ADR 0053) is declared once across the whole program — a required file's declarations count, since they share the requirer's tables. A module body is walked with its names qualified, so `A::x` twice refuses and `module A` reopened to add a second name does not; reopening is how ADR 0021's nesting forms already compose.
- **The wordings**, ADR 0047's voice, both oracles to the byte: `'apply' is already defined — rename one` at the second site, and `'panic' is a builtin — rename yours`. The checker says them at build with the line beneath; the seed and the hosted runtime say them when the second definition runs.
- **The one exemption is the REPL.** Redefining `greet` mid-session is the point of a session; the seed's interpreter carries a flag the REPL sets and a program never does.

## Consequences

- Struct, enum, trait, alias, and assignment nodes carry their line now, the #89 rule ("only when a check needs them") having been met.
- The seed gains one refusal function called from every definition site; the hosted runtime keeps the names declared so far in a binding that survives method scopes and crosses requires, since an enum binds no name of its own to look up.
- The checker's declarations gain the constants a file declares, so a requirer rebinding a required file's constant hears "bound once" rather than "already defined" — the seed's distinction, kept at build.
- Pinned by a two-oracle refusal test over the sentences and a build-time location test. Ledger: [redefinition.md](../ruby/redefinition.md).
- ADR 0010 and #30 are enforceable now: no program can define its way out of `panic`.
