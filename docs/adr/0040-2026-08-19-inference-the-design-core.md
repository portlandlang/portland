# 0040 — Inference: the design core

- **Status:** Accepted (decisions 1–3 of the [#9](https://github.com/portlandlang/portland/issues/9) design session; increments build against this)
- **Date:** 2026-08-19
- **Issue:** [#9](https://github.com/portlandlang/portland/issues/9)

## Context

The design session this issue always pointed at opened once its own precondition held: the surface stopped moving (import complete, inventory empty, ADRs 0030–0039). The [scope brief](https://github.com/portlandlang/portland/issues/9#issuecomment-5339257813) laid a seven-decision ladder; the first three are the design core and are decided here. The rest wait on purpose: annotation syntax until a real file wants to write one (the type-*rendering* vocabulary rides the error-voice decision), generics until pulled, error voice until there are concrete draft errors to react to, and ADR 0035's tripwire until narrowing exists to widen coverage.

## Decision

**1. Parameter types are hybrid: structural contracts from the body, concrete types flowed from call sites.** A `def`'s parameter contract is *what the body demands of it* — "responds to `kind` and `text`" — computable from the def alone, so checking stays per-def modular and the duck test moves to compile time exactly as [types.md](../ruby/types.md) promised. On top of that, the checker flows *concrete* caller types through the contract: when every reachable call passes a `Token`, downstream inference inside the body is `Token`-precise and errors may speak nominally. The contract is the requirement; the concrete flow is sharpening. A first ruling chose call-sites alone; it was flagged for retiring the structural promise and for Crystal's whole-program costs (checking blocked on visibility, distant errors, hostility to separate compilation ahead of [#5](https://github.com/portlandlang/portland/issues/5)), and the hybrid was chosen instead — the modular half is load-bearing, the whole-program half is optional precision.

**2. Requirements are structural; implementation is declared; traits name shapes.** Nothing satisfies a contract by declaration — if it quacks, it passes. Nothing *has* behavior except by declaration — a struct is its fields, its defs, and its explicit `include`s (ADR 0028, untouched). Between the two, a trait's name doubles as vocabulary: annotations and error messages may say `Describable` to mean that method-set-as-shape, and a struct satisfies it structurally whether or not it wrote the `include`. `include` inherits implementation; it is never a conformance gate. The known cost — accidental satisfaction of a named shape — is accepted: passing was already structural, so the name adds vocabulary, not looseness.

**3. `Maybe[T]` is a wrapper type, and the narrowing inventory is the ADRs' prose made rules.** A union (`T | nil`) cannot represent `some(nil)`, and `[nil].first ≠ [].first` is load-bearing (ADR 0005) — so the type is a real constructor, auto-wrap is the type-level identity on plain values, and every partial lookup answers `Maybe[T]`. Seven forms narrow, each already stated in ADRs 0005–0010 or the ledger: exit guards (`return if x.nil?` — plain `T` below), `some?`/`nil?` branches, the or-guard, `or` on a maybe (the result is plain), `case/in` over `nil`/`some(…)` arms, `== nil`/`!= nil` branches, and `&.` (a maybe of the result). **Deliberately trimmed:** the two *new refusals* that ride the maybe layer — the dead guard (`nil?` on a never-maybe) and ADR 0007's `Boolean?`-under-`or` — wait until narrowing has proven itself in the suite; narrowing first, new errors after. The same caution as ADR 0035's boundary, applied by hand.

## The build ladder

- **3a** — the type representation and silent synthesis (literals, locals, operators, builtin results): zero refusals, suite green, types visible only in a debug driver.
- **3b** — checking mode: block parameters from receivers, empty literals from expectation, enum widening (ADR 0023). Still silent.
- **3c** — contracts: body-demand computation per def, caller flow-through. First candidate refusals drafted but **held for the error-voice decision** — nothing refuses until the wording doctrine is ratified against concrete drafts.
- **3d** — narrowing (the seven forms), then the trimmed refusals, then ADR 0035's tripwire fires: coverage re-drawn with maybe-typed subjects demanding exhaustive handling.

## Consequences

- types.md's structural sentence stays true, and gains the sharpening nuance — updated citing this ADR.
- The checker remains decline-by-default at every increment: inference *widens where checks apply*; a program that builds today can only gain diagnostics, never lose ground (ADR 0034 §4, ADR 0035 §1).
- The seed never learns any of this — it stays the dynamic oracle, and the differential harness is untouched by design.
