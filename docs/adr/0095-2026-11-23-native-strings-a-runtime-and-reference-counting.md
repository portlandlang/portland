# 0095 — Native Strings, a runtime crate, and reference counting

<!-- generated: do not edit by hand — script/docs/generate -->

> [!IMPORTANT]
> **A decision record, not a reference.** It says why a ruling was made; a later ruling may have changed it without marking this file. See also: [architecture.md: The backend — designed, not built](../../docs/architecture.md#the-backend--designed-not-built).

<!-- /generated -->

- **Status:** Accepted (ruled by the deciding user 2026-11-23, one question at a time; tracked on [#461](https://github.com/portlandlang/portland/issues/461)). Built 2026-11-23 in [#462](https://github.com/portlandlang/portland/issues/462), [#463](https://github.com/portlandlang/portland/issues/463), [#464](https://github.com/portlandlang/portland/issues/464), [#465](https://github.com/portlandlang/portland/issues/465), and [#466](https://github.com/portlandlang/portland/issues/466).
- **Date:** 2026-11-23
- **Issue:** [#461](https://github.com/portlandlang/portland/issues/461)

## Context

The native spike ([ADR 0094](0094-2026-11-22-a-native-spike-through-llvm-ir-text.md)) compiles integers and Booleans, which live in registers and need nothing from the heap. Every other value does: a String, an array, a hash, a struct. So the second slice had to choose how a native Portland program holds memory, and where the code that manages it lives. A String is the smallest value that forces both, and `puts "hello"` is the first program anyone tries.

The memory model was open on [#12](https://github.com/portlandlang/portland/issues/12), with reference counting the plan: values never mutate ([ADR 0015](0015-2026-07-23-values-never-mutate.md)), so they cannot form cycles, and a count is exact. The floor was planned as Rust, thin and permanent. Neither had been built.

The deciding user asked for the most obvious approach and the least clever, in an area new to them. That request is itself the rule the decisions below follow.

## Decision

1. **The slice is Strings without measuring**: literals, `puts` of a String, `+` between Strings, interpolation of a String or an Integer, and `==`. `length` and `[]` count graphemes ([ADR 0038](0038-2026-08-19-what-a-character-is.md)) and are left out.
1. **A String is laid out plainly**: a pointer to a block holding a reference count, a byte length, and UTF-8 bytes. A literal is a constant in the binary whose count marks it as never freed. Inline small strings, slices that share bytes, ropes, and cached measures were each declined: each makes strings faster and adds a case every operation must handle. Nothing a program can observe depends on the layout, so it can be replaced later and the oracle test proves the replacement invisible.
1. **Memory is plain reference counting.** The compiler emits a retain where a value is bound or passed and a release where a scope ends, on every path out, with no cleverness. This is a ruling on [#12](https://github.com/portlandlang/portland/issues/12) for the native side only; arenas, escape analysis, and in-place reuse stay open there.
1. **A debug build proves the counting.** It counts live blocks and refuses at a clean exit if any remain, and refuses on the spot if a count goes below zero. Every native fixture runs under it. An exit by refusal skips the leak check, since a program stopped mid-way holds what it held.
1. **The runtime is a crate named `runtime`**, in the workspace, built as a static library that `clang` links. It owns memory and the operations a String needs. Nothing of the seed's code is reused. Its functions use the C calling convention and are named `portland_…`, and a panic inside it aborts and never unwinds through compiled Portland. `core`, `kernel`, and `foundation` were declined as names already meaning something else here or on this platform; "the floor" stays the architecture's word for the layer.
1. **`==` is canonical from the first day** ([ADR 0038](0038-2026-08-19-what-a-character-is.md)), so the runtime takes `unicode-normalization`, pinned once in the workspace and shared with the seed. The two then run the same code over the same tables and cannot disagree about any character. The operating system's own Unicode library was declined: Apple publishes only part of it, and a string's answer would change with a macOS update. Strings are UTF-8 and nothing else.
1. **Pinning gets a solver over all the defs together**, replacing the guess over a list of types ([ADR 0094](0094-2026-11-22-a-native-spike-through-llvm-ir-text.md)), and a def that answers nothing compiles.
1. **`script/build` links the release runtime when optimized and the debug runtime under `--unoptimized`**, so the build that shows what the compiler wrote is also the one that checks its counting.

## Not decided here

- Graphemes natively. When they come, the runtime shares the seed's `unicode-segmentation` as it shares normalization.
- Blocks. A block cannot be held in a value ([#77](https://github.com/portlandlang/portland/issues/77)), so it never outlives its method and may need no heap.
- Arrays, hashes, structs, maybes, and failures, and whether each keeps the plain layout.
- The one Unicode version Portland names: the seed's sources carry more than one ([#456](https://github.com/portlandlang/portland/issues/456)).

## Consequences

- A **deliberate non-difference** from Ruby and from the two interpreters: a program inside the slice prints the same bytes natively as on the seed.
- Rust stays under every native binary. That matches the architecture's permanent floor, and it is the part self-hosting does not remove.
- The Unicode tables are compiled into each binary that compares Strings.
- Reference counting is built into what the compiler emits, so it is the one decision here that is not freely replaced.
