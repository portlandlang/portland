# Removed syntax

**Status:** locked by the design brief's cut-list, except where noted.
The principle: redundant forms and footguns are deleted; each survivor is
the one Ruby's own style guides already preferred.

## Gone

- **`for x in list`** — Ruby's own community abandoned it (it leaks its
  variable into the enclosing scope). `each` is the one way.
- **The `and`/`or` secret precedence** — the words survive, dead-identical
  to `&&`/`||` ([word operators](word-operators.md)).
- **Globals** (`$foo`) — and the perlish special-variable zoo (`$_`, `$0`,
  `$:`, `$!`, `$~`, …). State that anyone anywhere can write was cut with
  the rest of the [runtime dynamism](metaprogramming.md); the handful of
  legitimate uses (args, program name) return as ordinary named APIs.
- **`BEGIN` / `END` blocks** — perlisms.
- **Truthiness** — see [truthiness](truthiness.md).
- **Ambient nil / `NilClass`** — see [nil and optionals](nil-and-optionals.md).
- **Bitwise operators** — tentative; see
  [bitwise operators](bitwise-operators.md).
- **Character literals (`?a`)** and flip-flops — perlisms; the seed never
  learned them and nothing has missed them.
- **Numbered block parameters (`_1`–`_9`)** — the line noise `it` was
  invented to replace (ADR 0017). `it` covers one parameter; names cover
  the rest. The polyfill autocorrects `_1 → it` for free.

## Kept, to be clear

Postfix guards, `unless`, `?`/`!` suffixes, `%w[]`, string interpolation,
heredocs (planned — the lexer work is charted in
[#6](https://github.com/portlandlang/portland/issues/6)), blocks, pattern
matching. The joy surface is the point; only the footguns and the
redundancies die.

## Migration

Every removal is a parse or compile error — the loudest possible kind of
divergence. Most are rare in modern style-guide Ruby; the linter
autocorrects the mechanical ones (`for` → `each`) and flags the rest.
