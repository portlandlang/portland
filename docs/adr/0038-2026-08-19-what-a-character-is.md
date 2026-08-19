# 0038 — What a character is: graphemes, canonical equality, Unicode casing

- **Status:** Accepted (built in both oracles, 2026-08-19)
- **Date:** 2026-08-19
- **Issue:** [#75](https://github.com/portlandlang/portland/issues/75)

## Context

The ruby/spec import parked four pendings it could not answer, because their expected values were unknowable until someone decided what a character is: `"café".length` (composed and decomposed), `"🇺🇸".length`, composed-versus-decomposed equality, and `"café".upcase`. What both oracles did in the meantime — scalar counting, byte equality — was recorded as *fact, not decision*: an accident of the Rust floor.

Three models were on the table for `length`: bytes (systems-honest, human-wrong), Unicode scalars (Ruby's and Python's answer, and the accidental status quo), and grapheme clusters (Swift's and Elixir's — what a human sees).

## Decision

**1. A character is an extended grapheme cluster.** `length`, `size`, `chars`, `reverse`, `slice`, `[range]`, `index` positions, and `count`'s single-character rule all speak graphemes: `"🇺🇸".length` is 1, `é` is one character however it is encoded, and `reverse` can never shred a flag into its regional indicators. This diverges **loudly** from Ruby, whose `length` counts scalars — and diverges in exactly the corner where Ruby's answer is the one no human ever wanted. Principle 1 outranks principle 2, and the platform agrees: an Apple-native language that disagreed with Swift about what a character is would invite a bug class at every OS boundary. Scalars and bytes are not banished, only unnamed — they arrive later under honest names (`codepoints`, `bytes`) when something pulls.

**2. String equality is canonical equivalence.** `==` compares the *text*, not the bytes: composed `é` (U+00E9) equals decomposed `é` (U+0065 U+0301), via NFC-normalized comparison with byte-equal and all-ASCII fast paths. **Storage is never normalized** — what was written is what is stored, and a `read_file`/`write_file` round-trip is byte-faithful; only the comparison normalizes. This is Swift's model, and it is the required completion of §1: without it, two strings could render identically, measure identically, and mysteriously differ — the exact invisible-bug class this ADR exists to kill (macOS filenames arrive decomposed; source literals are usually composed). Equality is canonical *everywhere a string can hide* — array membership, hash keys, `uniq`, pattern-match literals, enum payloads — because the seed's `Value` equality recurses, and the compiler rides the host's `==`, so parity is structural.

**3. Casing is full Unicode, locale-independent — ratified, not changed.** `"café".upcase` is `"CAFÉ"` and `"ß".upcase` is `"SS"`, which is what Ruby (since 2.4) and the Rust floor both already did; the pendings existed for want of a ruling, not a build. Pinned explicitly: the `ß` expansion means casing can change a string's `length`. Locale-aware casing (Turkish `İ`, Lithuanian accents) waits for a pull and would arrive as an explicit variant, never a silent behavior change.

**4. The recorded gap: search is not yet canonical.** `include?`, `index`, `start_with?`, `end_with?`, and `split` match bytes, so a composed needle does not yet find a decomposed haystack even though the two are `==`. Recorded rather than silent (principle 4): fixing it means position arithmetic across normalization forms — Swift's opaque-index problem — and that design rides with inference-era work rather than being improvised here. Nothing pinned today contradicts the future fix; canonical search can only turn misses into finds.

## Consequences

- The Rust floor gains its first two Unicode dependencies — `unicode-segmentation` and `unicode-normalization`, the reference implementations (principle 10) — and the seed's `Value` equality is hand-written for one arm's sake.
- The four parked pendings become examples, and **the compatibility-hole inventory reaches zero**: `script/spec | grep PENDING` is empty for the first time since the harness learned the word.
- The compiler needed *no changes*: every string builtin and `==` ride the host, so grapheme semantics and canonical equality arrived hosted the moment the seed built.
- Ledger: [characters.md](../ruby/characters.md) carries both divergences and the migration story; [string-count.md](../ruby/string-count.md)'s "single character" now means one grapheme.
- `spec/string/unicode_spec.pdx` contains invisible ink on purpose — composed and decomposed `é` render identically in every editor — and its header says so.
