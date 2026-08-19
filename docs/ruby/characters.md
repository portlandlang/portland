# Characters

**Summary:** A character is a grapheme cluster and string equality is canonical — `"🇺🇸".length` is 1 and composed `é` equals decomposed `é`, where Ruby counts scalars and compares bytes.

**Status:** decided ([ADR 0038](../adr/0038-2026-08-19-what-a-character-is.md)), built in both oracles. Canonical *search* (`include?`, `index` finding across encodings) is a recorded gap in the ADR, not yet built.

## Ruby

`String#length` counts Unicode scalars (code points): `"🇺🇸".length` is 2, and a decomposed `é` (e + combining accent) counts 2 where the composed form counts 1. `==` compares bytes, so two strings that render identically — a macOS filename, which arrives decomposed, and the same name typed composed in source — are silently unequal. Graphemes exist behind a separate method (`grapheme_clusters`), casing has been full-Unicode since 2.4.

## Portland

**A character is what a human sees** — an extended grapheme cluster. `length`, `chars`, `reverse`, `slice`, and `index` positions all count them: one flag is one character, `é` is one character however encoded, and `reverse` cannot shred an emoji. This is Swift's and Elixir's model, chosen over Ruby's for the same reason both of them chose it: the scalar answer is one no human ever wanted, and on Apple platforms specifically, disagreeing with Swift about what a character is buys a bug class at every OS boundary.

**Equality is canonical** — `==` asks "same text?", not "same bytes?", so the two spellings of `é` are equal, and equal everywhere a string can hide: array membership, hash keys, `uniq`, pattern-match literals. Storage is never normalized; a file round-trip is byte-faithful.

**Casing is Ruby's own** — full Unicode, locale-independent, `"ß".upcase == "SS"` — a non-difference, ratified.

## Migration

- ASCII-only code — the overwhelming majority — behaves identically down to the byte. Both divergences are invisible until a string leaves ASCII.
- Code that counted scalars on purpose (rare, usually encoding-layer work) diverges *silently in values*, not loudly — the one place this ledger's loud-divergence promise bends, because there is no spelling to refuse: `length` is the same method answering the better number. The polyfill linter can flag non-ASCII string literals near `length`/`slice` for review; `codepoints`/`bytes` arrive under their own names when pulled for.
- The macOS-filename bug class (`filename == typed_name` failing on decomposed input) is fixed rather than migrated: code that worked around it keeps working, code that never knew about it stops being wrong.
- Canonical search is not yet built: `include?`/`index` still match bytes, so a composed needle misses a decomposed haystack even though the two are `==`. The gap is recorded in the ADR and can only close in the forgiving direction (misses become finds).
