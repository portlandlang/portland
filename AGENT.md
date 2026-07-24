# Portland

A joyous programming language for Apple silicon.

**Status:** Stage 0 seed built, Stage 1 begun. The Rust seed (`seed/`)
lexes, parses, and interprets a real slice of Portland — including the
headline optionals feature — with a `pdx` binary and REPL. The Portland
trio (`compiler/lexer.pdx`, `parser.pdx`, `evaluator.pdx`) is Portland
written in Portland: the parser parses the whole compiler including
itself, and the evaluator runs the fixture suite byte-identical to the
seed. See [ROADMAP.md](ROADMAP.md) for the one-page burn-down,
[docs/language.md](docs/language.md) for exactly what's built, and the
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

## What's decided

**[docs/language.md](docs/language.md)** is what Portland speaks today —
every section marked built unless it says otherwise, with the four
governing rules up front (no shadowing, never guess, immutable by
default, no truthiness or ambient nil), a style section, and honest
"decided but unbuilt" and "not yet designed" lists at the end.

[docs/adr/](docs/adr/) is the decision log, one file per decision;
[docs/ruby/](docs/ruby/) is what each divergence costs a Rubyist. Neither
gets summarized here — a summary is a second home for a fact, and it goes
stale exactly this fast.

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
