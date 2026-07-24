# Roadmap

One page: where Portland is going and how close it is. Above the line is done 🎉; below the line is coming (not necessarily in this order). Details live in [ADRs](docs/adr/), [STAGE0](docs/STAGE0.md), the [Ruby ledger](docs/ruby/), the [open-decisions report](docs/reports/2026-07-22-open-decisions.md), and the [issues](https://github.com/portlandlang/portland/issues).

## Done 🎉

- ✅ The premise, designed: Apple-silicon-only, Ruby's joy, not Swift ([AGENT.md](AGENT.md), [DESIGN](docs/DESIGN.md))
- ✅ Namespaces squatted, repo public, CI green on macos-26
- ✅ **Stage 0 seed**: Rust lexer + parser + tree-walking interpreter + `pdx` binary and REPL, running a real slice of Portland ([STAGE0](docs/STAGE0.md))
- ✅ Paren-less calls the Portland way: command calls, no-shadow, never-guess errors
- ✅ Structs, blocks, `case/when`, guards, interpolation, `%w[]`, `require_relative`, depth-guarded deep stacks
- ✅ **Portland-on-Portland**: `parser.pdx` parses the whole compiler including itself (#18); `evaluator.pdx` runs the fixture suite byte-identical to the seed (#19)
- ✅ Decision log as ADRs; the `docs/ruby/` migration ledger; the two migration promises (loud-never-silent, the polyfill test)
- ✅ Decided: `mutable` (0001), the `~` task sigil (0002), bitwise out (0003, tentative), `together`/`meanwhile` (0004, tentative)
- ✅ Decided, **optionals**: wrapper model (0005), `nil`/`nil?` (0006), `or`/`and`/`not` dead-identical + typed (0007), the unwrap toolkit (0008), `some`/`some?` (0009), partial ops return maybes + `fetch` retires (0010), one `together` register (0011)
- ✅ **Optionals built** — the runtime half, in the seed _and_ the trio, differentially pinned; the exhibits that demanded the feature now use it
- ✅ Memory-model direction: RC is exact under immutability (no cycles); MTE/MIE as defense-in-depth, surveyed on #12
- ✅ Evidence engine: the [ruby_research](https://github.com/portlandlang/ruby_research) corpus reports
- ✅ **`case/in` built** (#26, ADR 0013) — the full runtime pattern surface in seed + trio, and the evaluator dispatches on its own AST with struct patterns
- ✅ Keyword arguments, Ruby 3 style (ADR 0014) — built in seed + trio; splats deferred
- ✅ **Mutability, complete** (#10, ADRs 0001 + 0015): values never mutate, names do; `mutable` enforced everywhere (parameters, closures, loop scopes); `<<`/`[]=` as alias-proof rebinding sugar; guest hashes became host hashes
- ✅ `case/in` specced (#20, ADR 0013): compile-checked exhaustiveness, static `===`, fenced captures, keyword-only struct patterns
- ✅ A branch that doesn't happen is nil (#22, ADR 0012) — branchless `if`, finished `while`, broken-out calls; built in seed + trio
- ✅ Zed support, v0 (#24): [zed-portland](https://github.com/portlandlang/zed-portland) — `.pdx` detected as Portland, highlighted via tree-sitter-ruby
- ✅ **Object model, first increment** (#27): methods in struct bodies + builtin type patterns, seed + trio — and the taste payoff: the trio's AST prints itself (`node.sexp`), Token knows its kinds, guest `with` works
- ✅ **Decided and built, seed + trio, differentially pinned** — brace blocks + the whose-block never-guess error (0016), `it` under no-shadow (0017), Ruby's floored `/` and `%` plus floats (0018), ranges with collection slices and membership patterns (0019), squiggly-only heredocs (0020)

## Dropped ❌ (on purpose, forever)

- ❌ Portability — Apple silicon and macOS 26+ only; the lock-in is the feature
- ❌ Runtime metaprogramming — monkeypatching, `method_missing`, runtime `define_method`, `eval`
- ❌ Ambient nil, `NilClass`, truthiness
- ❌ Globals and the perlisms — `for`, `$specials`, `BEGIN`/`END`, flip-flops, the `and`/`or` secret precedence
- ❌ The GIL and the `Thread` API
- ❌ `fetch`, force-unwrap operators, `if let`-style binding conditionals
- ❌ Bitwise operators (tentative; named methods instead)
- ❌ Positional `together` register
- ❌ Numbered block parameters (`_1`–`_9`) — `it` and named parameters cover the space

## Coming ⬜

### Language surface (decide, then build)

- [ ] #29 — the `%` literal zoo: which of `%w %i %q %Q %r %s %x` survive, and one delimiter or many; corpus-informed. `%w[]`'s can't-contain-`]` bug rides along
- [ ] Symbols — **core question decided**, ADR pending ([session notes](docs/reports/2026-07-23-symbols-first-pass.md)): `:foo` exists as a general type, checked for membership where a closed vocabulary is declared; `{name: "pdx"}` is symbol-keyed and ships
- [ ] Enums / sum types — in design; open: payloads (one feature or two?), where enums are declared, how their type is named from outside
- [ ] Hash shorthand `{name: "pdx"}` — table stakes, waits on the enum/symbol shape
- [ ] #27 — the object model: the full session (methods in struct bodies + type predicates already built)
- [ ] #28 — error handling: exceptions vs typed results (decides the deferred `!` with it)
- [ ] #11 — `together` semantics, serial implementation first
- [ ] #14 — compile-time macros

### The real compiler

- [ ] #9 — type inference (the optionals static half lives here: narrowing, unhandled-maybe errors, exhaustiveness)
- [ ] #5 — the compile pipeline: MLIR dialects, codegen, the road to deleting the seed
- [ ] Stage 1 — the compiler rewritten in Portland, built by the seed
- [ ] Stage 2 — the fixpoint; **the seed retires**
- [ ] Stage 3 — the primitive boundary descends

### Apple silicon 🎉

- [ ] #12 — memory model: RC + arenas + escape analysis; EMTE hardening (plan proposed)
- [ ] #13 — heterogeneous dispatch: one `.map`, many execution units (P/E cores, Metal, SME)

### Ecosystem

- [ ] #23 — the living ruby/spec fork as compatibility oracle
- [ ] The polyfill gem + migration linter (wish, not yet a commitment)
- [ ] #1 — brand story: voice, tagline, the teal carpet, the rose

### Tooling

- [ ] #24 — Zed, the rest: `tree-sitter-portland` fork as the grammars drift; registry publication
- [ ] #25 — benchmarks: `script/bench` harness for seed/trio workloads; eventually compiled Portland vs Ruby with receipts
