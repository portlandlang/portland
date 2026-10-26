# Conversions: `to_i`, `to_f`, `to_s`

**Summary:** `to_i` and `to_f` are Ruby's lenient readers — leading digits count, garbage is 0 — and `to_s` is universal except on nil; the strict `Integer()` and `Float()` wait for a pull.

**Status:** decided ([ADR 0061](../adr/0061-2026-10-25-to-i-by-rubys-rule.md), 2026-10-25, out of [#162](https://github.com/portlandlang/portland/issues/162); `to_f` brought to Ruby's rule under [#106](https://github.com/portlandlang/portland/issues/106)). Built in the seed and the compiler, differentially pinned.

## Ruby

`"12abc".to_i` is 12, `"abc".to_i` is 0, `"1_000".to_i` is 1000, `"0x1f".to_i(16)` is 31, `"0b101".to_i(0)` is 5. `to_f` reads the same way for decimals. `Integer("abc")` and `Float("abc")` raise instead. `nil.to_i` is 0 and `nil.to_s` is `""`.

## Portland

- **`to_i` is Ruby's, with `to_i(base)`**: whitespace, one sign, the base's prefix, digits with single underscores between them, and 0 when nothing parses. A base outside 2 to 36 refuses as `invalid radix 37`; base 0 reads the prefix. A number past 64 bits refuses — integers are 64-bit here ([#156](https://github.com/portlandlang/portland/issues/156)).
- **`to_f` is Ruby's**: the longest leading decimal, 0.0 for none, `"-0"` keeping its sign; no base prefixes, no `Infinity`.
- **`hex` and `oct`** read the same way at their bases.
- **`to_s` is universal** except on nil, which has no methods ([nil and optionals](nil-and-optionals.md)); a collection's `to_s` is its `inspect` ([ADR 0060](../adr/0060-2026-10-25-a-collections-to-s-is-its-inspect.md)).
- **`Integer(text)` and `Float(text)`**, the strict forms, do not exist yet; they arrive as type functions ([ADR 0058](../adr/0058-2026-10-16-builtin-types-carry-type-functions.md)) when a port pulls for them.

## Formatting: `format`, not `%`

Ruby's format mini-language — `%05.2f`, `%-10s`, `%<name>d` — is kept whole, as the function `format(template, values...)` with `sprintf` as its twin ([ADR 0067](../adr/0067-2026-10-25-format-is-a-function-not-the-percent-operator.md), decided 2026-10-25, not yet built). `%` stays arithmetic: `"%d" % x` will refuse naming the rewrite, `format("%d", x)`, so one operator never means remainder on a number and formatting on a string.

## Migrating

- `params[:page].to_i`, `"3 apples".to_i`, `line.split(",").map { it.to_i }` — port verbatim, same answers.
- `format(...)` and `sprintf(...)` — port verbatim once built; `template % values` becomes `format(template, *values)`, a mechanical rewrite, and the `%<name>d` hash form becomes keyword arguments.
- `Integer(input)` for validation — today, read with `to_i` and check the text yourself, or wait for the type function; `nil.to_i` becomes `value or 0`.
