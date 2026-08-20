# Error handling: results, not raises

**Summary:** `begin/rescue/raise` do not exist; a fallible operation returns its value or a failure, and the unwrap toolkit you already use for absence handles it — failure is absence with a reason.

**Status:** decided ([ADR 0027](../adr/0027-2026-07-27-errors-as-typed-results.md)) for the model — typed results, absence with a reason, the unwrap toolkit — and revised by [ADR 0044](../adr/0044-2026-08-19-propagation-is-the-toolkit.md) for propagation: the `!` operator is retired, propagation is the explicit toolkit, and `!` belongs to method names again. Built in the seed and the compiler, differentially pinned — see [the language](../language.md#absence).

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

```ruby
content = read_file(path) or ""

case read_file(path)
in reason then "could not read — #{reason}"
end
```

Failure is a value coming back: visible at the call site, destructured by pattern, greppable the way `panic` already is.

## Migrating

- `begin/rescue` with a fallback — becomes `or`, one line.
- `begin/rescue` with handler logic — becomes `case/in` on the result, binding the reason by label.
- `raise` deep in a call chain — becomes returning a failure; each frame it crosses is a spelled-out guard (`return value if value.failure?`), so the port itself writes the audit trail.
- `rescue SomeError => e; raise OtherError` (translate-and-re-raise, Ruby's fastest-growing rescue shape) — becomes matching the failure and returning your own, no keyword.
- `value rescue nil` — becomes nothing: there is no laundering form, on purpose.
- This is an idiom migration, not a respelling — and the one Ruby can rehearse in place: `case/in` and `Data.define` are shared syntax, so the shim gem ([#36](https://github.com/portlandlang/portland/issues/36)) lets a codebase adopt the result idiom before it leaves Ruby. This file grows the full rewrite table as the build lands.
