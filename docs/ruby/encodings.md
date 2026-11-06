# Encodings and binary data

**Summary:** A string is UTF-8 text, always — no encoding tag, so `encoding` answers `Encoding::UTF_8` and `valid_encoding?` is true on every string, `force_encoding` and `encode` have no meaning, and binary data is a different type that does not exist yet; `pack` and `unpack` wait for it.

**Status:** decided ([ADR 0069](../adr/0069-2026-11-03-strings-are-utf-8-always.md), 2026-11-03, out of [#119](https://github.com/portlandlang/portland/issues/119)); `encoding` and `valid_encoding?` built on both implementations; the bytes type is [#183](https://github.com/portlandlang/portland/issues/183).

## Ruby

A string is a byte sequence with an encoding tag, and the tag decides what the bytes mean: `"é".bytes` is `[195, 169]` either way, but `"é".length` is 1 under UTF-8 and 2 after `"é".dup.force_encoding("BINARY")`. `"a".encoding` is US-ASCII in a US-ASCII source and UTF-8 otherwise; `encode("Shift_JIS")` rewrites the bytes; `valid_encoding?` asks whether the bytes parse under the tag; `scrub` replaces the ones that do not; `b` retags as BINARY. Binary data is a string too — `[1000].pack("S>")` is `"\x03\xE8"` with the BINARY tag, `"\x03\xE8".unpack1("S>")` is 1000 — and `pack`'s template language covers both binary layouts (`C S L N E a Z x w`) and text encodings (`m` base64, `H` hex, `B` bits, `u` uuencode, `U` UTF-8).

## Portland

**One encoding.** Every string is UTF-8 text ([characters.md](characters.md): graphemes, canonical equality). `text.encoding` is `Encoding::UTF_8` — a constant bound to `"UTF-8"`, Ruby's name for it — and `text.valid_encoding?` is true, because no sequence a string can hold is invalid in the only encoding it has. A symbol's text is a string's, so `:foo.encoding` is the same `Encoding::UTF_8` where Ruby reads US-ASCII off an ASCII name ([#112](https://github.com/portlandlang/portland/issues/112)). `force_encoding`, `encode`, `b`, and `scrub` have no Portland meaning: there is no other encoding to force or transcode to, and nothing to scrub.

**Bytes are not strings.** `bytes`, `bytesize`, and `each_byte` answer a string's UTF-8 bytes as integers 0–255, which is also what `System.entropy` answers. A binary value — what `pack` produces and `unpack` reads, what `getbyte` and `byteslice` index, what `encode` would transcode into — is a different type, not built yet, spelled when a real file pulls for it ([#183](https://github.com/portlandlang/portland/issues/183)). Until then `pack`, `unpack`, `unpack1`, `getbyte`, `byteslice`, `byteindex`, `byterindex`, `encode`, `dump`, `undump`, `sum`, and Ruby 4.1's `String#bit_*` do not exist, and their ruby/spec rows stay owed rather than decided out.

## Migration

- `text.encoding == Encoding::UTF_8`, `text.valid_encoding?` — port verbatim, same answers — _free_. Code that branches on other encodings has one branch left.
- `force_encoding`, `encode`, `b`, `scrub` — refuse as undefined; the code they served (reading bytes off the wire, fixing mojibake) is bytes-type work that waits on [#183](https://github.com/portlandlang/portland/issues/183).
- `pack`/`unpack` — refuse as undefined until the bytes type exists; `Base64` and `Digest` callers get their tiles ([ADR 0064](../adr/0064-2026-10-25-the-standard-library-core-default-tiles-external-tiles.md)) rather than the template.
- `# encoding: ...` magic comments — a source file is UTF-8; the comment is noise, and a linter can drop it.
