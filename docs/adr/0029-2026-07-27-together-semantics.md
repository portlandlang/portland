# 0029 — `together` semantics: independence declared, failure held, nothing shared

- **Status:** Accepted (settles [#11](https://github.com/portlandlang/portland/issues/11); built serially in the seed and the trio, differentially pinned, 2026-07-27 — the serial build is the oracle the parallel one must match)
- **Date:** 2026-07-27

## Context

The vocabulary was settled long ago — `together` blocks, `meanwhile`/`~` task lines (ADRs 0002, 0004), one named-at-site register (ADR 0011) — but the semantics waited, and waited well: ADR 0027 dissolved the hardest question first. A task cannot *throw* anything across the join, because there is nothing to throw — failure is a value, and a value binds to a name.

```ruby
together do
  ~ user = fetch_user(id)
  meanwhile orders = recent_orders(id)   # ~ and meanwhile are dead-identical
end

report = user or "guest"                  # names are plain values after end
```

## Decision

Seven rules, each an application of an existing one:

1. **Task names bind at `end`, and not before.** The join is visible in the scoping rules: inside the block a task's name does not exist, so a plain line or a sibling task that mentions one gets exactly what the name deserves — nothing. Names bind into the enclosing scope as ordinary immutables, so no-shadow polices collisions with the outside world, and two task lines binding one name is a parse refusal (`two tasks bind user — every task line binds its own name`).
1. **Tasks are independent, and the grammar makes it so.** A task line is `~ name = expression` — nothing else. A task cannot see a sibling's name (rule 1); a bare `~ expression` with no binding is refused, because an unbound task whose value might be a failure is a swallow written in advance.
1. **A failed task is a bound failure.** No machinery: the task's value may be a `failure`, it binds like any value, and the toolkit handles it after the join — `or`, `case/in`, `!`. Siblings always run to completion; cancellation is tier 3's question for another day.
1. **A panic is still a panic.** The only crash is one you typed (ADR 0010), inside a task or not.
1. **Nothing unwinds across the join.** `return`, `break`, `next`, and `!` inside a task line are runtime refusals (`a task cannot unwind across the join — bind a name and handle it after end`). This is what keeps `!` honest: propagation marks a frame a failure crosses, and a join is not a frame — it is a barrier.
1. **A task cannot rebind an outer `mutable`.** The refusal names the rewrite: bind a name, combine after the join. This is safe-because-immutable made into a rule rather than a slogan — the accumulator pattern is a block licensing *sequential* rebinding, and a task line is a declaration that sequence does not exist here. Plain (unmarked) lines keep ordinary block behavior, locals dying at `end`.
1. **`together` produces nothing, and its parts exist nowhere else.** Using the block's value is an error (ADR 0012's dividing rule: its answer *is* the bound names). `meanwhile` and `~` outside a `together` are refusals — every task line lives in the block that joins it.

## What is promised, and what is deliberately not

The serial build runs task expressions in line order at the join boundary's pace. **The semantics promise only**: every task completes before `end`; every name is bound after it; failures bind; the refusals above refuse. **Cross-task effect ordering is not promised** — two tasks that both `puts` may interleave differently when the parallel runtime lands. The spec suite pins only the promised things, so the serial implementation is the permanent oracle for semantics while leaving the scheduler room to be a scheduler.

## Consequences

- Tier 1 (implicit parallelism inside `map` and friends) and tier 3 (cancellation, timeouts, racing) remain future work; nothing here constrains them beyond the vocabulary.
- The read-before-join refusal currently surfaces as the ordinary `undefined variable` message rather than a teaching one (`not yet joined`); a diagnostics pass can sharpen it without touching semantics.
- The parallel implementation, when it comes (#12/#13 territory), must match this build on everything promised — the differential harness's job description, extended to a scheduler.
