# 0084 — `Point = Data.define(:x, :y)` is a spelling of `struct Point`

- **Status:** Accepted (ruled by the deciding user 2026-11-03, accepting the recommendation in [#125](https://github.com/portlandlang/portland/issues/125); not built — the build is [#197](https://github.com/portlandlang/portland/issues/197))
- **Date:** 2026-11-03
- **Issue:** [#125](https://github.com/portlandlang/portland/issues/125)

## Context

Ruby 3.2's `Data` is an immutable value object with keyword construction, `with`, value equality, `to_h`, and `deconstruct_keys`: nearly Portland's struct exactly. ruby/spec `core/data/` has 13 files and 89 examples. `Data.define(:x, :y)` builds a class at runtime, which the closed runtime does not do; but the one place it is written in real programs is a constant's definition at the top level, `Point = Data.define(:x, :y)`, sometimes with a block of methods — a declaration wearing a call's clothes.

## Decision

1. **`Name = Data.define(:a, :b)` at a file's or a module's top level declares `struct Name` with fields `a` and `b`**, the same many-names-one-behavior move as `class` ([ADR 0056](0056-2026-09-27-class-is-a-spelling-of-struct.md)) and `attr_reader` ([ADR 0075](0075-2026-11-03-attr-reader-is-a-spelling-of-the-field-list.md)). The field names are symbol literals; anything else refuses.
1. **A `do … end` block on the call is the struct body**: `Point = Data.define(:x, :y) do; def distance … end; end` declares the method on the struct.
1. **`Data.define` anywhere else refuses** as the metaprogramming it is ([metaprogramming.md](../ruby/metaprogramming.md)), naming the top-level spelling: inside a method, assigned to a local, passed as an argument, or with computed field names.
1. **Construction is the struct's**: `Point.new(x: 1, y: 2)` is keyword construction already; Ruby's positional `Point.new(1, 2)` and `Point[1, 2]` refuse naming the keywords, since a struct's fields are names only ([ADR 0062](0062-2026-10-25-struct-fields-are-names-only.md)). `with`, `==`, `eql?`, `to_h`, `inspect`, and `deconstruct_keys`-as-a-pattern are the struct's own.
1. **`inspect` is Portland's struct rendering**, `Point(x: 1, y: 2)`, not Ruby's `#<data Point x=1, y=2>`: a struct declared two ways is one struct, and it renders one way.

## Consequences

- Both parsers recognize the constant-assigned `Data.define` form at top level and in a module body as a struct declaration; nothing new exists at runtime.
- `core/data/` ports as `core/struct` did ([#114](https://github.com/portlandlang/portland/issues/114)): each fixture's `Data.define` ports verbatim now, and the examples about construction, `with`, equality, `to_h`, `inspect`, and patterns port; the rows about `Data.define`'s runtime behavior, positional construction, and Ruby's `inspect` text are `partial_intentional`.
- Ledger: [classes.md](../ruby/classes.md).
