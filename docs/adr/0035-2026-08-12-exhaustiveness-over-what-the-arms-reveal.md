# 0035 — Exhaustiveness over what the arms reveal

- **Status:** Accepted (built in the compiler, 2026-08-12)
- **Date:** 2026-08-12
- **Issue:** [#9](https://github.com/portlandlang/portland/issues/9) increment 2

## Context

[ADR 0013](0013-2026-07-22-case-in-spec.md) §1 decided that a `case/in` must cover its subject or say `else`, §3 that unreachable arms are compile errors, and §4 that guarded arms do not count toward coverage. [ADR 0019](0019-2026-07-23-ranges.md) specified how range arms prove totality: sort by start, require a beginless first and an endless last, require no gaps. [ADR 0022](0022-2026-07-25-enums-with-payloads.md) gave enums payloads specifically so that one exhaustiveness mechanism would serve both payload-free and payload-carrying cases.

All of it was deferred to #9 because the seed cannot do it: a tree-walking interpreter reaches one arm at a time and can only panic when none matched. [ADR 0034](0034-2026-08-11-the-checker-and-the-oracle-succession.md) built the walker that can, and checked the narrowest thing there is — that a named case exists. Coverage is the next thing up, and it raises a question ADR 0034 answered in principle but not in practice: **the checker has no types yet, so how does it know what set the arms are supposed to cover?**

The tempting answer is to apply ADR 0013 §1 literally right now — every `case/in` without an `else` that the checker cannot prove total is refused. That is the eventual rule, and it is the wrong rule today: without inference, `case status in nil then … in some(name) then … end` cannot be proven total either, and demanding an `else` on it would make the checker a nuisance rather than a proof. Principle 4 exists for exactly this.

## Decision

**1. The checker refuses only where it can prove non-coverage, and declines where it merely cannot prove coverage.** These are different failures and only one of them is the author's. A provable gap over a set the arms themselves establish is a refusal; anything else passes silently and waits for inference to widen it. This is ADR 0034 §4 applied to coverage, and it means every refusal added later can only turn silence into a diagnostic — never a wrong diagnostic into a right one.

**2. The arms reveal the set in exactly two shapes today.**

- **An enum**, when at least one arm is a payload-carrying case pattern (`in :paid(on:)`). Only a declared enum case can wear a payload, so that arm names the enum, and the enum's declaration is the closed set. Bare symbol arms (`in :pending`) count as coverage of that set once an anchor arm has established it, but they can never establish it themselves — a payload-free case *is* a symbol (ADR 0022), and symbols are legal everywhere, so a `case/in` over plain symbols that happen to share an enum's spelling must not be second-guessed.
- **The integers**, when every unguarded arm is a range pattern. Range patterns match nothing but integers, so a `case/in` made of them is claiming to cover the integers, and ADR 0019's algorithm decides whether it does.

Anything else — mixed arm kinds, struct patterns, arrays, literals without an enum anchor — is a decline, not a refusal.

**3. The wordings, stated here because no oracle produces them** (ADR 0034 §1: where no oracle exists, the deciding ADR is the oracle, and the test pins its exact words). The seed's runtime refusal names the value that arrived (`no pattern matched 5 — add an in branch or an else`); a build-time refusal has no value to name, so it names what is missing instead:

| Situation | Wording |
| --------- | ------- |
| Enum cases uncovered, no `else` | `` case/in does not cover :refunded — add the arm, or an else `` (several: `:refunded, :void`) |
| Integers left uncovered, no `else` | `` case/in leaves 6..9 uncovered — add the arm, or an else `` (beginless and endless gaps read `..0` and `10..`) |
| An arm below a bare capture | `` this arm can never match — the capture above binds every subject `` |
| A case named twice | `` in :paid can never match — an arm above already matches :paid `` |

The last two are ADR 0013 §3's unreachable-arm errors, and they share the shape of ADR 0034 §3's first original diagnostic: *X can never match — why*.

**4. Guarded arms count for nothing, in both directions.** A guarded arm does not cover its pattern (ADR 0013 §4 — the compiler cannot prove an `if`), and it is never reported as unreachable either, since the guard may be what makes it reachable. Guards are simply not part of the coverage arithmetic.

**5. An `else` ends every question.** Its presence makes coverage moot, so a `case/in` with an `else` is never refused for coverage — only for unreachability, which is about arms above, not below.

## Consequences

- Three of the seven checks in #9's inventory move from "runtime panic" to "build refusal" in the shape their ADRs specified: `case/in` exhaustiveness (partially — enum sets today), range totality (fully, for all-range arms), and unreachable arms (fully, for the two shapes that need no types).
- The remaining coverage work is now explicitly *inference-gated* rather than unbuilt: maybe-shaped subjects, struct-pattern sets, and the full ADR 0013 §1 rule need the subject's static type, which is the same increment that gives every other check its reach.
- The differential harness is untouched: these programs run identically on the seed, which reaches the same failure at runtime or not at all. Architecture's third row — seed runs, compiler refuses — grows its second family.
- Ledger: [static-checks.md](../ruby/static-checks.md) carries the coverage half; Ruby's `NoMatchingPatternError` is the runtime shape being replaced, per ADR 0013's migration promise.
