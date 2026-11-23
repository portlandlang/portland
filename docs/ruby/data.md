# `__END__` and `DATA`

**Summary:** `__END__` ends the program text and `DATA` is what follows it, as in Ruby; Portland's `DATA` is a String where Ruby's is a File, so `DATA.read`, `DATA.lines`, and `DATA.each_line` port and the cursor methods refuse by name.

**Status:** decided ([ADR 0092](../adr/0092-2026-11-22-end-marker-and-data.md), 2026-11-22, [#360](https://github.com/portlandlang/portland/issues/360)); built on both implementations the same day.

## Ruby

A line that is exactly `__END__` ends the program. The rest of the file is not parsed, and the main script reads it through `DATA`, a `File` opened on the script and positioned just past the marker:

```ruby
DATA.each_line { |line| puts line.upcase }

__END__
rose
city
```

Because it is a File, `DATA` has a cursor: `DATA.read` consumes it, so a second `read` answers `""`; `DATA.gets` takes one line at a time; `DATA.rewind` moves to the head of the script, so the next read answers the program's own source. Without an `__END__` in the main script, `DATA` is not defined and reading it raises `NameError`. A required file's `__END__` ends that file and defines no `DATA`.

## Portland

The marker is Ruby's, exactly: `__END__` at the first column with nothing else on its line. `DATA` is the text after it in the file that is run — a String, bound as a top-level constant, so every def and every required file reads it:

```ruby
DATA.each_line { |line| puts line.upcase }

__END__
rose
city
```

`DATA.read` answers the String, every time; there is no cursor to run out. `DATA.lines` and `DATA.each_line` are the String's own. The methods that move or ask after a File's cursor refuse by name:

```text
DATA is a String, the text after __END__, with no cursor for 'rewind' to move — read it whole with DATA or DATA.read, or by line with DATA.lines or DATA.each_line
```

`DATA` with no `__END__` in the file that is run refuses when it is read, naming the fix:

```text
DATA is the text after an __END__ line, and the file that is run has none — end the program with __END__ and put the text below it
```

Why a String: Portland has no file handle. The floor reads a file whole ([`System`](system.md)), and a value with a cursor waits on the handle model ([#177](https://github.com/portlandlang/portland/issues/177)). `read` is special to the written name `DATA`; the String it answers, bound to another name, has no `read`.

## Migration

- `DATA.read`, `DATA.each_line`, and `DATA.lines` port unchanged — _free_.
- A second `DATA.read` answers the text again where Ruby answers `""`. Code that relied on the cursor running out is rare, and it changes meaning silently; it is the one difference here that does not refuse.
- `DATA.gets` in a loop, `DATA.readlines`, and `DATA.eof?` refuse naming `DATA.lines` and `DATA.each_line` — _gem-supplied_ lint: a linter can flag each in valid Ruby before the flip.
- `DATA.rewind` to read the script's own source has no port.
