# 0025 — `return` unwinds to the method it was written in

- **Status:** Accepted (built in the seed and the trio, differentially pinned, 2026-07-27)
- **Date:** 2026-07-27

## Context

[language.md](../language.md#control-flow) has promised since the day the sentence was written: "`return` exits the enclosing method, unwinding through loops *and* blocks." For blocks run by builtins, both oracles kept it — `numbers.each do |n| return n if n.even? end` exits the method that wrote it, and the or-guard (`user = find(id) or return`, ADR 0008) leans on exactly this. But a block handed across a `yield` told a different story, and nobody had noticed until [#48](https://github.com/portlandlang/portland/issues/48) probed it:

```ruby
def apply
  yield
  "apply's answer"
end

def outer
  result = apply do
    return "unwound"     # written in outer; runs while apply is on the stack
  end
  "outer's answer"
end
```

Both oracles consumed the `return` at the **first method boundary the unwind met** — `apply` returned `"unwound"`, and `outer` carried on. Ruby exits `outer`. The docs' plain reading exits `outer`. The implementations did neither.

## Decision

**A `return` belongs to the method its block was written in, and unwinds until it gets there** — through any method that merely yielded to the block, whose own result never happens. Ruby's rule, and the one language.md already promised.

The alternatives, and why not:

- **First-boundary** (the accidental status quo) makes `return` in a block mean two things: "exit *my* method" when the iterator is a builtin, "exit *the helper*" when it is a user method. The difference is invisible at the call site — and scheduled to move, since Stage 3 pushes builtins down into Portland. The day `each` becomes a user method, every `return`-in-a-block would quietly change meaning. A rule that cannot survive the roadmap is not a rule.
- **Refusing** a `return` that crosses a `yield` would be never-guess purism against a construct with one defensible reading, and it would break the or-guard idiom inside any yielded block — which is where people write it.

This is a deliberate **non-difference** from Ruby: `return` in a block behaves exactly as a migrating Rubyist expects. (The orphaned-block case — a block outliving its method, where Ruby raises `LocalJumpError` — cannot yet be written, because blocks are not values here.)

## Implementation

Both oracles carry the same idea: a `return` is aimed at a frame, and a method boundary consumes it only when it is that frame.

- The **seed** threads a `home_depth` beside `call_depth`: a method body's home is its own frame; a block's home is its writer's, carried in the handed-block tuple and swapped in around `yield` exactly as the writer's scope already was. `Pending::Return` carries the target; a boundary it is not aimed at re-pends it and discards its own result.
- The **trio** needed almost nothing: a `__frame__` binding rides the bindings like `__block__` does, and since a block already closes over its writer's bindings, a `return` inside one reads its writer's frame *for free*. The `Outcome` signal gained a `target` that every propagation site carries.

Pinned by the `evaluator_block_interrupts` differential (both oracles changed in the same commit, so the differential alone would have stayed green — the probe matrix in the test is what pins the *direction*) and by `control_flow_spec`.

## Consequences

- The builtin-iteration case is untouched, in behavior and in meaning — but its *justification* improved: it is no longer a special case, just the write-site rule where writer and caller coincide. Stage 3 can move the primitive boundary without touching this semantics.
- A yielding helper is transparent to `return`, so DSLs built on `yield` — `describe`/`specify`, a future `with_transaction` — cannot swallow an early exit that was aimed past them.
- Probing this ADR's cases surfaced a separate, pre-existing seed bug: `yield` *inside* a yielded block re-runs the block itself, forever — the trio delegates correctly. Recorded in its own issue, not fixed here.
