# Error handling: rescue by pattern, not by ancestry

**Summary:** `begin/rescue` survives, but errors are plain struct values and `rescue` filters with a pattern; the rescue modifier, bare `retry`, and ancestry filtering are gone.

**Status:** tentative ([ADR 0027](../adr/0027-2026-07-27-errors-as-rescue-blocks.md), one of three competing drafts for [#28](https://github.com/portlandlang/portland/issues/28)).

## Ruby

<!-- not-portland: Ruby, shown for contrast -->

```ruby
begin
  read_file(path)
rescue Errno::ENOENT => error
  ""
end

value rescue nil     # the modifier: any failure becomes nil
```

Rescue filters by class ancestry — `rescue StandardError` catches every descendant — and the modifier form launders any failure into nil.

## Portland (this draft)

<!-- not-portland: proposed syntax from a tentative ADR; nothing here is built -->

```ruby
begin
  read_file(path)
rescue ReadFailed(reason:)
  ""
end
```

An error is any value you `raise`; `rescue` takes the same patterns `case/in` takes, so filtering is structural and checkable where ancestry never was.

## Migrating

- `begin/rescue/else` — unchanged in shape; the filter becomes a pattern.
- `rescue SomeClass => e` — becomes `rescue SomeStruct(field:)`, binding by label instead of assigning the whole error.
- `value rescue nil` — no equivalent, deliberately: there is no ambient nil to launder into. Write the `begin/rescue` with a real fallback.
- Bare `retry` — gone; write the loop you meant.
- `ensure` — deferred, pending the resource story; nothing to migrate to yet.
