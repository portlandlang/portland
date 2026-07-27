# 0026 — `inspect` is a method, and nil's third

- **Status:** Tentative (one of two competing drafts for [#38](https://github.com/portlandlang/portland/issues/38); the branch not merged closes unmerged)
- **Date:** 2026-07-27

## Context

`p` prints a value's source-shaped rendering and hands the value back — the whole of interactive debugging. What it cannot do is hand over the rendering **as a string**, so nothing composed can use it: not a panic message, not a log line, and not the spec harness's failure lines, which rendered through `to_s` and could not tell `"ROSE"` from `:ROSE` from `ROSE`. The rendering itself has existed on both oracles since [#39](https://github.com/portlandlang/portland/issues/39); the only open question was the spelling of the door to it.

Ruby's spelling is `Object#inspect`, and `nil.inspect` answers `"nil"`. Portland's wall is that absence has no methods (ADR 0006) — `nil?` and `some?` were "the one dispatch a maybe allows" — so the method spelling forces the question Ruby never faces: what does `nil.inspect` do?

## Decision

**`value.inspect` is a method on every value, nil included** — Ruby's spelling and Ruby's behavior, a deliberate **non-difference**. `nil.inspect` answers `"nil"`, because a debugging rendering that refuses the value you most need to see would fail at its one job, and `p` already promised the behavior in prose: "p renders nil for debugging."

The honest cost, stated as the amendment it is: **the maybe's dispatch surface widens from two to three** — `nil?`, `some?`, `inspect`. The justification is that all three share a property no other method has: they are total over presence and absence *by meaning*, not by exception. `nil?`/`some?` ask which side you are on; `inspect` names the side in writing. Nothing about `upcase` or `length` can say that, so the wall against ambient nil's method soup still holds — it just counts to three now.

The rendering is the one `p` prints — strings with their quotes, symbols with their colons, hash pairs written the way they would be typed, structs as their constructors read. A struct may define its own `inspect`, which wins over the builtin, exactly as method dispatch already works.

## Consequences

- Maximum Ruby fidelity: a migrating Rubyist's `value.inspect` works unchanged, nil included. No ledger entry needed beyond this ADR's non-difference note.
- The spec harness's `shown` workaround is deleted, as its own comment asked; failure lines now read `expected nil, got "rose"`.
- Every place that documents "the one dispatch a maybe allows" is amended in the same breath (language.md, ADR 0006's gloss); the count is the price paid for the spelling.
- If a future decision regrets the third method, the walk-back is mechanical: `value.inspect` → `inspect(value)` is a regex, and the rendering engine underneath does not move.
