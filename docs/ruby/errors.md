# Error handling: results underneath, rescue on top

**Summary:** failures are values, never flights — `!` propagates one marked frame at a time, and a method-level `rescue` catches by pattern what its own body's `!`s passed up.

**Status:** tentative ([ADR 0027](../adr/0027-2026-07-27-errors-as-results-under-rescue-sugar.md), one of three competing drafts for [#28](https://github.com/portlandlang/portland/issues/28)).

## Ruby

<!-- not-portland: Ruby, shown for contrast -->

```ruby
def load_settings(path)
  parse_settings(File.read(path))   # Errno::ENOENT flies from here...
end

def main
  render(load_settings(ARGV[0]))
rescue Errno::ENOENT => error       # ...to here, crossing frames unannounced
  render(defaults)
end
```

## Portland (this draft)

<!-- not-portland: proposed syntax from a tentative ADR; nothing here is built -->

```ruby
def load_settings(path)
  parse_settings(read_file!(path))  # ! — the failure returns from here, visibly
end

def main
  settings = load_settings!(argv.first)
  render(settings)
rescue ReadFailed(reason:)          # lands only what this body's !s passed up
  render(default_settings)
end
```

## Migrating

- Method-level `rescue` keeps its position and its reading; the filter becomes a pattern.
- Every frame a failure crosses gains a `!` — the port itself writes the audit trail, and a missing `!` is a compile-time hole, not a silent swallow.
- `raise` becomes returning a failure value; `value rescue nil`, bare `retry`, and ancestry filtering have no equivalents.
- The one behavioral difference to internalize: **failures never cross an unmarked frame.** Ruby's action-at-a-distance rescue is the thing this draft deliberately does not import.
