# Core and tiles: what goes where

**Summary:** A draft sorting of Portland's library surface into a core base, core tiles, default tiles, and external tiles, with Ruby's core and standard library put in the same terms, and a survey of what macOS could supply in place of code Portland would write.

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

## What the system could provide

A second thread from the same conversation. Portland runs on Apple silicon only, so anything macOS ships is simply there: less to write once for the seed, again for the hosted compiler, and again for native, and what is there has had far more testing than a new port would. The survey below asks, of what is built and what is on or near the roadmap, which of it the system could supply.

### Built

| What                                | Size in the seed                               | The system offers                                                                                                   | Could it replace ours                                                                       |
| ----------------------------------- | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| Regex Unicode tables                | the largest file in the seed by far, generated | ICU character properties (`uchar.h`, in the SDK)                                                                    | Probably, for the property lookups                                                          |
| Regex engine                        | large                                          | ICU regex (`uregex.h`), POSIX `regex.h`                                                                             | No. Neither has Ruby's syntax or semantics, and the linear-time guarantee is Portland's own |
| Number formatting (`format`)        | medium                                         | `snprintf`                                                                                                          | No. Ruby rounds `2.675` to `2.68`; libc gives the correctly rounded `2.67`                  |
| JSON                                | medium                                         | Nothing in C, as [ADR 0064](../adr/0064-2026-10-25-the-standard-library-core-default-tiles-external-tiles.md) found | No                                                                                          |
| Normalization and grapheme clusters | two crates                                     | CoreFoundation's string functions; ICU's public headers include neither                                             | Yes, with the version cost below                                                            |
| Random                              | small                                          | `arc4random`                                                                                                        | In part ([#548](https://github.com/portlandlang/portland/issues/548))                       |
| Math                                | small                                          | libm                                                                                                                | It is the system's, through Rust                                                            |
| Clocks, entropy, files              | small                                          | libc                                                                                                                | It is the system's                                                                          |
| Line editing in the console         | the `rustyline` crate                          | `libedit`                                                                                                           | Yes                                                                                         |

### Not built

| What                                                                             | The system offers                                                                  | Fit                                                                        |
| -------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| `together` running in parallel                                                   | Grand Central Dispatch (`dispatch`, a C interface)                                 | Strong. A scheduler tuned for these cores is the thing least worth writing |
| Time zones and the calendar                                                      | libc's `localtime_r` and `strftime`, and the zone database the OS keeps up to date | Strong. It removes the question of zone data as a tile                     |
| `digest` (MD5, SHA)                                                              | CommonCrypto                                                                       | Strong                                                                     |
| `securerandom` and UUIDs                                                         | `arc4random_buf`, `uuid`                                                           | Strong                                                                     |
| Compression                                                                      | `libz`, `libcompression`                                                           | Strong                                                                     |
| Spawning a process ([#179](https://github.com/portlandlang/portland/issues/179)) | `posix_spawn`                                                                      | Strong                                                                     |
| Copying a file                                                                   | `copyfile`, which clones on APFS                                                   | Strong                                                                     |
| HTTP and TLS                                                                     | the Network framework's C interface; `libcurl` ships                               | Strong, and it avoids a port of `openssl`                                  |
| A `logger` sink                                                                  | `os_log`                                                                           | Optional                                                                   |
| Hash and Set tables for native                                                   | nothing suitable                                                                   | Portland's to write                                                        |
| Leak checking in the debug runtime                                               | `leaks` and malloc's debugging                                                     | Beside Portland's own check, not in place of it                            |

### What decides each case

- **Matching Ruby byte for byte.** This rules the system out for regex, `format`, and a seeded random sequence. Where the answer must be Ruby's, the system's answer is only close. Where Ruby's own answer comes from the OS (zones, digests, compression, spawning), the system is the right source.
- **The system's version is the user's.** A program using the system's Unicode data behaves by the macOS it runs on, so one binary can differ across OS versions. For the regex tables and normalization that is a real cost, and it touches [#456](https://github.com/portlandlang/portland/issues/456), where the seed carries two Unicode versions.

### How it fits what is decided

[ADR 0068](../adr/0068-2026-10-25-system-is-the-floor-module.md) says a `System` member is anything that touches the host. Each row marked strong is a floor member with a thin Portland wrapper over it, written once and shared by the seed, the hosted compiler, and native. Leaning on the system grows the floor and shrinks what is written several times over; it does not grow the core.

### Random, as the worked case

The seed's generator is Ruby's Mersenne Twister, written once in Rust; the hosted compiler hands `rand`, `srand`, and `Random.new` to it, and the native runtime has none.

- Drawing everything from the system with no seeding would delete the Twister, the `Random` value, the hidden generator, and every spec that pins a seeded sequence, and would spare native a port. Its price is that no program can repeat a sequence.
- A seeded generator as a tile meets two obstacles: `srand` changes what later bare `rand` calls answer, and a generator changes as it is drawn from, which a Portland value cannot do ([ADR 0015](../adr/0015-2026-07-23-values-never-mutate.md)).
- Choosing at build time deletes nothing but removes the hidden generator from every program that never says `srand`. The runtime is closed, so a scan for `srand` is a proof. This is the shape recorded on #548.
- How often gems name a seed, and where, is the `random-seeding` report in [ruby_research#20](https://github.com/portlandlang/ruby_research/pull/20), which had not been run when this was written.
