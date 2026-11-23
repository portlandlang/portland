# 0091 — A match's slices are collections, and brackets take two values on a match

<!-- generated: do not edit by hand — script/docs/generate -->

> [!IMPORTANT]
> **A decision record, not a reference.** It says why a ruling was made; a later ruling may have changed it without marking this file. See also: [language.md: The library](../../docs/language.md#the-library) · [docs/ruby/regex.md](../../docs/ruby/regex.md).

<!-- /generated -->

- **Status:** Accepted (ruled by the deciding user on [#214](https://github.com/portlandlang/portland/issues/214): a slice of a match keeps ADR 0019's position, and the two-argument bracket `md[1, 2]` is added; built on both implementations 2026-11-22)
- **Date:** 2026-11-22
- **Issue:** [#214](https://github.com/portlandlang/portland/issues/214)

## Context

[ADR 0086](0086-2026-11-07-regex-is-rubys-and-a-match-is-a-value.md) gave `MatchData` Ruby's reading API, built without the slice forms: `found[start, length]`, `found[range]`, and `values_at` with ranges. Two things stood in the way.

Ruby answers nil for a slice that starts outside the groups (`found[-30..2]`, `found[6, 2]`), and [ADR 0019](0019-2026-07-23-ranges.md) rules that a slice is always a collection, never a maybe. An array's slices follow that ruling, and a match's `to_a` is an array.

The grammar had no two-value index: `array[1, 2]` was a parse refusal, and an array and a string spell the same read `slice(1, 2)`. Ruby's `MatchData` has no `slice`.

## Decision

1. **A slice of a match is a collection** (ADR 0019). `found[range]` and `found[start, length]` answer the array that the same slice of `found.to_a` answers: empty for a start before or past the groups, where Ruby answers nil. A negative length refuses, as an array's `slice` does. One group by number or name is still a maybe, as one element of an array is.
1. **Brackets take two values on a match.** `found[1, 2]` is a start and a length, both integers.
1. **On every other receiver two values between brackets refuse**, in a sentence that names the spelling where one exists: `an Array takes one index between brackets — with two, write slice(1, 2)`. Whether a string's brackets take two values is [#314](https://github.com/portlandlang/portland/issues/314)'s to settle.
1. **`values_at` takes ranges**, each picking its run of groups as an array's `values_at` does: nil past the end, and a range that starts before the groups refuses in Ruby's words (`-6..3 out of range`).

## Consequences

- Both parsers read a second value between brackets as a call of `[]` with both; the receiver decides when it runs.
- The upstream `[start, length]`, range, and `values_at` range rows port, except the rows that expect nil for an out-of-range slice, which are pinned as empty in Portland's own spec.
- Migration: `md[1, 2]`, `md[1..3]`, and `md.values_at(0..2, 4)` port verbatim. Code that tests a match's slice for nil tests it for `empty?` instead.
- Ledger: [regex.md](../ruby/regex.md).
