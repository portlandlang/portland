# 0087 — Normalization forms are Ruby's

- **Status:** Accepted (ruled by the deciding user 2026-11-08 on [#106](https://github.com/portlandlang/portland/issues/106)'s status; built on both implementations the same day)
- **Date:** 2026-11-08
- **Issue:** [#106](https://github.com/portlandlang/portland/issues/106)

## Context

Ruby's `String#unicode_normalize(form)` rewrites text into one of Unicode's four normalization forms, `:nfc`, `:nfd`, `:nfkc`, `:nfkd`, and `unicode_normalized?(form)` asks whether text is already in one. `core/string/unicode_normalize_spec.rb` and `unicode_normalized_spec.rb` hold 25 examples.

Portland's string equality is already canonical ([ADR 0038](0038-2026-08-19-what-a-character-is.md)): `==` asks "same text?", so the composed and decomposed spellings of `é` are equal, and storage is never normalized. That made it an open question whether the forms should be exposed at all, since `==` cannot tell them apart. They differ in `bytes` and `codepoints`, which is exactly what a program writing for another system needs to control: a file name a filesystem stores decomposed, a database key compared byte for byte, a hash over the bytes.

## Decision

1. **`unicode_normalize` and `unicode_normalized?` are Ruby's**, with Ruby's behavior: the form defaults to `:nfc`, the four forms are Ruby's symbols, and the answers are Ruby's codepoint for codepoint. `unicode_normalized?` is true when normalizing would change nothing.
1. **An unknown form refuses in Ruby's words**, `Invalid normalization form bogus.`, and so does a form given as a string where Ruby wants a symbol.
1. **Equality stays canonical.** Exposing the forms changes nothing about `==`: two spellings of the same text remain equal, and a normalized string is equal to the one it came from. A spec that checks a form therefore checks its `codepoints`, not `==`.
1. **The bang form is not spelled.** `unicode_normalize!` refuses as every mutator does ([ADR 0015](0015-2026-07-23-values-never-mutate.md)); the rewrite is rebinding.

## Consequences

- Both implementations gain the two methods over the seed's existing Unicode normalization tables, the same ones canonical equality already uses.
- `core/string/unicode_normalize_spec.rb` and `unicode_normalized_spec.rb` port; the non-Unicode-encoding rows stay upstream, every string being UTF-8 ([ADR 0069](0069-2026-11-03-strings-are-utf-8-always.md)).
- Ledger: [characters.md](../ruby/characters.md) gains the forms beside canonical equality.
