# JSON

**Summary:** `JSON.parse`, `JSON.generate`, and `JSON.pretty_generate` answer what Ruby's json gem answers, byte for byte — `null` is `nil`, keys are strings or (with `symbolize_names: true`) symbols, duplicate keys and bad documents are failures in Ruby's sentences — with two loud differences: a struct does not silently stringify, and a number past 64 bits is a failure rather than a bignum.

**Status:** decided ([ADR 0072](../adr/0072-2026-11-03-json-is-core-written-twice.md), 2026-11-03, out of [#182](https://github.com/portlandlang/portland/issues/182)); not yet built — #182 is the build issue. Core, no `require`; written in Rust in the seed and in Portland on the compiler side.

## Ruby

`require "json"`, then `JSON.parse(text)` answers a hash of string keys (symbols with `symbolize_names: true`), `nil` for `null`, Integer or Float by the number's spelling, a bignum past 64 bits; a bad document raises `JSON::ParserError` with a position (`unexpected character: ']' at line 1 column 4`), a duplicate key raises since json 3.0, nesting past 100 raises `NestingError`. `JSON.generate(value)` and `pretty_generate` write any object — a struct becomes `"#<struct a=1>"` through `to_s` — and `to_json` is defined on everything. `JSON.dump`/`load` are the file-shaped twins with their own nil rules.

## Portland

No `require` — JSON is core ([standard-library.md](standard-library.md)). Three type functions:

- **`JSON.parse(text)`** answers the document's value, or a **failure** ([errors.md](errors.md)) carrying Ruby's sentence: `unexpected end of input at line 1 column 1`, `duplicate key "a" at line 1 column 1`, `nesting of 101 is too deep`, `incomplete surrogate pair at '\ud83d"' at line 1 column 2`. `null` is `nil` — and because a lookup answers a maybe ([nil-and-optionals.md](nil-and-optionals.md)), `document["a"]` is `some(nil)` for a present null and `nil` for an absent key, which Ruby cannot tell apart. Keys are strings; `symbolize_names: true` makes them symbols ([symbols.md](symbols.md), a symbol of no declared vocabulary). Numbers split on Ruby's rule; a number past 64 bits is a failure ([integers.md](integers.md)), not a bignum. Top-level scalars parse. Handing `parse` a non-string refuses: that is the program's mistake, not the document's.
- **`JSON.generate(value)`** and **`JSON.pretty_generate(value)`** answer Ruby's text for the JSON-shaped values — nil, booleans, numbers, strings, arrays, hashes, symbols as strings — including the json gem's float spellings (`1e+20`, `0.00000015`, `100.0`) and escapes (`\u0001`, `/` bare). `NaN` and `Infinity` are failures (`NaN not allowed in JSON`). **A struct, range, or enum case refuses**, naming `to_h`: Ruby writes `"#<struct a=1>"`, which no reader wanted.

## Migration

- `JSON.parse(text)`, `JSON.parse(text, symbolize_names: true)`, `JSON.generate(value)`, `JSON.pretty_generate(value)` — port verbatim, same text — _free_. Drop the `require "json"` ([#116](https://github.com/portlandlang/portland/issues/116) says what it does meanwhile).
- `rescue JSON::ParserError` — becomes `or` / `case/in` on the failure ([errors.md](errors.md)): `document = JSON.parse(text) or return failure(:bad_json)`.
- `value.to_json` — not yet; write `JSON.generate(value)`. `JSON.dump`/`load`, `object_class:`, `max_nesting:` — not yet, each its own pull.
- A struct passed to `generate` — refuses; give it a `to_h` and generate that. A bignum in a document — a failure; the document needs a string there.
