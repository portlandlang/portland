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

- [Aliases](aliases.md) — The famous twins ship with Ruby's meanings; the long tail of Ruby's remaining aliases refuses by naming the surviving spelling.
- [Bitwise operators](bitwise-operators.md) — Out of the grammar; named methods instead.
- [Characters](characters.md) — A character is a grapheme cluster and string equality is canonical — `"🇺🇸".length` is 1 and composed `é` equals decomposed `é`, where Ruby counts scalars and compares bytes.
- [`class`: four jobs, four homes](classes.md) — the keyword is gone — declined, not deferred — and its four jobs re-homed: data-with-behavior is a `struct`, shared behavior is a `trait`, construction logic is `def self.` on the type with `new` definable, and stateful objects wait on the concurrency story.
- [Concurrency](concurrency.md) — No GIL, no `Thread` — one model baked in, spelled `together` / `meanwhile` / `~`.
- [Enumerators](enumerators.md) — There are none; a method Ruby answers with a lazy enumerator answers the finished collection here, and `.to_a` on a collection is the identity.
- [Enums](enums.md) — Ruby has none; Portland's are closed vocabularies of symbol cases, with keyword payloads and checked exhaustiveness.
- [Error handling: results, not raises](errors.md) — `begin/rescue/raise` do not exist; a fallible operation returns its value or a failure, and the unwrap toolkit you already use for absence handles it — failure is absence with a reason.
- [Heredocs](heredocs.md) — Squiggly `<<~` only, SCREAMING_CAPS terminators; `<<` stays the append operator.
- [`inspect` is a function, not a method](inspect.md) — the rendering is Ruby's, the spelling is `inspect(value)` — a function, because Portland's nil has no methods and absence is inspect's most important input.
- [Lookups and `fetch`](lookups.md) — Partial operations return maybes; the only crash is one you typed; `fetch` retires.
- [Metaprogramming](metaprogramming.md) — The runtime kind is gone; compile-time macros replace it.
- [Sharing behavior: traits, not modules or superclasses](mixins-and-inheritance.md) — `include` survives — but it takes a `trait`, a stateless method bundle, never a module or a superclass; collisions are refused by name, and `extend`/`prepend`/subclassing are gone.
- [Mutability](mutability.md) — Immutable by default; `mutable` marks the exception, and it governs names rather than values.
- [Namespaces and modules](namespaces.md) — `module` is namespace-only; `::` names and `.` invokes; names are always fully qualified.
- [nil and optionals](nil-and-optionals.md) — No ambient nil; absence is an explicit maybe; the word is still `nil`.
- [Parentheses and ambiguity](parentheses.md) — Command calls, the no-shadow rule, and never-guess errors instead of whitespace heuristics.
- [Pattern matching](pattern-matching.md) — `case/in` is compile-checked exhaustive, `===` is static, and captures are fenced.
- [Ranges](ranges.md) — Slices are collections, never maybes; range patterns prove exhaustiveness; endless ranges close loudly.
- [Removed syntax](removed-syntax.md) — `for`, globals, perlisms, and the other deliberate deletions — plus what is merely deferred.
- [Static checks: the build refuses what Ruby ships](static-checks.md) — Ruby checks everything at runtime or never; Portland's compiler refuses code that is written wrong, whether or not it would ever run.
- [`String#count` and character sets](string-count.md) — `count` takes a single character; Ruby's multi-character set-count spelling refuses instead of quietly meaning something else.
- [String escapes](string-escapes.md) — The escape set is closed — Ruby passes an unknown escape through as its bare character, and Portland refuses it.
- [Symbols](symbols.md) — `:foo` survives with its spelling intact, but the set can be declared and checked; no `to_sym`, and hash shorthand is the only symbol-key form.
- [Truthiness](truthiness.md) — Conditions take booleans, and nothing else.
- [Types](types.md) — Static and inferred, written only at public boundaries; duck typing becomes structural.
- [Word operators: `or`, `and`, `not`](word-operators.md) — `or`/`||`, `and`/`&&`, `not`/`!` are dead-identical, and `or` is typed.

<!-- /generated -->
