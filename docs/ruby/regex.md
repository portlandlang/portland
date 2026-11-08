# Regex

**Summary:** Ruby's literals and Onigmo's pattern language, whole; `=~` and `match` both answer a `MatchData` or nil, the match globals (`$~`, `$1`) refuse, positions count graphemes, and replacement strings keep Ruby's `\1` escapes.

**Status:** decided ([ADR 0086](../adr/0086-2026-11-07-regex-is-rubys-and-a-match-is-a-value.md), 2026-11-07, out of [#74](https://github.com/portlandlang/portland/issues/74)) and **built** on both implementations 2026-11-07, with `core/regexp/`, `core/matchdata/`, and `language/regexp/` in the manifest and `spec/regex/` holding the ported rows. The engine is Onigmo ported to the seed, layer for layer from Ruby 4.0.9's own copy ([#212](https://github.com/portlandlang/portland/issues/212)), agreeing with Ruby on all 1,075 cases of Onigmo's own suite; the surface over it is `seed/src/regexp.rs`, the one boundary a program sees. Ruby behavior below was verified against Ruby 4.0.9.

## Ruby

Regex is Onigmo. `/\d+/`, `%r{a/b}`, and `Regexp.new("\\d+")` build one; the flags `i`, `m`, `x`, and `o` follow the literal. `=~` answers the index of the match or nil and sets `$~`, from which `$1`, `$&`, `` $` ``, and `$'` read; `match` answers a `MatchData`; `match?` a boolean. `sub`, `gsub`, `scan`, `split`, `index`, `slice`, and `start_with?` take a pattern. A replacement string reads `\0`, `\1`, `\k<name>`, and `\\`; a `gsub` block receives the matched text and reads captures from `$1`. Positions are codepoint indices, and `.` matches one codepoint.

<!-- not-portland: Ruby, shown for contrast -->

```ruby
if line =~ /(\d+)-(\d+)/
  from = $1
  to = $2
end
pos = line =~ /x/          # an Integer or nil
```

## Portland

- **Literals are Ruby's**: `/\d+/`, the `i`, `m`, and `x` flags, interpolation, `%r{...}` with `[]`, `()`, or `{}` ([percent-literals.md](percent-literals.md)). `Regexp.new`, `Regexp.escape`, and `Regexp.union` are type functions.
- **`Regexp.linear_time?` is Ruby's**: true when the engine's match cache covers the pattern, so matching runs in time linear in the text whatever the input — the question to ask before running a pattern on text a user typed. A backreference defeats the cache, `Regexp.linear_time?(/(a)\1/)` is false. It takes a pattern, or a string with flags; flags beside a pattern refuse, where Ruby warns that it ignores them. The `o` flag refuses by name — drop it; a constant pattern is hoisted anyway.
- **The pattern language is Onigmo's**: backreferences, lookahead and lookbehind, named captures, atomic and possessive groups, subexpression calls, Unicode properties, every construct Ruby reads. A construct the engine has not built yet refuses by name; nothing matches differently in silence.
- **A match is a value.** `=~` and `match` both answer the `MatchData` or nil, so a match narrows like any maybe; `match?` and `!~` answer booleans. `MatchData` reads as Ruby's: `found[0]`, `found[1]`, `found[:year]`, `captures`, `named_captures`, `pre_match`, `post_match`, `begin(0)`, `to_a`. The one change from Ruby: `=~` no longer answers an index, because without `$~` an index alone cannot reach the captures.
- **The globals refuse**: `$~`, `$1`–`$9`, `$&`, `` $` ``, `$'`, and `Regexp.last_match`, each with the rewrite — bind the match, read it.
- **Positions count graphemes** ([characters.md](characters.md)): `begin`, `end`, `pre_match`, and `index(/x/)` answer numbers `slice` accepts. Inside the pattern, `.` and a character class match one codepoint as Onigmo's do, and `\X` matches one grapheme; on text with combining marks the two differ, and those rows are recorded against this page.
- **Replacement strings keep Ruby's escapes**: `\0`, `\&`, `\1`–`\9`, `\k<name>`, `` \` ``, `\'`, `\\`. A `gsub` block receives the matched text as a String, as Ruby's does; a block that needs captures uses the escapes form or a `match` loop.
- **`case` works both ways**: `when /pattern/` by `===`, and `in /pattern/` with captures bound by name once a port pulls for it.
- **`inspect` is the function, as everywhere**: `inspect(/a/i)` is `/a/i` and `inspect(found)` is `#<MatchData "…">`; `/a/.inspect` refuses as `"a".inspect` does.
- **Not yet, each waiting for a pull**: `found[start, length]` and the range forms of `[]` and `values_at`; `text[/pattern/, group]` through the index form (`slice(/pattern/, group)` reads the same); `match` with a block; `named_captures(symbolize_names:)`; a match in `case/in`; and `to_s` folding a lone leading group's options into the wrapper the way Ruby's does — `/(?i:a)/.to_s` is `(?-mix:(?i:a))` here, `(?i-mx:a)` in Ruby.

```ruby
found = line =~ /(\d+)-(\d+)/ or panic("no range in #{line}")
from = found[1]
to = found[2]
puts("no digits") unless line.match?(/\d/)
pos = line.index(/x/)

"2026-11-07".sub(/(\d+)-(\d+)-(\d+)/, '\3/\2/\1')   # "07/11/2026"
"hello world".gsub(/\w+/) { |word| word.capitalize }   # "Hello World"

case line
when /^#/ then :comment
when /^\s*$/ then :blank
end
```

## Why

Regex is the largest single refusal in the gem corpus: a literal stops roughly 30,000 gems at the lexer. Keeping Ruby's spelling and Ruby's engine semantics is the whole migration argument (principle 2). The one deliberate change, `=~` answering the match, follows from dropping the globals ([removed-syntax.md](removed-syntax.md)): a spelling Ruby has should not mean less here. Grapheme positions follow from what a character is ([ADR 0038](../adr/0038-2026-08-19-what-a-character-is.md)), so a position a match answers and a position `slice` takes are the same number.

## Migrating

- `if line =~ /x/` — `if line.match?(/x/)`, or `if (line =~ /x/).some?`: a `MatchData` or nil is a maybe, and an `if` takes a boolean, never a maybe's truthiness ([nil and optionals](nil-and-optionals.md)).
- `$1` after a match — bind the match: `found = line =~ /(\d+)/ or panic("…")`, then `found[1]`; or branch on `found.some?`. The refusal names this rewrite.
- `pos = line =~ /x/` — `line.index(/x/)`, a maybe of the position; or bind the match and read `found.begin(0)`.
- `gsub(/x/, '\1')` and `gsub(/x/) { |word| ... }` — port verbatim.
- `gsub(/x/) { $1 }` — the escapes form, `gsub(/x/, '\1')`, or a `scan` with captures.
- A position compared against `length` or handed to `slice` — the same number here, where in Ruby a combining mark made the two drift.
