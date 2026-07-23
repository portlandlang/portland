# Word operators: `or`, `and`, `not`

**Status:** decided ([ADR 0007](../adr/0007-2026-07-22-or-and-not-dead-identical.md)).
Not yet implemented.

## Ruby

`and`/`or`/`not` exist alongside `&&`/`||`/`!` with *lower precedence than
assignment* — a secret second precedence responsible for a classic bug
family (`x = a or b` assigns `a`, not `a or b`).

## Portland

One operator, two spellings, dead-identical — the same rule as
`meanwhile`/`~`: `or` ≡ `||`, `and` ≡ `&&`, `not` ≡ `!`. Same precedence,
same semantics, everywhere. The footgun was never the words; it was the
precedence difference, and that dies.

With no truthiness, `or` gets its meaning from types:

- **Booleans** — logical or, short-circuiting.
- **Maybes** — unwrap-or-else: the value if present, otherwise the right
  side; the result is a plain unwrapped value. The right side may instead
  diverge, making the or-guard — and `or panic "why"` is the language's
  entire force-unwrap story:

```ruby
name = nickname or "friend"                        # default
user = find_user(id) or return                     # bind-or-bail
row  = lookup(key) or panic "row #{key} must exist"  # assert, loudly
```

Two never-guess compile errors where the idiom is genuinely ambiguous:

- **`Boolean?` on the left.** The one type with two different "no"s
  (`nil` and `false`). Ruby's reading steamrolls an explicit `false` into
  the default — the `@enabled ||= true` bug class. Portland refuses and
  offers unambiguous spellings (`if x.nil? ...` / `x == true`).
- **A left side that can never be absent.** The right side is unreachable
  — dead code, named as such.

## Migration

- **Compiles verbatim, same meaning:** `nickname || "friend"`, boolean
  logic, `find_user(id) or return`, `... or raise`-shaped guards.
- **Parses differently, behaves identically:** `x = a or return` — Ruby
  binds it `(x = a) or return`, Portland `x = (a or return)`; the guard
  idiom is observably the same. Exotic uses that depended on Ruby's loose
  precedence are the linter's job to flag.
- **Loud errors:** `||` on a `Boolean?` or on a never-absent left side.
