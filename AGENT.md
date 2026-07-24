# Portland

A joyous programming language for Apple silicon.

**Status:** Stage 0 seed built, Stage 1 begun. The Rust seed (`seed/`)
lexes, parses, and interprets a real slice of Portland — including the
headline optionals feature — with a `pdx` binary and REPL. The Portland
trio (`compiler/lexer.pdx`, `parser.pdx`, `evaluator.pdx`) is Portland
written in Portland: the parser parses the whole compiler including
itself, and the evaluator runs the fixture suite byte-identical to the
seed. See [ROADMAP.md](ROADMAP.md) for the one-page burn-down,
[docs/STAGE0.md](docs/STAGE0.md) for exactly what's built, and the
[issues](https://github.com/portlandlang/portland/issues) for what's
next.

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

## How decisions get made

**[docs/principles.md](docs/principles.md) is the working method** — the
eleven rules that settle arguments, in precedence order: the joy bar, tie
goes to Ruby, never guess (in the language _and_ in the implementation),
loud-never-silent plus the polyfill test, demand-driven, the seed as
oracle, verify-don't-remember, recommend-don't-enumerate, prior art
first, and communication as a feature.

Read it before deciding anything. Several of those rules exist because we
broke them once, and the file says which.

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
- **Namespaces** (ADR 0021 — designed _and_ built): `module` is a namespace
  and nothing else, so mixins can get their own keyword later and
  `include Comparable` can never be confused with `include Math`. `::`
  names, `.` invokes — a rule, not Ruby's convention. Names are always
  fully qualified: no import, no aliasing, no injection, with lexical
  nesting the only shortening. `module A::B` and nested blocks are
  semantically identical, dropping Ruby's `Module.nesting` trap. Types nest
  in types; modules don't nest in structs. Constants needed no new concept —
  immutability already made `MAX = 5` unrebindable, so all that was missing
  was a place for the name to live.
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

## How it's built

**[docs/architecture.md](docs/architecture.md)** — the seed and the trio
and why they both exist, the direct-vs-hosted differential contract, the
two kinds of Rust (disposable seed vs permanent floor), where the trio
declines to check and why, the bootstrap ladder, and the undesigned parts:
MLIR/LLVM (#5), the memory model (#12), inference (#9), heterogeneous
dispatch (#13), and the honest Neural Engine limit.

## Name & namespaces (done)

**Portland**, extension **`.pdx`** — keep-it-weird craft ethos, the teal
PDX-carpet identity, a faint Rose City → Ruby lineage echo. Repo:
[portlandlang/portland](https://github.com/portlandlang/portland)
(public; `pdxlang` org squatted; crates.io `portland` is a name squat
only). Companions: `ruby_research` (evidence),
`zed-portland` (`.pdx` editor support, shipped). Brand story is banked,
not done (#1).

See [docs/history/](docs/history/) for the original thinking behind the
decisions above — frozen, never current — and the
[issues](https://github.com/portlandlang/portland/issues) for everything
in motion.
