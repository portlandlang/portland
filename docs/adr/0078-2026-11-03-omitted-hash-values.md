# 0078 — `{name:, port:}` reads as Ruby 3.1 reads it

- **Status:** Accepted (ruled by the deciding user 2026-11-03, accepting the recommendation in [#167](https://github.com/portlandlang/portland/issues/167); not built — the build is [#191](https://github.com/portlandlang/portland/issues/191))
- **Date:** 2026-11-03
- **Issue:** [#167](https://github.com/portlandlang/portland/issues/167)

## Context

Ruby 3.1 reads `{name:, port:}` as `{name: name, port: port}`: a key with no value takes the local or method of the same name. Both implementations refuse it as a parse error, `unexpected token ,`. ruby/spec `language/hash_spec.rb` carries four rows for it, ported `partial_planned`.

The shorthand `{name: "pdx"}` is already the only symbol-key spelling ([ADR 0023](0023-2026-07-25-symbols.md), [symbols.md](../ruby/symbols.md)), and the omitted-value form is its natural end; it is also what makes keyword forwarding read well — `build(name:, port:)` — which [ADR 0014](0014-2026-07-22-keyword-arguments.md) keeps as Ruby 3's.

## Decision

1. **A bare `name:` in a hash literal, or in a call's keyword arguments, means `name: name`**, resolved as any bare name is: a local first, then a method in scope. A name nothing binds refuses as any undefined name does.
1. **The no-shadow rule makes the resolution unambiguous**: a local and a method cannot share the name, so there is never a question which `name` the key took.
1. **Rejected:** refusing the form with `write name: name`. Keeping literals explicit buys nothing when the resolved name is unmistakable, and it fights current Ruby's own style.

## Consequences

- Both parsers accept a label followed by `,` or `}` or `)` as a key with an omitted value; the checker sees the same node as `name: name`.
- The four manifest rows move from `partial_planned` to the build, then to ported.
- Ledger: [symbols.md](../ruby/symbols.md) names the form beside the one-spelling rule.
