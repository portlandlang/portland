# 0050 — Integer literals: three prefixes, no leading-zero octal

- **Status:** Accepted (ruled and built in both oracles, 2026-09-24)
- **Date:** 2026-09-24
- **Issue:** [#72](https://github.com/portlandlang/portland/issues/72), out of the [#23](https://github.com/portlandlang/portland/issues/23) import

## Context

The ruby/spec import's Float batch compared against `0x4fffffff` and the literal split in the lexer: `0xff` lexed as the integer `0` followed by the identifier `xff`. Underscored literals (#71) were the same class of gap and went in without ceremony, because they matched Ruby exactly. Prefixed literals asked a question first: Ruby's full shape is `0x`/`0b`/`0o` in either case, `0d` for decimal, and a bare leading zero meaning octal — `017` is fifteen. Keeping all of it means keeping the last one.

## Decision

1. **The three prefixes are Ruby's.** `0x` hexadecimal, `0b` binary, `0o` octal, the letter in either case, underscores between digits as in any number (#71), a unary minus folded as for any literal. The token keeps the spelling as written; each parser folds it at its literal choke point, so a stored value and an S-expression dump hold decimal digits, the seed's shape.
1. **A bare leading zero refuses.** `017` is a C-era trap: it reads as seventeen and is fifteen. Python 3 made it a syntax error, JavaScript's strict mode forbids it, Rust never had it. Portland refuses it at the lexer, and the sentence names both readings: `'017' has a leading zero — write 0o17 for octal or 17 for decimal`. A float with one refuses too: `'01.5' has a leading zero — write 1.5`. `0` alone and `0.5` are what they always were.
1. **`0d` is not taken.** It exists in Ruby for symmetry and is not written; a lexer branch and a doc line would buy nothing.
1. **A prefix must be followed by digits of its base.** `'0x' has no digits after its prefix`; `'0b102' has a digit outside its base — binary digits are 0 and 1` (hexadecimal `0-9 and a-f`, octal `0-7`); an underscore against the prefix is the loose underscore #71 already refuses. All of it in ADR 0047's voice: the spelling in single quotes, a dash, the next step where there is one.

## Consequences

- Both lexers refuse with one wording each, pinned by the seed's unit tests and by an integration test that runs `017` through both oracles; the seed marks it a parse refusal for the probe (#88). The values are pinned by a two-oracle evaluator test and by `spec/number/integer/literal_spec.pdx`.
- The import's exclusion note in the Float comparison spec flips: a hex row is back.
- The ledger's [integer-literals.md](../ruby/integer-literals.md) records the one real difference for a migrating Rubyist: `File.chmod(0644, path)` refuses here and is written `0o644`. The migration linter (#36) can rewrite it mechanically, since the refusal's sentence already names the spelling.
- ADR 0003 took the bitwise operators out of the grammar, so a hex literal here is read by named methods rather than `&` and `|`; that does not change what the literal is worth.
