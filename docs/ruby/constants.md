# Constants

**Summary:** `SCREAMING_CASE = value` at the top level is a constant — bound once, computed once, readable from every def and every requiring file; reassigning it refuses instead of warning.

**Status:** decided ([ADR 0053](../adr/0053-2026-09-24-constants.md), 2026-09-24), built in both oracles the same day; module constants were already [ADR 0021](../adr/0021-2026-07-24-namespaces-and-modules.md)'s.

## Ruby

A name starting with a capital is a constant by convention only. Reassigning one warns and proceeds; the object it names mutates freely; `Module.const_get`, `const_set`, and `remove_const` reach them at runtime; and a method cannot assign one at all (`dynamic constant assignment`). `SOME_SIGNIFICANT_NUMBER = 8_675_309` is how every Ruby file names a fixed value, and every migrating file carries them.

## Portland

The spelling is Ruby's, and the guarantee is real:

```ruby
SOME_SIGNIFICANT_NUMBER = 8_675_309
LABELS = %w[low high]

def scaled(bar) = bar * SOME_SIGNIFICANT_NUMBER   # a def reads it

struct Gauge
  level
  def high? = level > SOME_SIGNIFICANT_NUMBER      # so does a struct method
end

module Config
  LIMIT = 3                                        # Config::LIMIT, as before
end
```

The shape is two characters or more, a capital first, then capitals, digits, and underscores. `CamelCase` is still a type's shape; a lone capital is neither and refuses. The value is computed once, when the line runs, and crosses a `require_relative` with the file's defs (a required file's *locals* do not — [#95](https://github.com/portlandlang/portland/issues/95)).

What refuses, each on both oracles and at build with its line:

| Ruby | Portland says |
|---|---|
| `LIMIT = 10` a second time | `'LIMIT' is a constant — it is bound once` |
| `mutable LIMIT = 9` | `'LIMIT' is a constant — a constant cannot be mutable` |
| `X = 1` at the top level | `'X' is a single capital — spell a constant with two characters or more, a local in lowercase` |
| `def LIMIT` beside `LIMIT = 1` | `'LIMIT' is already defined — rename one` ([redefinition.md](redefinition.md)) |

## Migration

- A `SCREAMING_CASE` constant migrates verbatim, and gains the guarantee Ruby never gave it.
- Code that reassigns a constant refuses at build where Ruby warned at runtime; the rewrite is a `mutable` local with a lowercase name, if it was really a variable.
- `const_get` and friends are the runtime metaprogramming Portland dropped ([metaprogramming.md](metaprogramming.md)).
- A one-letter constant (`N = 10`) is spelled longer here.
