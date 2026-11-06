# Sets

**Summary:** `Set` is a builtin immutable value, as it is core in Ruby 4 — `Set.new`, `Set[...]`, and `to_set` build one, `add` and `delete` answer a new one, and the query and algebra surface is Ruby's under its word names.

**Status:** decided ([ADR 0082](../adr/0082-2026-11-03-set-is-a-builtin-value.md), 2026-11-03, out of [#123](https://github.com/portlandlang/portland/issues/123)); **built** on both implementations 2026-11-05 under [#195](https://github.com/portlandlang/portland/issues/195), with `core/set/` and `core/enumerable/to_set_spec.rb` in the manifest. One edge is open: `|`, `&`, and `^` are out of the grammar ([ADR 0003](../adr/0003-2026-07-20-bitwise-operators-out.md)), and [#207](https://github.com/portlandlang/portland/issues/207) holds whether a set on the left brings them back.

## Ruby

`Set` was a standard library class for twenty years and became core in Ruby 4, so `Set.new([1, 2])` needs no `require`. It is mutable: `add`, `<<`, `delete`, `merge` change the receiver; `|`, `&`, `-`, `^` answer new sets. `Set[1, 2, 3]` is the inspect form. Insertion order is kept. `SortedSet` left Ruby in 3.0.

## Portland

```ruby
seen = Set.new(["a", "b"])
seen.include?("a")               # true
seen.add("c")                    # Set["a", "b", "c"] — a new set
seen.union(Set["b", "d"])        # Set["a", "b", "d"]
seen + ["d"]                     # Set["a", "b", "d"] — + is union, - is difference
["x", "x", "y"].to_set           # Set["x", "y"]
```

- **A value.** Two sets holding the same members are `==` and `eql?` in any order ([equality.md](equality.md)), so a set is a hash key; members match by `eql?`, so `1` and `1.0` are two members; nothing changes a set in place ([mutability.md](mutability.md)).
- **No literal.** `Set.new(enumerable)` — a Set, an Array, a Range, or a Hash as its pairs — `Set[1, 2, 3]`, and `to_set` on those collections build one; a block maps each member first, as Ruby's does.
- **`add` and `delete` answer a new set.** `<<`, `add?`, `delete?`, `merge`, `subtract`, `clear`, `replace`, `keep_if`, `delete_if`, and the bang names refuse naming the spelling that answers a new set, as `push` does on an array, until [#103](https://github.com/portlandlang/portland/issues/103) decides what mutable state is.
- **The algebra under Ruby's word names**: `union`, `intersection`, `difference`, with `+` and `-` as Ruby's own aliases for the first and the last; each takes any enumerable. `|`, `&`, and `^` are out of the grammar ([ADR 0003](../adr/0003-2026-07-20-bitwise-operators-out.md)); [#207](https://github.com/portlandlang/portland/issues/207) holds whether a set brings them back, and the symmetric difference is `(a + b) - a.intersection(b)` meanwhile.
- **Ruby's surface otherwise**: `include?`, `member?`, and `===` under `when`; `size`, `length`, `empty?`; `subset?`/`<=`, `superset?`/`>=`, `proper_subset?`/`<`, `proper_superset?`/`>`, each taking a set, and `<=>` as 0, -1, 1, or nil by the same relations; `disjoint?` and `intersect?` over any enumerable; `flatten`, `classify`, `join`, `to_a`; and the Enumerable methods answering as on an array of the members — `each` answers the set, `select` and `reject` a set of what they kept, `map` and `sort` arrays. `inspect` and `to_s` are `Set[1, 2, 3]`; insertion order is kept.
- **`SortedSet` stays gone.**

## Migration

- Reading a set — membership, the algebra by its word names, iteration — ports verbatim. _Free._
- `set << x` and `set.add(x)` as a statement become `set = set.add(x)`, as every collection mutation does here. _Mechanical, loud._
- `a | b`, `a & b`, and `a ^ b` become `a.union(b)`, `a.intersection(b)`, and `(a + b) - a.intersection(b)` until [#207](https://github.com/portlandlang/portland/issues/207) decides. _Mechanical, loud._
- `require "set"` is a line to delete; it is core on both sides now ([requires.md](requires.md)).
