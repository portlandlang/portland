# 0064 — Randomness: Ruby's both, step for step

- **Status:** Accepted (ruled by the deciding user 2026-10-25 against the recommendation in [#169](https://github.com/portlandlang/portland/issues/169), choosing its option 2; built in the seed and the trio, differentially pinned, 2026-10-25)
- **Date:** 2026-10-25
- **Issue:** [#169](https://github.com/portlandlang/portland/issues/169)

## Context

Ruby draws random numbers two ways: ambiently — `rand`, `sample`, `shuffle` from a process-global generator that `srand` reseeds — and explicitly, from a `Random.new(seed)` handed in with `sample(random:)`, `shuffle(random:)`, and `random.rand(n)`. ruby/spec's core/array `sample` and `shuffle`, and core/random, needed a source; Portland had none. The recommendation was the explicit source alone, for reproducibility; the ruling took Ruby's both, since migrating code writes `rand(6)` and `list.shuffle` far more often than it threads a generator.

## Decision

1. **Both forms, as Ruby spells them.** Bare `rand`, `rand(n)`, `rand(float)` (Kernel's: truncated to a count, 0 falling back to a float), `rand(range)`, `srand(seed)` answering the seed before; `Random.new(seed)` as a type function ([ADR 0058](0058-2026-10-16-builtin-types-carry-type-functions.md)), `random.rand(...)` (restrictive: 0 or a negative bound refuses as `invalid argument - 0`), `random.seed`, `==` on two generators by seed and position; `sample`, `sample(n)`, `shuffle`, each with or without `random:`.
1. **The generator is Ruby's, step for step.** MT19937 seeded as `random.c` seeds it, the limited draw's retry mask, the 53-bit float, and CRuby's shuffle and sample walks — so `Random.new(42).rand(10)` is 6 here as there, `%w[a b c].shuffle(random: Random.new(1))` is `["a", "c", "b"]`, and a seeded test written against Ruby's numbers passes unchanged. ruby/spec pins these sequences by value.
1. **A generator is the one value that moves.** It advances as it is drawn from, by nature; it is compared by seed and position, and `inspect` writes the spelling that builds one from the same seed, `Random.new(42)`.
1. **Not built.** `Random.bytes`, `urandom`, and `Random::Formatter` (byte and encoding questions, [#119](https://github.com/portlandlang/portland/issues/119)); `Random::DEFAULT`, which the bare functions are; `random: Random` the class, since a type is not a value; float ranges, which ranges here do not hold ([#149](https://github.com/portlandlang/portland/issues/149)).

## Consequences

- `sample` of an empty array is nil, and a single `sample` is a maybe, as lookups are ([ADR 0010](0010-2026-07-22-partial-operations-return-maybes.md)).
- A program that never seeds gets a different sequence every run, as Ruby's does; `srand(seed)` or `Random.new(seed)` pins it.
- Recorded in [randomness.md](../ruby/randomness.md).
