# 0092 — `__END__` ends the program, and `DATA` is the text after it

<!-- generated: do not edit by hand — script/docs/generate -->

> [!IMPORTANT]
> **A decision record, not a reference.** It says why a ruling was made; a later ruling may have changed it without marking this file. See also: [language.md: Names and binding](../../docs/language.md#names-and-binding) · [docs/ruby/data.md](../../docs/ruby/data.md).

<!-- /generated -->

- **Status:** Accepted (ruled by the deciding user on [#360](https://github.com/portlandlang/portland/issues/360): "Keep both `__END__` and `DATA` in Portland"; built on both implementations 2026-11-22). Decision 3, what `DATA` is, was the builder's reading of a point the issue left open; the deciding user accepted it 2026-11-22.
- **Date:** 2026-11-22
- **Issue:** [#360](https://github.com/portlandlang/portland/issues/360)

## Context

Ruby ends a program's text at a line that is exactly `__END__`, and hands the rest of the file to the program as `DATA`. A script and its input, its template, or its fixtures travel as one file. The ruling on [#360](https://github.com/portlandlang/portland/issues/360) keeps both: they make for great one-off standalone scripts.

Ruby's `DATA` is a `File`, opened on the script and positioned just past the marker. It carries a cursor: `read` consumes it, `gets` advances it, `rewind` moves it to the head of the script itself. Portland has no handle value. The floor is coarse — `System.read_file` answers one whole file as a String ([ADR 0064](0064-2026-10-25-the-standard-library-core-default-tiles-external-tiles.md), [ADR 0068](0068-2026-10-25-system-is-the-floor-module.md)) — and the fine model, a handle with a lifetime and a cursor, is parked on [#177](https://github.com/portlandlang/portland/issues/177). A cursor is also state a value cannot hold, since values never mutate ([ADR 0015](0015-2026-07-23-values-never-mutate.md)). The issue asks for `DATA` "as a read-only String" and leaves open what of the File's surface it answers.

## Decision

1. **`__END__` alone on a line ends the program text**, Ruby's rule exactly: at the line's first column, with a newline or the end of the file straight after it. Nothing after it is lexed or parsed, so it may be anything. Indented, followed by a space, inside a string, or inside a heredoc body, it is not the marker.
1. **`DATA` is the text after the `__END__` line of the file that is run**, bound before the program's first line runs. It is a top-level constant ([ADR 0053](0053-2026-09-24-constants.md)): read from every def, every struct method, and every required file, and bound once, so `DATA = …` in such a program refuses as any second binding does. Nothing after the marker is the empty String. A required file's `__END__` ends that file and defines nothing, as in Ruby.
1. **`DATA` is a String, not a File.** `DATA.read` answers the String, so Ruby's commonest spelling ports unchanged; `DATA.lines` and `DATA.each_line` are the String's own. A name that moves or asks after a File's cursor — `rewind`, `gets`, `readline`, `readlines`, `pos`, `seek`, `eof`, `eof?`, `lineno`, `close` — refuses by name: `DATA is a String, the text after __END__, with no cursor for 'rewind' to move — read it whole with DATA or DATA.read, or by line with DATA.lines or DATA.each_line`. If the handle model of [#177](https://github.com/portlandlang/portland/issues/177) arrives, whether `DATA` becomes a handle is that decision's to make.
1. **`DATA` with no `__END__` refuses by name**, where Ruby raises `NameError`: `DATA is the text after an __END__ line, and the file that is run has none — end the program with __END__ and put the text below it`. The refusal comes when the name is read. The checker leaves the name alone, since a file checked on its own cannot know which file will be run. A program with no marker may bind `DATA` as a constant of its own, as it may in Ruby.

## Consequences

- Both lexers stop at the marker, and both heredoc passes stop rewriting there, so the text after it reaches `DATA` as written. Line endings are the program's: a `\r\n` reads as a newline there as everywhere in a source file.
- The two special readings of decision 3, `read` and the cursor names, apply to the written name `DATA` where it is the predefined constant. The value itself is an ordinary String: bound to another name, it answers what any String answers, and `read` is not among those.
- Each spec file is the file that is run, on both implementations, so a spec may carry its own `__END__` and read it.
- Migration: a script that reads `DATA.read`, `DATA.each_line`, or `DATA.lines` ports unchanged. One that reads by `gets` in a loop becomes `DATA.each_line`; one that rewinds to read its own source has no port.
- Ledger: [data.md](../ruby/data.md).
