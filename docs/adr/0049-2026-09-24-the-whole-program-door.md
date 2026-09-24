# 0049 — The whole-program door: a program is its require closure

- **Status:** Accepted (rulings 2026-09-24; built the same day — every checking tool follows requires, and run mode sharpens the entry file's own defs)
- **Date:** 2026-09-24
- **Issue:** [#94](https://github.com/portlandlang/portland/issues/94), out of [#9](https://github.com/portlandlang/portland/issues/9)

## Context

The checker checked one file. ADR 0048 found that caller flow — "every reachable call passes a `Token`" — is not knowable from one file, since the compiler's files require one another, and filed #94. Probing the require boundary for this brief found the per-file checker already refusing working code: a construction naming an enum case declared in a required file refused with `no enum declares a case :paid`, ADR 0034's first check firing on a declaration it could not see. A checker that refuses working code is a checker bug by doctrine (ADR 0040), and the door is its fix.

The same probe found a required file's top-level locals leaking into the requirer on the seed, which no ADR decides; that is [#95](https://github.com/portlandlang/portland/issues/95), and this door neither refuses nor cements it.

## Decision

1. **A program is its entry file and everything `require_relative` pulls in, transitively**, resolved as the runtime resolves it — relative to the requiring file, `.pdx` assumed unless the last segment brought its own extension, each file once. The rule lives once, in `paths.pdx`, and the evaluator resolves by it too. Only a literal path resolves; a computed one is an edge the walk declines, and that file is checked as if alone. A cycle finds the file it came from still in progress and takes nothing from it, which is what the runtime's load-once does.
1. **Declarations cross a file boundary; locals never do.** A file is checked with the defs (returns, contracts, arities), structs, traits, and enums of its transitive requires in view, its own declarations first so a name declared on both sides resolves to its own. A required file's locals stay unknown to its requirer, deliberately (#95).
1. **Each file is checked once, in dependency order.** `check_closure` walks the requires depth-first and memoises what each file exports — its own declarations plus its imports — so a helper a hundred specs require is checked once per process, and `check.pdx` handed several files checks their shared libraries once.
1. **Every tool that checks follows requires; only a tool that executes may sharpen.** `run.pdx`, the spec harness, and `check.pdx` all walk the closure. Sharpening a parameter to the type every caller passes (ADR 0048 ruling 4) is honest only in a closed world, and only a tool that executes a program has one: nothing outside the closure `run.pdx` runs can call into it, where a file `check.pdx` is handed may be a library with callers nobody has written. So `run.pdx` and the spec harness sharpen; `check.pdx` never does. **And only the entry file's own defs are sharpened**, a refinement found in the build: a required file is a library by nature — something requires it — so its defs may have callers outside this closure, where the entry's defs have every caller in view, calls from required files included (the runtime binds late, and a library may call back into its requirer). This also keeps the spec harness's memo honest: a shared helper checked once is never sharpened under the first spec's world alone. **And only from callers that satisfy the contract:** a caller whose argument fails a parameter's contract refuses at its own call site — the better sentence, where the mistake was made (ADR 0048) — and disqualifies the parameter from sharpening, so the body never refuses first with a lesser one about the same mistake.
1. **A location names its file** when a program spans more than one: `  lib.pdx:7 | puts name.knd`, the renderer's one function as before.

## Consequences

- The corpus is honest about requires: `check.pdx compiler/*.pdx` checks the compiler as the programs it is, and the false refusal across a require is gone. The cost is real — a closure is the whole compiler for the drivers — so `check.pdx` takes several files and shares one memo, and the self-check test hands it the corpus in one process, about ninety seconds for the compiler, the fixtures, and the spec suite together.
- Arity, contracts, shape 2 on struct receivers, and annotations now reach across files, where every one of them declined as unknown before.
- Sharpening's corpus is the spec suite: 138 programs, each run in run mode with helper defs called at concrete types, all green. The compiler's own drivers define almost nothing at top level, so a whole-program twin of the self-check would exercise little; it is not built.
- A check-only door for an application — "check this file as a program, sharpen, do not run" — is a small driver if a real file pulls for it; nothing does yet.
- Ledger: [static-checks.md](../ruby/static-checks.md) — Ruby's `require_relative` loads at runtime and checks nothing; here it draws the program's boundary for the build.
