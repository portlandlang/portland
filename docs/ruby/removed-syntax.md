# Removed syntax

**Summary:** `for`, globals, perlisms, and the other deliberate deletions — plus what is merely deferred.

**Status:** locked by the design brief's cut-list, except where noted. The principle: redundant forms and footguns are deleted; each survivor is the one Ruby's own style guides already preferred.

## Gone

- **`for x in list`** — Ruby's own community abandoned it (it leaks its variable into the enclosing scope). `each` is the one way.
- **The `and`/`or` secret precedence** — the words survive, dead-identical to `&&`/`||` ([word operators](word-operators.md)).
- **Globals** (`$foo`) — and the perlish special-variable zoo (`$_`, `$0`, `$:`, `$!`, `$~`, …). State that anyone anywhere can write was cut with the rest of the [runtime dynamism](metaprogramming.md); the handful of legitimate uses (args, program name) return as ordinary named APIs.
- **`BEGIN` / `END` blocks** — perlisms.
- **Truthiness** — see [truthiness](truthiness.md).
- **Ambient nil / `NilClass`** — see [nil and optionals](nil-and-optionals.md).
- **Bitwise operators** — tentative; see [bitwise operators](bitwise-operators.md).
- **Character literals (`?a`)** and flip-flops — perlisms; the seed never learned them and nothing has missed them.
- **Numbered block parameters (`_1`–`_9`)** — the line noise `it` was invented to replace (ADR 0017). `it` covers one parameter; names cover the rest. The polyfill autocorrects `_1 → it` for free.
- **Plain and dash heredocs (`<<EOS`, `<<-EOS`) and lowercase terminators** — squiggly `<<~SQL` is the only opener and terminators are SCREAMING_CAPS; see [heredocs](heredocs.md).
- **`%q`, `%Q`, `%()`, `%s`, `%W`, `%I`, `%x`** — the `%` family's synonyms and its shell spawn, each refusing with its rewrite; `%w[]` and `%i[]` stay, with three delimiters — see [the `%` literal family](percent-literals.md).
- **`do` after a `while` or `until` condition** ([ADR 0079](../adr/0079-2026-11-03-no-do-after-a-while-condition.md), 2026-11-03; refusal sentence not yet built — [#192](https://github.com/portlandlang/portland/issues/192)) — `while i < 3 do` refuses with `'do' after a while condition is a newline here`, and the same-line body goes with it. `do` means "a block follows" everywhere else in the grammar, and Ruby's own default linter (`Style/WhileUntilDo`) already deletes this one.
- **Subjectless `case`** — declined for now, with [#84](https://github.com/portlandlang/portland/issues/84) as the standing reconsideration: without a subject, `when` flips from `===`-matching to truth-testing — one arm shape, two readings — and Ruby's own default linter (`Style/EmptyCaseCondition`) autocorrects the form away. The refusal names the rewrite: `case takes a subject — a subjectless case is an if/elsif chain here`. Reopens on corpus evidence, user demand, or Portland's own code wanting it.

## Kept, to be clear

Postfix guards, `unless`, `until`, the postfix loop modifiers, and `loop do` ([ADR 0037](../adr/0037-2026-08-18-the-loop-spellings.md) — every loop pre-checked: with no `begin`, Ruby's hidden do-while mode has nothing to attach to; `loop` is a keyword here rather than `Kernel#loop`, so `def loop`, assigning a loop, and the blockless form refuse), `?`/`!` suffixes, `%w[]`, string interpolation, heredocs ([squiggly only](heredocs.md)), blocks, pattern matching. Keyword arguments keep Ruby 3 semantics exactly — labels, defaults referencing earlier parameters, strict positional/keyword separation, and no Ruby 2 hash-to-kwargs autoconversion ([ADR 0014](../adr/0014-2026-07-22-keyword-arguments.md)). The joy surface is the point; only the footguns and the redundancies die.

## Deferred, not removed

Not yet buildable is different from ruled out. These currently give parse errors and are expected to arrive:

- **Splats (`*args`, `**kwargs`)** — deferred by ADR 0014, which shipped the rest of Ruby 3's keyword-argument story. Ruby code using them gets a clean parse error until they land.
- **`&&=`** — undecided on purpose: its maybe personality would silently decide `and`-on-a-maybe semantics, and [ruby_research#2](https://github.com/portlandlang/ruby_research/issues/2) sizes whether the idiom outlived `&.` at all. (`||=` shipped — [ADR 0043](../adr/0043-2026-08-19-or-equals.md).)

(Floats, ranges, brace blocks, `it`, and symbols once sat here as undecided or decided-not-built; all are long since decided — symbols by ADR 0023, after the [first-pass tabling](../history/2026-07-23-symbols-first-pass.md) — built, and specced.)

## Migration

Every removal is a parse or compile error — the loudest possible kind of divergence. Most are rare in modern style-guide Ruby; the linter autocorrects the mechanical ones (`for` → `each`) and flags the rest.

Each refusal names its rewrite, on both implementations ([#135](https://github.com/portlandlang/portland/issues/135)): `'for' is not a Portland loop — walk the collection with 'each'`, `'$x' is a global, which Portland does not have — pass the value in, or name a constant`, `'<<-EOS' is a dash heredoc — write '<<~EOS'`, `'*args' is a splat, which is deferred (ADR 0014)`, `'raise' has no Portland meaning — return a failure`, `'fetch' retires here — '[]' answers a maybe`, `'push' mutates its receiver, and values never mutate here (ADR 0015)`, `'Thread' has no Portland meaning — run independent work in a 'together' block`, `'&' is a bitwise operator, which is out of the grammar (ADR 0003)`, and so on through `send`, `eval`, `proc`, `->`, `&block`, `&:name`, `alias_method`, `freeze`, and `Object`. Three rows of that table are retiring: `attr_reader` becomes a spelling of the field list ([ADR 0075](../adr/0075-2026-11-03-attr-reader-is-a-spelling-of-the-field-list.md)), `require` returns with a load path ([ADR 0077](../adr/0077-2026-11-03-require-by-name-on-a-load-path.md), [requires.md](requires.md)), and `to_sym` returns as a symbol of no vocabulary ([ADR 0071](../adr/0071-2026-11-03-a-symbol-may-be-made-from-text.md)) — each decided, none built, so each still refuses today with its old sentence. Where the ledger names no rewrite — `retry`, `redo`, `undef`, `defined?`, `__method__`, `is_a?` — the fact stands alone. Three shapes the gem census turned up refuse the same way: `a, b = pair` is a multiple assignment (match the value: `pair => [a, b]`), `render :partial => "x"` is a bare hash argument (write `partial: "x"`), and `validates :email` is a call in a type body, where Portland only declares.
