# 0011 — `together` has one register: named-at-site

- **Status:** Accepted
- **Date:** 2026-07-22
- **Issue:** [#3](https://github.com/portlandlang/portland/issues/3) — closes its last open item

## Context

ADR 0004 locked the named-at-site form and left open whether a terse
positional register also exists:

```ruby
user, orders = together do
  ~ fetch_user(id)
  ~ recent_orders(id)
end
```

## Decision

**No positional register.** Named-at-site is the only form:

```ruby
together do
  ~ user = fetch_user(id)
  ~ orders = recent_orders(id)
end
```

The positional form recouples results to task *order* — reorder the
lines, silently swap the values — which is the exact fragile-counting
bug named-at-site was invented to kill. It also can't interleave plain
statements between task lines, and it smuggles in a dependency on
destructuring assignment (`a, b = ...`), a feature Portland has not
designed and which should be decided on its own merits, not as a side
effect of concurrency syntax.

## Consequences

- One way to write a `together` block; nothing to choose between at the
  call site.
- Destructuring assignment remains undecided and unentangled.
- Everything on #3 is now resolved (sigil: 0002, bitwise: 0003,
  vocabulary: 0004, register: here).
- Nothing implemented yet; `together` semantics remain #11.
