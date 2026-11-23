# 0096 — Native blocks, arrays, and maybes

<!-- generated: do not edit by hand — script/docs/generate -->

> [!IMPORTANT]
> **A decision record, not a reference.** It says why a ruling was made; a later ruling may have changed it without marking this file. See also: [architecture.md: The backend — designed, not built](../../docs/architecture.md#the-backend--designed-not-built).

<!-- /generated -->

- **Status:** Accepted (ruled by the deciding user 2026-11-23, one question at a time; tracked on [#491](https://github.com/portlandlang/portland/issues/491)).
- **Date:** 2026-11-23
- **Issue:** [#491](https://github.com/portlandlang/portland/issues/491)

## Context

The core became Portland source ([ADR 0093](0093-2026-11-22-the-core-is-portland-source.md)), and the first type moved into it: an array's `map`, `select`, `reject`, `find`, and `empty?` are written in `core/` and run by both interpreters. Measured on [#438](https://github.com/portlandlang/portland/issues/438), a program that walks arrays ran slower afterward, and the compiler, which is itself a Portland program that walks arrays, slower again. Part of that is inherent to an interpreter running a method where a Rust arm ran before.

The native build ([ADR 0094](0094-2026-11-22-a-native-spike-through-llvm-ir-text.md), [ADR 0095](0095-2026-11-23-native-strings-a-runtime-and-reference-counting.md)) compiles integers, Strings, and top-level defs. It compiles nothing a core method is made of: a block, an array, a maybe, a method on a type. So a core method is only ever interpreted, and moving one shows its cost and none of its gain.

The deciding user set the order all of this follows: make it work, even if slow; make it good; make it fast, measuring first and never passing the burden to whoever writes Portland.

## Decision

1. **The core holds at Array.** The loader and the methods already moved stay, as the proof that the mechanism works. No further type moves until the native build can compile a method written in the core.
1. **A native block is its own function**, handed a pointer to its writer's locals, which it reads and rebinds through that pointer. A block cannot be held in a value ([#77](https://github.com/portlandlang/portland/issues/77)), so it never outlives the call it was handed to and needs no heap. Writing the block inline where it is yielded to was declined as the first form: it cannot serve a def that calls itself, and it is an optimization the plain form can take later, part of which `clang` does unasked.
1. **A native array has one layout per element type**, picked by the types the build already pins. Under the types there are two kinds of slot: a plain word, which is the value, and a counted pointer, which the array retains when it stores and releases when it is dropped. An array is itself counted, as a String is, and lives in the `runtime` crate. One layout that tags every element was declined: it doubles each element and checks a tag on every read, to serve the mixed array, which is the rare one.
1. **The element types come in an order**: Integer, Boolean, String, and an array of arrays first, since those values are already native; then Symbols and their arrays; then Floats and theirs. A mixed array refuses at build.
1. **A native maybe is a flag beside the value**, for every type: one word saying present or absent, and the value. A null pointer standing for nil where the value is a pointer was declined as the first form. It is the well-known saving and changes nothing a program can observe, so it can come later; the flag is one form for every type, and a maybe of a maybe needs no case of its own.
1. **The order is blocks, arrays, maybes, methods on a type, then Symbols and Floats.** Methods on a type come ahead of the last two because they are what ends the hold in the first decision.

## Not decided here

- A mixed array, and an array of structs, hashes, or ranges.
- `break` and `return` written inside a block, which reach past the block's own function.
- How a method on a type is compiled.
- What a block call costs on the seed, which is its own investigation ([#488](https://github.com/portlandlang/portland/issues/488)).

## Consequences

- Until the hold ends, an array's walks stay slower interpreted than they were as Rust arms. The switch back is one line in `core/core.pdx`, and it was not taken.
- Each form chosen here is the one that handles every case, and each has a known faster form declined by name. Replacing one is invisible to a program, and the oracle test that holds every native fixture to the seed proves it so.
- A **deliberate non-difference** from Ruby and from the two interpreters: a program inside the slice prints the same bytes natively as on the seed.
- A program with a mixed array runs on the interpreters and refuses natively, by name, until a later ruling.
