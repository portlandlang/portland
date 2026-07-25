# Symbols

**Summary:** `:foo` survives with its spelling intact, but the set can be declared and checked; no `to_sym`, and hash shorthand is the only symbol-key form.

**Status:** decided ([ADR 0023](../adr/0023-2026-07-25-symbols.md)). Built in the seed; the trio waits — see [the language](../language.md#symbols-and-enums).

## Ruby

A symbol is an interned name, and the interning is the interpreter's own identifier table exposed as values. A bare Ruby 4.0.6 has **3,599 symbols interned before any user code runs** — its method, ivar, and constant names — which is why symbols and metaprogramming are inseparable there: `send(:foo)` works because `:foo` *is* the key in the method table.

<!-- not-portland: Ruby, shown for contrast -->

```ruby
status = :pending
status = :pendign          # equally valid; fails later, or never

{name: "pdx"}              # symbol keys, modern form
{:name => "pdx"}           # symbol keys, older form — same thing
{"name" => "pdx"}          # string keys — a different hash

"pend" + "ing"             # and then:
"pending".to_sym           # => :pending, built at runtime from data
[1, 2].map(&:to_s)         # symbol-to-proc
```

## Portland

The spelling is unchanged. What changes is that the **set can be closed**, and that a symbol can no longer be conjured from a string.

```ruby
purchase.with(status: :paid)      # checked against Purchase::Status
purchase.with(status: :pendign)   # compile error — not a case of that enum

config = {name: "pdx", port: 80}  # symbol-keyed, Ruby-verbatim
config[:name]                     # works
config["name"]                    # compile error — a String is not a Symbol
```

Four differences:

- **The set can be declared, and then it is checked.** Ruby's problem with `status = :pending` was never the syntax — it is that the set is open. [Enums](enums.md) close it; everywhere else a symbol is still just a value.
- **No `to_sym`.** `Symbol#to_s` exists; `String#to_sym` does not. A symbol built from a string at runtime cannot be checked against any vocabulary, so cutting it is what makes checking possible rather than best-effort. Every symbol in a Portland program is written literally.
- **One symbol-key spelling.** `{name: "pdx"}` is it. `{:name => "pdx"}` is a compile error naming the rewrite — a second spelling of a sentence the language already says.
- **No operator symbols.** `:+`, `:[]`, `:<=>` existed for `send`, `define_method`, and `&:`, all of which are gone.

Comparison is equality only. `:paid == "paid"` is a compile error rather than Ruby's silent `false`, and symbols are not ordered.

## Migration

- **`:symbol` literals** — compile verbatim, same meaning. Free tier.
- **`{name: "x"}`** — compiles verbatim, and it is currently a *parse error*, so this is a gain rather than a divergence.
- **`{:name => "x"}`** — a compile error naming the shorthand. Free-tier autocorrect: RuboCop's `Style/HashSyntax` already defaults to enforcing the shorthand, so conforming Ruby already complies.
- **`.to_sym`** — no rewrite exists. Code that builds symbol names from data is doing the thing Portland removed; the fix is a declared vocabulary ([enums](enums.md)) or a plain String. **Locked until flip**, and the loudest single break in this file.
- **`&:upcase`** — becomes `{ it.upcase }` (ADR 0017). Free tier once the polyfill teaches it.
- **`send(:name)`, `define_method(:name)`, `respond_to?(:name)`** — gone with runtime metaprogramming, not with symbols; see [metaprogramming](metaprogramming.md).
- **Symbol/String key mixing** — Rails' indifferent access does not port; each site becomes a compile error. Mechanically fixable, but real work for Rails-shaped codebases, and the size of that work is a corpus question still to be run.
