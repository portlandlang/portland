# Pattern matching

**Status:** specced ([ADR 0013](../adr/0013-2026-07-22-case-in-spec.md);
the pattern-edge calls are tentative) and the **runtime half is built** in
the seed and the trio (2026-07-23) — including the payoff: the trio's
evaluator dispatches on its own AST with struct patterns. One-line forms
are seed-only so far; the static half (exhaustiveness, unreachable arms,
the unused-capture lint) waits for #9, previewed as runtime panics.

## Ruby

Two `case` forms: `when` compares values through monkeypatchable `===`;
`in` (2.7+) matches shapes and binds, raising `NoMatchingPatternError`
at runtime when nothing matches. A bare lowercase pattern silently
captures anything.

## Portland

Both forms survive, Ruby's split kept — `when` for "which value is it,"
`in` for "what shape is it, hand me the pieces":

```ruby
case node
in ReturnNode(value: nil)      then "(return)"
in ReturnNode(value:)          then sexp(value)
in BreakNode | NextNode        then simple(node)
in ^expected                   then "the one we wanted"
in Token(kind:) if kind == "integer" then integer(node)
in nil                         then "absent"
end
```

What changes from Ruby:

- **Exhaustiveness is compile-checked** for `case/in`: cover the subject
  or write `else`. A maybe missing `in nil` refuses to build. (`when`
  requires `else` instead when coverage can't be proven.)
- **`===` is a statically resolved method** — `when Integer` /
  `when 1..9` / user-defined `===` all work; runtime redefinition is
  gone with the rest of the runtime magic.
- **Captures are Ruby's**, fenced by no-shadow (collision = error),
  exhaustiveness (catch-alls make later arms unreachable = error), and
  an unused-capture lint. The silent-capture typo class dies.
- **Struct patterns are keyword-only** and ceremony-free — no
  `deconstruct_keys`; fields are the pattern surface.
- **In:** pin `^`, guards, alternatives `|`, one-line `=>` and `in`.
  **Out for now:** the find pattern (`in [*, x, *]`), until demanded.
  **Out:** positional struct patterns.
- `pair => [a, b]` is Portland's destructuring; `a, b = pair` multiple
  assignment stays out.

## Migration

- `case/in` code compiles verbatim where patterns are the supported set;
  non-exhaustive matches become compile errors naming the missing case —
  loud, never silent, and strictly fewer runtime crashes.
- `case/when` migrates untouched, including custom `===`, minus runtime
  `===` redefinition (loud).
- Find-pattern uses get a clean "not supported yet" parse error.
- Polyfill: tier 1 — the supported pattern grammar is a subset of
  Ruby's, meaning the same thing; the linter fakes exhaustiveness
  warnings pre-flip.
