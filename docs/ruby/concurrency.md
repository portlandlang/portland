# Concurrency

**Status:** vocabulary decided-tentative
([ADR 0002](../adr/0002-2026-07-20-together-task-sigil.md),
[ADR 0004](../adr/0004-2026-07-20-together-meanwhile.md)); semantics are
[#11](https://github.com/portlandlang/portland/issues/11). Nothing
implemented.

## Ruby

The GIL serializes CPU work; `Thread` is a footgun museum; the
"recommended way" (Thread → Fiber → Ractor → async gems) changes every few
years, so most Rubyists — reasonably — never learned any of them.

## Portland

One model, baked into the language, three tiers; you live almost entirely
in tier 1:

1. **Implicit — you type nothing.** `photos.map { it.thumbnail }` spreads
   across cores when it's worth it, safe *because* values are immutable
   (see [mutability](mutability.md)). No concurrency word appears.
2. **`together` — declare independence.** Structured fork-join: each
   marked line is a task, the block's `end` is the join, results are named
   at the task site. `meanwhile` and `~` are dead-identical word/sigil
   forms:

   ```ruby
   together do
     meanwhile user = fetch_user(id)
     ~ orders = recent_orders(id)
     ~ news = latest_news
   end

   render(user, orders, news)     # plain values after end
   ```

   No handles, no futures, no `await`, no computer words anywhere.
3. **Explicit control — rare.** Cancellation, timeouts, racing.

The runtime places work on P/E cores (and, for data-parallel tier 1, GPU) —
you declare *concurrency* (independence); the machine harvests
*parallelism* when it pays.

## Migration

- Ruby code that never touched `Thread` — most of it — migrates with
  nothing to do and gets tier 1 speedups anyway.
- `Thread.new`/`Mutex`/`Queue` code doesn't port; it shrinks into
  `together` blocks (the common fork-join cases) or tier 3 (the rare
  controlled cases).
- `together do ... end` parses as a Ruby method-with-block, and `~ x = y`
  parses as Ruby too — the polyfill gem can make the vocabulary *run*
  (serially) in Ruby before the flip.
