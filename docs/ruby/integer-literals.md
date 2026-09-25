# Integer literals

**Summary:** `0x`, `0b`, `0o` and underscores as in Ruby; a bare leading zero refuses instead of meaning octal, and `0d` is gone.

**Status:** decided ([ADR 0050](../adr/0050-2026-09-24-integer-literals.md), 2026-09-24), built in both oracles the same day.

## Ruby

Ruby reads six spellings of an integer: decimal, `0x` hexadecimal, `0b` binary, `0o` octal, `0d` decimal, and — inherited from C — a bare leading zero as octal, so `017` is fifteen. The prefix letter may be either case, and underscores may sit between digits anywhere. The leading-zero form survives in the wild almost only as a permission mask: `File.chmod(0644, path)`.

## Portland

The three prefixes and the underscores are Ruby's, unchanged:

```ruby
0xff        # 255
0b1010      # 10
0o644       # 420
0xFF_FF     # 65535, either case, grouped
-0x10       # -16
```

Two things are not taken:

- **A bare leading zero refuses**, and the refusal names both readings: `'017' has a leading zero — write 0o17 for octal or 17 for decimal`. The only Ruby that breaks is `0644`-style masks, and the sentence is the rewrite; the migration linter ([#36](https://github.com/portlandlang/portland/issues/36)) can apply it mechanically.
- **`0d` is gone.** It was symmetry, never a spelling anyone wrote.

A prefix with nothing after it, or a digit outside its base, refuses at the lexer too: `'0b102' has a digit outside its base — binary digits are 0 and 1`. Bit-twiddling on what a hex literal names goes through named methods, since the bitwise operators are out of the grammar ([bitwise-operators.md](bitwise-operators.md)).
