# Architecture

**For:** anyone who wants to know how Portland is built, what runs what, and which parts are meant to be thrown away.

What Portland _is_ — syntax, rules, style — lives in [language.md](language.md). This file is the machinery underneath it.

Portland is being bootstrapped. That single fact explains most of the layout: there are two implementations of the language in this repo, one in Rust and one in Portland, and they are not rivals. One is the reference; the other is the future. Keeping them byte-identical is the central discipline of the project.

## The shape today

```text
compiler/*.pdx     the trio — Portland, written in Portland
       ↑ runs on
seed/  (Rust)      the seed — lexer, parser, tree-walking interpreter, pdx binary
       ↑ runs on
macOS 26+ / Apple silicon
```

There is no compiler yet in the sense of "emits a binary." The seed interprets. The trio is interpreted _by_ the seed. Everything below the seed is, for now, just the operating system.

![Three states of the architecture: now, with a fat Rust seed and the trio interpreted on top of it; next, with the seed retired and a Portland compiler lowering through LLVM to a native binary over a thin Rust floor; and the ideal future, adding inference and heterogeneous dispatch across P/E cores, SME, and the Metal GPU, with FFI and Apple-framework bridges, the Rust floor still thin and shrinking.](architecture.svg)

## The two kinds of Rust

This distinction is the one to keep straight, because "there's a lot of Rust here" is misleading:

- **The disposable seed** (`seed/`, ~7,900 lines). Crude on purpose. It never has to be good — it has to exist and be correct. It is **deleted at Stage 2** and nothing in it is a commitment.
- **The permanent-but-shrinking floor** (does not exist yet). Memory, LLVM/MLIR glue, GPU dispatch, syscalls. The irreducible bottom that cannot be written in Portland. The standing design pressure is to push this boundary _down_.

Almost all the Rust you can see today is the first kind. That is why its crudeness is a non-issue and why adding `rustyline` to it was an easy call — you do not optimize something you plan to delete.

## The seed

`seed/` is the `portland-seed` crate, never published. One dependency, `rustyline`, for REPL line editing.

| File             | Job                                                                                                                                     |
| ---------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `lexer.rs`       | source → tokens                                                                                                                         |
| `heredoc.rs`     | expands `<<~` into string literals _before_ the lexer, because `Token.text` borrows the source and a dedented body is not a slice of it |
| `parser.rs`      | hand-written recursive descent → AST                                                                                                    |
| `ast.rs`         | the node types                                                                                                                          |
| `interpreter.rs` | tree-walking evaluation; the reference semantics live here                                                                              |
| `value.rs`       | the runtime value representation                                                                                                        |
| `main.rs`        | the `pdx` binary — runs a file, or opens the REPL                                                                                       |

The parser is hand-written recursive descent, which is what every language that cares about error messages does. Parser-generator and PEG libraries give worse errors and can silently misparse, and Ruby-shaped grammar is context-sensitive exactly where it matters. Prism's C lexer is the textbook for the hard lexical parts; `ruby_prism` itself is no help — it is parse-only, with no lexer API and no grammar hooks, so it can serve as a reference oracle during development and nothing more.

The interpreter runs on a **512 MB-stack thread**. On the default 8 MB main stack the seed died at roughly 1,200 nested parens, 1,500-term arithmetic chains, and 900 Portland call frames — and died as a silent macOS _hang_, not a crash, because even a trivial `fn f() { f() }` in Rust hangs on overflow under macOS 26. The fat stack moves the real limits about 64× out. Above that, explicit depth guards fail as clean Portland errors long before the Rust stack is at risk: expression nesting over 10,000 at parse time, expression evaluation over 100,000, call stack over 10,000 frames.

**Panics are the seed's whole error story.** Parse errors, type errors, arity mismatches, a missing `end` — all panics. Real diagnostics are a headline joy feature of the actual compiler and deliberately not the seed's job.

## The trio

`compiler/` is Portland written in Portland — the beginning of Stage 1.

| File            | Lines | Job                                 |
| --------------- | ----- | ----------------------------------- |
| `lexer.pdx`     | 446   | source → tokens                     |
| `parser.pdx`    | 1,961 | tokens → AST, with `sexp` rendering |
| `evaluator.pdx` | 963   | walks the AST                       |

Plus three small drivers that make each stage runnable on its own: `tokenize.pdx` dumps a token stream, `parse.pdx` prints one S-expression per statement, `run.pdx` evaluates a file. Each is under ten lines, because each is one composed expression — `evaluate_program(parse_program(lex(read_file(argv.first))))`.

Two milestones are reached: **`parser.pdx` parses the whole compiler including itself**, and **`evaluator.pdx` runs the fixture suite byte-identically to the seed**. The evaluator also dispatches on its own AST using `case/in` struct patterns, and the AST nodes print themselves via a `sexp` method — Portland matching on Portland, which is the point.

## The contract between them: direct vs hosted

The differential harness is the reason two implementations do not drift. Every fixture runs twice:

- **direct** — the seed runs `fixture.pdx`
- **hosted** — the seed runs `compiler/run.pdx`, which runs `fixture.pdx`

The two stdouts are compared byte-for-byte. **Expected output is never hand-written**; the seed is the oracle, so the test asserts agreement rather than a transcript somebody typed. This covers **error wording as well as results** — where the trio can diagnose at all, it must say exactly what the seed says, pinned by `portland_evaluator_reports_the_seed_wording_on_errors`.

