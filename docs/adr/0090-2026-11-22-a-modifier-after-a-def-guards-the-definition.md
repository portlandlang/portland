# 0090 — A modifier after a `def` guards the definition

<!-- generated: do not edit by hand — script/docs/generate -->

> [!IMPORTANT]
> **A decision record, not a reference.** It says why a ruling was made; a later ruling may have changed it without marking this file. See also: [language.md: Methods and calls](../../docs/language.md#methods-and-calls).

<!-- /generated -->

- **Status:** Accepted (ruled by the deciding user on [#238](https://github.com/portlandlang/portland/issues/238): "match Ruby"; built on both implementations 2026-11-22)
- **Date:** 2026-11-22
- **Issue:** [#238](https://github.com/portlandlang/portland/issues/238)

## Context

[ADR 0045](0045-2026-08-20-the-def-shapes.md) read an endless def's body as one guarded statement, so `def cautious = "ran" if false` defined `cautious` and answered nil when the guard failed. Ruby applies the modifier to the whole `def`: the method is never defined. The def shapes are presented as Ruby's, so this was an unrecorded divergence, and a silent one — the same line means two things.

The two implementations also disagreed on a `def` closed by `end`: the hosted parser read `end if false` as a guard on the definition, and the seed refused the modifier as a missing newline.

## Decision

1. **A modifier after a `def` guards the definition, as Ruby reads it.** `def cautious = "ran" if false` defines nothing; `def eager = "ran" if true` defines `eager`. `unless`, `while`, and `until` read the same way. This holds for the endless form and for a `def` closed by `end`. It is a non-difference from Ruby.
1. **An endless body is one statement**, not one guarded statement. A condition on the body goes in a `def` closed by `end`, or in a ternary.
1. **A struct body and a trait body refuse the modifier**, naming the def: their methods are a fixed set, read before anything runs, so there is no definition for a condition to guard. `def area in a struct body is always defined — 'if' cannot guard it; put the condition inside the method`.

## Consequences

- Both parsers hand the modifier to the statement that holds the `def`.
- The checker cannot know whether a condition held, so a def under a top-level condition or loop declares what a top-level def declares, and a call to it checks as a call to a defined method. Where the condition failed, the call refuses when it runs.
- A program that leaned on the guarded-body reading refuses at the first call of the undefined method, by name, where it once answered nil.
- Migration: Ruby's `def name = value if condition` ports verbatim, with Ruby's meaning.
