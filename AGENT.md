# Portland

A joyous programming language for Apple silicon.

**Status:** Stage 0 seed built, Stage 1 begun. The Rust seed (`seed/`)
lexes, parses, and interprets a real slice of Portland — including the
headline optionals feature — with a `pdx` binary and REPL. The Portland
trio (`compiler/lexer.pdx`, `parser.pdx`, `evaluator.pdx`) is Portland
written in Portland: the parser parses the whole compiler including
itself, and the evaluator runs the fixture suite byte-identical to the
seed. See [ROADMAP.md](ROADMAP.md) for the one-page burn-down,
[docs/STAGE0.md](docs/STAGE0.md) for exactly what's built, and
[docs/reports/2026-07-22-open-decisions.md](docs/reports/2026-07-22-open-decisions.md)
for what's next.

## North star

Programmer happiness first, like Ruby. Job one is the joy of reading and
writing the code. Safety and performance are job 1.1 — not tradeoffs
_against_ joy, but _contributors_ to it. The rule every feature must
pass: does this make the beautiful line also the safe, fast line — or
does it force a different, uglier line to get safe and fast? Reject the
latter.

The premise: a language that runs **only** on Apple silicon (A-series /
M-series, macOS 26+), and is **not** Swift. Locking to one vendor's
hardware is the feature — it lets us make assumptions general, portable
languages are forbidden to make.

## How decisions get made (the working method)

- **ADRs** — every language decision is one file in
  [docs/adr/](docs/adr/) (`NNNN-date-slug`, Accepted/Tentative/
  Superseded). Issues discuss; ADRs decide. Twelve exist so far.
- **The Ruby ledger** — every Ruby-divergent decision gets a file in
  [docs/ruby/](docs/ruby/), updated alongside its ADR. Two promises
  govern it: divergence is **loud, never silent** (agreements compile
  verbatim; differences are compile errors with suggested rewrites), and
  the **polyfill test** (a future gem + linter should be able to teach
  each idiom inside valid Ruby before a codebase flips).
- **The Ruby-match tiebreaker** — for anything end users type, matching
  Ruby is the preferred answer unless it costs a design principle. Tie
  goes to Ruby.
- **Never guess** — where one spelling has two genuine readings
  (`puts -1`, `Boolean?` with `or`), Portland errors and asks rather
  than picking. No shadowing: a name is a local or a method, never both.
