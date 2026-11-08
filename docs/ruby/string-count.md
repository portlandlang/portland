# `String#count`, `tr`, and character sets

**Summary:** `count` takes a single character, and `tr`, `tr_s`, `delete`, and `squeeze` read their arguments as lists of literal characters; Ruby's set spellings — ranges, negation, escapes — refuse instead of quietly meaning something else.

**Status:** decided in practice (the [#23](https://github.com/portlandlang/portland/issues/23) import, 2026-08-12; ratified 2026-08-18); no ADR — the refusal falls straight out of principle 5, and a future substring-count would be a new name, not a new reading of this one. A corpus question is filed ([ruby_research#1](https://github.com/portlandlang/ruby_research/issues/1)): how often real code passes a multi-character argument, and which meaning it intends.

## Ruby

`String#count` looks like a substring counter and is not one. Its argument is a **character set**: `"banana".count("a")` is 3, but `"banana".count("an")` is 5 — every `a` plus every `n`, not occurrences of `"an"`. Behind that lurk set negation (`"^a"`), ranges (`"a-z"`), and multiple-argument intersection. It is one of Ruby's most reliably misread methods.

## Portland

`count` takes exactly one character — one *grapheme*, per [ADR 0038](../adr/0038-2026-08-19-what-a-character-is.md), so `count("🇺🇸")` is legal — and counts its occurrences, canonically. The reading both languages agree on:

```ruby
"banana".count("a")    # 3, same as Ruby
```

More than one character is a compile-away refusal, not a guess:

```text
count takes a single character — Ruby's multi-character count is a character set, not a substring, and Portland does not guess
```

Counting occurrences of a substring is a different job and would get a different name if a real file pulls for it; the set semantics (negation, ranges) are declined, not deferred.

**`tr`, `tr_s`, `delete`, and `squeeze` read their characters literally** ([ADR 0088](../adr/0088-2026-11-08-character-lists-are-literal.md)). Their arguments are plainly lists of characters, so the substring confusion that keeps `count` to one character does not arise; each character stands for itself, with Ruby's rules for everything else — the to-list padded with its last character, the last occurrence winning, an empty to-list deleting, several lists to `delete` and `squeeze` intersecting:

```ruby
"hello".tr("el", "ip")     # "hippo", same as Ruby
"hello".delete("l", "lo")  # "heo", same as Ruby
"yellow moon".squeeze      # "yelow mon", same as Ruby
```

The three set spellings refuse, naming the pattern that says the same: a `-` between two characters, a `^` first with more after it, and any backslash. A lone `^`, and a `-` first or last, are literal in Ruby too and read as themselves.

```text
'a-y' is a character range, which tr does not read here — its characters are literal; use a pattern: gsub(/[a-y]/) { ... }
```

A character is a grapheme, as for `count`, so the two differ from Ruby only around combining marks.

## Migration

- Single-character `count` — the overwhelmingly common case — migrates verbatim, same answer.
- `tr`, `tr_s`, `delete`, and `squeeze` with plain character lists migrate verbatim. A range or a negation refuses naming its `gsub`; `tr("a-z", "A-Z")` is `upcase`.
- Multi-character `count` fails loudly with the explanation above; the migrating author decides whether they meant the set (rare, and usually a surprise to them) or a substring count (rewrite when the name exists; today, `split` or a loop).
- Tier: _free_ for single characters; _gem-supplied_ lint for the rest — a linter can flag every multi-character `count` in valid Ruby before the flip.
