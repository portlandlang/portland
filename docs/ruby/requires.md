# `require` and `require_relative`

**Summary:** `require "name"` searches a load path for `name.pdx` or `name.rb`, and `require_relative` resolves against the requiring file; both take a string literal at a file's top level, since a program is its require closure, known at build.

**Status:** `require_relative` built on both implementations ([ADR 0049](../adr/0049-2026-09-24-the-whole-program-door.md), [#95](https://github.com/portlandlang/portland/issues/95)); `require` decided ([ADR 0077](../adr/0077-2026-11-03-require-by-name-on-a-load-path.md), 2026-11-03, out of [#116](https://github.com/portlandlang/portland/issues/116)) and not built — the build is [#190](https://github.com/portlandlang/portland/issues/190). Until it lands, `require` refuses as a retired spelling.

## Ruby

`require "foo/bar"` searches `$LOAD_PATH` for `foo/bar.rb` (or a native `.so`/`.bundle`), loads it once, and answers true the first time and false after; `require_relative` resolves against the requiring file. The argument may be any expression, the call may sit anywhere — inside a method as lazy loading, inside a `rescue LoadError` as a feature probe — and a missing file raises `LoadError` at the moment it runs. RubyGems extends `require` to activate a gem whose `lib/` holds the name; Bundler narrows the path to the Gemfile's gems.

## Portland

```ruby
require "shop/token"          # lib/shop/token.pdx, or lib/shop/token.rb
require_relative "helpers"    # ./helpers.pdx beside this file
```

- **The load path** is the program's own `lib/`, then each dependency's `lib/`, then the default tiles' ([standard-library.md](standard-library.md)); `pdx -I dir` adds a directory at the front. How a dependency's `lib/` joins the path is the tile manifest's ([#174](https://github.com/portlandlang/portland/issues/174)); until it exists the path is `lib/` and `-I`.
- **`.pdx` or `.rb`.** A name without an extension finds either; both present refuses and names both, never picks. A `.rb` file is parsed as Portland — the extension is Ruby's, the grammar is this one's — which is what lets a gem's `lib/` load without renaming.
- **A string literal, at the top level.** The build walks every `require` to see the whole program ([ADR 0049](../adr/0049-2026-09-24-the-whole-program-door.md)), so a computed name or a `require` inside a method, block, or conditional refuses with the rewrite named: move it to the top of the file. This is the one divergence, and it is the closed runtime's, not taste.
- **A core name is a quiet no-op.** `require "json"`, `require "set"`: these are core here and need no line, so the line does nothing and answers `false`, as Ruby 4's `require "set"` does for a library already loaded — a file that runs under both Ruby and Portland keeps running. The linter may flag the line as removable. A default tile's name resolves on the path like any other once the tiles ship; until then it is a missing file and refuses, so `require "base64"` stays loud.
- **A missing file refuses** in [the error voice](../adr/0047-2026-09-22-the-error-voice.md): `cannot load 'foo/bar' — not found on the load path (lib/, …)`.
- **Each file loads once**; `require` answers true the first time and false after, as Ruby's does. Declarations cross; locals do not.

## Migration

- A gem's `require "gem_name/part"` lines at the top of each file port verbatim once the gem's `lib/` is on the path. _Free._
- `require "json"` and other core names: port verbatim, a no-op answering `false`; the linter can offer to delete the line. _Free._
- `require` inside a method or a `rescue LoadError` probe: move to the top of the file, or drop the probe — a build either has the file or refuses. _Rewrite, loud._
- `$LOAD_PATH.unshift` and `$:` are globals, gone ([removed-syntax.md](removed-syntax.md)); `-I` is the spelling.
