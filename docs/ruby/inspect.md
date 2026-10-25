# `inspect` is a function, not a method

**Summary:** the rendering is Ruby's, the spelling is `inspect(value)` — a function, because Portland's nil has no methods and absence is inspect's most important input.

**Status:** decided ([ADR 0026](../adr/0026-2026-07-27-inspect-as-a-function.md)). Built in the seed and the compiler, differentially pinned.

## Ruby

`inspect` is a method on every object, `nil` included:

```ruby
"rose".inspect   # => "\"rose\""
nil.inspect      # => "nil"
```

This works because Ruby's `nil` is a full object — `NilClass` answers `inspect`, `to_s`, `to_a`, and dozens more.

## Portland

The same rendering, spelled as a builtin function beside `p` and `some`:

```ruby
inspect("rose")   # => "\"rose\""
inspect(nil)      # => "nil"
```

## Why

Portland has no ambient nil and absence has no methods (ADR 0006) — `nil?` and `some?` are the one dispatch a maybe allows. A method spelling would force a choice Ruby never faces: refuse `nil.inspect`, crippling the debugging tool on its most important input, or grant nil a third method and chip the wall. A function takes absence as an argument without ceremony, exactly as `some(nil)` already does.

## A collection's `to_s` is its `inspect`

As Ruby's ([ADR 0060](../adr/0060-2026-10-25-a-collections-to-s-is-its-inspect.md)): `[1, "two"].to_s` is `[1, "two"]`, `"#{{a: 1}}"` is `{a: 1}`, and `puts list` prints the same. The quotes and colons are what tell `["1"]` from `[1]` in a log line. `join` renders each element with its own `to_s`, nested arrays flattened into the join, as Ruby's does; the scalars are untouched — `:a.to_s` is `a`.

## Migrating

`value.inspect` becomes `inspect(value)` — mechanical, and the output is byte-for-byte what Ruby prints for the shapes both languages share. `p` is unchanged from Ruby: it prints the inspect rendering and returns its argument. A collection interpolated or passed to `puts` prints as Ruby prints it.
