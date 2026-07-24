# Roadmap

_For: anyone asking "how far along is this?"_

One page, one line per item. Details live behind the links — [ADRs](docs/adr/) decide, [the language](docs/language.md) records what is built, [the architecture](docs/architecture.md) records how, and the [issues](https://github.com/portlandlang/portland/issues) track what is in motion.

## Where we are

**Stage 0 is done and Stage 1 has begun.** The Rust seed interprets a real slice of Portland. The trio — `lexer.pdx`, `parser.pdx`, `evaluator.pdx` — is Portland written in Portland: the parser parses the whole compiler including itself, and the evaluator matches the seed byte-for-byte. There is no compiler emitting binaries yet.

**Next:** enums and sum types, now that namespaces exist to hold them. That unblocks symbols, which unblocks hash shorthand.

## Done 🎉

### The language

| | Decided | Built |
|---|---|---|
| Optionals — no ambient nil, absence is a typed maybe | [0005](docs/adr/0005-2026-07-22-optionals-wrapper-model.md) [0006](docs/adr/0006-2026-07-22-absence-word-nil.md) [0008](docs/adr/0008-2026-07-22-unwrap-toolkit.md) [0009](docs/adr/0009-2026-07-22-presence-word-some.md) [0010](docs/adr/0010-2026-07-22-partial-operations-return-maybes.md) | ✅ runtime half |
| `or`/`and`/`not` dead-identical to the sigils; `or` is typed | [0007](docs/adr/0007-2026-07-22-or-and-not-dead-identical.md) | ✅ |
| A branch that doesn't happen is nil | [0012](docs/adr/0012-2026-07-22-branchless-if-is-nil.md) | ✅ |
| Immutability — `mutable` gates names; values never mutate | [0001](docs/adr/0001-2026-07-20-mutable-keyword.md) [0015](docs/adr/0015-2026-07-23-values-never-mutate.md) | ✅ |
| `case/in` pattern matching, exhaustive and static | [0013](docs/adr/0013-2026-07-22-case-in-spec.md) | ✅ runtime half |
| Keyword arguments, Ruby 3 style; splats deferred | [0014](docs/adr/0014-2026-07-22-keyword-arguments.md) | ✅ |
| Brace blocks, with the whose-block never-guess error | [0016](docs/adr/0016-2026-07-23-brace-blocks-never-guess-owner.md) | ✅ |
| `it` as a normal binding under no-shadow | [0017](docs/adr/0017-2026-07-23-it-under-no-shadow.md) | ✅ |
| Numbers — Ruby's floored division, floats | [0018](docs/adr/0018-2026-07-23-numbers-ruby-division.md) | ✅ |
| Ranges — slices are collections, never-guess ends | [0019](docs/adr/0019-2026-07-23-ranges.md) | ✅ |
| Heredocs — squiggly only | [0020](docs/adr/0020-2026-07-23-heredocs-squiggly-only.md) | ✅ |
| Namespaces — `module`, `::` names and `.` invokes | [0021](docs/adr/0021-2026-07-24-namespaces-and-modules.md) | ✅ |
| Concurrency vocabulary — `together`, `meanwhile`, `~` | [0002](docs/adr/0002-2026-07-20-together-task-sigil.md) [0004](docs/adr/0004-2026-07-20-together-meanwhile.md) [0011](docs/adr/0011-2026-07-22-together-single-register.md) _tentative_ | — |
| Bitwise operators out; named methods instead | [0003](docs/adr/0003-2026-07-20-bitwise-operators-out.md) _tentative_ | ✅ |
| Paren-less calls: command calls, no-shadow, never-guess | pre-log | ✅ |
| Structs, methods in struct bodies, builtin type patterns | [#27](https://github.com/portlandlang/portland/issues/27) first increment | ✅ |

### The implementation

- ✅ **Stage 0 seed** — Rust lexer, parser, tree-walking interpreter, `pdx` binary and REPL ([architecture](docs/architecture.md))
- ✅ **`parser.pdx` parses the whole compiler including itself** ([#18](https://github.com/portlandlang/portland/issues/18))
- ✅ **`evaluator.pdx` matches the seed byte-for-byte on the fixture suite** ([#19](https://github.com/portlandlang/portland/issues/19))
- ✅ Trio error fidelity — the seed's exact wording, or an honest recorded gap
- ✅ Memory-model direction — RC is exact under immutability, no tracing GC ([#12](https://github.com/portlandlang/portland/issues/12))

### The project

- ✅ Repo public, CI green on `macos-26`, namespaces squatted
- ✅ Decision log as [ADRs](docs/adr/); the [Ruby ledger](docs/ruby/) and its two migration promises
- ✅ Evidence engine — [ruby_research](https://github.com/portlandlang/ruby_research) corpus reports
- ✅ Zed support v0 — [zed-portland](https://github.com/portlandlang/zed-portland) ([#24](https://github.com/portlandlang/portland/issues/24))
- ✅ Git hooks tracked in `script/hooks`; `script/docs/check` gating doc discipline, `script/docs/generate` building every index

## Coming ⬜

### Language surface — decide, then build

- **Enums and sum types** — next up; open: payloads, and whether `enum` declares the field
- Symbols — core question decided, ADR waits on the enum shape ([notes](docs/history/2026-07-23-symbols-first-pass.md))
- Hash shorthand `{name: "pdx"}` — table stakes; waits on symbols
- [#27](https://github.com/portlandlang/portland/issues/27) — the object model: mixins, inheritance, visibility
- [#28](https://github.com/portlandlang/portland/issues/28) — error handling: exceptions vs typed results; decides the deferred `!`
- [#29](https://github.com/portlandlang/portland/issues/29) — the `%` literal zoo; carries the `%w[]`-can't-contain-`]` bug
- [#11](https://github.com/portlandlang/portland/issues/11) — `together` semantics, serial implementation first
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
- [#25](https://github.com/portlandlang/portland/issues/25) — `script/bench`, and eventually receipts against Ruby
- [#30](https://github.com/portlandlang/portland/issues/30) — `portland report`: a non-judgmental self-audit of `panic`, `mutable`, `parallel`
- [#31](https://github.com/portlandlang/portland/issues/31) — markdown linting, and whether `overcommit` earns its keep
- [#32](https://github.com/portlandlang/portland/issues/32) — a performance tripwire so a slow test can't hide
- [#33](https://github.com/portlandlang/portland/issues/33) — the guidance-doc audit: one source of truth per fact
- [#34](https://github.com/portlandlang/portland/issues/34) — RC-exact `<<`, killing ~200M array copies in the self-parse test
- [#35](https://github.com/portlandlang/portland/issues/35) — `pdx --parse`, so the doc checks can verify every code sample in the docs
- The polyfill gem and migration linter — a wish, not yet a commitment

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
