# 0069 — Strings are UTF-8, always; bytes are a type that waits for its pull

- **Status:** Accepted (ruled by the deciding user 2026-11-03, accepting the recommendation in [#119](https://github.com/portlandlang/portland/issues/119); `encoding` and `valid_encoding?` built on both implementations the same day, `core/encoding/` recorded never-port, the byte-shaped rows waiting on [#183](https://github.com/portlandlang/portland/issues/183))
- **Date:** 2026-11-03
- **Issue:** [#119](https://github.com/portlandlang/portland/issues/119)

## Context

A Ruby string is a byte sequence tagged with one of over a hundred encodings, and the tag changes what every method means: `"é".length` is 1 in UTF-8 and 2 after `force_encoding("BINARY")`; `"a".encoding` is US-ASCII for a literal in a US-ASCII source and UTF-8 otherwise; `valid_encoding?` asks whether the bytes parse under the tag. `Array#pack` answers such a string with the BINARY tag — `[1.5].pack("E").encoding` is `ASCII-8BIT` — and `String#unpack` reads one back. ruby/spec's `core/encoding/*` (320 examples), `core/array/pack/*` (568), `core/string/unpack/*` (614), and `core/string/valid_encoding/*` (28) all rest on that model, and the import ([#23](https://github.com/portlandlang/portland/issues/23)) left them untouched.

[ADR 0038](0038-2026-08-19-what-a-character-is.md) had already made a string text: a character is a grapheme, equality is canonical. A per-string encoding tag is one spelling with many behaviors, chosen at runtime by a value's state — the shape principle 3 keeps out. And the examination of `pack`/`unpack` for this ruling found it is two things in one: a binary layout codec (`C S L N E a Z x w` — the C struct, the TCP header, the PNG chunk) and a grab-bag of text encodings (`m` base64, `H` hex, `B` bits, `u` uuencode, `U` UTF-8), silent where Portland is loud (`[256].pack("C")` is `"\x00"`; `"\x01\x02".unpack("C C C")` is `[1, 2, nil]`). Ruby's own `base64.rb` is six calls to `pack("m")`.

## Decision

1. **A Portland string is UTF-8, always.** There is no encoding tag, so nothing to force, transcode, or fall out of: `force_encoding`, `encode`, `b`, and `scrub` have no Portland meaning, and `core/encoding/` is never-port.
1. **`encoding` answers `Encoding::UTF_8`, and `valid_encoding?` answers true**, on every string — the two rows of Ruby's that survive. `Encoding::UTF_8` is a builtin constant ([ADR 0053](0053-2026-09-24-constants.md)) bound to the string `"UTF-8"`, Ruby's spelling of the encoding's name, so `text.encoding == Encoding::UTF_8` ports verbatim. Ruby answers US-ASCII for an ASCII-only literal; here every string answers the one encoding it has.
1. **Binary data is a different type from text**, spelled when a real file pulls for it (principle 6). Until then `bytes`, `bytesize`, and `each_byte` answer a string's UTF-8 bytes as integers 0–255 — the same shape `System.entropy` answers ([ADR 0068](0068-2026-10-25-system-is-the-floor-module.md)) — and `pack`, `unpack`, `unpack1`, `getbyte`, `byteslice`, `byteindex`, `byterindex`, `encode`, `dump`, `undump`, `sum`, and Ruby 4.1's `String#bit_*` wait on that type in [#183](https://github.com/portlandlang/portland/issues/183). They are not built on strings.
1. **The ruby/spec rows.** `core/encoding/` and `core/string/valid_encoding/` are skipped; `core/string/{b,force_encoding,scrub}_spec.rb` are skipped; `core/string/encoding_spec.rb` and `core/string/valid_encoding_spec.rb` are imported in part, one row each, into `spec/portland/string/encoding_spec.pdx` and `spec/core/string/valid_encoding_spec.pdx`. The byte-shaped rows stay *missing* — owed, in the dashboard's denominator — since nothing can be imported in part against a type that does not exist; #119's recommendation said `partial_planned`, and this is what that honestly is.

## Consequences

- The `base64` default tile ([ADR 0064](0064-2026-10-25-the-standard-library-core-default-tiles-external-tiles.md)) is the likeliest first pull for the bytes type, since its whole Ruby body is `pack("m")`; the `digest` tile would be the second. [#183](https://github.com/portlandlang/portland/issues/183) sketches the shape — a `Bytes` value type, the layout half of `pack`'s template kept (tie goes to Ruby), the text-encoding half given names.
- Ledger: [encodings.md](../ruby/encodings.md); [characters.md](../ruby/characters.md) is the other half of what a string is. [randomness.md](../ruby/randomness.md)'s `Random.bytes` now waits on #183.
- The string method table gains `encoding/0` and `valid_encoding?/0`; the checker types `encoding` as a String.
- The stub count falls by the skipped directories' 350-odd pendings, which were never Portland's to owe.
