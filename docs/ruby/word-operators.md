# Word operators: `or`, `and`, `not`

**Summary:** `or`/`||`, `and`/`&&`, `not`/`!` are dead-identical, and `or` is typed.

**Status:** decided ([ADR 0007](../adr/0007-2026-07-22-or-and-not-dead-identical.md)). Built in the seed and the compiler — see [the language](../language.md#absence).

## Ruby

`and`/`or`/`not` exist alongside `&&`/`||`/`!` with _lower precedence than assignment_ — a secret second precedence responsible for a classic bug family (`x = a or b` assigns `a`, not `a or b`).

## Portland

One operator, two spellings, dead-identical — the same rule as `meanwhile`/`~`: `or` ≡ `||`, `and` ≡ `&&`, `not` ≡ `!`. Same precedence, same semantics, everywhere. The footgun was never the words; it was the precedence difference, and that dies.

With no truthiness, `or` gets its meaning from types:

- **Booleans** — logical or, short-circuiting.
- **Maybes** — unwrap-or-else: the value if present, otherwise the right side; the result is a plain unwrapped value. The right side may instead diverge, making the or-guard — and `or panic "why"` is the language's entire force-unwrap story:

```ruby
name = nickname or "friend"                          # default
user = find_user(id) or return                       # bind-or-bail
row  = lookup(key) or panic "row #{key} must exist"  # assert, loudly
```

Two never-guess compile errors where the idiom is genuinely ambiguous — both **decided (ADR 0007) but not yet enforced**: they are static checks awaiting inference ([#9](https://github.com/portlandlang/portland/issues/9); [ADR 0040](../adr/0040-2026-08-19-inference-the-design-core.md) deliberately holds them until narrowing proves itself). Until they land, both cases compute silently, Ruby's way — the honest current state, not the promised one:

- **`Boolean?` on the left.** The one type with two different "no"s (`nil` and `false`). Ruby's reading steamrolls an explicit `false` into the default — the `@enabled ||= true` bug class — and *today Portland's runtime reproduces that outcome*, because a present `false` is indistinguishable from a plain one without types. When the check lands: refusal with unambiguous rewrites (`enabled = true if enabled.nil?` fills only genuine absence), while `or` on a *plain* Boolean stays legal boolean algebra — a split Ruby structurally cannot make.
- **A left side that can never be absent.** The right side is unreachable — dead code, named as such when the same increment lands.

## Migration

- **Compiles verbatim, same meaning:** `nickname || "friend"`, boolean logic, `find_user(id) or return`, `... or raise`-shaped guards.
- **Parses differently, behaves identically:** `x = a or return` — Ruby binds it `(x = a) or return`, Portland `x = (a or return)`; the guard idiom is observably the same. Exotic uses that depended on Ruby's loose precedence are the linter's job to flag.
- **Loud errors, once the static checks land (#9):** `||` on a `Boolean?` or on a never-absent left side. Until then these compute silently with Ruby's outcomes — migrating code hits no wall today and a designed one later, in that order on purpose.
