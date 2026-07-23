# 0007 — `or`/`||`, `and`/`&&`, `not`/`!` are dead-identical; `or` is typed

- **Status:** Accepted
- **Date:** 2026-07-22
- **Issue:** [#4](https://github.com/portlandlang/portland/issues/4) (unwrap ergonomics thread)

## Context

Ruby's `and`/`or`/`not` bind looser than `&&`/`||`/`!` — a secret second
precedence that is on the locked cut-list (perlism). But the unwrap
ergonomics want the word form badly: `user = find_user(id) or return` is
the beautiful line. The governing precedent already exists: word and sigil
forms are **dead-identical** (`meanwhile`/`~`, ADR 0004; the `lambda`/`->`
rule from the design brief). The footgun was never the words — it was the
precedence difference.

With no truthiness in Portland (strict booleans, no ambient nil), Ruby's
one-rule `||` ("left if truthy, else right") cannot survive unchanged; its
meaning must come from types.

## Decision

- **One operator, two spellings.** `or` ≡ `||`, `and` ≡ `&&`, `not` ≡ `!`.
  Same precedence, same semantics, everywhere. Ruby's precedence trap dies.
- **`or` is typed:**
  - On booleans — logical or, short-circuiting. (`and`, `not` likewise.)
  - On a maybe (`T?`) — unwrap-or-else: the left side's value if present,
    otherwise the right side. The result is a plain `T`. The right side may
    instead diverge (`return` / `break` / `next` / `panic`), which makes
    the or-guard: bind-or-bail, and with `panic "why"` the escape hatch.
- **Never-guess errors** where the idiom is genuinely ambiguous:
  - `Boolean?` on the left is a compile error. It is the unique type with
    two different "no"s (`nil` and `false`); Ruby's reading steamrolls an
    explicit `false` into the default (the `@enabled ||= true` bug class),
    and the maybe-reading silently diverges from Ruby. The error names the
    three states and offers unambiguous spellings (`if x.nil? ...`,
    `x == true`).
  - A left side that can never be absent is a compile error: the right
    side is unreachable.

## Consequences

- The everyday Ruby idioms compile verbatim and mean the same thing:
  `nickname || "friend"`, boolean logic, `find_user(id) or return`,
  `... or raise`-style guards (as `or panic`).
- Ruby's `x = a or b` parses differently (`x = (a or b)`, not
  `(x = a) or b`) but the guard idiom is observably identical; exotic
  precedence-dependent uses are the polyfill linter's job to flag.
- All divergences from Ruby are loud (compile errors), never silent —
  the ADR 0006 migration property, held.
