# Heredocs

**Status:** decided
([ADR 0020](../adr/0020-2026-07-23-heredocs-squiggly-only.md)), not yet
built. Ruby behavior below was verified against Ruby 4.0.6.

## Ruby

Three openers, differing in what happens to indentation:

```ruby
<<EOS      # terminator must sit at column 0; content indentation kept
<<-EOS     # terminator may be indented; content indentation still kept
<<~EOS     # squiggly: strips the common leading indentation
```

The terminator can be any identifier — `<<~sql`, `<<~Sql`, `<<~_x`,
`<<~q1`, even `<<~puts` all parse — and the closing line must match
exactly. `<<~ EOS`, with a space, is a syntax error.

And `<<` does double duty: it is also the shift/append operator. Ruby tells
the two apart by asking whether the receiver is a known **local variable**
or a **method**:

```ruby
x = []
x << EOS        # append. EOS is a constant reference → NameError.
                # Ruby does not look for a heredoc body here.

puts <<~EOS     # heredoc, because `puts` is a method, not a local
  from a command call
EOS
```

## Portland

**Squiggly `<<~` is the only opener, and terminators are SCREAMING_CAPS.**

```ruby
db = <<~SQL
  select *
    from orders          -- relative indentation preserved
SQL
```

The rest follows Ruby: the terminator may be indented, `<<~'SQL'`
suppresses interpolation (single quotes mean that everywhere), method calls
attach to the heredoc (`<<~SQL.strip`), and multiple heredocs can open on
one line. No space is permitted between `<<~` and the terminator — already
a Ruby syntax error, so nothing diverges there.

**Why only squiggly.** Beyond one-way-to-do-it, dropping the other two
openers dissolves a collision. [ADR 0015](../adr/0015-2026-07-23-values-never-mutate.md)
made `<<` Portland's append operator, and Ruby's rule for separating the two
depends on local-vs-method guessing — exactly what the
[no-shadow rule](parentheses.md) exists to eliminate, so it cannot be
inherited. With squiggly as the only opener, `<<` is *always* append and
`<<~` is *always* a heredoc. No positional rule, no disambiguation error,
nothing to learn.

`<<~` also loses less than it appears: it strips only the *common*
indentation, so relative structure survives. The one thing it cannot express
is uniform absolute leading whitespace, which can be written explicitly.

**Why SCREAMING_CAPS.** The convention is already universal, and RuboCop
encodes it as `Naming/HeredocDelimiterCase` with uppercase as the default
enforced style — so conforming Ruby already complies. A delimiter is
punctuation, not an identifier, and uppercase makes the closing line
unmistakable at a glance. Corpus evidence may revisit this; it is a prior,
not a closed book.

## Migration

The two rewrites sit in different tiers, and the difference matters:

- **`<<~` with an uppercase terminator** — compiles verbatim.
- **Lowercase terminators** — *free-tier* autocorrect. Upcasing the opener
  and the closing line preserves the string's value exactly, and RuboCop
  already flags them.
- **`<<` and `<<-`** — parse errors naming the fix, but the rewrite is an
  **unsafe autocorrect**. Switching the opener to `<<~` strips common
  indentation and therefore *changes the string's value* whenever the
  content was indented. It is safe only when the content has no common
  leading indentation; a linter must verify that before offering to apply
  it. For indented content the author has to decide whether the leading
  whitespace mattered.
