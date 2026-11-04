# Symbols

**Summary:** `:foo` survives with its spelling intact, but the set can be declared and checked; `to_sym` exists, and a symbol made from text is refused only where a declared vocabulary is expected; hash shorthand is the only symbol-key form.

**Status:** decided ([ADR 0023](../adr/0023-2026-07-25-symbols.md); its `to_sym` cut superseded by [ADR 0071](../adr/0071-2026-11-03-a-symbol-may-be-made-from-text.md), 2026-11-03). Built in the seed and the compiler, differentially pinned — see [the language](../language.md#symbols-and-enums); `to_sym`, `intern`, and interpolated symbols built 2026-11-03 ([#184](https://github.com/portlandlang/portland/issues/184)). The refusal at a checked position waits on there being one: a struct's fields declare no vocabulary ([ADR 0062](../adr/0062-2026-10-25-struct-fields-are-names-only.md)), so no call site yet expects a particular enum, and the check lands with the first that does ([#200](https://github.com/portlandlang/portland/issues/200)).

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

The spelling is unchanged. What changes is that the **set can be closed**, and that a symbol conjured from a string cannot pass where a closed set is checked.

```ruby
purchase.with(status: :paid)      # checked against Purchase::Status
purchase.with(status: :pendign)   # compile error — not a case of that enum

config = {name: "pdx", port: 80}  # symbol-keyed, Ruby-verbatim
config[:name]                     # works
config["name"]                    # compile error — a String is not a Symbol
```

Four differences:

- **The set can be declared, and then it is checked.** Ruby's problem with `status = :pending` was never the syntax — it is that the set is open. [Enums](enums.md) close it; everywhere else a symbol is still just a value.
- **A symbol from text is a symbol of no vocabulary** ([ADR 0071](../adr/0071-2026-11-03-a-symbol-may-be-made-from-text.md)). `"pending".to_sym`, `text.intern`, and `:"prefix_#{name}"` all work and answer a plain `Symbol`, welcome as a hash key, in a comparison, as an argument. Where a declared vocabulary is expected — `purchase.with(status: name.to_sym)` — the build refuses, naming the gate: match it against the enum with `case/in` first, and inside each arm the value is the literal it matched. ADR 0023 had cut `to_sym` to buy this check at the constructor; the check now sits at the one position it protects.
- **One symbol-key spelling.** `{name: "pdx"}` is it. `{:name => "pdx"}` is a compile error naming the rewrite — a second spelling of a sentence the language already says.
- **A key with no value takes the name's value** ([ADR 0078](../adr/0078-2026-11-03-omitted-hash-values.md), built on both implementations 2026-11-03): `{name:, port:}` is `{name: name, port: port}` and `build(name:, port:)` forwards two keywords, as Ruby 3.1 reads them. The bare name resolves as any bare name does, a local first and then a method, and the no-shadow rule means there is never a question which one it took. Inside braces or parens a newline ends nothing, so `{name:\n}` omits and `{name:\n 2}` takes the next line's value, as Ruby's do; a paren-less call's label at the end of its line, `build name:`, is the one place Ruby reads two ways, and it refuses — `'name:' at the end of a line is ambiguous — write 'name: name', or put the value on this line`.
- **No operator symbols.** `:+`, `:[]`, `:<=>` existed for `send`, `define_method`, and `&:`, all of which are gone.

Comparison is equality only. `:paid == "paid"` is a compile error rather than Ruby's silent `false`, and symbols are not ordered.

## Migration

- **`:symbol` literals** — compile verbatim, same meaning. Free tier.
- **`{name: "x"}`** — compiles verbatim, and it is currently a *parse error*, so this is a gain rather than a divergence.
- **`{:name => "x"}`** — a compile error naming the shorthand. Free-tier autocorrect: RuboCop's `Style/HashSyntax` already defaults to enforcing the shorthand, so conforming Ruby already complies.
- **`.to_sym`, `.intern`, `:"#{...}"`** — port verbatim, same answers — _free_. A quoted symbol takes the string's escapes, so `:"a\nb"` is a name with a newline, as in Ruby. The one new refusal is a runtime symbol handed to a checked position, when there is one; the rewrite is a `case/in` over the vocabulary, which a linter can scaffold. An interpolated symbol in a *pattern*, `in :"#{name}"`, refuses — bind the value to a local and pin it, `in ^name`.
- **`&:upcase`** — becomes `{ it.upcase }` (ADR 0017). Free tier once the polyfill teaches it.
- **`send(:name)`, `define_method(:name)`, `respond_to?(:name)`** — gone with runtime metaprogramming, not with symbols; see [metaprogramming](metaprogramming.md).
- **Symbol/String key mixing** — Rails' indifferent access does not port; each site becomes a compile error. Mechanically fixable, but real work for Rails-shaped codebases, and the size of that work is a corpus question still to be run.
