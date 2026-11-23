# 0094 — A native spike: `fib(30)` through LLVM IR as text

<!-- generated: do not edit by hand — script/docs/generate -->

> [!IMPORTANT]
> **A decision record, not a reference.** It says why a ruling was made; a later ruling may have changed it without marking this file. See also: [architecture.md: The backend — designed, not built](../../docs/architecture.md#the-backend--designed-not-built).

<!-- /generated -->

- **Status:** Accepted (ruled by the deciding user 2026-11-22, one question at a time; tracked on [#443](https://github.com/portlandlang/portland/issues/443)). Unbuilt: the steps are [#444](https://github.com/portlandlang/portland/issues/444), [#445](https://github.com/portlandlang/portland/issues/445), [#446](https://github.com/portlandlang/portland/issues/446), [#447](https://github.com/portlandlang/portland/issues/447), and [#448](https://github.com/portlandlang/portland/issues/448).
- **Date:** 2026-11-22
- **Issue:** [#443](https://github.com/portlandlang/portland/issues/443)

## Context

Portland is two interpreters: the Rust seed, and the compiler written in Portland that the seed runs. Nothing emits a binary. The plan for the backend is MLIR on LLVM ([#5](https://github.com/portlandlang/portland/issues/5)), and every task on it is open: which dialects, where the parallelism decision lives, what the floor is written in. The memory model is open beside it ([#12](https://github.com/portlandlang/portland/issues/12)).

The deciding user asked what is truly missing before native compilation can be explored at all, even as a proof of concept, for a fraction of the language. Two things were found by looking. The compiler cannot run a process ([#179](https://github.com/portlandlang/portland/issues/179)), so it can write a file and no more. And inference does not give a code generator what it needs: a plain recursive `fib` has a parameter with a contract and a return of `Unknown`, which is right for a checker that refuses only what it can disprove, and not enough to choose a machine type.

Self-hosting is the point of the exercise: the seed stays disposable only if Portland can one day compile itself. This spike is the first step on that road, taken small enough to finish.

## Decision

1. **The first slice writes LLVM IR as text and hands it to `clang`.** The compiler stays Portland source that writes a file; nothing is added to the seed. MLIR remains the plan. The lowering goes in two steps — the checked AST to a small typed intermediate representation of Portland's own, then that representation to text — so the first step survives a move to MLIR and only the printer is replaced. Starting in MLIR was refused for now: its tools are not installed with Apple's toolchain, a dialect of our own is C++ and TableGen, and the first binary would wait on decisions about MLIR and not about Portland.
1. **Types are pinned from the call sites.** A def's parameters take their types from how it is called, the body's types follow, and a recursive call is assumed to answer the type being computed, with the assumption checked. The slice is Integer and Boolean. A def that cannot be pinned refuses by name. The pinned types are read by the native build alone: a checker that knows more disproves more, so feeding them to the checker is a separate proposal. Boxing every value was refused as a compiled interpreter that sidesteps the question; leaning on annotations was refused as making native code depend on comments.
1. **The slice is `fib(30)` and a counting `while` loop**: integers with `+`, `-`, `*`, the comparisons, bindings and `mutable` rebinding, `if`/`elsif`/`else`, `while`, `def` with positional parameters, calls, recursion, and `puts` of one Integer. Everything else refuses at build in one named sentence, and nothing is written.
1. **Arithmetic is checked.** Integers are 64-bit and refuse on overflow ([ADR 0070](0070-2026-11-03-integers-are-64-bit-for-now.md)); the native `+`, `-`, and `*` use LLVM's overflow-checking intrinsics, and overflow writes the seed's sentence and exits as the seed does. Wrapping silently was refused: the semantics would be wrong, and a benchmark would be flattered by a check it must pay later.
1. **There is no runtime.** The emitted IR calls libc for `puts` and for the refusal. A runtime linked in, in Rust or in C, was refused for the spike: it would begin the floor before deciding what the floor is. Nothing of the seed's Rust is reused.
1. **The native build is held to the seed.** A test helper runs a program on the seed, builds it natively, and compares output and exit status exactly; it runs in the fast gate. A construct is native when its fixture passes. `script/bench` gains a native column, with a note that an integer-only number says nothing about strings, collections, or memory.
1. **The compiler is Apple's `clang`, through `xcrun`.** `script/bootstrap` checks for it and prints its version. Its version follows Xcode's, and CI's runner label is the pin of record. A Homebrew LLVM waits for MLIR, which needs its tools.
1. **The minimum macOS is the current one**, written in `.macos-version` at the repo root ([#444](https://github.com/portlandlang/portland/issues/444)). Until a first public release there is nothing older to support. The build passes it to `clang` as the deployment target. CI's runner may trail it, and there the build uses the runner's own version with a notice.
1. **The files** are `compiler/intermediate_representation.pdx`, `compiler/llvm.pdx`, and `compiler/build.pdx`, with a `script/` wrapper that runs `clang`. Output goes under `target/portland/`, named for what produced it: Cargo's own directories there hold the seed.

## Not decided here

- The long-term floor, and what a runtime is written in ([#5](https://github.com/portlandlang/portland/issues/5)).
- The memory model ([#12](https://github.com/portlandlang/portland/issues/12)): the next decision once anything in a native program needs the heap, which strings, arrays, and blocks all do.
- MLIR's dialects ([#5](https://github.com/portlandlang/portland/issues/5)).
- How the compiler runs `clang` itself ([#179](https://github.com/portlandlang/portland/issues/179)).
- Runaway recursion: a native stack overflow is a fault, where an interpreter can say a sentence.

## Consequences

- A **deliberate non-difference** from Ruby and from the two interpreters: a program inside the slice prints the same bytes and exits the same way natively as on the seed. Nothing a program reads changes.
- The spike is far from the fixpoint. For the compiler to compile itself, native needs strings, arrays, hashes, structs, blocks, and pattern matching.
- There are three implementations while this grows. The core written in Portland ([ADR 0093](0093-2026-11-22-the-core-is-portland-source.md)) is what keeps that from tripling the library: a method in core is compiled, not written a third time, and what core does not define is the list a native runtime must supply.
- The compiler itself still runs on the seed, so building is slow; the binary it produces is not.
