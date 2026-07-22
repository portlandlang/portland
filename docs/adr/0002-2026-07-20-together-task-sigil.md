# 0002 — The `together` task sigil is `~`

- **Status:** Accepted
- **Date:** 2026-07-20
- **Issue:** [#3](https://github.com/portlandlang/portland/issues/3)

## Context

Inside `together` blocks, each concurrent task line carries a per-line
marker (which is what allows interleaving plain lines — a left-side
`async let` list can't express that). The working placeholder was `•`,
which is beautiful but non-ASCII. The ASCII inventory is nearly exhausted
by Ruby-shaped syntax; candidates surviving statement-position analysis
were `*` (markdown-bullet reading, but future splat collision), `>`, `~`,
`&`, `$`.

## Decision

**`~`** marks a task line:

```ruby
together do
  ~ user = fetch_user(id)
  ~ orders = recent_orders(id)
  ~ news = latest_news
end
```

Why `~`: it arrives clean to Ruby hands — unary bitwise NOT is rare, `=~`
is fading (and Portland may never have regex literals), `Regexp#~` is a cut
perlism, and the warm associations (`<<~` squiggly heredocs, `~>`
twiddle-wakka) are gentle. In Portland it is unambiguous in **every**
position, not just statement start — no positional rules, no future
collisions.

## Consequences

- Depends on ADR 0003 (bitwise operators out), which frees `~` entirely.
- Still open on #3: the word form (`spawn` is an unconfirmed placeholder)
  and whether the terse positional register
  (`a, b = together do ... end`) earns its existence.
- Not yet implemented; `together` itself is future work (#11).
