# 0063 — A hash has no default

- **Status:** Accepted (ruled by the deciding user 2026-10-25 on the recommendation in [#170](https://github.com/portlandlang/portland/issues/170); `Hash.new` and the refusals built in the seed and the trio, differentially pinned, 2026-10-25)
- **Date:** 2026-10-25
- **Issue:** [#170](https://github.com/portlandlang/portland/issues/170)

## Context

In Ruby, `Hash.new(0)[:missing]` is 0, and `Hash.new { |hash, key| hash[key] = [] }` builds nested structures on first touch; `default` and `default_proc` read them back. The counting idiom `counts[word] += 1` leans on the first, and the grouping idiom on the second.

In Portland a lookup answers a maybe ([ADR 0010](0010-2026-07-22-partial-operations-return-maybes.md), [lookups.md](../ruby/lookups.md)): `counts[:missing]` is nil, and the idioms are `(counts[word] or 0) + 1`, `tally`, and `group_by`. A default would make `[]` total on some hashes and partial on others — the one hash type with two lookup behaviors — and a default proc is a mutating callback inside a value, which [ADR 0015](0015-2026-07-23-values-never-mutate.md) has no room for.

## Decision

1. **A hash has no default.** `[]` on every hash answers a maybe; the or-guard is the default, written at the lookup, lazily, as [lookups.md](../ruby/lookups.md) already has it for `fetch`.
1. **`Hash.new` bare builds `{}`**, a type function in [ADR 0058](0058-2026-10-16-builtin-types-carry-type-functions.md)'s shape, for migrating code.
1. **`Hash.new(value)` and `Hash.new { }` refuse naming the or-guard**: `'Hash.new(0)' gives a hash a default, which Portland does not have — a lookup answers a maybe; write 'hash[key] or 0'`.
1. **`default` and `default_proc` do not exist**, and say so: `'default' has no Portland meaning — a hash has no default (ADR 0063); a lookup answers a maybe, so write 'hash[key] or fallback'`.

## Consequences

- The counting idiom migrates to `tally`, or `counts[word] = (counts[word] or 0) + 1`; the grouping idiom to `group_by`.
- ruby/spec's core/hash `new`, `default`, and `default_proc` are recorded: the empty-hash row ports, the rest are decided out.
- Recorded in [lookups.md](../ruby/lookups.md).
