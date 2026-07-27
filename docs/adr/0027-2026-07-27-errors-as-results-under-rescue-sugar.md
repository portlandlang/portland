# 0027 — Recoverable errors: typed results under rescue-shaped sugar

- **Status:** Tentative (one of three competing drafts for [#28](https://github.com/portlandlang/portland/issues/28) — this, `begin/rescue`, and typed results; the branches not merged close unmerged)
- **Date:** 2026-07-27

## Context, shared by all three drafts

`panic` is deliberately unrecoverable — the only crash is one you typed (ADR 0010) — and Portland has no story for errors a program should *survive*. The first genuinely fallible operations are already here and currently panic: `read_file` on a missing path, `write_file` on a bad one, and every tool that parses many files and should report all failures rather than die at the first.

The census, across all 195,390 gems on RubyGems.org with era cohorts (ruby_research, 2026-07): **38.5% of gems have a `rescue` clause** (45.4% of 2020+ gems), and raising outstrips rescuing — 48.6% of gems raise, and raise is 54% of all error-handling *sites* in modern code, up from 36% pre-2015. The constructs this decision must place: the rescue modifier sits at 8.2% of gems with its density collapsing by era (12.4 → 3.1 sites per 100k AST nodes), bare `retry` at 3.4% and halving, `ensure` at a real 12.0%. Two trend lines matter most: **40.2% of 2020+ gems define custom error classes** (up from 20.5% pre-2015) — the ecosystem is already moving toward typed, specific errors — and **33.1% of gems swallow an error somewhere**, the pathology invisible flight makes easy.

Each draft answers the same three programs, so they can be read side by side: **a fallback** (read an optional config, default if absent), **collect-don't-stop** (check many files, report every failure), and **the deep unwind** (a helper three frames down fails; the top decides).

## Decision (this draft)

**The typed-results semantics, wearing rescue-shaped sugar.** Underneath, exactly the sibling draft: a fallible operation returns its value or a `failure` struct, failures are ordinary values, `or` and `case/in` handle them, no exception machinery exists. On top, two pieces of sugar for the shapes that make pure results wordy:

1. **`!` is unwrap-or-propagate** (settling ADR 0015 §5): `read_file!(path)` yields the content or returns the failure from the enclosing method. The propagation is one character, and `grep '!'` audits every path a failure can travel — the visible-flight property `raise` gives up.
1. **`rescue` is a method-level handler over results**, not a control-flow construct: it catches what `!` propagated *in this method's body*, by pattern, at the bottom where Ruby's eye expects it.

<!-- not-portland: this draft's proposed syntax; nothing here is built -->

```ruby
# Program 1 — the fallback: the results draft's one-liner, unchanged
config = read_file("portland.toml") or ""

# Program 2 — collect, don't stop: case/in, unchanged from the results draft
mutable failures = []
paths.each do |path|
  case read_file(path)
  in ReadFailed(reason:) then failures << "#{path}: #{reason}"
  in content             then check(content)
  end
end

# Program 3 — the deep unwind: `!` propagates, `rescue` lands it
def load_settings(path)
  parse_settings(read_file!(path))     # ! : the failure returns from here, visibly
end

def main
  settings = load_settings!(argv.first)
  render(settings)
rescue ReadFailed(reason:)
  puts "no settings — #{reason}"
  render(default_settings)
end
```

The load-bearing difference from real exceptions: **a failure never crosses a frame that did not mark it.** `read_file!` propagates one level — to `load_settings`'s caller — because the `!` is written there; `main` sees it only because `main` also wrote `!`. Delete either suffix and the failure is a plain return value at that boundary, handled with `or`/`case` like any other. `rescue` without a `!` in the body above it is dead code, and the compiler (#9) can say so.

## The trade, stated plainly

**Bought:** the results draft's visibility and #9 story, plus Ruby's *reading* comfort — program 3's handler sits at the method bottom where the 38.5% of rescuing gems put it, and the propagation ceremony that draft pays is one character here. The census hands `!` its strongest argument: rescue-to-re-raise is the fastest-growing rescue shape (24.4% of 2020+ gems, up from 15.2% pre-2015) — Ruby programmers already write propagation by hand, a `rescue => e; raise` at a time, and `!` is that idiom compressed to a character with the swallow (33.1% of gems!) made impossible to write by accident. `rescue nil` stays impossible; ancestry filtering stays impossible; failures stay printable, storable values.

**Spent:** two constructs instead of zero, and the risk named by "never guess": sugar that *resembles* Ruby's rescue but differs in reach (one marked frame at a time, never a flight) may read as false familiarity — a Rubyist could assume an unmarked frame passes failures through. The refusal teaches ("`rescue` with no `!` above it — failures cannot arrive here"), but the lesson exists where the results draft has nothing to teach.

## Interactions

- **#9 inference:** identical to the results draft — failure types are return types, `!` is sugar over them, dead `rescue` is statically detectable. No effect system.
- **#11 `together`:** a task's failure is a value at the join, as in the results draft.
- **Migration:** `begin/rescue` bodies move to method-level `rescue` mostly intact; `raise` becomes returning a failure; the `!` marks arrive during the port and become the audit trail. Cheaper than pure results for the 23/50, honest about the one behavioral difference (no unmarked flight).
