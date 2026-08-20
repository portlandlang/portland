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
1. **The one-line `case/in` needed a carve-out — and it is Portland's own, not Ruby's.** *Corrected the day it was written: a first draft called this "Ruby's own carve-out," and probing Ruby 4.0.6 (after first falling into principle 8's documented system-2.6 pothole) showed Ruby has no such thing — its one-line `case x in :a then … end` is refused outright, the subject eating `x in :a` as the standalone match test and leaving no arms; Ruby dodges the collision only because a newline ends the multiline subject.* Portland's rule: a `case` subject parses one notch below full expression, so a following `in` always reads as the first arm rather than as the one-line match test (`expr in pattern`, ADR 0013 §4). This makes the one-line `in` form legal here where Ruby refuses it — a strict superset, no Ruby meaning changed — at the cost that a genuinely match-test subject needs parens. One position, one meaning.

## Consequences

- **A non-difference from Ruby, with one superset edge** — this ADR removes divergences (the three quiet strictnesses) and adds a single extension: the one-line `case/in` form parses here and not in Ruby. Migrating code is untouched in both directions (Ruby code containing the form does not exist, since Ruby refuses it); the [pattern-matching ledger](../ruby/pattern-matching.md) carries the line. language.md's "`then` belongs to `case`, not to `if`" sentence dies.
- The one-line `if a then b else c end` is now the honest interim spelling for the committed-but-undesigned ternary ([#83](https://github.com/portlandlang/portland/issues/83)).
- The invented-syntax parse test loses its one-line-if row — the syntax stopped being invented — and keeps the endless method (#82) and the ternary (#83), each now pointing at a commitment rather than a void.
- The decision-table motivation for subjectless `case` is retired (noted in [#84](https://github.com/portlandlang/portland/issues/84)): boolean tables get the aligned look in `if`-land.
