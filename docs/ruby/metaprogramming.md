# Metaprogramming

**Summary:** The runtime kind is gone; compile-time macros replace it.

**Status:** the cut is locked by the design brief; compile-time macros are planned, undesigned ([#14](https://github.com/portlandlang/portland/issues/14)).

## Ruby

The runtime is open: monkeypatch any class, `method_missing`, runtime `define_method`, `eval`, `instance_variable_get`, reopening core classes from any gem. Magical, and the biggest source of un-debuggable action-at-a-distance at scale — plus the single largest blocker to static compilation and speed.

## Portland

**Runtime metaprogramming is gone — the whole family.** No monkeypatching, no open classes, no `method_missing`, no runtime `define_method`, no `eval`. What you read in the source is what runs; nothing rewires the program while it's running.

The replacement is **compile-time macros**: the metaprogramming joy (generating boilerplate, DSLs) without the runtime mystery or cost. Everything a macro does is done, inspectable, and type-checked before the program exists. Their design is an open project ([#14](https://github.com/portlandlang/portland/issues/14)).

This cut is not incidental — the cut-list and the "what blocks static safety and speed" list are nearly the same set. Deleting runtime dynamism is what buys inferred static types, AOT compilation, and fearless inlining.

A builtin type is not a class object either — nothing to `allocate` from, no ancestors to ask about, no `instance_method` to compare — but its name carries Ruby's type functions ([ADR 0058](../adr/0058-2026-10-16-builtin-types-carry-type-functions.md)): `Array.new(3) { |index| index * 2 }` and `Integer.sqrt(24)` answer as Ruby's do, and the name on its own still refuses. `Array[1, 2]` and `Hash[[["a", 1]]]` are decided the same way, Ruby's behavior and refusals, and not yet built ([#218](https://github.com/portlandlang/portland/issues/218)); until then they refuse.

The one piece of reflection that stays is the static kind ([ADR 0074](../adr/0074-2026-11-03-class-answers-the-type.md), built 2026-11-03): `value.class` answers the type, which has a `name`, a `new`, its type functions, and identity `==`, and nothing else. `ancestors`, `instance_methods`, `superclass`, `allocate`, `method_defined?`, `instance_variable_get`, and `const_get` refuse by name, since each asks about a runtime that could have changed and this one cannot.

## Migration

- Code that _uses_ metaprogrammed APIs (a `has_many`-style DSL) can look unchanged — the DSL words become macros.
- Code that _does_ runtime metaprogramming does not port; it gets redesigned (usually into macros or plain explicit code). The linter can inventory a codebase's metaprogramming surface ahead of the flip.
- `define_method` in a loop, `const_get` dispatch, `respond_to_missing?` tricks: all loud, none silent.
