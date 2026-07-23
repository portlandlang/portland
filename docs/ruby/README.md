# Portland, for Rubyists

Portland keeps Ruby's surface and replaces what's underneath. This folder
is the ledger of differences — one file per difference, each with the Ruby
behavior, the Portland behavior, why, and what happens to migrating code.

Smooth Ruby → Portland migration is a standing design criterion. Two
promises govern every difference here:

1. **Divergence is loud, never silent.** Where Ruby code compiles in
   Portland, it means the same thing. Where the semantics differ, the code
   fails to compile with an explanation and a suggested rewrite. No
   spelling is reused with quietly different behavior.
2. **The polyfill test.** A future gem + linter/autocorrector should be
   able to teach Portland idioms inside valid Ruby before a codebase flips.
   Differences are graded by tier: *free* (already valid Ruby, same
   meaning), *gem-supplied* (valid Ruby grammar, gem adds the method or
   lint), *locked-until-flip* (new grammar Ruby can't parse).

## The big picture

- **Kept — the surface.** Blocks as prose (`.map`/`.each`/`yield`),
  everything-is-an-expression, implicit returns, `?`/`!` method suffixes,
  postfix guards, keyword arguments, Enumerable as one protocol, pattern
  matching (promoted to load-bearing). ~90% of Ruby's felt joy is this
  surface, and it survives static compilation.
- **Cut — the runtime.** Monkeypatching, `method_missing`, runtime
  `define_method`, `eval`, globals, truthiness, ambient nil, the GIL.
  The cut-list and the "blocks static safety and speed" list are nearly
  the same set.
- **Changed — the ground it stands on.** Portland is ahead-of-time
  compiled (MLIR/LLVM), statically typed with inferred-not-written types,
  and runs only on Apple silicon (macOS 26+). Ruby is a portable
  interpreted VM; Portland is a native binary for one vendor's hardware,
  on purpose.

## The differences

- [nil and optionals](nil-and-optionals.md) — no ambient nil; absence is
  an explicit maybe; the word is still `nil`
- [truthiness](truthiness.md) — conditions take booleans, nothing else
- [lookups and `fetch`](lookups.md) — partial operations return maybes;
  the only crash is one you typed; `fetch` retires
- [word operators](word-operators.md) — `or`/`||`, `and`/`&&`, `not`/`!`
  are dead-identical; `or` is typed
- [mutability](mutability.md) — immutable by default; `mutable` marks the
  exception
- [metaprogramming](metaprogramming.md) — the runtime kind is gone;
  compile-time macros replace it
- [types](types.md) — static and inferred; duck typing becomes structural
- [concurrency](concurrency.md) — no GIL, no `Thread`; `together` /
  `meanwhile` / `~`
- [parentheses and ambiguity](parentheses.md) — command calls, the
  no-shadow rule, never-guess errors
- [bitwise operators](bitwise-operators.md) — out of the grammar; named
  methods instead
- [removed syntax](removed-syntax.md) — `for`, globals, perlisms, and the
  other deliberate deletions

`../adr/` records the decisions behind these; `../STAGE0.md` records what
the seed actually speaks today. Where a difference is **Tentative** or
merely **Sketched**, its file says so.
