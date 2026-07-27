# 0027 — Recoverable errors: failure is absence with a reason

- **Status:** Tentative (one of three competing drafts for [#28](https://github.com/portlandlang/portland/issues/28) — this, `begin/rescue`, and a hybrid; the branches not merged close unmerged)
- **Date:** 2026-07-27

## Context, shared by all three drafts

`panic` is deliberately unrecoverable — the only crash is one you typed (ADR 0010) — and Portland has no story for errors a program should *survive*. The first genuinely fallible operations are already here and currently panic: `read_file` on a missing path, `write_file` on a bad one, and every tool that parses many files and should report all failures rather than die at the first.

The census, across all 195,390 gems on RubyGems.org with era cohorts (ruby_research, 2026-07): **38.5% of gems have a `rescue` clause** (45.4% of 2020+ gems), and raising outstrips rescuing — 48.6% of gems raise, and raise is 54% of all error-handling *sites* in modern code, up from 36% pre-2015. The constructs this decision must place: the rescue modifier sits at 8.2% of gems with its density collapsing by era (12.4 → 3.1 sites per 100k AST nodes), bare `retry` at 3.4% and halving, `ensure` at a real 12.0%. Two trend lines matter most: **40.2% of 2020+ gems define custom error classes** (up from 20.5% pre-2015) — the ecosystem is already moving toward typed, specific errors — and **33.1% of gems swallow an error somewhere**, the pathology invisible flight makes easy.

Each draft answers the same three programs, so they can be read side by side: **a fallback** (read an optional config, default if absent), **collect-don't-stop** (check many files, report every failure), and **the deep unwind** (a helper three frames down fails; the top decides).

## Decision (this draft)

**No new control flow at all.** A fallible operation returns its value or a `failure` — a marked struct carrying the reason — and the entire unwrap toolkit already knows what to do, because ADRs 0005–0013 built it for absence and **failure is absence with a reason**. `or` unwraps-or-else, the or-guard diverts, `case/in` destructures the reason, `&.` short-circuits. Nothing to learn that a Portland programmer does not already know.

<!-- not-portland: this draft's proposed semantics; `failure` and fallible read_file are not built -->

```ruby
# A failure is a struct like any other, wrapped by `failure(...)` the way
# absence is wrapped by some(...) — one builtin, no hierarchy, no raise.
struct ReadFailed
  path
  reason
end

# Program 1 — the fallback: already spelled, today, by the or you know
config = read_file("portland.toml") or ""

# Program 2 — collect, don't stop: case/in was built for this
mutable failures = []
paths.each do |path|
  case read_file(path)
  in ReadFailed(reason:) then failures << "#{path}: #{reason}"
  in content             then check(content)
  end
end

# Program 3 — the deep unwind: each frame passes the failure up, visibly
def load_settings(path)
  content = read_file(path) or return failure_of(read_file(path))
  parse_settings(content)
end
```

The `or`/or-guard semantics extend exactly as they extended from booleans to maybes (ADR 0007's "typed or"): a failure on the left takes the right side. A stored failure is a value — it can sit in an array, print with `p`, match in a pattern — because it *is* one.

## The trade, stated plainly

**Bought:** every failure path is **visible at the call site and greppable** — program 3's propagation is written, not flown, which keeps the `grep panic` property for recoverable errors too. Zero new control-flow constructs; #9 can check exhaustive handling of failures exactly as it will check maybes, with no effect system.

**Spent:** ceremony, and program 3 is where it bites — the draft's `or return failure_of(...)` spelling is honest about the wart: each intermediate frame must re-state the propagation, and without sugar the re-spelling is awkward (the example calls the fallible operation twice, which is wrong; the real shape needs a binding form). **This is the strongest argument for settling the deferred `!` here** (ADR 0015 §5): `content = read_file!(path)` as *unwrap-or-propagate* — one suffix character marking exactly the call sites a failure can pass through, which makes `grep '!'` the audit of every propagation path the way `grep panic` audits every accepted crash. With `!`, program 3 collapses to:

<!-- not-portland: the `!` propagation sugar this draft proposes alongside -->

```ruby
def load_settings(path)
  parse_settings(read_file!(path))
end
```

## Interactions

- **#9 inference:** failure types are ordinary return types — inferred, not written; unhandled failures become the same compile error as unhandled maybes. No effect tracking needed, the cleanest #9 story of the three drafts.
- **#11 `together`:** a task's failure is a value at the join point — no flight to intercept, the policy question shrinks to "which value".
- **Migration:** the hardest of the three — `begin/rescue` has no direct equivalent, and the 38.5% of gems using it rewrite handler-by-handler into `case/in` or `or` forms. The ledger entry records the rewrites; the honest note is that this is a different idiom, not a respelling.
- **The corpus tailwinds, named:** modern Ruby is already drifting this way. 40.2% of 2020+ gems define custom error classes — errors as specific, typed things, which is what a `failure` struct *is* with the flight removed. And rescue-to-re-raise is the fastest-growing rescue shape (17.2% of gems overall, 24.4% of 2020+): Ruby programmers already write propagation ceremony by hand, one `rescue => e; raise` at a time, which is exactly what `!` collapses to a character.
