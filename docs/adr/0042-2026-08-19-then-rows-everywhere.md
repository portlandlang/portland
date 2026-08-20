# 0042 — `then` rows everywhere

- **Status:** Accepted (built in both oracles, 2026-08-19)
- **Date:** 2026-08-19
- **Issue:** none — proposed directly by the deciding user during the subjectless-`case` discussion (#84 records that adjacent decline)

## Context

Portland's `case`/`when` always had the one-line row — `when x then y`, `then`s aligned into a decision table — while `if`/`elsif` required indented bodies, and even `case` was stricter than Ruby in three quiet ways: no `then`-as-separator, no same-line `else` body, no all-on-one-line form. The gap surfaced when a boolean decision table came up in the subjectless-`case` discussion: the table typography existed only in `case`, so declining subjectless `case` would have taken the aesthetic hostage. The deciding user's position: `if` and `case` should share Ruby's typography wholesale.

## Decision

**Every branch header takes Ruby's full `then` typography, in `if`, `elsif`, `unless`, `when`, and `in` alike; `else` takes a same-line body; whole constructs fit on one line.** Pure typography: the same nodes parse out of every shape, the evaluator and checker see nothing new, and the block forms remain exactly what they were — this is ADR 0016's braces-and-`do` move, replayed for branch rows.

<!-- not-portland: sketches; several shapes per block for comparison -->

```ruby
discount = if total > 100 then 20
           elsif total > 50 then 10
           else 0
           end

label = if 1 > 2 then "big" else "small" end
word  = case 2 when 2 then "two" else "no" end
note  = case status in :paid then "receipt" else "other" end

when 60 then          # `then` as a bare separator before an indented body
  celebrate
```

The rules, each Ruby's:

1. **A `then` row holds one statement on the same line.** After it: a newline, or the next branch keyword directly (the one-line forms). Anything else refuses — `a then row takes one statement`.
1. **`then` against a terminator is an empty branch** (`if a then end`), which is ADR 0012's nil wearing a `then`.
1. **`else` takes a same-line body** (`else 0`), an empty body (`else end`), or the indented form.
1. **A same-line body without `then` stays refused** on `if`/`elsif`/`unless`/`when`/`in` headers — Ruby's own rule; the word is the separator.
1. **The one-line `case/in` needed Ruby's carve-out, taken deliberately:** a `case` subject parses one notch below full expression, so a following `in` reads as the first arm rather than being swallowed by the one-line match test (`expr in pattern`, ADR 0013 §4). A genuinely match-test subject can still be written with parens. One position, one meaning.

## Consequences

- **A non-difference from Ruby** — this ADR removes divergences (the three quiet strictnesses) rather than adding any; `docs/ruby` needs no new entry, and language.md's "`then` belongs to `case`, not to `if`" sentence dies.
- The one-line `if a then b else c end` is now the honest interim spelling for the committed-but-undesigned ternary ([#83](https://github.com/portlandlang/portland/issues/83)).
- The invented-syntax parse test loses its one-line-if row — the syntax stopped being invented — and keeps the endless method (#82) and the ternary (#83), each now pointing at a commitment rather than a void.
- The decision-table motivation for subjectless `case` is retired (noted in [#84](https://github.com/portlandlang/portland/issues/84)): boolean tables get the aligned look in `if`-land.
