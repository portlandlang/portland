# Roadmap

**For:** anyone asking "how far along is this?"

One page, one line per item. Details live behind the links — [ADRs](docs/adr/) decide, [the language](docs/language.md) records what is built, [the architecture](docs/architecture.md) records how, and the [issues](https://github.com/portlandlang/portland/issues) track what is in motion.

## Where we are

**Stage 0 is done and Stage 1 has begun.** The Rust seed interprets a real slice of Portland. The trio — `lexer.pdx`, `parser.pdx`, `evaluator.pdx` — is Portland written in Portland: the parser parses the whole compiler including itself, and the evaluator matches the seed byte-for-byte. There is no compiler emitting binaries yet.

**Next:** the `%` zoo (#29), smart constructors (job 3 of [#61](https://github.com/portlandlang/portland/issues/61)), or the parallel runtime that ADR 0029's serial oracle now waits for (#12/#13 territory). The 2026-07-27 wave landed ADRs 0025–0029 decided _and_ built — write-site `return`, `inspect()`, typed results with `!`, traits, and serial `together` — the decisions by competing draft PRs, the roads not taken recorded in closed siblings. The transitional-tooling trio (#36/#57/#58) is deliberately parked while the language grows.

## Done 🎉

### The language

|                                                              | Decided                                                                                                                                                                                                                                                                                         | Built           |
| ------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------- |
| Optionals — no ambient nil, absence is a typed maybe         | [0005](docs/adr/0005-2026-07-22-optionals-wrapper-model.md) [0006](docs/adr/0006-2026-07-22-absence-word-nil.md) [0008](docs/adr/0008-2026-07-22-unwrap-toolkit.md) [0009](docs/adr/0009-2026-07-22-presence-word-some.md) [0010](docs/adr/0010-2026-07-22-partial-operations-return-maybes.md) | ✅ runtime half |
| `or`/`and`/`not` dead-identical to the sigils; `or` is typed | [0007](docs/adr/0007-2026-07-22-or-and-not-dead-identical.md)                                                                                                                                                                                                                                   | ✅              |
| A branch that doesn't happen is nil                          | [0012](docs/adr/0012-2026-07-22-branchless-if-is-nil.md)                                                                                                                                                                                                                                        | ✅              |
| Immutability — `mutable` gates names; values never mutate    | [0001](docs/adr/0001-2026-07-20-mutable-keyword.md) [0015](docs/adr/0015-2026-07-23-values-never-mutate.md)                                                                                                                                                                                     | ✅              |
| `case/in` pattern matching, exhaustive and static            | [0013](docs/adr/0013-2026-07-22-case-in-spec.md)                                                                                                                                                                                                                                                | ✅ runtime half |
| Keyword arguments, Ruby 3 style; splats deferred             | [0014](docs/adr/0014-2026-07-22-keyword-arguments.md)                                                                                                                                                                                                                                           | ✅              |
| Brace blocks, with the whose-block never-guess error         | [0016](docs/adr/0016-2026-07-23-brace-blocks-never-guess-owner.md)                                                                                                                                                                                                                              | ✅              |
| `it` as a normal binding under no-shadow                     | [0017](docs/adr/0017-2026-07-23-it-under-no-shadow.md)                                                                                                                                                                                                                                          | ✅              |
| Numbers — Ruby's floored division, floats                    | [0018](docs/adr/0018-2026-07-23-numbers-ruby-division.md)                                                                                                                                                                                                                                       | ✅              |
| Ranges — slices are collections, never-guess ends            | [0019](docs/adr/0019-2026-07-23-ranges.md)                                                                                                                                                                                                                                                      | ✅              |
| Heredocs — squiggly only                                     | [0020](docs/adr/0020-2026-07-23-heredocs-squiggly-only.md)                                                                                                                                                                                                                                      | ✅              |
| Namespaces — `module`, `::` names and `.` invokes            | [0021](docs/adr/0021-2026-07-24-namespaces-and-modules.md)                                                                                                                                                                                                                                      | ✅              |
| Concurrency vocabulary — `together`, `meanwhile`, `~`        | [0002](docs/adr/0002-2026-07-20-together-task-sigil.md) [0004](docs/adr/0004-2026-07-20-together-meanwhile.md) [0011](docs/adr/0011-2026-07-22-together-single-register.md)                                                                                                                     | ✅              |
| `together` semantics — tier two, serial, the future oracle   | [0029](docs/adr/0029-2026-07-27-together-semantics.md)                                                                                                                                                                                                                                          | ✅ serial       |
| Enums — closed vocabularies, symbol cases, keyword payloads  | [0022](docs/adr/0022-2026-07-25-enums-with-payloads.md)                                                                                                                                                                                                                                         | ✅              |
| Symbols, and `{name: "pdx"}` hash shorthand                  | [0023](docs/adr/0023-2026-07-25-symbols.md)                                                                                                                                                                                                                                                     | ✅              |
| Bitwise operators out; named methods instead                 | [0003](docs/adr/0003-2026-07-20-bitwise-operators-out.md) _tentative_                                                                                                                                                                                                                           | ✅              |
| Paren-less calls: command calls, no-shadow, never-guess      | pre-log                                                                                                                                                                                                                                                                                         | ✅              |
| Structs, methods in struct bodies, builtin type patterns     | [#27](https://github.com/portlandlang/portland/issues/27) first increment                                                                                                                                                                                                                       | ✅              |
| `return` unwinds to its write site, through yield            | [0025](docs/adr/0025-2026-07-27-return-unwinds-to-its-write-site.md)                                                                                                                                                                                                                            | ✅              |
| `inspect(value)` — p without the printing                    | [0026](docs/adr/0026-2026-07-27-inspect-as-a-function.md)                                                                                                                                                                                                                                       | ✅              |
| Errors as typed results; `!` is unwrap-or-propagate          | [0027](docs/adr/0027-2026-07-27-errors-as-typed-results.md)                                                                                                                                                                                                                                     | ✅              |
| The object model: structs and traits, `include`              | [0028](docs/adr/0028-2026-07-27-object-model-structs-and-traits.md)                                                                                                                                                                                                                             | ✅              |

### The implementation

- ✅ **Stage 0 seed** — Rust lexer, parser, tree-walking interpreter, `pdx` binary and REPL ([architecture](docs/architecture.md))
- ✅ **`parser.pdx` parses the whole compiler including itself** ([#18](https://github.com/portlandlang/portland/issues/18))
- ✅ **`evaluator.pdx` matches the seed byte-for-byte on the fixture suite** ([#19](https://github.com/portlandlang/portland/issues/19))
- ✅ Trio error fidelity — the seed's exact wording, or an honest recorded gap
- ✅ **The language spec, a third oracle** — `spec/` pins the ADRs' promises as ~190 executable examples, each run on both oracles by `script/spec`; writing it surfaced nine oracle divergences, all closed ([#39](https://github.com/portlandlang/portland/issues/39)–[#47](https://github.com/portlandlang/portland/issues/47))
- ✅ Memory-model direction — RC is exact under immutability, no tracing GC ([#12](https://github.com/portlandlang/portland/issues/12))

### The project

- ✅ Repo public, CI green on `macos-26`, namespaces squatted
- ✅ Decision log as [ADRs](docs/adr/); the [Ruby ledger](docs/ruby/) and its two migration promises
- ✅ **Docs restructured** — one home per fact, split Now / Decided / Happened; every index generated from the files it indexes ([#33](https://github.com/portlandlang/portland/issues/33))
- ✅ Evidence engine — [ruby_research](https://github.com/portlandlang/ruby_research) corpus reports
- ✅ Zed support v0 — [zed-portland](https://github.com/portlandlang/zed-portland) ([#24](https://github.com/portlandlang/portland/issues/24))
- ✅ Git hooks tracked in `script/hooks`; `script/docs/check` gating doc discipline, `script/docs/generate` building every index
- ✅ Markdown linting via `mdl`, rules in `script/docs/lib/markdownlint.rb` ([#31](https://github.com/portlandlang/portland/issues/31); `overcommit` declined on the merits)
- ✅ **RC-exact `<<`** — appends update in place when unshared; the self-parse test went 32.7s → 7.2s on CI ([#34](https://github.com/portlandlang/portland/issues/34))
- ✅ A 20s wall-clock tripwire on every test that shells out to `pdx` ([#32](https://github.com/portlandlang/portland/issues/32))
- ✅ `pdx --parse`, and a doc check that every code sample parses as Portland ([#35](https://github.com/portlandlang/portland/issues/35))

## Coming ⬜

### Language surface — decide, then build

- [#29](https://github.com/portlandlang/portland/issues/29) — the `%` literal zoo; `%w[]`'s content rules are decided and built ([ADR 0030](docs/adr/0030-2026-07-27-word-array-contents.md)), the rest waits on the corpus
- [#61](https://github.com/portlandlang/portland/issues/61) — the four jobs Ruby's `class` did and where each goes; job 3 is decided ([ADR 0031](docs/adr/0031-2026-07-31-smart-constructors-definable-new.md), building in [#67](https://github.com/portlandlang/portland/issues/67)), job 4 waits on #11
- Visibility — undecided on purpose; waits to be pulled for (noted closing [#27](https://github.com/portlandlang/portland/issues/27))
- [#14](https://github.com/portlandlang/portland/issues/14) — compile-time macros
- Regex — undecided, and a real hole
- String semantics — no ADR on Unicode, length, or normalization

### The real compiler

- [#9](https://github.com/portlandlang/portland/issues/9) — type inference; the static half of optionals lives here
- [#5](https://github.com/portlandlang/portland/issues/5) — the compile pipeline: MLIR dialects, codegen
- **Stage 2** — the fixpoint; **the seed retires** and `architecture.md`'s seed section is deleted
- **Stage 3** — the primitive boundary descends

### Apple silicon

- [#12](https://github.com/portlandlang/portland/issues/12) — memory model: RC, arenas, escape analysis; EMTE hardening
- [#13](https://github.com/portlandlang/portland/issues/13) — heterogeneous dispatch: one `.map`, many execution units

### Ecosystem and tooling

- [#23](https://github.com/portlandlang/portland/issues/23) — a living ruby/spec fork as compatibility oracle
- [#1](https://github.com/portlandlang/portland/issues/1) — brand story: voice, tagline, the teal carpet, the rose
- [#24](https://github.com/portlandlang/portland/issues/24) — Zed, the rest: `tree-sitter-portland`, registry publication
- [#25](https://github.com/portlandlang/portland/issues/25) — `script/bench` exists, baseline taken (hosted lex: 4741x, the fixpoint's scoreboard); receipts against Ruby still to come
- [#30](https://github.com/portlandlang/portland/issues/30) — `portland report`: a non-judgmental self-audit of `panic`, `mutable`, `parallel`
- [#36](https://github.com/portlandlang/portland/issues/36) — the polyfill shim gem and migration linter; a placeholder holding the tier inventory, not a commitment

## Dropped ❌ — on purpose, forever

- Portability — Apple silicon and macOS 26+ only; the lock-in is the feature
- Runtime metaprogramming — monkeypatching, `method_missing`, runtime `define_method`, `eval`
- Ambient nil, `NilClass`, truthiness
- Globals and the perlisms — `for`, `$specials`, `BEGIN`/`END`, flip-flops, the `and`/`or` precedence trick
- The GIL and the `Thread` API
- `fetch`, force-unwrap operators, `if let`-style binding conditionals
- In-place mutators and bang methods — values never mutate
- Numbered block parameters `_1`–`_9` — `it` and named parameters cover it
- Positional `together` register
