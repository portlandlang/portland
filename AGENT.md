# Portland

A joyous programming language for Apple silicon.

**Status:** pre-code. This file and `docs/DESIGN.md` are a design brief captured from a brainstorming session on 2026-06-28 (originally held in another project's working dir, hence this handoff). There is no compiler yet. Don't assume code exists — we're at the "squat the namespaces and write the first files" stage.

## North star

Programmer happiness first, like Ruby. Job one is the joy of reading and writing the code. Safety and performance are job 1.1 — not tradeoffs _against_ joy, but _contributors_ to it. The rule every feature must pass: does this make the beautiful line also the safe, fast line — or does it force a different, uglier line to get safe and fast? Reject the latter.

The premise: a language that runs **only** on Apple silicon (A-series / M-series), and is **not** Swift. Locking to one vendor's hardware is the feature — it lets us make assumptions general, portable languages are forbidden to make.

## Locked design decisions

- **Ruby's good parts, kept (the surface):** blocks-as-prose (`.map`/`.each`/`yield`), everything-is-an-expression, implicit returns, `?`/`!` method suffixes, postfix guards, keyword args, Enumerable as one unifying protocol, modern pattern matching (promoted to load-bearing). ~90% of Ruby's joy is this ergonomic surface and it survives static compilation.
- **Ruby's bad parts, cut (the runtime):** runtime monkeypatching / open classes, `method_missing`, runtime `define_method`, `eval`, globals, perlisms, redundant footguns (`for`, `and`/`or` precedence). Replace runtime metaprogramming with **compile-time macros**. The cut-list and the "blocks static safety/speed" list are nearly the same set.
- **No ambient nil.** Absence is one explicit case of an optional (`User?`), never a free-floating bottom every value can secretly be. You only meet it when the type says so. Kills `NoMethodError on nil` — Ruby's #1 production crash.
- **Immutable by default; mutable when local.** Bare binding is immutable (`foo = "abc"`); mutability needs an explicit marker (`let`/`var`/`mut` — TBD). The real line is **immutable when shared, mutable when local**: mutate freely in your own scope, but a value frozen the moment it crosses a boundary where it could race. The compiler enforces it.
- **Types inferred, not written.** Hindley-Milner-style; types present (the safety) but invisible (the joy). Annotations only at public boundaries, as docs. Duck typing becomes **structural** typing — "if it quacks," checked at compile time.

## Concurrency (one model, baked in — never a library that gets deprecated)

Three tiers; you live almost entirely in tier 1.

1. **Implicit — you do nothing.** `photos.map { it.thumbnail }` spreads across cores when worth it, safe _because_ values are immutable. You never typed a concurrency word.
2. **`together` — say "these are independent."** Name each result at its task site so there's no fragile positional coupling:

   ```ruby
   together do
     user  = ~ fetch_user
     posts = ~ fetch_posts
   end
   # user, posts in scope here
   ```

   The `~` marks "runs concurrently"; the block is the join boundary. Two registers: terse positional (`a, b = together do … end`) and named-at-site (above). Both a word form (`spawn`) and a symbol form (the sigil), like `lambda`/`->` — but they must be **dead-identical** (don't repeat the `lambda`/`proc` `return` footgun) and the symbol must be one easy keystroke.

3. **Explicit control — rare.** Cancellation, timeouts, racing strategies.

## Implementation strategy

- **Frontend in Rust. Backend on MLIR / LLVM.** MLIR isn't cargo-cult: the heterogeneous-compute thesis (one program → CPU, GPU, matrix unit) is exactly what MLIR exists for, and on Apple silicon the road to the metal _is_ LLVM (Metal's AIR is LLVM; SME has LLVM intrinsics).
- **Parser: hand-written recursive descent.** Not the `ruby_prism` crate (parse-only FFI over C, no lexer API, can't extend the grammar). Not PEG / parser-combinator libs (wrong fit for Ruby's context-sensitive lexing; weaker errors). Instead: **port Prism's lexer from its C source** as the textbook for Ruby's lexical hell (heredocs, regex-vs-division, interpolation), and **grow our own grammar fresh in Rust** against our own AST. "Port the hard lexical bits, grow your own grammar." Don't fork-and-prune Prism — subtraction is cost with no payoff.
- **Self-host as early as possible** (Rubinius creed). Bootstrap:
  - **Stage 0** — Rust _seed_ compiler for a tiny subset. Crude and disposable on purpose.
  - **Stage 1** — rewrite the compiler in that subset; build it with the seed.
  - **Stage 2 (fixpoint)** — the compiler compiles itself; **retire the seed**.
  - **Stage 3** — push the primitive boundary _down_: stdlib + compiler internals move into the language; the Rust floor shrinks to its irreducible core (memory, LLVM/MLIR glue, GPU dispatch, syscalls).
  - Keep the last-known-good compiler binary (bootstrap chore) and the Rust seed buildable as an escape hatch.
- **Greenfield, NOT an alt-implementation.** No incumbent to displace, so it dodges the social trap that actually sank Rubinius (which was never a technical failure). The cost is instead cold-start adoption — solved with joy + the killer niche (Apple-silicon-native Ruby-joy).

## Hardware bets (turn these into language semantics, not library calls)

- **Unified memory (UMA):** no host/device distinction, no `device` keyword, data lives once. The same `.map` line is a GPU dispatch on a big array and one core on a small one.
- **Heterogeneous units:** P/E cores, Metal GPU, SME matrix (M4+) — the runtime _places_ work.
- **Hardware safety:** Memory Tagging (MTE) → memory safety with zero annotations, no GC pause, no borrow-checker ceremony. PAC everywhere.
- **Energy topology:** P/E cores + QoS as a type-level effect (`@ecore @background` = placement as an effect).
- **Honest limit:** the Neural Engine and old AMX are not openly programmable — reach them via CoreML / Accelerate / MPS. The GPU (Metal) and CPU (NEON, SME on M4+) _are_ directly targetable. Don't pretend we compile straight to the NPU.

## Name & namespaces

- **Name: Portland. Extension: `.pdx`.** (First pick was Hop/`.hop`, but `@hoplang` + `hoplang.com` were already taken by another new language.) Portland fits better anyway: keep-it-weird craft ethos, the teal PDX-airport-carpet as a ready-made visual identity, a faint Rose City → red → Ruby lineage echo, and `.pdx` is the user's hometown airport code stamped on every file.
- **Availability checked:** domains `pdxlang.org` / `portlandlang.org` (+ hyphenated) free; GitHub orgs `pdxlang` / `portlandlang` free (bare `portland`/`pdx` blocked — taken as _users_, same namespace as orgs); crates.io `portland` free (bare `pdx` is a reserved placeholder). Plan: org = `pdxlang` or `portlandlang`; crate = `portland`.
- `.pdx` collides with an Adobe Acrobat catalog index format — deemed a non-issue for source files.

## Next steps

1. Squat the namespaces: crate `portland`, GitHub org (`pdxlang`/`portlandlang`), domain.
2. Brand story (banked but not done): voice, tagline ("close to the metal, on Metal" is a positioning asset), the teal-carpet / Rose City identity.
3. First real technical drill-down still open: the compile pipeline inside the Rust floor (lexer → parser → inference → MLIR → CPU/GPU/SME).

See `docs/DESIGN.md` for the fuller rationale behind every decision above.
