# Roadmap

**For:** anyone asking "how far along is this?"

One page, one line per item. Details live behind the links — [ADRs](docs/adr/) decide, [the language](docs/language.md) records what is built, [the architecture](docs/architecture.md) records how, and the [issues](https://github.com/portlandlang/portland/issues) track what is in motion.

## Where we are

**Stage 0 is done and Stage 1 has begun.** The Rust seed interprets a real slice of Portland. The compiler — `lexer.pdx`, `parser.pdx`, `evaluator.pdx`, plus the two walkers the seed will never have, `checker.pdx` and `inference.pdx` — is Portland written in Portland: the parser parses the whole compiler including itself, the evaluator matches the seed byte-for-byte, the checker refuses only what it can disprove, and inference is what its refusals read — across a whole program, with run mode sharpening parameters from their callers. There is no compiler emitting binaries yet.

**Next:** with [#9](https://github.com/portlandlang/portland/issues/9)'s ladder climbed end to end — ADRs 0047–0049 in three days: the error voice, contracts, the whole-program door with run-mode sharpening — the checker's remaining reach is a matter of positions on the other node kinds and reporting every refusal at once, both pulled when a real file wants them. The language questions come back to the front: the `%` zoo (#29), redefinition and the hijackable builtins (#70), ordering and `<=>` (#76), and the stdlib story (#78), which is also when the builtin method table retires. The transitional-tooling trio (#36/#57/#58) is deliberately parked while the language grows.

## Done 🎉

### The language

|                                                              | Decided                                                                                                                                                                                                                                                                                         | Built           |
| ------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------- |
| Optionals — no ambient nil, absence is a typed maybe         | [0005](docs/adr/0005-2026-07-22-optionals-wrapper-model.md) [0006](docs/adr/0006-2026-07-22-absence-word-nil.md) [0008](docs/adr/0008-2026-07-22-unwrap-toolkit.md) [0009](docs/adr/0009-2026-07-22-presence-word-some.md) [0010](docs/adr/0010-2026-07-22-partial-operations-return-maybes.md) | ✅              |
| `or`/`and`/`not` dead-identical to the sigils; `or` is typed | [0007](docs/adr/0007-2026-07-22-or-and-not-dead-identical.md)                                                                                                                                                                                                                                   | ✅              |
| A branch that doesn't happen is nil                          | [0012](docs/adr/0012-2026-07-22-branchless-if-is-nil.md)                                                                                                                                                                                                                                        | ✅              |
| Immutability — `mutable` gates names; values never mutate    | [0001](docs/adr/0001-2026-07-20-mutable-keyword.md) [0015](docs/adr/0015-2026-07-23-values-never-mutate.md)                                                                                                                                                                                     | ✅              |
| `case/in` pattern matching, exhaustive and static            | [0013](docs/adr/0013-2026-07-22-case-in-spec.md) [0035](docs/adr/0035-2026-08-12-exhaustiveness-over-what-the-arms-reveal.md)                                                                                                                                                                   | ✅              |
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
| Errors as typed results; propagation is the explicit toolkit | [0027](docs/adr/0027-2026-07-27-errors-as-typed-results.md) [0044](docs/adr/0044-2026-08-19-propagation-is-the-toolkit.md)                                                                                                                                                                       | ✅              |
| The object model: structs and traits, `include`              | [0028](docs/adr/0028-2026-07-27-object-model-structs-and-traits.md)                                                                                                                                                                                                                             | ✅              |
| Smart constructors: `def self.`, definable `new`, `fields`   | [0031](docs/adr/0031-2026-07-31-smart-constructors-definable-new.md)                                                                                                                                                                                                                            | ✅              |
| A bare `{` after a call name; `%w[]`'s contents              | [0024](docs/adr/0024-2026-07-26-brace-after-a-call-name.md) [0030](docs/adr/0030-2026-07-27-word-array-contents.md)                                                                                                                                                                             | ✅              |
| Dotted calls take paren-less arguments; `**` and `pow`       | [0032](docs/adr/0032-2026-08-11-dotted-calls-take-paren-less-arguments.md) [0033](docs/adr/0033-2026-08-11-exponent-is-starstar-and-pow.md)                                                                                                                                                     | ✅              |
| The famous twins ship; `alias` as a second name              | [0036](docs/adr/0036-2026-08-18-the-famous-twins-ship.md) [0039](docs/adr/0039-2026-08-19-alias-a-second-name-for-a-method.md)                                                                                                                                                                   | ✅              |
| The loop spellings: `until`, postfix loops, `loop do`; `\|\|=` | [0037](docs/adr/0037-2026-08-18-the-loop-spellings.md) [0043](docs/adr/0043-2026-08-19-or-equals.md)                                                                                                                                                                                             | ✅              |
| A character is a grapheme; equality is canonical             | [0038](docs/adr/0038-2026-08-19-what-a-character-is.md)                                                                                                                                                                                                                                         | ✅              |
| `then` rows everywhere; the def shapes; the ternary          | [0042](docs/adr/0042-2026-08-19-then-rows-everywhere.md) [0045](docs/adr/0045-2026-08-20-the-def-shapes.md) [0046](docs/adr/0046-2026-08-20-the-ternary.md)                                                                                                                                       | ✅              |
| Return annotations as arrow comments, checked                | [0041](docs/adr/0041-2026-08-19-return-annotations-are-arrow-comments.md)                                                                                                                                                                                                                       | ✅              |
| Inference — hybrid contracts, traits as shapes, `Maybe[T]`   | [0040](docs/adr/0040-2026-08-19-inference-the-design-core.md) [0048](docs/adr/0048-2026-09-23-contracts-the-rulings.md)                                                                                                                                                                         | ✅ 3a–3d        |
| The error voice — one renderer, the wordings, the line beneath | [0047](docs/adr/0047-2026-09-22-the-error-voice.md)                                                                                                                                                                                                                                             | ✅              |
| The checker; a program is its require closure                | [0034](docs/adr/0034-2026-08-11-the-checker-and-the-oracle-succession.md) [0049](docs/adr/0049-2026-09-24-the-whole-program-door.md)                                                                                                                                                             | ✅              |

### The implementation

- ✅ **Stage 0 seed** — Rust lexer, parser, tree-walking interpreter, `pdx` binary and REPL ([architecture](docs/architecture.md))
- ✅ **`parser.pdx` parses the whole compiler including itself** ([#18](https://github.com/portlandlang/portland/issues/18))
- ✅ **`evaluator.pdx` matches the seed byte-for-byte on the fixture suite** ([#19](https://github.com/portlandlang/portland/issues/19))
- ✅ Trio error fidelity — the seed's exact wording, or an honest recorded gap
- ✅ **The language spec, a third oracle** — `spec/` pins the ADRs' promises as ~500 executable examples (870 runs), each run on both oracles by `script/spec` and, hosted, through the checker; writing it surfaced nine oracle divergences, all closed ([#39](https://github.com/portlandlang/portland/issues/39)–[#47](https://github.com/portlandlang/portland/issues/47))
- ✅ **The checker** — the first walker the seed will never have ([ADR 0034](docs/adr/0034-2026-08-11-the-checker-and-the-oracle-succession.md)): enum vocabularies, coverage, and the inference-backed family — conditions, operators, missing methods, unhandled maybes, lying annotations, arity, contracts — every wording rendered in one place and pinned, every refusal pointing at its line
- ✅ **Inference, 3a through 3d** — synthesis, checking mode, narrowing, contracts; the builtin method table derived from the seed's behavior and read by the seed at build for its own wordings
- ✅ **The whole-program door** — a program is its require closure, each file checked once with its requires' declarations in view; run mode sharpens the entry's parameters from their callers ([ADR 0049](docs/adr/0049-2026-09-24-the-whole-program-door.md))
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
- [#61](https://github.com/portlandlang/portland/issues/61) — the four jobs Ruby's `class` did and where each goes; jobs 1–3 are decided and built ([ADR 0031](docs/adr/0031-2026-07-31-smart-constructors-definable-new.md) closed the ledger); job 4, stateful objects, was parked until `together` had semantics — it has ([ADR 0029](docs/adr/0029-2026-07-27-together-semantics.md)) — and now waits to be pulled for
- Visibility — undecided on purpose; waits to be pulled for (noted closing [#27](https://github.com/portlandlang/portland/issues/27))
- [#14](https://github.com/portlandlang/portland/issues/14) — compile-time macros
- [#74](https://github.com/portlandlang/portland/issues/74) — regex: literals, engine, and what a match answers
- ✅ [#75](https://github.com/portlandlang/portland/issues/75) — string semantics: a character is a grapheme, equality is canonical, casing is full Unicode ([ADR 0038](docs/adr/0038-2026-08-19-what-a-character-is.md)); canonical *search* is the recorded follow-on
- [#76](https://github.com/portlandlang/portland/issues/76) — ordering: `<=>`, Comparable-as-a-trait, user types sorting
- [#77](https://github.com/portlandlang/portland/issues/77) — can a block be held in a value? the proc question
- [#78](https://github.com/portlandlang/portland/issues/78) — the stdlib story, and where the OS surface lives
- ✅ [#79](https://github.com/portlandlang/portland/issues/79) — the alias-cluster policy: the famous twins ship ([ADR 0036](docs/adr/0036-2026-08-18-the-famous-twins-ship.md)); an unshipped alias refuses by naming the survivor
- ✅ [#73](https://github.com/portlandlang/portland/issues/73) — five probed spellings, all resolved: `until`, the postfix loops, `loop do` ([ADR 0037](docs/adr/0037-2026-08-18-the-loop-spellings.md)), and `||=` ([ADR 0043](docs/adr/0043-2026-08-19-or-equals.md)) shipped; subjectless `case` declined with [#84](https://github.com/portlandlang/portland/issues/84) standing

### Toward the fixpoint

- [#9](https://github.com/portlandlang/portland/issues/9) — type inference: ADR 0040's ladder is climbed and the static half of optionals is built; what remains — positions on every node kind, reporting every refusal at once — waits to be pulled for
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
- In-place mutators — values never mutate; a `!` in a method's name is just a name ([ADR 0044](docs/adr/0044-2026-08-19-propagation-is-the-toolkit.md))
- Numbered block parameters `_1`–`_9` — `it` and named parameters cover it
