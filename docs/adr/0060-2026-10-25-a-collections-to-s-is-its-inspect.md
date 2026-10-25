# 0060 — A collection's `to_s` is its `inspect`

- **Status:** Accepted (ruled by the deciding user 2026-10-25 on the recommendation in [#160](https://github.com/portlandlang/portland/issues/160); built in the seed and the trio, differentially pinned, 2026-10-25)
- **Date:** 2026-10-25
- **Issue:** [#160](https://github.com/portlandlang/portland/issues/160)

## Context

Ruby's `Array#to_s` and `Hash#to_s` are their `inspect`: `["x", :a].to_s` is `["x", :a]`, and `"got #{names}"` interpolates the same way. Portland had rendered the elements with their own `to_s` — `[x, a]`, and `{a => x}` for a hash — a consistent house behavior that no ledger page decided and one spec pinned without a reason. Interpolating a collection is everyday Ruby, and the quotes and colons are what tell `["1"]` from `[1]` in a log line.

## Decision

1. **A collection's `to_s` is its `inspect` form**, and interpolation follows: `[1, "two"].to_s` is `[1, "two"]`, `"#{{a: 1}}"` is `{a: 1}`. Tie goes to Ruby (principle 2), and the old rendering was a silent difference (principle 5) in exactly the place people look at output.
1. **`join` is unaffected.** Ruby's `join` renders each element with its own `to_s`, nested arrays flattened into the join, and Portland's already did.
1. **Scalars are unaffected.** `:a.to_s` stays `a`, `"x".to_s` stays `x`, `nil` still has no `to_s` ([ADR 0006](0006-2026-07-22-absence-word-nil.md)).

## Consequences

- The seed's `Display` for arrays and hashes, and the hosted `render`, hand off to `inspect`; `puts list` and `"#{list}"` change their output where a collection was printed bare.
- Recorded in [inspect.md](../ruby/inspect.md).
