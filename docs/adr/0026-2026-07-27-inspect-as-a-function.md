# 0026 — `inspect` is a function, because nil is its most important input

- **Status:** Tentative (one of two competing drafts for [#38](https://github.com/portlandlang/portland/issues/38); the branch not merged closes unmerged)
- **Date:** 2026-07-27

## Context

`p` prints a value's source-shaped rendering and hands the value back — the whole of interactive debugging. What it cannot do is hand over the rendering **as a string**, so nothing composed can use it: not a panic message, not a log line, and not the spec harness's failure lines, which rendered through `to_s` and could not tell `"ROSE"` from `:ROSE` from `ROSE`. The rendering itself has existed on both oracles since [#39](https://github.com/portlandlang/portland/issues/39); the only open question was the spelling of the door to it.

Ruby's spelling is a method, `Object#inspect`, and `nil.inspect` answers `"nil"` — which works *because* Ruby's nil is a full object with a class and dozens of methods. That is precisely what Portland removed: absence has no methods (ADR 0006), and `nil?`/`some?` are "the one dispatch a maybe allows." A method spelling therefore forces a sub-choice Ruby never faces: refuse `nil.inspect` (the debugging tool failing on the value you most want to see) or widen nil's method surface from two to three (chipping the language's central wall for a convenience).

## Decision

**`inspect(value)` is a builtin function**, beside `p`, `puts`, `panic`, and `some` — p without the printing. A function takes absence as an *argument* without ceremony: `inspect(nil)` is `"nil"`, and ADR 0006's wall stands exactly where it was built. The precedent is already in the language: `some(nil)` works this way and for the same reason.

One argument, exactly: `inspect(a, b)` is refused. The rendering is the one `p` prints — strings with their quotes, symbols with their colons, hash pairs written the way they would be typed, structs as their constructors read.

## Consequences

- The spec harness's `shown` workaround is deleted, as its own comment asked; failure lines now read `expected nil, got "rose"` — unambiguous about type and presence.
- A migrating Rubyist writes `inspect(value)` where they wrote `value.inspect` — one mechanical spelling change, recorded in [docs/ruby/inspect.md](../ruby/inspect.md). The behavior on every value, nil included, is exactly Ruby's.
- If the object model (#27) later brings machinery that makes a method spelling natural, the function is one deprecation away — the rendering engine underneath does not move.
