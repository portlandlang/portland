# Portland, for Rubyists

**For:** Rubyists sizing up what moving a codebase would actually cost.

Portland keeps Ruby's surface and replaces what's underneath. This folder is the ledger of differences — one file per difference, each with the Ruby behavior, the Portland behavior, why, and what happens to migrating code.

Smooth Ruby → Portland migration is a standing design criterion. Two promises govern every difference here:

1. **Divergence is loud, never silent.** Where Ruby code compiles in Portland, it means the same thing. Where the semantics differ, the code fails to compile with an explanation and a suggested rewrite. No spelling is reused with quietly different behavior.
1. **The polyfill test.** A future gem + linter/autocorrector should be able to teach Portland idioms inside valid Ruby before a codebase flips. Differences are graded by tier: _free_ (already valid Ruby, same meaning), _gem-supplied_ (valid Ruby grammar, gem adds the method or lint), _locked-until-flip_ (new grammar Ruby can't parse).

## The big picture

- **Kept — the surface.** Blocks as prose (`.map`/`.each`/`yield`), everything-is-an-expression, implicit returns, `?`/`!` method suffixes, postfix guards, keyword arguments, Enumerable as one protocol, pattern matching (promoted to load-bearing). ~90% of Ruby's felt joy is this surface, and it survives static compilation.
- **Cut — the runtime.** Monkeypatching, `method_missing`, runtime `define_method`, `eval`, globals, truthiness, ambient nil, the GIL. The cut-list and the "blocks static safety and speed" list are nearly the same set.
- **Changed — the ground it stands on.** Portland is ahead-of-time compiled (MLIR/LLVM), statically typed with inferred-not-written types, and runs only on Apple silicon (macOS 26+). Ruby is a portable interpreted VM; Portland is a native binary for one vendor's hardware, on purpose.

`../adr/` records the decisions behind these; [`../language.md`](../language.md) records what Portland speaks today. Where a difference is **Tentative** or merely **Sketched**, its file says so — check the Status line at its top before relying on it.

## Every difference

Each line is that file's own one-line summary, so the two cannot drift. Adding a ledger file means writing the file — `script/docs/generate` picks it up and `script/docs/check generated` fails until you run it.

<!-- generated: do not edit by hand — script/docs/generate -->

- [Bitwise operators](bitwise-operators.md) — Out of the grammar; named methods instead.
- [Concurrency](concurrency.md) — No GIL, no `Thread` — one model baked in, spelled `together` / `meanwhile` / `~`.
- [Enums](enums.md) — Ruby has none; Portland's are closed vocabularies of symbol cases, with keyword payloads and checked exhaustiveness.
- [Heredocs](heredocs.md) — Squiggly `<<~` only, SCREAMING_CAPS terminators; `<<` stays the append operator.
- [Lookups and `fetch`](lookups.md) — Partial operations return maybes; the only crash is one you typed; `fetch` retires.
- [Metaprogramming](metaprogramming.md) — The runtime kind is gone; compile-time macros replace it.
- [Sharing behavior: traits, not modules or superclasses](mixins-and-inheritance.md) — `include`/`extend`/`prepend` and subclassing are gone; a struct `carries` traits — method bundles with no state, resolved after the struct's own methods, collisions refused by name.
- [Mutability](mutability.md) — Immutable by default; `mutable` marks the exception, and it governs names rather than values.
- [Namespaces and modules](namespaces.md) — `module` is namespace-only; `::` names and `.` invokes; names are always fully qualified.
- [nil and optionals](nil-and-optionals.md) — No ambient nil; absence is an explicit maybe; the word is still `nil`.
- [Parentheses and ambiguity](parentheses.md) — Command calls, the no-shadow rule, and never-guess errors instead of whitespace heuristics.
- [Pattern matching](pattern-matching.md) — `case/in` is compile-checked exhaustive, `===` is static, and captures are fenced.
- [Ranges](ranges.md) — Slices are collections, never maybes; range patterns prove exhaustiveness; endless ranges close loudly.
- [Removed syntax](removed-syntax.md) — `for`, globals, perlisms, and the other deliberate deletions — plus what is merely deferred.
- [Symbols](symbols.md) — `:foo` survives with its spelling intact, but the set can be declared and checked; no `to_sym`, and hash shorthand is the only symbol-key form.
- [Truthiness](truthiness.md) — Conditions take booleans, and nothing else.
- [Types](types.md) — Static and inferred, written only at public boundaries; duck typing becomes structural.
- [Word operators: `or`, `and`, `not`](word-operators.md) — `or`/`||`, `and`/`&&`, `not`/`!` are dead-identical, and `or` is typed.

<!-- /generated -->
