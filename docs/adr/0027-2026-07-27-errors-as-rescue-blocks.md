# 0027 — Recoverable errors: `begin/rescue`, Portland-shaped

- **Status:** Tentative (one of three competing drafts for [#28](https://github.com/portlandlang/portland/issues/28) — this, typed results, and a hybrid; the branches not merged close unmerged)
- **Date:** 2026-07-27

## Context, shared by all three drafts

`panic` is deliberately unrecoverable — the only crash is one you typed (ADR 0010) — and Portland has no story for errors a program should *survive*. The first genuinely fallible operations are already here and currently panic: `read_file` on a missing path, `write_file` on a bad one, and every tool that parses many files and should report all failures rather than die at the first. The census at n=50: `begin` appears in 23 gems, `rescue nil` in 8, bare `retry` in 1.

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

- **The rescue modifier** (`value rescue nil`, 8/50 gems) — it is the fetch-with-default of errors, and it launders every failure into ambient nil, which does not exist here. The rewrite is `begin/rescue` with a real pattern, or nothing.
- **Bare `retry`** (1/50) — an invisible `goto` to an invisible label.
- **`ensure`** — deferred, not refused; it wants the resource story (files are the only resource today, and `read_file` slurps).
- **Rescue-by-ancestry** — impossible without inheritance, replaced by patterns, which are checkable (#9) where ancestry never was.

## The trade, stated plainly

**Bought:** maximum migration comfort for the 23/50 — `begin/rescue` reads exactly as a Rubyist expects; deep unwinding costs the intermediate frames nothing, so program 3 is the shortest of the three drafts' versions.

**Spent:** invisible control flow — `read_file(path)` in program 3 can transfer control to a handler three frames up, and nothing at the call site says so. That is the exact property `or panic` was designed to make greppable, surrendered for recoverable errors: `grep panic` stays an audit, but `grep raise` only finds the throw sites, never the flights. The `!` suffix (deferred by ADR 0015 §5) would likely mark raising variants (`read_file!`), which restores the marker only by convention.

## Interactions

- **#9 inference:** rescue patterns are checkable like any pattern; nothing new to infer, but raise-flights are invisible to signatures unless #9 grows effect tracking — the largest hidden cost of this draft.
- **#11 `together`:** a raise escaping a task needs a policy (fail the block? surface at the join?) — unresolved in all three drafts, but hardest here because flights are invisible.
- **Migration:** `begin/rescue/else` map one-to-one; `rescue nil`, `retry`, and ancestry filtering do not, and the ledger entry records each rewrite.
