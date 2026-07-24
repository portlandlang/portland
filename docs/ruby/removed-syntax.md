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
- **Plain and dash heredocs (`<<EOS`, `<<-EOS`) and lowercase
  terminators** — squiggly `<<~SQL` is the only opener and terminators are
  SCREAMING_CAPS; see [heredocs](heredocs.md).

## Kept, to be clear

Postfix guards, `unless`, `?`/`!` suffixes, `%w[]`, string interpolation,
heredocs ([squiggly only](heredocs.md)), blocks, pattern matching. Keyword
arguments keep Ruby 3 semantics exactly — labels, defaults referencing
earlier parameters, strict positional/keyword separation, and no Ruby 2
hash-to-kwargs autoconversion ([ADR 0014](../adr/0014-2026-07-22-keyword-arguments.md)).
The joy surface is the point; only the footguns and the redundancies die.

## Deferred, not removed

Not yet buildable is different from ruled out. These currently give parse
errors and are expected to arrive:

- **Splats (`*args`, `**kwargs`)** — deferred by ADR 0014, which shipped
  the rest of Ruby 3's keyword-argument story. Ruby code using them gets a
  clean parse error until they land.
- **Floats** and **ranges** — decided (ADRs 0018, 0019, and
  [ranges](ranges.md)), not yet built.
- **Brace blocks** and **`it`** — decided (ADRs 0016, 0017, and
  [parentheses](parentheses.md)), not yet built.
- **Symbols** — genuinely undecided and
  [tabled](../reports/2026-07-23-symbols-first-pass.md), entangled with the
  enum/sum-type question.

## Migration

Every removal is a parse or compile error — the loudest possible kind of
divergence. Most are rare in modern style-guide Ruby; the linter
autocorrects the mechanical ones (`for` → `each`) and flags the rest.
