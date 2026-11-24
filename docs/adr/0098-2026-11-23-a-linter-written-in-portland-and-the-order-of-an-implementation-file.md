# 0098 — A linter written in Portland, and the order of an implementation file

<!-- generated: do not edit by hand — script/docs/generate -->

> [!IMPORTANT]
> **A decision record, not a reference.** It says why a ruling was made; a later ruling may have changed it without marking this file. See also: [architecture.md: The backend — designed, not built](../../docs/architecture.md#the-backend--designed-not-built).

<!-- /generated -->

- **Status:** Accepted (ruled by the deciding user 2026-11-23; built in [#508](https://github.com/portlandlang/portland/issues/508), [#509](https://github.com/portlandlang/portland/issues/509), and [#510](https://github.com/portlandlang/portland/issues/510)).
- **Date:** 2026-11-23
- **Issue:** [#510](https://github.com/portlandlang/portland/issues/510)

## Context

Nothing read a `.pdx` file for how it was written. In one week two slips went unseen for that. Lines of a list in `compiler/llvm.pdx` sat at the left margin, because a scripted edit dropped their indentation and every line added beside them copied it ([#508](https://github.com/portlandlang/portland/issues/508)). And the implementation's files held their structs, constants, and defs in the order they were written, each near where it was first used: an order that was in the writer's head and not on the page.

The deciding user asked whether RuboCop could be made to read Portland.

## Decision

1. **Portland has a linter of its own, written in Portland**, over the compiler's own lexer and parser, so a file is read as the language reads it and never by pattern. `script/lint` runs it over `compiler/` and `core/`, and the gate runs that.
1. **It reports and changes nothing.** A formatter that rewrites a file waits on a tree that keeps every comment and column, which the compiler's does not.
1. **RuboCop made to read Portland was declined.** It reads Ruby's syntax tree, and Portland is not Ruby: `struct Token` and its `end` do not parse, and `mutable total = 0` parses as a call to a method named `mutable`. It would take a translation of Portland's tree into Ruby's with a position for every node, most cops turned off for giving Ruby's advice, and custom cops written anyway. A tool for the language written in the language is also what self-hosting asks for.
1. **Implementation code is in a predictable order; a program reads as a story.** In `compiler/` and `core/` a file is its header comment, its requires, its constants, its traits, its structs, its defs, and then what it runs, each kind alphabetical, the methods inside a struct or a trait too. A def used by one other def is sorted with the rest, not kept under its caller. A fixture, a spec, a benchmark, or an example in the docs reads top to bottom.
1. **Order is kept where it means something**: a constant after the one it is built from, a trait ahead of a struct that includes it, the arms of a `case`, a struct's fields.
1. **In the core, a method with more than one name names its twins** in its comment, since sorted they no longer sit side by side.
1. **An acronym in a type's name is kept whole**, as Ruby keeps it: `LLVMEmitter`, not `LlvmEmitter`.
1. **The Rust is left alone.** The seed is meant to be replaced, and the runtime follows `cargo fmt`.

## Not decided here

- A formatter.
- The rules beyond the first three: a name of one character, an acronym kept whole, and alphabetical order wherever order does not matter are stated and not yet written as rules.

## Consequences

- A **deliberate non-difference** from Ruby: none of this is about what a program means.
- A heading over a run of defs has nothing following it once the file is sorted. Each such comment joined its file's header.
- `git blame` on a sorted file stops at the commit that sorted it.
- The linter takes about ten seconds over the implementation on the seed, which is the cost [#511](https://github.com/portlandlang/portland/issues/511) is aimed at.