- **Demand-driven** — features get built when a real Portland file pulls
  for them; issues are commitments, not wishes. The evidence engine is
  [portlandlang/ruby_research](https://github.com/portlandlang/ruby_research)
  (corpus reports over rubygems.org).
- **The differential harness** — the seed is the oracle; the trio must
  match it byte-identically. Never hand-write expected output.

## Decided (ADRs, in brief)

- **Ruby's good parts, kept (the surface):** blocks-as-prose,
  everything-is-an-expression, implicit returns, `?`/`!` suffixes,
  postfix guards, keyword args, Enumerable as one protocol, pattern
  matching (promoted to load-bearing).
- **Ruby's bad parts, cut (the runtime):** monkeypatching / open
  classes, `method_missing`, runtime `define_method`, `eval`, globals,
  truthiness, perlisms (`for`, the `and`/`or` secret precedence, …).
  Runtime metaprogramming's replacement is **compile-time macros**
  (undesigned, #14).
- **Optionals** (ADRs 0005–0010, 0012 — designed _and_ built, runtime
  half): no ambient nil; absence is one explicit case of a maybe. The
  wrapper model with a collapsed-feeling surface; the words are
  `nil`/`nil?` and `some`/`some?`; `or`/`and`/`not` are dead-identical
  to their sigils and `or` is typed (unwrap-or-else, with `or return` /
  `or panic "why"` as the or-guard); the unwrap toolkit is narrowing,
  or-guard, `&.`, `case/in` — no `if let`, no force-unwrap; partial
  operations (`[].first`, `hash[missing]`, out-of-range indexing) return
  maybes and `fetch` retires; a branch that doesn't happen is nil. The
  only crash is one you typed.
- **Immutable by default;** the mutability keyword is **`mutable`**
  (ADR 0001), fused to first assignment, gating rebinding only. The real
  line is immutable-when-shared, mutable-when-local; mutable _values_
  (`push!`, `<<`) are deliberately undecided (#10).
- **Concurrency vocabulary** (ADRs 0002, 0004, 0011 — tentative,
  unimplemented): `together` blocks with `meanwhile`/`~` dead-identical
  task markers, named-at-site as the only register. Semantics are #11.
- **Bitwise operators out** (ADR 0003, tentative) — named methods
  instead; `<<` append travels with the mutable-values decision.
- **Types inferred, not written** — design open (#9). The lean is
  **bidirectional inference with local generalization**, not
  Hindley-Milner purity: better errors, and it plays well with
  structural typing and future macros. Annotations only at public
  boundaries, as docs. Duck typing becomes structural. The optionals
  _static_ half (narrowing, unhandled-maybe errors, exhaustiveness)
  lives there.

## Concurrency (one model, baked in — never a library that gets deprecated)

Three tiers; you live almost entirely in tier 1.

1. **Implicit — you do nothing.** `photos.map { it.thumbnail }` spreads
   across cores when worth it, safe _because_ values are immutable.
2. **`together` — say "these are independent."**

   ```ruby
   together do
     meanwhile user = fetch_user(id)
     ~ orders = recent_orders(id)      # ~ and meanwhile are dead-identical
   end
   render(user, orders)                # plain values after end
   ```

3. **Explicit control — rare.** Cancellation, timeouts, racing.

## Implementation strategy

- **Frontend in Rust. Backend on MLIR / LLVM** (#5, undesigned). MLIR
  isn't cargo-cult: the heterogeneous-compute thesis (one program → CPU,
  GPU, matrix unit) is what MLIR exists for, and on Apple silicon the
  road to the metal _is_ LLVM.
- **Parser: hand-written recursive descent** (built). Prism's C lexer is
  the textbook for the hard lexical parts still to come (heredocs, #6).
- **Memory model** (#12, plan proposed): the language is memory-safe by
  semantics on every chip; reference counting is _exact_ under
  immutability (immutable values can't form cycles) — no tracing GC, no
  borrow-checker ceremony. EMTE/MIE (A19/M5+) is defense-in-depth for
  the Rust floor, never the foundation.
- **Self-host as early as possible** (Rubinius creed). Stage 0 seed
  (built, disposable on purpose) → Stage 1 rewrite in Portland (the trio
  is its beginning) → Stage 2 fixpoint, **seed retires** → Stage 3 the
  primitive boundary descends. The three states are drawn in
  [docs/architecture.svg](docs/architecture.svg) — now, next, and the
  ideal future (Rust stays as a thin floor; the seed goes).
- **Greenfield, NOT an alt-implementation.** No incumbent to displace —
  dodges the social trap that sank Rubinius. The cost is cold-start
  adoption, solved with joy + the killer niche. The migration story
  (ledger, polyfill test, eventual ruby/spec fork — #23) is how Rubyists
  get here.

## Hardware bets (language semantics, not library calls)

- **Unified memory:** no host/device distinction; the same `.map` line
  is a GPU dispatch on a big array and one core on a small one (#13).
- **Heterogeneous units:** P/E cores, Metal GPU, SME matrix — the
  runtime _places_ work.
- **Hardware safety:** MIE/EMTE as hardening (see #12); PAC for the
  runtime floor.
- **Honest limit:** the Neural Engine isn't openly programmable — the
  only door is CoreML, and CoreML picks the units itself (Accelerate
  is the CPU path, MPS the GPU one; neither reaches the ANE). Don't
  pretend we compile straight to the NPU.

## Name & namespaces (done)

**Portland**, extension **`.pdx`** — keep-it-weird craft ethos, the teal
PDX-carpet identity, a faint Rose City → Ruby lineage echo. Repo:
[portlandlang/portland](https://github.com/portlandlang/portland)
(public; `pdxlang` org squatted; crates.io `portland` is a name squat
only). Companions: `ruby_research` (evidence),
`zed-portland` (`.pdx` editor support, shipped). Brand story is banked,
not done (#1).

See [docs/DESIGN.md](docs/DESIGN.md) for the original rationale behind
every decision above, and the [issues](https://github.com/portlandlang/portland/issues)
for everything in motion.
