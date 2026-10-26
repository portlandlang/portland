# The standard library

**Summary:** Ruby's current shape, not its 1.8 one — a language core that needs no `require`, default tiles installed beside `pdx` that are real packages, and external tiles; no bundled tier, no monolithic stdlib.

**Status:** decided ([ADR 0064](../adr/0064-2026-10-25-the-standard-library-core-default-tiles-external-tiles.md), 2026-10-25, out of [#78](https://github.com/portlandlang/portland/issues/78)); nothing built yet — the first primitives are [#176](https://github.com/portlandlang/portland/issues/176), the package system is [#174](https://github.com/portlandlang/portland/issues/174).

## Ruby

Three tiers, and the word "stdlib" covers all of them loosely. **Core** is compiled into the interpreter — `Array`, `Hash`, `Set`. **Default gems** (46 in Ruby 4.0) ship with Ruby and always `require` — `json`, `securerandom`, `time`, `digest`, `timeout` — but each is a gem with its own repo, version, and release cadence. **Bundled gems** arrive in the tarball as ordinary installed gems that Bundler ignores unless the Gemfile names them: `base64`, `csv`, `bigdecimal` since 3.4; `logger`, `benchmark`, `ostruct` since 4.0; `tsort` from 4.1. The movement is one way, core → default → bundled → out, and every step moved a `require` line into a gemspec dependency across the ecosystem — which is why `logger` and `base64` top the corpus's list of dependencies holding back otherwise-parsing gems.

The OS surface is core: `File`, `Dir`, `ENV`, `Time`, `Process`, `Signal`, `IO` and its descriptors.

## Portland

**Core** needs no `require`, ships inside `pdx`, and is versioned with the language: the builtin types, plus what Ruby made core — `Set`, `Math`, `Random`, `Time` — as each is decided. Core also owns the **floor**: a small set of coarse primitives, one crossing each, that are the only things written in Rust. Today those are `System.arguments`, `System.read_file`, `System.write_file`, `puts`, and `p`; the first wave adds a wall clock, a monotonic clock, entropy, a write to stderr, and a file append. Core also holds anything Rust-backed that is not a floor crossing — today `JSON.parse` and `JSON.generate` over `serde_json`, a recorded loan; a Rust-backed library cannot be a tile, so `json` is core until it is written in Portland.

**Default tiles** are Portland packages installed beside `pdx` — their own version and repo, upgradable without upgrading the language — that load by `require "name"` with no manifest line. The first four, in the corpus's order: `base64`, `logger`, `securerandom`, `benchmark`. Each is spelled as Ruby spells it: `Base64.strict_encode64`, `Logger.new`, `SecureRandom.hex`, `Benchmark.measure`.

**External tiles** are declared in the manifest and installed. Everything else lives here.

**No bundled tier.** A library is in the default set or it is external; nothing arrives with `pdx` yet needs a line to load.

**Declined by doctrine**, each refusing by name when the tile system exists: `ostruct`, `delegate`, `forwardable`, `singleton`, `observer`, `weakref`, `monitor`, `thread`, `timeout`, `English`, `pp`. The reasons are the ledger's existing ones — [metaprogramming.md](metaprogramming.md), [mutability.md](mutability.md), [concurrency.md](concurrency.md), [removed-syntax.md](removed-syntax.md).

**The OS surface** is Ruby's spelling over the floor: `File.read` over the read primitive, `Time.now` over the clock, each arriving when a real file pulls for it. The floor is coarse — `File.read` hands back the whole file — so streaming a file larger than memory has no spelling yet; the fine handle model is parked, not refused.

## Migration

- `require "logger"` and the rest of the first four migrate verbatim once their tiles exist — _free_. Until then they refuse, pointing here ([#116](https://github.com/portlandlang/portland/issues/116) row 4). `require "json"` is a no-op while `json` is core, as `require "set"` is in Ruby 4 — whether it refuses or is accepted silently is #116's.
- A Gemfile line for a bundled gem (`gem "csv"`, `gem "base64"`) has no Portland twin; a default tile needs none, and an external tile's declaration is [#174](https://github.com/portlandlang/portland/issues/174)'s shape — _gem-supplied_ in the linter's terms, since the manifest is not Ruby.
- `require "ostruct"` and the other declined names fail loudly at the require, never silently; the rewrite is the one the cited ledger page names.
- `File.open(path) { |file| file.each_line { } }` has no translation until the fine model lands; `File.read(path).lines.each { }` is the coarse spelling, and it holds the whole file.
