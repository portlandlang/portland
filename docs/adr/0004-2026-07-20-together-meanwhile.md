# 0004 — Concurrency vocabulary: `together` + `meanwhile` + `~`

- **Status:** Tentative (strongest known candidate; open to better pre-1.0)
- **Date:** 2026-07-20
- **Issue:** [#3](https://github.com/portlandlang/portland/issues/3)

## Context

Tier-2 concurrency is structured fork-join: task lines are closures handed
to the runtime scheduler (P/E-core thread pool); the block's `end` is the
join barrier; tasks are lexically caged — no handles, no futures, no
`await`. The programmer declares *concurrency* (independence); the runtime
harvests *parallelism* when it pays. The vocabulary must therefore describe
the declaration, not the machinery — which ruled out `spawn` (OS word,
leaks implementation) and `launch` (owned by app launchers, on this exact
platform).

The pair must also read as one thought, the way "async/await" does.

## Decision

```ruby
together do
  meanwhile user = fetch_user(id)
  ~ orders = recent_orders(id)      # ~ and meanwhile are dead-identical
  ~ news = latest_news
end

render(user, orders, news)          # names are plain values after end
```

- **`together`** — the block. States the whole contract in plain English:
  run together, finish together.
- **`meanwhile`** — the word form of the task marker. It is the prose of
  the semantics: "these go off on their own in the meantime, and we stay
  here until they're done." Zero identifier theft, zero machinery.
- **`~`** — the sigil form (ADR 0002), dead-identical to `meanwhile`.

No computer words anywhere in the concurrency story.

## Consequences

- Rejected along the way: `spawn` (machinery), `launch` (app-launcher
  collision), `task`/`start`/`all` (identifier theft), `gather` (join-only
  emphasis), register-mismatched pairs generally.
- Still open on #3: whether the terse positional register
  (`a, b = together do ... end`) earns its existence.
- Nothing implemented yet; `together` semantics are #11.
