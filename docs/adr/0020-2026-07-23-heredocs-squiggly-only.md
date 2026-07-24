# 0020 — Heredocs: squiggly only

- **Status:** Accepted (not yet built)
- **Date:** 2026-07-23

## Context

Heredocs were deliberately untracked until a real file pulled for one, and
the lexer homework was scoped in the now-closed
[#6](https://github.com/portlandlang/portland/issues/6) as "heredocs (all
flavors, squiggly, interpolating)." This ADR narrows that scope.

Ruby has three openers:

```ruby
<<EOS      # terminator must sit at column 0; content indentation kept
<<-EOS     # terminator may be indented; content indentation still kept
<<~EOS     # squiggly: strips the common leading indentation
```

ADR 0015 made `<<` Portland's append operator, which collides with Ruby's
heredoc opener. Ruby resolves that collision by asking whether the receiver
is a known **local variable** or a **method** — verified on 4.0.6:

```ruby
x = []
x << EOS        # append; EOS is a constant reference → NameError.
                # Ruby does not look for a heredoc body here.

puts <<~EOS     # heredoc, because `puts` is a method, not a local
  from a command call
EOS
```

That is precisely the local-vs-method guessing the no-shadow rule exists to
eliminate, so it cannot be inherited.

## Decision

**Only `<<~` survives. `<<` and `<<-` are out.**

This is the usual one-way-to-do-it call, but it also has a structural
payoff: **the collision dissolves.** With squiggly as the only opener, `<<`
is *always* the append operator and `<<~` is *always* a heredoc. No
positional rule, no local-vs-method test, no never-guess error to write —
ADR 0015's operator and heredocs stop competing for the same token.

`<<~` loses less than it appears: it strips only the *common* indentation,
so relative structure survives. The single thing it cannot express is
uniform absolute leading whitespace, which can be written explicitly.

The rest follows Ruby, verified on 4.0.6:

```ruby
foo = <<~EOS        # no space between `<<~` and the terminator
  lorem
    ipsum           # relative indentation preserved
EOS

def show
  bar = <<~EOS      # the terminator may be indented
    lorem
  EOS
end

baz = <<~'EOS'      # single quotes suppress interpolation, as everywhere else
  #{not_interpolated}
EOS

qux = <<~EOS.strip  # method calls attach to the heredoc
  trimmed
EOS

pair = [<<~A, <<~B] # multiple heredocs per line
  first
A
  second
B
```

**No space is permitted between `<<~` and the terminator.** `<<~ EOS` is
already a syntax error in Ruby, so this codifies existing behavior rather
than diverging.

## Consequences

- **Interaction with the `~` together sigil** (ADRs 0002/0004, Tentative):
  `list << ~task` (spaced) is append-a-task, while `list <<~task` (unspaced)
  lexes as a heredoc opener. The no-space rule is what separates them, and
  Portland's lexer already tracks `leading_space` per token for exactly this
  class of rule. The failure mode is safe: an unspaced `<<~task` meant as
  append finds no `task` terminator and fails as an unterminated heredoc —
  loud, not a silently different program. Revisit when `~` is built.
- **Lexer**: `<<~` is a three-character token, alongside `...` from
  ADR 0019. Interpolation, escapes, and terminator scanning reuse the
  existing string machinery.
- **Migration**: `<<~` heredocs compile verbatim. `<<` and `<<-` become
  parse errors naming the fix. The rewrite is an **unsafe autocorrect**, not
  a free one: switching the opener to `<<~` strips common indentation and
  therefore *changes the string's value* whenever the content was indented.
  It is safe only when the content has no common leading indentation; the
  linter must check that before offering to apply it.
- Issue #6's "all flavors" checkbox is superseded — only the squiggly
  flavor needs porting, which removes a chunk of the months-of-tedium list.