The failure mode this harness has already shown: it stayed green through an entire batch of new syntax the trio did not understand, because no fixture used it. Green is not covered — see [principles](principles.md#7-the-seed-is-the-oracle).

## Where the trio falls short, on purpose

The trio's parser is **functional** — it holds no mutable per-block state — where the seed's is not. So there are checks the seed can make that the trio structurally cannot, and where it cannot tell, it **declines to check** rather than checking wrongly.

Two rules are currently accepted silently by the trio and refused by the seed: a nested `it`, and an `it` colliding with a local of that name. Both need per-block frames a flat token scan does not have — a scan cannot tell an `it` of _this_ block from one belonging to a block inside it. Everything else reports the seed's wording verbatim, including the never-guess brace menu at both widths.

The related check that _is_ possible — `it` mixed with declared `|parameters|` — works only because it declines to guess whenever the block body opens a block of its own. That is the whole principle in miniature: a gap leaves the trio incomplete, a false positive would make it unusable, and the seed catches it either way.

## One known cost

`<<` is rebinding sugar (ADR 0015), and the seed implements it by cloning: appending to an array copies the whole array. Parsing 1,961 lines of Portland with the Portland parser therefore costs on the order of 200 million element copies, and that single test is 31.5 seconds of a 33-second suite. It is why the git hooks split — `pre-commit` runs the fast gate, `pre-push` runs everything.

This is not a semantics problem, it is a naive implementation of correct semantics. At refcount 1 the old array is dead and can be mutated in place, which is exactly what reference-count exactness buys (#34, on the road to #12).

## The bootstrap ladder

- **Stage 0** — Rust seed for a tiny subset. Crude, disposable, done.
- **Stage 1** — the compiler rewritten in that subset, built by the seed. Begun: the trio is its beginning.
- **Stage 2** — the fixpoint. Feed the compiler its own source to the previous build of itself, and out comes a compiler that no longer needs the seed. **The seed is deleted.** This is what "bootstrapped" means.
- **Stage 3** — gardening. Push the primitive boundary down; move more of the standard library and compiler internals into Portland; shrink the Rust floor toward its irreducible core.

The design question is never "when is the language done." It is **where is the primitive boundary** — the smallest set of operations that genuinely need Rust underneath. Draw it low and tight.

Self-hosting early is deliberate, and it comes from Rubinius. So does the strategic framing: Portland is **greenfield, not an alt-implementation**. Rubinius's failure to overtake MRI was social, not technical — an alt-implementation has to convince a happy community to leave a beloved incumbent. Greenfield has no incumbent to displace, so it dodges that trap entirely. Its different problem is cold-start adoption, and the answers to that are joy, the killer niche, and the migration story in [docs/ruby/](ruby/).

## The backend — designed, not built

Everything in this section is direction, not code. Issue #5 owns it.

**MLIR on LLVM.** Not cargo cult: on Apple silicon the road to the metal _is_ LLVM. Metal's GPU IR is LLVM underneath, SME's matrix instructions are LLVM intrinsics, and Clang and Swift are both LLVM. MLIR is the layer that exists for precisely our thesis — one program, many execution targets. The premise nearly dictates the choice.

**Memory (#12, plan proposed).** Portland is memory-safe by semantics on every chip, not by hardware. The key observation: **immutable values cannot form reference cycles**, so plain reference counting is _exact_ — no tracing garbage collector, no weak/unowned ceremony, no borrow-checker bookkeeping in the surface language. MIE and EMTE (A19 and M5 or newer, not M1–M4) are defense-in-depth for the Rust floor, never the foundation.

**Types (#9).** Inference is the real compiler's job; the seed is dynamically checked at runtime. The lean is bidirectional inference with local generalization rather than Hindley-Milner purity — better errors, and it plays well with structural typing and future macros. The static half of optionals lives here: flow narrowing, unhandled-maybe errors, exhaustiveness.

## The Apple silicon layer — bets, not library calls

These are language semantics, not an SDK we call into. Issue #13 owns the dispatch story.

- **Unified memory.** No host/device distinction. The same `.map` line is a GPU dispatch on a big array and one core on a small one, and the line does not change.
- **Heterogeneous units.** P and E cores, the Metal GPU, the SME matrix unit. The runtime _places_ work; you declare independence and it decides. Immutability is what makes that safe enough to do without asking.
- **Hardware safety.** MIE/EMTE as hardening for the floor; PAC for the runtime. Defense in depth beneath a language that is already safe.
- **The honest limit.** The Neural Engine is not openly programmable. The only door is CoreML, and CoreML picks its own units — Accelerate is the CPU path, MPS runs on the GPU, and neither reaches the ANE. Portland does not compile to the NPU, and the placement decision there belongs to Apple, not to our runtime. We do not pretend otherwise.

## Repo layout

| Path                   | What                                                                |
| ---------------------- | ------------------------------------------------------------------- |
| `seed/`                | the disposable Rust seed and its tests                              |
| `compiler/`            | the trio, in Portland                                               |
| `crate/`               | the crates.io `portland` placeholder — a name squat, nothing more   |
| `script/`              | Scripts to Rule Them All: `bootstrap`, `test`, `console`, `cibuild` |
| `script/docs/`         | `check` and `generate`, plus one file per check and per generator   |
| `script/hooks/`        | tracked git hooks, installed by `bootstrap` via `core.hooksPath`    |
| `seed/tests/fixtures/` | `.pdx` programs that are both tests and living documentation        |
