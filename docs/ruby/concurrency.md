# Concurrency

**Summary:** No GIL, no `Thread` — one model baked in, spelled `together` / `meanwhile` / `~`.

**Status:** vocabulary decided ([ADR 0002](../adr/0002-2026-07-20-together-task-sigil.md), [ADR 0004](../adr/0004-2026-07-20-together-meanwhile.md)); semantics decided and built serially ([ADR 0029](../adr/0029-2026-07-27-together-semantics.md)) — the serial build is the oracle the parallel runtime must match. Tiers one and three remain future work.

## Ruby

The GIL serializes CPU work; `Thread` is a footgun museum; the "recommended way" (Thread → Fiber → Ractor → async gems) changes every few years, so most Rubyists — reasonably — never learned any of them.

## Portland

One model, baked into the language, three tiers; you live almost entirely in tier 1:

1. **Implicit — you type nothing.** `photos.map { it.thumbnail }` spreads across cores when it's worth it, safe _because_ values are immutable (see [mutability](mutability.md)). No concurrency word appears.
1. **`together` — declare independence.** Structured fork-join: each marked line is a task, the block's `end` is the join, results are named at the task site. `meanwhile` and `~` are dead-identical word/sigil forms:

   <!-- not-portland: `together` is decided but unbuilt (ADRs 0002/0004/0011) -->

   ```ruby
   together do
     meanwhile user = fetch_user(id)
     ~ orders = recent_orders(id)
     ~ news = latest_news
   end

   render(user, orders, news)     # plain values after end
   ```

   No handles, no futures, no `await`, no computer words anywhere. And one register only — results are always named at the task site; there is no positional `a, b = together do ... end` form ([ADR 0011](../adr/0011-2026-07-22-together-single-register.md)).

1. **Explicit control — rare.** Cancellation, timeouts, racing.

The runtime places work on P/E cores (and, for data-parallel tier 1, GPU) — you declare _concurrency_ (independence); the machine harvests _parallelism_ when it pays.

## Migration

- Ruby code that never touched `Thread` — most of it — migrates with nothing to do and gets tier 1 speedups anyway.
- `Thread.new`/`Mutex`/`Queue` code doesn't port; it shrinks into `together` blocks (the common fork-join cases) or tier 3 (the rare controlled cases).
- `together do ... end` parses as a Ruby method-with-block, and `~ x = y` parses as Ruby too — the polyfill gem can make the vocabulary _run_ (serially) in Ruby before the flip.
