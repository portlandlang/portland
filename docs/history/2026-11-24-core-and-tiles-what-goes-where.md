# Core and tiles: what goes where

**Summary:** A draft sorting of Portland's library surface into a core base, core tiles, default tiles, and external tiles, with Ruby's core and standard library put in the same terms.

**Status:** exploration, not a decision. Captured from a design conversation, 2026-11-24, to think against. [ADR 0064](../adr/0064-2026-10-25-the-standard-library-core-default-tiles-external-tiles.md) decided the three tiers and [#174](https://github.com/portlandlang/portland/issues/174) owns the package system. A row marked "draft" is a proposal from the conversation and nothing more.

## The idea being tested

Core stays one tier, as ADR 0064 says, but is made of tiles: the same layout and the same loader a default tile would use. What makes a tile core is two facts and not a different mechanism: it needs no `require`, and its version is the language's. A default tile differs on exactly those two facts.

Two tests for core come apart, and the split below follows them:

- Does the language itself need it? Array does: literals, `each`, the checker, and the native build all know it. That is the **core base**, which cannot be pulled apart.
- Does a Ruby programmer expect it with no `require`? `Time`, `Math`, and `File` pass this test and fail the first. Each is plain Portland over a few floor members. Those are the **core tiles**.

## Portland

### Core base

Version pinned to the language. No `require`. One unit that cannot be separated.

| Thing                                                                                       | Status                                                                                                                                                                                                                                                                          |
| ------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Array                                                                                       | decided                                                                                                                                                                                                                                                                         |
| Boolean                                                                                     | decided                                                                                                                                                                                                                                                                         |
| Complex                                                                                     | decided                                                                                                                                                                                                                                                                         |
| Float                                                                                       | decided                                                                                                                                                                                                                                                                         |
| Hash                                                                                        | decided                                                                                                                                                                                                                                                                         |
| Integer                                                                                     | decided                                                                                                                                                                                                                                                                         |
| MatchData                                                                                   | decided                                                                                                                                                                                                                                                                         |
| Random                                                                                      | decided as core ([#169](https://github.com/portlandlang/portland/issues/169)); in the base because the seed builds it as a value with a generator behind it. [#548](https://github.com/portlandlang/portland/issues/548) proposes a change that would make part of it separable |
| Range                                                                                       | decided                                                                                                                                                                                                                                                                         |
| Rational                                                                                    | decided                                                                                                                                                                                                                                                                         |
| Regexp                                                                                      | decided                                                                                                                                                                                                                                                                         |
| Set                                                                                         | decided                                                                                                                                                                                                                                                                         |
| String                                                                                      | decided                                                                                                                                                                                                                                                                         |
| Symbol                                                                                      | decided                                                                                                                                                                                                                                                                         |
| `nil`, a present maybe, a failure, an enum case with a payload, a type as a value, a struct | builtin values that are not named types                                                                                                                                                                                                                                         |
| `Enumerable` and any other trait the types share                                            | decided                                                                                                                                                                                                                                                                         |
| The bare functions: `puts`, `p`, `print`, `format`, `putc`, `rand`, `srand`                 | decided                                                                                                                                                                                                                                                                         |
| `System`, the floor                                                                         | decided ([ADR 0068](../adr/0068-2026-10-25-system-is-the-floor-module.md)); the one part that can never be Portland source                                                                                                                                                      |

The type list is the seed's value kinds in `seed/src/value.rs`.

### Core tiles

Version pinned to the language. No `require`. Each its own unit. That core is split this way is the draft part.

| Thing                            | Written over           | Status                                                                                                                             |
| -------------------------------- | ---------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| `Time`                           | the wall clock         | core decided; its surface is [#334](https://github.com/portlandlang/portland/issues/334)                                           |
| `Math`                           | float primitives       | core decided ([#126](https://github.com/portlandlang/portland/issues/126))                                                         |
| `File`, `Dir`                    | `System` members       | core decided                                                                                                                       |
| `ENV`, `ARGV`, `Process`, `warn` | `System` members       | core decided                                                                                                                       |
| `JSON`                           | nothing; written twice | core decided ([ADR 0072](../adr/0072-2026-11-03-json-is-core-written-twice.md)); ADR 0064 names its Portland twin as a future tile |

### Default tiles

Ship with `pdx`. Loaded by `require "name"` with no manifest line. Own version and repo. Each `pdx` release still ships one particular version of each.

| Thing                                     | Status            |
| ----------------------------------------- | ----------------- |
| `base64`                                  | decided           |
| `logger`                                  | decided           |
| `securerandom`                            | decided           |
| `benchmark`                               | decided           |
| `json`, once tiles exist                  | named in ADR 0064 |
| `time` (parsing, ISO 8601), `date`        | draft             |
| `optparse`, `shellwords`, `strscan`       | draft             |
| `fileutils`, `find`, `tempfile`, `tmpdir` | draft             |
| `uri`, `digest`                           | draft             |

### External tiles

Declared in the manifest and installed. Own version.

| Thing                            | Status                                                               |
| -------------------------------- | -------------------------------------------------------------------- |
| `bigdecimal`                     | [#124](https://github.com/portlandlang/portland/issues/124)          |
| `csv`, `yaml`, `erb`             | draft                                                                |
| Time zone data                   | draft; it changes on the world's schedule and not the language's     |
| `net-http`                       | waits on `together` running in parallel                              |
| `open3` and anything that spawns | waits on [#179](https://github.com/portlandlang/portland/issues/179) |

### Tools

| Thing                                    | Status                                                                                                       |
| ---------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| The linter, a formatter, the spec runner | draft: default tiles that carry a command, pinned to the language's version because each parses the language |

### Declined

By ADR 0064: `ostruct`, `delegate`, `forwardable`, `singleton`, `observer`, `weakref`, `monitor`, `thread`, `timeout`, `English`, `pp`.

## Which have their version pinned to the language

| Tier           | Version                                                 | `require`                     |
| -------------- | ------------------------------------------------------- | ----------------------------- |
| Core base      | the language's                                          | none                          |
| Core tiles     | the language's                                          | none                          |
| Default tiles  | their own; one particular version ships with each `pdx` | by name, no manifest line     |
| External tiles | their own                                               | by name, with a manifest line |
| Tools          | the language's (draft)                                  | not decided                   |

## Ruby's terms in Portland's

| Ruby                                                                                    | What it is                                  | Portland                                                                             |
| --------------------------------------------------------------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------ |
| Core, the types (`Array`, `Hash`, `Set`)                                                | compiled into the interpreter               | core base                                                                            |
| Core, the rest (`Time`, `Math`, `File`, `Dir`, `ENV`, `Process`)                        | compiled into the interpreter               | core tiles                                                                           |
| Core, `Kernel` and `Random`                                                             | compiled into the interpreter               | core base                                                                            |
| Core, `Signal`, `IO::Buffer`, `ARGF`                                                    | compiled into the interpreter               | proposed never to port ([#175](https://github.com/portlandlang/portland/issues/175)) |
| Core, `Thread`                                                                          | compiled into the interpreter               | declined; `together` does its work                                                   |
| Default gems (`json`, `securerandom`, `time`, `digest`, `optparse`, `uri`, `fileutils`) | ship with Ruby, need `require`, own version | default tiles, except `json`, which is core, and the declined ones                   |
| Bundled gems (`base64`, `csv`, `bigdecimal`, `logger`, `benchmark`, `ostruct`)          | in the tarball, need a Gemfile line         | no such tier: each is default, external, or declined                                 |
| "Standard library"                                                                      | default and bundled gems together, loosely  | default tiles                                                                        |

## Questions the exercise turned up

- **One core tile or several.** The base is one unit. Whether `Time`, `Math`, `File`, and `JSON` are each a tile or one lump with the base is the open call. Several gives a boundary a linter can hold (the base must not reach into `Time`).
- **"No `require`" need not mean "always loaded".** The build sees the whole program ([ADR 0049](../adr/0049-2026-09-24-the-whole-program-door.md)), so a core tile could load only when its name appears, and a program that never says `Time` would not pay for it. Not measured.
- **Whether a tile can carry a command**, as a gem carries `exe/`. The linter is the first case. Not on #174's list.
- **Whether a tile can carry a native part.** ADR 0064 says a Rust-backed library is core by definition. Ruby's `openssl`, `zlib`, `psych`, and `digest` are C underneath; under the rule each is core or is rewritten in Portland, which suits `digest` and not `openssl`. Not on #174's list.
- **The names a program may not reopen** (`core_type_name?` in `compiler/core.pdx`) are Array, Float, Hash, Integer, Range, Set, String, and Symbol. Boolean, Complex, MatchData, Random, Rational, and Regexp are not among them, and whether a program's own `struct Regexp` is accepted was not tested.
