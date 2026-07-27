# `inspect` is a function, not a method

**Summary:** the rendering is Ruby's, the spelling is `inspect(value)` — a function, because Portland's nil has no methods and absence is inspect's most important input.

**Status:** decided ([ADR 0026](../adr/0026-2026-07-27-inspect-as-a-function.md)). Built in the seed and the trio, differentially pinned.

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

## Migrating

`value.inspect` becomes `inspect(value)` — mechanical, and the output is byte-for-byte what Ruby prints for the shapes both languages share. `p` is unchanged from Ruby: it prints the inspect rendering and returns its argument.
