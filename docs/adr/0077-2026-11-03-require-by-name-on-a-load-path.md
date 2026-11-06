# 0077 — `require "name"`: a load path, `.pdx` or `.rb`, literal and top-level

- **Status:** Accepted (ruled by the deciding user 2026-11-03, accepting the recommendation in [#116](https://github.com/portlandlang/portland/issues/116); built on both implementations 2026-11-06 under [#190](https://github.com/portlandlang/portland/issues/190) — the load path is `-I` and the program's `lib/` until the tile manifest ([#174](https://github.com/portlandlang/portland/issues/174)) adds the dependencies')
- **Date:** 2026-11-03
- **Issue:** [#116](https://github.com/portlandlang/portland/issues/116)

## Context

`require_relative` is built on both implementations: resolved against the requiring file, `.pdx` implied, each file loaded once, declarations crossing and locals not ([ADR 0049](0049-2026-09-24-the-whole-program-door.md), [#95](https://github.com/portlandlang/portland/issues/95)). No gem loads without `require "foo/bar"` — a bare name found on a load path — and today `require` refuses as a retired spelling pointing at `require_relative`, which is the wrong answer for a gem's `lib/`. [ADR 0064](0064-2026-10-25-the-standard-library-core-default-tiles-external-tiles.md) made the standard library tiles that load by `require "name"`, so this is also the tile system's front door ([#174](https://github.com/portlandlang/portland/issues/174)).

## Decision

1. **`require "foo/bar"` searches a load path in order**: the program's own `lib/`, then each dependency's `lib/`, then the default tiles'. `pdx -I dir` adds a directory at the front, as `ruby -I` does. How a dependency's `lib/` gets onto the path is [#174](https://github.com/portlandlang/portland/issues/174)'s manifest; until it exists, the path is the program's `lib/` and `-I`.
1. **A name without an extension finds `foo/bar.pdx` or `foo/bar.rb`.** If both exist, it refuses and names both (principle 3), never picks. A name with its extension finds that file.
1. **The argument is a string literal, and the call sits at a file's top level**, so the build sees every file it links ([ADR 0049](0049-2026-09-24-the-whole-program-door.md)): the require closure stays a static fact. A computed name, or a `require` inside a method, block, or conditional, refuses with the rewrite named — move it to the top of the file. This is the one divergence, and it is the closed runtime's (thesis), not taste; Ruby's `require` inside a method is a lazy-loading idiom, and the build loads everything once anyway.
1. **A core name is a quiet no-op answering `false`.** `require "json"` and `require "set"` name things that are core here ([ADR 0064](0064-2026-10-25-the-standard-library-core-default-tiles-external-tiles.md), [ADR 0072](0072-2026-11-03-json-is-core-written-twice.md)) and need no require, so the line does nothing and answers `false` — exactly what Ruby 4's `require "set"` does for a library already loaded. The line stays legal because a file that runs under both Ruby and Portland is the migration asset the free tier exists for (the ledger's polyfill test); the migration linter may flag it as removable, as a lint, never a refusal. A default tile's name resolves on the path like any other once the tiles ship, and until then it is a missing file, refused by ruling 5 — `require "base64"` stays loud. (ADR 0064 left this to [#174](https://github.com/portlandlang/portland/issues/174); it was first recorded here as a refusal and amended to the no-op on 2026-11-05 by the deciding user.)
1. **A missing file refuses in [ADR 0047](0047-2026-09-22-the-error-voice.md)'s voice** — `cannot load 'foo/bar' — not found on the load path (lib/, …)` — instead of the panic `require_relative` gives today, which this ADR brings to the same sentence.
1. **`require` answers true the first time and false after**, as Ruby's does and `require_relative` already does.

## Consequences

- The retired-spelling row for `require` is retired; `require_relative` keeps its path semantics unchanged.
- The whole-program door's walk follows `require` as it follows `require_relative`, resolving on the path, so the checker sees a gem's `lib/` as one program.
- `.rb` files are parsed as Portland: the file extension is Ruby's, the grammar is Portland's, and a `.rb` file that uses what Portland refuses refuses at its line as any file does. This is what makes a gem's `lib/` loadable without renaming.
- Ledger: a new [requires.md](../ruby/requires.md); `core/kernel/require_spec.rb` and `require_relative_spec.rb` port under [#185](https://github.com/portlandlang/portland/issues/185) once built.
