# String escapes

**Summary:** The escape set is closed — Ruby passes an unknown escape through as its bare character, and Portland refuses it.

**Status:** standing behavior, with no ADR — both implementations refuse unknown escapes; the Ruby-compat argument for pass-through is live and unlitigated, and [#304](https://github.com/portlandlang/portland/issues/304) carries the escapes beyond the closed set.

## Ruby

Unknown escapes are the character itself: `"\q"` is `"q"`, `"\e"` is an escape character but `"\y"` is just `y`. A typo therefore compiles and runs, producing a string that looks right in most fonts and is wrong.

## Portland

The escapes are exactly `\n`, `\r`, `\t`, `\"`, `\\`, `\#` in a double-quoted string, and anything else refuses:

```text
unknown escape sequence \q
```

A trailing lone backslash refuses too (`a string ends with a lone backslash`). Single-quoted strings are unchanged from Ruby: only `\'` and `\\` mean anything, every other backslash stays a backslash.

The reasoning is the never-guess family's, one step short of it: `\q` has one plausible reading (Ruby's) and one likely intent (a typo), and where those disagree Portland asks rather than picks. It is also the cheap direction — a closed set can be opened later without breaking any program, while pass-through cannot be closed without breaking some.

## Migration

- Every escape Ruby and Portland share migrates verbatim — the entire common set.
- An unknown escape is a loud compile error naming the sequence; the rewrite is the bare character (`"\q"` → `"q"`) or the escape that was meant.
- Tier: _gem-supplied_ — a linter can flag unknown escapes inside valid Ruby before the flip, since Ruby's own parser already warns about some of them.
- What is **not** covered: `\u` Unicode escapes, `\e`, `\0`, and octal/hex forms, none of which exist in either direction here; [#304](https://github.com/portlandlang/portland/issues/304) tracks them. Each one added is a program that refused and then works — never the reverse.
