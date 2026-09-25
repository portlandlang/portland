# 0055 — No lazy tier: eager answers stay, external iteration is declined, and the rest waits on its real pull

- **Status:** Accepted (recommended 2026-09-25 under the deciding user's standing "make the case, recommend, build; course-correct after"; nothing to build — the eager behavior is what ships)
- **Date:** 2026-09-25
- **Issue:** [#80](https://github.com/portlandlang/portland/issues/80)

## Context

The [#23](https://github.com/portlandlang/portland/issues/23) import settled that Portland has no enumerators: `step` and `each_slice` answer the finished collection, `.to_a` on a collection is the identity, and the eager answers are spec-pinned ([enumerators.md](../ruby/enumerators.md)). #80 asked whether a lazy tier ever arrives. Ruby's enumerators buy three separable things, and each has its own answer.

## Decision

1. **No intermediate arrays — left to the compiler.** `.lazy.map { }.select { }.first(5)` exists so a chain does not allocate at each link. A compiler that fuses `.map`/`.select` chains gets that win without a runtime laziness type, and fusion is already the compile pipeline's plan ([#5](https://github.com/portlandlang/portland/issues/5), [#13](https://github.com/portlandlang/portland/issues/13)). Fusion is timing, not meaning: the pinned eager answers are what a fused chain must still produce.
1. **External iteration — declined.** `enumerator.next` pauses a walk between pulls, which means a value carrying has-this-run-yet state that changes when read. Values never mutate ([ADR 0015](0015-2026-07-23-values-never-mutate.md)); a stepper whose `next` answers something different each time is exactly the mutation that rule excludes. Where a program needs to walk two sequences in step, `zip` and index-walking cover it; where it needs a cursor, the cursor is a `mutable` index the program owns and can see.
1. **Infinite sequences — wait on the real pull.** `(1..).each` walked lazily is the one capability the eager tier cannot express. Nothing in the compiler, the spec suite, or the fixtures reaches for it, and the likeliest pull — streaming IO, a generator over a file too large to hold — belongs to the stdlib's OS surface ([#78](https://github.com/portlandlang/portland/issues/78)). Until that pull arrives, a beginless or endless range still refuses to be walked, loudly, which keeps the door open: a lazy walk added later can only make a refusal succeed, never change an answer.

### The case against, and the answer

A migrating gem with `.lazy` chains or `Enumerator.new` fails at the first use, where a lazy tier would have carried it. The corpus has not been asked for their frequency; the answer does not turn on it, since the failure is loud and the rewrite (materialize, or restructure the loop) is local. If the census ever shows `.lazy` is common in maintained gems, that is the reopening signal for decision 3, filed as its own issue.

## Consequences

- Nothing changes in either oracle; the eager answers remain the spec-pinned semantics every later tier must preserve.
- The ledger's [enumerators.md](../ruby/enumerators.md) cites this ADR in place of its "no ADR" status line.
- The issue closes. Decision 3 reopens as a new issue when a real file pulls for an infinite walk, most likely out of #78.
