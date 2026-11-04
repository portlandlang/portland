# 0079 — No `do` after a `while` or `until` condition

- **Status:** Accepted (ruled by the deciding user 2026-11-03, accepting the recommendation in [#165](https://github.com/portlandlang/portland/issues/165); the refusal sentence is not yet built — [#192](https://github.com/portlandlang/portland/issues/192))
- **Date:** 2026-11-03
- **Issue:** [#165](https://github.com/portlandlang/portland/issues/165)

## Context

Ruby accepts an optional `do` after a `while` or `until` condition, `while i < 3 do`, with the body allowed on the same line. Both implementations refuse it today with an undesigned sentence, `expected a newline after statement, got do`. ruby/spec `language/while_spec.rb` and `until_spec.rb` carry two rows each for it, ported `partial_planned`. [ADR 0037](0037-2026-08-18-the-loop-spellings.md) did not mention the form. Ruby's default linter removes it (`Style/WhileUntilDo`), so modern Ruby rarely writes it.

## Decision

1. **The `do` is refused, with the rewrite named**: `'do' after a while condition is a newline here`, and the same for `until`. A `do` means "a block follows" everywhere else in the grammar, and this keeps it meaning that.
1. **The same-line body goes with it.** `while i < 3 do i += 1 end` has no spelling here; the body starts on the next line, as ADR 0037's loops do.
1. **Rejected:** accepting `do` as a no-op token. One token in each parser, but a second spelling of a sentence the language already says (principle 5), and the one Ruby's own style guide deletes.

## Consequences

- The two parsers gain one refusal in [ADR 0047](0047-2026-09-22-the-error-voice.md)'s voice; the migration linter's fix is a one-token delete.
- The four manifest rows move from `partial_planned` to `partial_intentional`, linking [removed-syntax.md](../ruby/removed-syntax.md).
- Ledger: [removed-syntax.md](../ruby/removed-syntax.md).
