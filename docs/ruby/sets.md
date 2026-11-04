# Sets

**Summary:** `Set` is a builtin immutable value, as it is core in Ruby 4 — `Set.new`, `Set[...]`, and `to_set` build one, `add` and `delete` answer a new one, and the query and algebra surface is Ruby's.

**Status:** decided ([ADR 0082](../adr/0082-2026-11-03-set-is-a-builtin-value.md), 2026-11-03, out of [#123](https://github.com/portlandlang/portland/issues/123)); not built — the build is [#195](https://github.com/portlandlang/portland/issues/195). `core/set/` and `core/enumerable/to_set_spec.rb` port under it.

## Ruby

`Set` was a standard library class for twenty years and became core in Ruby 4, so `Set.new([1, 2])` needs no `require`. It is mutable: `add`, `<<`, `delete`, `merge` change the receiver; `|`, `&`, `-`, `^` answer new sets. `Set[1, 2, 3]` is the inspect form. Insertion order is kept. `SortedSet` left Ruby in 3.0.

## Portland

<!-- not-portland: decided, not built (ADR 0082); Set[...] and | on a set do not parse yet -->

```ruby
seen = Set.new(["a", "b"])
seen.include?("a")               # true
seen.add("c")                    # Set["a", "b", "c"] — a new set
seen | Set["b", "d"]             # Set["a", "b", "d"]
["x", "x", "y"].to_set           # Set["x", "y"]
```

- **A value.** Two sets with the same members are `==` and `eql?` ([equality.md](equality.md)); nothing changes a set in place ([mutability.md](mutability.md)).
- **No literal.** `Set.new(array)`, `Set[1, 2, 3]`, and `to_set` on a collection build one.
- **`add` and `delete` answer a new set.** `<<`, `add?`, `delete?`, `merge`, `subtract`, and the other mutators refuse naming the non-bang spelling, as `push` does on an array, until [#103](https://github.com/portlandlang/portland/issues/103) decides what mutable state is.
- **Ruby's surface otherwise**: `include?`, `member?`, `===`, `size`, `empty?`, `|`, `&`, `-`, `^` and their word names, `subset?`, `superset?`, `proper_subset?`, `proper_superset?`, `disjoint?`, `intersect?`, `each`, and the Enumerable methods answering as on an array; `inspect` is `Set[1, 2, 3]`; insertion order kept.
- **`SortedSet` stays gone.**

## Migration

- Reading a set — membership, algebra, iteration — ports verbatim. _Free._
- `set << x` and `set.add(x)` as a statement become `set = set.add(x)`, as every collection mutation does here. _Mechanical, loud._
- `require "set"` is a line to delete; it is core on both sides now ([requires.md](requires.md)).
