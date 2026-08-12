# 0034 — The checker arrives, and the oracle succession begins

- **Status:** Accepted (settling the doctrine half of [#9](https://github.com/portlandlang/portland/issues/9)'s first increment; built the same day — `compiler/checker.pdx`, wired ahead of evaluation)
- **Date:** 2026-08-11

## Context

The trio grew its first walker the seed will never have: a checker, run before evaluation, refusing programs whose *written* code is wrong whether or not it would ever run. [Architecture.md](../architecture.md) predicted this moment — the contract's third row, *trio refuses, seed runs* — and predicted it would look like a regression the first time it happened. It happened on 2026-08-11, with the narrowest check that exists: ADR 0022's closed vocabularies, statically.

The doctrine question it forces: **"the seed is the oracle" (principle 7) cannot govern a refusal the seed cannot make.** A build-time wording has no seed output to match byte-for-byte. Something has to say what pins it.

## Decision

1. **The checker is the trio's alone, forever.** The seed stays a runtime; teaching it static checks would be polishing the disposable half. The differential harness narrows to programs both accept — which stays the overwhelming majority — and the third row becomes its own test axis: *the seed runs this program to completion; the trio refuses it; both facts are asserted.*
1. **Where a check moves an existing runtime refusal to build time, the seed's wording moves with it, byte for byte.** The enum construction checks say exactly what `check_payload_labels` says at runtime — `no enum declares a case :shipped`, `` `:paid` takes (on:) `` — because the check is the same check; only the timing moved. The seed remains the oracle for the *words* even where it stops being one for the *moment*.
1. **Where a check has no runtime analog, the ADR that decides the check states its wordings, and hand-written tests pin them.** Principle 7's "never hand-write expected output" amends to: *never hand-write what an oracle can produce; where no oracle exists, the deciding ADR is the oracle, and the test pins its exact words.* The first such wording exists as of this ADR: a pattern naming an undeclared case is a branch that can never fire — the seed answers it with silence — and the trio says `in :payed can never match — no enum declares a case :payed`.
1. **The checker declines wherever it cannot tell** (principle 4, now as method rather than apology): unknown node kinds pass through silently, bare symbols are never checked (a payload-free case is simply a symbol, ADR 0022), and each future check fires only where the AST alone proves it applies. Inference (#9's later increments) widens where checks apply; it does not gate their existence.
1. **The whole tree is read before any line is judged** — declarations register program-wide, so a construction textually before its enum's declaration checks clean. The seed's runtime stays order-sensitive; a program that passes the checker may still refuse at runtime, which is a gap, not a contradiction.

## Consequences

- The milestone test (`the_checker_refuses_what_the_seed_cannot_see`) holds four programs the seed runs happily and the trio refuses — including the dead-branch label typo and the pattern typo, the classic silent bugs. These are the suite's first hand-written expectations, and this ADR is why that is now allowed.
- Spec files cannot cover checker refusals — every spec runs on both oracles, and a checker-refused program has no seed run to agree with. Refusal coverage lives in the Rust axis; the spec suite stays the both-accept corpus.
- Hosted runs pay one extra AST walk (~5s across the 116-file spec suite); the bench watches it like everything else.
- `compiler/check.pdx` is the checker's own door — check without running. The seed has no counterpart, on purpose.
- Exhaustiveness, flow narrowing, and the rest of #9's inventory land as further check families under rules 3 and 4, each with its wordings ADR-stated and test-pinned.
