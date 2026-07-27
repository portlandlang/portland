# Error handling: results, not raises

**Summary:** `begin/rescue/raise` do not exist; a fallible operation returns its value or a failure, and the unwrap toolkit you already use for absence handles it — failure is absence with a reason.

**Status:** tentative ([ADR 0027](../adr/0027-2026-07-27-errors-as-typed-results.md), one of three competing drafts for [#28](https://github.com/portlandlang/portland/issues/28)).

## Ruby

<!-- not-portland: Ruby, shown for contrast -->

```ruby
begin
  content = read_file(path)
rescue Errno::ENOENT
  content = ""
end
```

Failure is an exception in flight: invisible at the call site, caught by ancestry, capable of crossing any number of frames unannounced.

## Portland (this draft)

<!-- not-portland: proposed semantics from a tentative ADR; nothing here is built -->

```ruby
content = read_file(path) or ""

case read_file(path)
in ReadFailed(reason:) then report(reason)
in found               then check(found)
end
```

Failure is a value coming back: visible at the call site, destructured by pattern, greppable the way `panic` already is.

## Migrating

- `begin/rescue` with a fallback — becomes `or`, one line.
- `begin/rescue` with handler logic — becomes `case/in` on the result, binding the reason by label.
- `raise` deep in a call chain — becomes returning a failure, propagated visibly at each frame (or by the proposed `!` sugar, which marks the path).
- `value rescue nil` — becomes nothing: there is no laundering form, on purpose.
- This is the largest idiom change of the three drafts — a rewrite, not a respelling, and this file grows the full table if the draft is chosen.
