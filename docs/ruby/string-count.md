# `String#count` and character sets

**Summary:** `count` takes a single character; Ruby's multi-character set-count spelling refuses instead of quietly meaning something else.

**Status:** decided in practice (the [#23](https://github.com/portlandlang/portland/issues/23) import, 2026-08-12); no ADR — the refusal falls straight out of principle 5, and a future substring-count would be a new name, not a new reading of this one.

## Ruby

`String#count` looks like a substring counter and is not one. Its argument is a **character set**: `"banana".count("a")` is 3, but `"banana".count("an")` is 5 — every `a` plus every `n`, not occurrences of `"an"`. Behind that lurk set negation (`"^a"`), ranges (`"a-z"`), and multiple-argument intersection. It is one of Ruby's most reliably misread methods.

## Portland

`count` takes exactly one character and counts its occurrences — the reading both languages agree on:

```ruby
"banana".count("a")    # 3, same as Ruby
```

More than one character is a compile-away refusal, not a guess:

```text
count takes a single character — Ruby's multi-character count is a character set, not a substring, and Portland does not guess
```

Counting occurrences of a substring is a different job and would get a different name if a real file pulls for it; the set semantics (negation, ranges) are declined, not deferred.

## Migration

- Single-character `count` — the overwhelmingly common case — migrates verbatim, same answer.
- Multi-character `count` fails loudly with the explanation above; the migrating author decides whether they meant the set (rare, and usually a surprise to them) or a substring count (rewrite when the name exists; today, `split` or a loop).
- Tier: _free_ for single characters; _gem-supplied_ lint for the rest — a linter can flag every multi-character `count` in valid Ruby before the flip.
