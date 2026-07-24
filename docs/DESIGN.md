# Portland — design notes

Captured 2026-06-28 from a brainstorming session, and kept as that moment's record — the original thinking, preserved so it isn't lost. `AGENT.md` is the living lean summary; much of what's below has since been built (see `STAGE0.md`) or decided precisely (see `adr/`). Where this document and an ADR disagree, the ADR wins.

## The seed question

> What could a programming language be if it were _only_ for Apple devices running A-series or M-series silicon — but which isn't Swift?

The power of the premise is monoculture. Every mainstream language is conservative because it must run on hardware it has never seen. Drop portability and the constraints become features: you can hardcode assumptions a general language can't. Swift specifically _can't_ take these bets — it now targets Linux, Windows, embedded, and is ABI-frozen, so it stays generic.

## The soul: joy first (Ruby's lesson), safety/perf as contributors to joy

Like Ruby, the primary concern is programmer happiness. ~90%+ of Ruby's felt joy is the **ergonomics** — readability, expression-orientation, blocks-as-prose, the no-ceremony surface, "everything is an object" — _not_ the runtime dynamism. That part survives static compilation.

The user's framing: safety and performance are themselves _sources_ of joy — but only when you never feel them. The moment they show up as bookkeeping (Rust's borrow checker ceremony; Metal as a separate language you context-switch into), they've failed the joy test even if correct.

Design bar for every feature: **the beautiful line must also be the safe, fast line.** If going safe/fast forces a different, uglier line, reject it.

## "Ruby: the good parts"

Crockford's move was subtraction as design. Applied to Ruby, the finding is that the cut-list and the "blocks static safety + AOT speed" list are _nearly the same set_. So:

