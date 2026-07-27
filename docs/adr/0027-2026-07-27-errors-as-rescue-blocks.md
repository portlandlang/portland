# 0027 — Recoverable errors: `begin/rescue`, Portland-shaped

- **Status:** Tentative (one of three competing drafts for [#28](https://github.com/portlandlang/portland/issues/28) — this, typed results, and a hybrid; the branches not merged close unmerged)
- **Date:** 2026-07-27

## Context, shared by all three drafts

`panic` is deliberately unrecoverable — the only crash is one you typed (ADR 0010) — and Portland has no story for errors a program should *survive*. The first genuinely fallible operations are already here and currently panic: `read_file` on a missing path, `write_file` on a bad one, and every tool that parses many files and should report all failures rather than die at the first.

The census, across all 195,390 gems on RubyGems.org with era cohorts (ruby_research, 2026-07): **38.5% of gems have a `rescue` clause** (45.4% of 2020+ gems), and raising outstrips rescuing — 48.6% of gems raise, and raise is 54% of all error-handling *sites* in modern code, up from 36% pre-2015. The constructs this decision must place: the rescue modifier sits at 8.2% of gems with its density collapsing by era (12.4 → 3.1 sites per 100k AST nodes), bare `retry` at 3.4% and halving, `ensure` at a real 12.0%. Two trend lines matter most: **40.2% of 2020+ gems define custom error classes** (up from 20.5% pre-2015) — the ecosystem is already moving toward typed, specific errors — and **33.1% of gems swallow an error somewhere**, the pathology invisible flight makes easy.

Each draft answers the same three programs, so they can be read side by side: **a fallback** (read an optional config, default if absent), **collect-don't-stop** (check many files, report every failure), and **the deep unwind** (a helper three frames down fails; the top decides).

## Decision (this draft)

**Ruby's `begin/rescue`, with errors as plain struct values and rescue filtering by pattern.** Portland has no class hierarchy, so Ruby's rescue-by-ancestry cannot exist — and does not need to: `case/in` already is the filtering construct, so `rescue` takes a pattern, and an error is any value you `raise`.

<!-- not-portland: this draft's proposed syntax; nothing here is built -->

```ruby
struct ReadFailed
  path
  reason
end

# Program 1 — the fallback
config = begin
  read_file("portland.toml")
rescue ReadFailed
  ""
end

# Program 2 — collect, don't stop
mutable failures = []
paths.each do |path|
  begin
    check(read_file(path))
  rescue ReadFailed(path:, reason:)
    failures << "#{path}: #{reason}"
  end
end

# Program 3 — the deep unwind: raise crosses frames for free
def load_settings(path)
  parse_settings(read_file(path))     # read_file raises ReadFailed
end

settings = begin
  load_settings(argv.first)
rescue ReadFailed(reason:)
  puts "no settings — #{reason}"
  default_settings
end
```

What Ruby has that this draft **cuts**, each with its census number:

- **The rescue modifier** (`value rescue nil`, 8.2% of gems, and dying — 12.4 → 3.1 sites per 100k nodes across the eras) — it is the fetch-with-default of errors, and it launders every failure into ambient nil, which does not exist here. The corpus says the ecosystem is already abandoning it; this draft finishes the job.
- **Bare `retry`** (3.4% of gems, density halved in modern code) — an invisible `goto` to an invisible label.
- **`ensure`** — deferred, not refused, and the census prices the debt honestly: 12.0% of gems use it, one in eight. It wants the resource story (files are the only resource today, and `read_file` slurps).
- **Rescue-by-ancestry** — impossible without inheritance, replaced by patterns, which are checkable (#9) where ancestry never was. The corpus is friendlier to this cut than expected: rescue-*specific* already beats rescue-bare three to one (33.0% vs 14.9% of gems), and modern code is the most specific cohort.

## The trade, stated plainly

**Bought:** maximum migration comfort for the 38.5% of gems with rescue clauses — 45.4% of the modern cohort — since `begin/rescue` reads exactly as a Rubyist expects; deep unwinding costs the intermediate frames nothing, so program 3 is the shortest of the three drafts' versions.

**Spent:** invisible control flow — `read_file(path)` in program 3 can transfer control to a handler three frames up, and nothing at the call site says so. That is the exact property `or panic` was designed to make greppable, surrendered for recoverable errors: `grep panic` stays an audit, but `grep raise` only finds the throw sites, never the flights. The `!` suffix (deferred by ADR 0015 §5) would likely mark raising variants (`read_file!`), which restores the marker only by convention. The census shows this cost at population scale: 33.1% of gems swallow an error somewhere — a rescue body with no re-raise — which is what handlers far from their flights make easy.

## Interactions

- **#9 inference:** rescue patterns are checkable like any pattern; nothing new to infer, but raise-flights are invisible to signatures unless #9 grows effect tracking — the largest hidden cost of this draft.
- **#11 `together`:** a raise escaping a task needs a policy (fail the block? surface at the join?) — unresolved in all three drafts, but hardest here because flights are invisible.
- **Migration:** `begin/rescue/else` map one-to-one; `rescue nil`, `retry`, and ancestry filtering do not, and the ledger entry records each rewrite.
