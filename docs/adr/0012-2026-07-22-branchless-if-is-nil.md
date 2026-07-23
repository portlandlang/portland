# 0012 — A branch that doesn't happen produces nil

- **Status:** Accepted
- **Date:** 2026-07-22
- **Issue:** [#22](https://github.com/portlandlang/portland/issues/22)

## Context

Deferred by ADR 0010: what do a branchless `if` (condition false, no
else), a finished `while`, and a broken-out-of call produce? Ruby says
nil to all three. The seed said *nothing* — a distinct "no value" state
that panics when used ("produced no value").

Option B ("no value" as a static condition — using a branchless `if` is
a compile error) was rejected: it invents a second absence-like concept
right after ADRs 0005–0010 unified absence into exactly one, it breaks
working Ruby that option A accepts, and its every error message would
tell you to write what A gives you for free.

## Decision

**They're maybes.** An `if` with no taken branch, a normally-finished
`while`, and a call ended by `break` all produce `nil`, typed as maybes
where the compiler exists to type them:

```ruby
greeting = if morning? then "gm" end   # String? — nil when not morning
puts greeting or "hello"               # handled with the ADR 0008 toolkit
```

Ruby-match, with Ruby's silent nil converted into a handled nil — the
standing Portland trick, applied once more. The static strictness B
wanted arrives anyway: an unhandled maybe refuses to compile.

The dividing rule, stated once: **could the expression have produced a
value? Then the absence of one is nil. Could it never? Then using it is
an error.** `if`/`while`/broken-out calls are the first kind. `puts` is
the second — its result stays unusable, as does a body whose final
statement produces nothing (`x = if c then puts("hi") end` is still an
error on the taken path).

- `while` is nil *always* (Ruby's rule); `break value` waits for real
  demand.
- An `else` that exists but is empty also yields nil — same absence of
  an answer, same spelling.

## Consequences

- The seed's "produced no value" panic family shrinks to the genuinely
  valueless (`puts`, definitions).
- The trio evaluator's slot conflation ("empty slot reads as nil") stops
  being a documented crudeness for branches and becomes the spec.
- STAGE0's "Where nil would have been" ledger closes: all four Ruby-nil
  sites now have Portland answers.
- Migration: `x = if c then a end` and `until`-less `while` idioms
  compile verbatim with the same meaning; nothing silent changes.