- **Keep the entire surface.** Blocks, expression-orientation, implicit returns, `?`/`!` suffixes, postfix guards, keyword args, Enumerable-as-one-protocol, pattern matching. All syntactic, all free under static compilation.
- **Cut the entire runtime.** Runtime monkeypatching, `method_missing`, runtime `define_method`, `eval`, globals, perlisms, footgun redundancies. Replace runtime metaprogramming with compile-time macros (the metaprogramming joy without the runtime mystery or cost). The dynamism we trade away (late-bound monkeypatching) is also Ruby's biggest pain at scale.
- **Fix Ruby's actively un-joyous parts** — and the fix _is_ the safety story, sold as deleting pain, never as new constraints:
  - **nil** → optionals + pattern matching. Kills `NoMethodError on nil`.
  - **mutable-by-default** → immutable by default. Also the precondition that makes auto-parallel `.map` safe (immutable data can't race).
  - **the GIL / threads** → redesign wholesale around P/E cores + structured concurrency.

Headline: **keep how it reads, replace what it does underneath.** The joy lives in the surface; the pain and the perf/safety blockers both live in the dynamic, mutable runtime.

The "types are ceremony, that's un-Ruby" objection is answered by **inference**: types present (safety) but invisible (joy), Crystal already shows Ruby-shaped syntax can compile statically. Annotations only at public boundaries. Duck typing → structural typing.

### nil, precisely

No _ambient_ nil — the bug is that in Ruby every reference is haunted by a possible nil. Absence is real, so it exists as one explicit case of an **optional** (`User?` may be absent; `User` never can). The word may survive (`nil`/`none`/`empty` — bikeshed later); the _spooky_ nil is gone.

### Immutability, precisely

Not "always immutable" — local mutation (loop counters, building a buffer, `i += 1`) is joyous and safe. The danger is **shared** mutation (two execution units writing the same memory = races). So: mutate freely inside your own scope; the moment a value is shared (handed to a parallel `.map`, sent across a concurrency boundary) it's frozen. Bare binding is immutable; mutability is the marked, rare case — ceremony goes on the rare thing, never the common thing. A pile of existing Ruby is already correct under this rule.

## Concurrency

The user hasn't written much concurrency code — partly because Ruby makes it clumsy and the "recommended way" changes every few years. Design spec straight from that: if it's dead simple and pretty, it stops being a big scary tool and just becomes how you write. So:

- **Tier 1 (implicit):** immutability lets the runtime parallelize freely _because_ it's provably safe. `.map` over a big collection just spreads across cores. You never asked.
- **Tier 2 (`together`):** declare independence in one word. Name results at the task site to kill positional miscounting; this also enables interleaving plain lines with task lines, which a positional left-side list can't express. Precedent: Swift's `async let`. Two registers (terse positional + named-at-site); word + symbol forms that are dead-identical.
- **Tier 3 (control):** cancellation, timeouts, racing — rare.

One model, built into the language. Immutability is what lets it be this simple — the machine does the figuring-out instead of handing you manual tools.

`together { }` shape note: a normal block returns only its last value, so `together` needs language support to treat each marked line as a task (the bit of "magic"). The per-line marker (`~`/`meanwhile`) makes it explicit and non-magical: a marked line is a task, an unmarked line is ordinary code.

## Implementation

### Backend: MLIR / LLVM

On Apple silicon the road to the metal _is_ LLVM — Metal's GPU IR (AIR) is LLVM under the hood, SME's matrix instructions are LLVM intrinsics, Clang/Swift are LLVM. MLIR (the layer on top) exists for exactly our thesis: one program, many execution targets. So the backend choice is almost dictated by the premise, and it points at the good tool.

### Frontend: Rust

Fast, safe, modern, good LLVM bindings, incremental-compilation speed (matters for the `irb`-fast feedback loop, itself a joy goal), and the language contributors already know. Alternatives noted: OCaml (classic compiler language) or Swift (cheeky — implement the not-Swift language in Swift). The user doesn't know Rust but knows some of the Rust core team. Crucially, the _only_ part that needs Rust is the thin primitive floor + backend glue — everything else is written in Portland itself.

### Parser: hand-write it, with Prism as a textbook

- Hand-written recursive descent is what every language that cares about error messages and speed does (Rust, Clang, Go, Prism itself). PEG / parser-combinator libraries give worse errors and can silently misparse, and Ruby's grammar is context-sensitive exactly where it matters (regex-vs-division, heredocs, block binding, paren-less calls) — needing lexer feedback that generic CFG/PEG tools handle badly.
- `ruby_prism` (the Rust crate) is FFI bindings over the C library, **parse-only (no lexer API), no grammar-extension hooks.** So it can't parse Portland and doesn't even hand you the lexer — its only use is a _reference oracle_ during dev and AST-shape inspiration.
- `parsanol` (PEG / parser-combinator lib, Rust + Ruby) — wrong architecture for Ruby's context-sensitivity, weaker errors, and too small (2 stars) to anchor a compiler on.
- Strategy: **port Prism's lexer from its C source** (the months-of-tedium part: heredocs, regex/division, interpolation, `%w[]`) and **grow the grammar fresh in Rust** against Portland's own AST. Don't fork-and-prune Prism — for a disposable bootstrap, _don't rip out_ anything; unused productions cost nothing. Add what's new, ignore the rest.

### Self-hosting (the Rubinius creed: "Write Ruby")

Self-host _as early as possible_, before the language is finished. The design problem isn't "when is the language done" but "where's the primitive boundary — the smallest set of operations that genuinely need Rust underneath." Draw it low and tight; everything above it is written in Portland.

Bootstrap timeline:

- **Stage 0** — Rust seed compiler for a tiny subset. Crude, disposable on purpose; it never has to be good.
- **Stage 1** — rewrite the compiler in that subset, built by the seed (first self-host).
- **Stage 2 (fixpoint)** — feed the compiler its own source to the previous build of itself; out comes a compiler that no longer needs the seed. Delete the seed. _This_ is "bootstrapping."
- **Stage 3** — gardening: push the boundary down, move stdlib + compiler internals into Portland, shrink the Rust floor toward its irreducible core.

Two kinds of "Rust": the **disposable seed** (deleted at Stage 2) and the **permanent-but-shrinking floor** (memory, LLVM/MLIR glue, GPU dispatch, syscalls). The daunting-looking Rust is mostly the seed, which is allowed to be crude. Bootstrap chore: keep the last-known-good compiler binary to build the next one; keep the seed buildable as an escape hatch.

Strategic reframe: Portland is **greenfield, not an alt-implementation**. Rubinius's failure to overtake MRI was _social, not technical_ — an alt-impl must convince a happy community to leave a beloved incumbent. Greenfield has no incumbent to displace, so it dodges that trap. Its different challenge is cold-start adoption, solved with joy + the killer niche.

## Architecture, in three pictures (described, since the diagrams don't travel)

1. **Existing pipelines we borrow from.** Every serious language has the same shape (source → parse → typed middle → lower to target). We take one proven piece from each: Prism's _lexer_, rustc's hand-written-RD _approach_ (and Rust as impl language), Swift/Apple's _LLVM/MLIR backend_.
2. **Our layer cake.** Top to bottom: your code → standard library → the compiler → **[primitive boundary]** → primitive floor (Rust + MLIR) → LLVM/MLIR backend → Apple silicon (P/E cores · GPU · SME). Everything above the dashed boundary is written in Portland itself; the gray floor below is the only non-self-hosted part, and it's the thinnest layer. The design pressure is: push the boundary down.
3. **Bootstrap timeline.** Stage 0 seed (all Rust) → Stage 1 first self-host → Stage 2 fixpoint (seed retired) → Stage 3 boundary descends. The teal (self-hosted) portion grows over time; the gray (Rust) shrinks — first by deleting the seed, then by whittling the floor.

## Branding (banked, not finished)

Name **Portland**, extension **`.pdx`**. The city's brand _is_ the language's brand: keep-it-weird, craft-over-scale, opinionated-because-right. Ready-made visual identity in the teal PDX-airport carpet (note: teal is the color the design diagrams kept reaching for). Faint Rose City → red → Ruby lineage echo gives an elegant rose mark instead of another faceted gem. `.pdx` is the user's hometown airport code — a quiet personal signature on every file. Positioning asset banked: "close to the metal, on Metal." Open threads: voice, tagline, full identity.

## Open questions / next

- Bikeshed: the mutability keyword (`let`/`var`/`mut`), the concurrency sigil, the "absence" word.
- The compile pipeline inside the Rust floor (lexer → parser → inference → MLIR → CPU/GPU/SME) — not yet drilled into.
- Squat the namespaces (crate `portland`, org `pdxlang`/`portlandlang`, domain), then first files.
