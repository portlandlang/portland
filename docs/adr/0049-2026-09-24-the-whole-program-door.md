# 0049 — The whole-program door: a program is its require closure

- **Status:** Accepted (rulings 2026-09-24; the door built the same day — every checking tool follows requires; sharpening in run mode is the next increment)
- **Date:** 2026-09-24
- **Issue:** [#94](https://github.com/portlandlang/portland/issues/94), out of [#9](https://github.com/portlandlang/portland/issues/9)

## Context

The checker checked one file. ADR 0048 found that caller flow — "every reachable call passes a `Token`" — is not knowable from one file, since the compiler's files require one another, and filed #94. Probing the require boundary for this brief found the per-file checker already refusing working code: a construction naming an enum case declared in a required file refused with `no enum declares a case :paid`, ADR 0034's first check firing on a declaration it could not see. A checker that refuses working code is a checker bug by doctrine (ADR 0040), and the door is its fix.

The same probe found a required file's top-level locals leaking into the requirer on the seed, which no ADR decides; that is [#95](https://github.com/portlandlang/portland/issues/95), and this door neither refuses nor cements it.

## Decision

1. **A program is its entry file and everything `require_relative` pulls in, transitively**, resolved as the runtime resolves it — relative to the requiring file, `.pdx` assumed unless the last segment brought its own extension, each file once. The rule lives once, in `paths.pdx`, and the evaluator resolves by it too. Only a literal path resolves; a computed one is an edge the walk declines, and that file is checked as if alone. A cycle finds the file it came from still in progress and takes nothing from it, which is what the runtime's load-once does.
1. **Declarations cross a file boundary; locals never do.** A file is checked with the defs (returns, contracts, arities), structs, traits, and enums of its transitive requires in view, its own declarations first so a name declared on both sides resolves to its own. A required file's locals stay unknown to its requirer, deliberately (#95).
1. **Each file is checked once, in dependency order.** `check_closure` walks the requires depth-first and memoises what each file exports — its own declarations plus its imports — so a helper a hundred specs require is checked once per process, and `check.pdx` handed several files checks their shared libraries once.
1. **Every tool that checks follows requires; only a tool that executes may sharpen.** `run.pdx`, the spec harness, and `check.pdx` all walk the closure. Sharpening a parameter to the type every caller passes (ADR 0048 ruling 4) is honest only in a closed world, and only a tool that executes a program has one: nothing outside the closure `run.pdx` runs can call into it, where a file `check.pdx` is handed may be a library with callers nobody has written. So `run.pdx` and the spec harness will sharpen; `check.pdx` never does. Sharpening is the next increment, not this one.
1. **A location names its file** when a program spans more than one: `  lib.pdx:7 | puts name.knd`, the renderer's one function as before.

## Consequences

- The corpus is honest about requires: `check.pdx compiler/*.pdx` checks the compiler as the programs it is, and the false refusal across a require is gone. The cost is real — a closure is the whole compiler for the drivers — so `check.pdx` takes several files and shares one memo, and the self-check test hands it the corpus in one process, about ninety seconds for the compiler, the fixtures, and the spec suite together.
- Arity, contracts, shape 2 on struct receivers, and annotations now reach across files, where every one of them declined as unknown before.
- Next: sharpening in run mode (ruling 4's other half), with a whole-program twin of the self-check so the compiler's own source stays the corpus for the sharpened checks.
- Ledger: [static-checks.md](../ruby/static-checks.md) — Ruby's `require_relative` loads at runtime and checks nothing; here it draws the program's boundary for the build.
