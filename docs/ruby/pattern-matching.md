# Pattern matching

**Summary:** `case/in` is compile-checked exhaustive, `===` is static, and captures are fenced.

**Status:** specced ([ADR 0013](../adr/0013-2026-07-22-case-in-spec.md); the pattern-edge calls are tentative) and the **runtime half is built** in the seed and the compiler (2026-07-23) — including the payoff: the compiler's evaluator dispatches on its own AST with struct patterns. The static half (exhaustiveness, unreachable arms, the unused-capture lint) waits for #9, previewed as runtime panics.

## Ruby

Two `case` forms: `when` compares values through monkeypatchable `===`; `in` (2.7+) matches shapes and binds, raising `NoMatchingPatternError` at runtime when nothing matches. A bare lowercase pattern silently captures anything.

## Portland

Both forms survive, Ruby's split kept — `when` for "which value is it," `in` for "what shape is it, hand me the pieces":

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

- **Exhaustiveness is compile-checked** for `case/in`: cover the subject or write `else`. A maybe missing `in nil` refuses to build. (`when` requires `else` instead when coverage can't be proven.)
- **`===` is a statically resolved method** — `when Integer` / `when 1..9` / user-defined `===` all work; runtime redefinition is gone with the rest of the runtime magic. Built on both implementations 2026-09-27 ([#150](https://github.com/portlandlang/portland/issues/150), [#151](https://github.com/portlandlang/portland/issues/151)): until then `when` had compared by equality alone, so `when 1..9` silently missed. The `===` operator itself reads the same way, `Integer === 5`, and a `def ===` answering anything but true or false refuses.
- **Captures are Ruby's**, fenced by no-shadow (collision = error), exhaustiveness (catch-alls make later arms unreachable = error), and an unused-capture lint. The silent-capture typo class dies.
- **Struct patterns are keyword-only** and ceremony-free — no `deconstruct_keys`; fields are the pattern surface.
- **In:** pin `^`, guards, alternatives `|`, one-line `=>` and `in`. **Out for now:** the find pattern (`in [*, x, *]`), until demanded. **Out:** positional struct patterns.
- **One superset edge** ([ADR 0042](../adr/0042-2026-08-19-then-rows-everywhere.md)): the whole-construct one-liner `case status in :paid then "receipt" end` parses here and not in Ruby, whose subject eats `status in :paid` as the match test and errors. Portland's case subject parses one notch below full expression, so a following `in` is always the first arm; a match-test *subject* takes parens.
- `pair => [a, b]` is Portland's destructuring; `a, b = pair` multiple assignment stays out.
- A block's parameters spread a yielded array as Ruby's do — `pairs.each { |key, value| }` — with one difference ([ADR 0057](../adr/0057-2026-10-16-block-parameters-spread-a-yielded-array.md)): where Ruby fills a parameter the array cannot with nil, Portland refuses, naming the array and the count. `|(a, b), c|` nesting waits for a pull.

## Pattern arguments

Ruby's Enumerable takes a pattern where it also takes a block — `all?(1..3)`, `any?(2)`, `none?`, `one?`, `grep(3..7)`, `grep_v`, `slice_before(pattern)`, `slice_after(pattern)` — asking `pattern === element`. Here they take one too ([ADR 0066](../adr/0066-2026-10-25-pattern-arguments-take-the-when-rule.md)), under the `when` rule: a range covers a number, anything else asks `==`. `grep(3..7) { it + 1 }` is `filter_map` under that rule. A pattern beside a block refuses — `'all?' takes a pattern or a block, not both` — and a type name is not a pattern argument, since a type is not a value: `all? { it in Integer }` is the spelling. A regex pattern waits on [#74](https://github.com/portlandlang/portland/issues/74).

## Migration

- `case/in` code compiles verbatim where patterns are the supported set; non-exhaustive matches become compile errors naming the missing case — loud, never silent, and strictly fewer runtime crashes.
- `case/when` migrates untouched, including custom `===`, minus runtime `===` redefinition (loud).
- Find-pattern uses get a clean "not supported yet" parse error.
- Polyfill: tier 1 — the supported pattern grammar is a subset of Ruby's, meaning the same thing; the linter fakes exhaustiveness warnings pre-flip.
