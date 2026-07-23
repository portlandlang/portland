# Metaprogramming

**Status:** the cut is locked by the design brief; compile-time macros are
planned, undesigned ([#14](https://github.com/portlandlang/portland/issues/14)).

## Ruby

The runtime is open: monkeypatch any class, `method_missing`, runtime
`define_method`, `eval`, `instance_variable_get`, reopening core classes
from any gem. Magical, and the biggest source of un-debuggable
action-at-a-distance at scale — plus the single largest blocker to static
compilation and speed.

## Portland

**Runtime metaprogramming is gone — the whole family.** No monkeypatching,
no open classes, no `method_missing`, no runtime `define_method`, no
`eval`. What you read in the source is what runs; nothing rewires the
program while it's running.

The replacement is **compile-time macros**: the metaprogramming joy
(generating boilerplate, DSLs) without the runtime mystery or cost.
Everything a macro does is done, inspectable, and type-checked before the
program exists. Their design is an open project
([#14](https://github.com/portlandlang/portland/issues/14)).

This cut is not incidental — the cut-list and the "what blocks static
safety and speed" list are nearly the same set. Deleting runtime dynamism
is what buys inferred static types, AOT compilation, and fearless
inlining.

## Migration

- Code that *uses* metaprogrammed APIs (a `has_many`-style DSL) can look
  unchanged — the DSL words become macros.
- Code that *does* runtime metaprogramming does not port; it gets
  redesigned (usually into macros or plain explicit code). The linter can
  inventory a codebase's metaprogramming surface ahead of the flip.
- `define_method` in a loop, `const_get` dispatch, `respond_to_missing?`
  tricks: all loud, none silent.
