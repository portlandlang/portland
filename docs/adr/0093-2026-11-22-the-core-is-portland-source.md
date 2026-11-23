# 0093 — The core is Portland source, loaded before every program

<!-- generated: do not edit by hand — script/docs/generate -->

> [!IMPORTANT]
> **A decision record, not a reference.** It says why a ruling was made; a later ruling may have changed it without marking this file. See also: [architecture.md: The core](../../docs/architecture.md#the-core) · [docs/ruby/metaprogramming.md](../../docs/ruby/metaprogramming.md) · [docs/ruby/system.md](../../docs/ruby/system.md).

<!-- /generated -->

- **Status:** Accepted (ruled by the deciding user 2026-11-22 on [#211](https://github.com/portlandlang/portland/issues/211), one question at a time). Decision 6 is provisional and may change after the Ruby corpus research. The loader is built ([#436](https://github.com/portlandlang/portland/issues/436), 2026-11-23), with `putc` the first function written in core. Unbuilt: the declaration form and its refusal are [#437](https://github.com/portlandlang/portland/issues/437), and the first type moved is [#438](https://github.com/portlandlang/portland/issues/438).
- **Date:** 2026-11-22
- **Issue:** [#211](https://github.com/portlandlang/portland/issues/211)

## Context

Nothing in Portland is loaded without a `require`. Every name a program reaches bare, and every method a builtin value answers, is an arm in the seed's Rust and a mirror of that arm in the hosted evaluator. So every builtin is written twice, and the seed that was meant to be disposable grows with each one.

Portland is self-hosted in the Rubinius and Smalltalk sense: the language's own library is written in the language. The deciding user's direction is a core organized as Ruby's is, one file per type or trait (`array.pdx`, `hash.pdx`, `enumerable.pdx`), with the Rust beneath it shrinking to what cannot be written in Portland. A method written once in core runs on both implementations, and both of its copies are deleted.

[#211](https://github.com/portlandlang/portland/issues/211) asked what is loaded implicitly, in what order, and how each implementation finds it. [ADR 0068](0068-2026-10-25-system-is-the-floor-module.md) had already leaned toward a `Kernel` module under a prelude rule, and [ADR 0064](0064-2026-10-25-the-standard-library-core-default-tiles-external-tiles.md) had named the core as the part of the library that ships inside `pdx`.

## Decision

1. **`core/core.pdx` is the entry file.** It holds only `require_relative` lines, one per core file, in the order they load. Both implementations load it before the program. The order is written in source and reviewed in diffs. Loading by file name was refused as implicit, and a manifest was refused as waiting on the package system ([#174](https://github.com/portlandlang/portland/issues/174)).
1. **The core is embedded in the binary.** The seed bakes every file under `core/` in at build, so `pdx` stays one file that runs from any directory. The hosted evaluator must evaluate the core itself, as guest source, ahead of the guest program, and for that it needs the text: `System.core_source(name)` answers one embedded file's text as a String and refuses a name the binary does not hold. Reading `core/` from disk on the hosted side was refused: the two implementations would read different bytes whenever the binary was older than the checkout.
1. **In core, a struct named for a builtin type is that type.** `class Array … end` in a file loaded through `core/core.pdx` attaches its methods to the builtin array. It takes no fields; the value's representation stays the floor's. `Enumerable` and `Comparable` are traits written in core and included by the core types, as [ADR 0028](0028-2026-07-27-object-model-structs-and-traits.md) anticipated. The privilege comes from where the source was loaded, never from its text. A core-only keyword was refused as costing a word no program may use; traits alone were refused because a type's own methods would have nowhere to live; module functions were refused because core would stop reading as Ruby.
1. **What core does not define stays a floor arm.** `def first = self[0]` calls the floor's `[]`. Moving a method is one change: write it in core, delete its Rust arm and its evaluator mirror. A method is never defined in both places. The stated destination is primitives declared in the core file itself, so that every method a type answers appears in its file and the marked ones are the native runtime's interface; that form is not designed here.
1. **The bare functions are top-level `def`s in `core/kernel.pdx`.** The core loads ahead of the program in one environment, so a top-level `def` there is callable bare with no exception to [ADR 0021](0021-2026-07-24-namespaces-and-modules.md). There is no `Kernel` module, so `Kernel.puts` does not exist. This sets aside ADR 0068's lean; the defs can move into a module later without changing a call site. `puts`, `p`, `print`, and `format` stay floor arms while splat parameters are deferred ([ADR 0014](0014-2026-07-22-keyword-arguments.md)).
1. **A program that declares a struct named for a core type refuses**, naming the closed runtime and the rewrites: a trait, a struct that wraps the value, or a function. A Rubyist who writes `class Array` in a program means to reopen it, and handing them an unrelated struct is a guess. This reverses, for structs, the line of [ADR 0058](0058-2026-10-16-builtin-types-carry-type-functions.md) that says a struct wearing a builtin type's name wins. Provisional.

## Not decided here

- Whether a program may define a top-level function the core already defines.
- Whether a local may still wear a core type's name.
- How much speed a method may cost when it moves from Rust to interpreted Portland before it waits for the backend ([#5](https://github.com/portlandlang/portland/issues/5)). The first type moved measures it ([#438](https://github.com/portlandlang/portland/issues/438)).
- The form that declares a primitive in a core file.

## Consequences

- The runtime stays closed. The core says what each builtin type answers once, before any program runs; after that the method set is fixed, and no program can add to it.
- The seed stops taking new builtins that can be written in Portland. It keeps the primitives, and that set is the interface a native runtime must supply.
- The seed parses the core on every start, so startup grows as files move in; `script/bench` tracks it.
- A method in core is interpreted by the seed's tree-walker, and on the hosted side by the evaluator the seed is interpreting. Both are slower than a Rust arm, and stay so until Portland compiles to native code.
- Migration: a program that declared its own struct named `Array`, `Hash`, `Set`, or another core type must rename it.
- Ledger: [metaprogramming.md](../ruby/metaprogramming.md), [system.md](../ruby/system.md).
