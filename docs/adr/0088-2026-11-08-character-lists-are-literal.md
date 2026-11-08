# 0088 — `tr`, `tr_s`, `delete`, and `squeeze` read their characters literally

- **Status:** Accepted (ruled by the deciding user 2026-11-08 on [#106](https://github.com/portlandlang/portland/issues/106)'s status, choosing literal characters over Ruby's set language; built on both implementations the same day)
- **Date:** 2026-11-08
- **Issue:** [#106](https://github.com/portlandlang/portland/issues/106)

## Context

Ruby's `tr`, `tr_s`, `delete`, `squeeze`, and `count` take their arguments in a small character-set language: `a-z` is a range, a leading `^` negates, a backslash escapes, and several arguments to `delete`, `squeeze`, and `count` intersect. `core/string/{tr,tr_s,delete,squeeze}_spec.rb` hold 97 examples.

[string-count.md](../ruby/string-count.md) already declined the set language for `count`: it takes one character, since a multi-character `count` reads as a substring count and is not one. The four methods here have no such confusion — their arguments are plainly lists of characters — so the question was only whether the list may be spelled in Ruby's set language.

## Decision

1. **An argument is a list of literal characters.** Each character in it stands for itself: `"hello".tr("el", "ip")` is `"hippo"`, `"hello".delete("lo")` is `"he"`, as Ruby's are. Everything Ruby means by such an argument without set syntax, Portland means too: the to-list padded with its last character, the last occurrence of a repeated character winning, an empty to-list deleting, `tr_s` squeezing only the runs it translated, `delete` and `squeeze` intersecting several lists.
1. **The three set spellings refuse**, naming the pattern that says it plainly: a `-` between two characters (a range), a `^` first with more after it (a negation), and any backslash (an escape). `"hello".tr("a-y", "b-z")` refuses with `'a-y' is a character range, which tr does not read here — its characters are literal; use a pattern: gsub(/[a-y]/) { ... }`. A lone `^`, and a `-` first or last, are literal in Ruby too and are read as themselves.
1. **A character is a grapheme** ([ADR 0038](0038-2026-08-19-what-a-character-is.md)), as for `count` and every other character-counting method, compared canonically. Ruby works in codepoints, so the two differ only where a combining mark is involved: `"éé".squeeze` is one `é` here and unchanged in Ruby.
1. **`count` is unchanged.** It still takes a single character, for string-count.md's reason, which does not apply to these four.
1. **The bang forms are not spelled** ([ADR 0015](0015-2026-07-23-values-never-mutate.md)), and Ruby 4.1's hash form of `tr` waits for a pull.

## Consequences

- Both implementations gain the four methods; the builtin table's string row gains them.
- The upstream rows without set syntax port; the range, negation, and escape rows stay upstream, each a refusal with its rewrite.
- Migration: `tr("aeiou", "*")`, `delete("lo")`, `squeeze`, and `squeeze(" ")` port verbatim. `tr("a-z", "A-Z")` and its kin refuse naming the `gsub` that says the same, or a better name (`upcase`) where one exists.
- Ledger: [string-count.md](../ruby/string-count.md) widens to the five set-language methods.
