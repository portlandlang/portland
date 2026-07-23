# Lookups and `fetch`

**Status:** decided
([ADR 0010](../adr/0010-2026-07-22-partial-operations-return-maybes.md)).
Built in the seed (runtime half), 2026-07-22 — see `../STAGE0.md`.

## Ruby

Partial operations — lookups and aggregates that can't always answer —
return nil: `[].first`, `array[99]`, `hash[missing]`, `[].max`. Except
when they raise instead: `hash.fetch(key)`, and `fetch`'s default-value
arity has an eager-evaluation gotcha that forced a third, block arity
into existence.

## Portland

One sentence: **partial operations return maybes; the only crash is one
you typed.**

```ruby
scores.first or return               # might be empty? say what happens
config["theme"] or "teal"            # missing key? default — lazy, always
row = grid[index] or panic "row #{index} must exist"   # certain? assert it
```

The language never panics implicitly. `or panic "why"` is the sole crash
spelling (ADR 0007/0008), so `grep panic` lists every crash site in a
codebase. Negative indices survive, made safe by the maybe
(`array[-99]` is nil, not a crash).

`fetch` retires — all three arities are the or-guard:

```ruby
h.fetch(:key)                       # ⇒ h[:key] or panic 'key not found: :key'
h.fetch(:key, :default)             # ⇒ h[:key] or :default
h.fetch(:key) { |key| load(key) }   # ⇒ h[:key] or load(:key)
```

The middle rewrite is an upgrade, not a trade: `or`'s right side only
runs on absence, so the lazy behavior Ruby reserves for the block arity
is the only behavior. And `fetch`'s stored-nil rule survives exactly —
a stored nil is `some(nil)`, which the or-guard treats as present
(ADR 0005's wrapper earning its keep).

## Migration

- `hash[key] || default`, `array.first || default` — compile verbatim,
  same meaning.
- Bare `array[i]` / `hash[k]` used as if always-present — loud compile
  error (unhandled maybe), fixed with the unwrap toolkit.
- `fetch` — mechanical linter rewrite per the table above, marked
  **unsafe autocorrect** (RuboCop's `-A` tier): pre-flip Ruby is
  collapsed and can't distinguish a stored nil from a missing key, so
  hashes that store nil/false diverge until the flip.
