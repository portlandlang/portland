# nil and optionals

**Status:** decided — the model ([ADR 0005](../adr/0005-2026-07-22-optionals-wrapper-model.md)),
both words ([ADR 0006](../adr/0006-2026-07-22-absence-word-nil.md),
[ADR 0009](../adr/0009-2026-07-22-presence-word-some.md)), and the unwrap
toolkit ([ADR 0008](../adr/0008-2026-07-22-unwrap-toolkit.md)). The
runtime half is built in the seed (2026-07-22, `../STAGE0.md`); the static
half (narrowing, compile errors) waits for the real compiler.

## Ruby

Every reference can secretly be `nil` — an ambient singleton object that
responds to methods, is falsy, and detonates in production as
`NoMethodError on nil`, Ruby's #1 crash. `Hash#[]` can't say whether the
key was missing or the value was nil, so `key?` exists as a sidecar.

## Portland

Absence is one explicit case of an **optional** (`User?`, a maybe-User).
A plain `User` can never be absent. You only meet absence where the type
admits it, and the compiler won't let you forget the case.

The word is still **`nil`** / **`nil?`** — the baggage was never the word,
it was the ambient-ness. But Portland's `nil` is a different creature: the
empty case of a maybe. It has no methods, is not falsy, and belongs to no
`NilClass`.

Underneath, the optional is a wrapper (nestable — `hash[key]` on a hash of
optional values keeps "missing key" and "absent value" distinguishable).
On the surface it never shows in unnested code: methods auto-wrap
(`return user`, not `some(user)`), patterns match the payload or `nil`
directly, chains flatten. Rust-style wrapping ceremony is explicitly
rejected.

```ruby
def find_user(id)
  return nil if id < 0
  User.new(name: "Aaron")        # auto-wrapped; the compiler infers User?
end

case find_user(id)
in User(name:) then greet(name)  # payload directly — no wrapper word
in nil         then puts "who?"  # valid Ruby pattern grammar, today
end

user = find_user(id) or return   # bind-or-bail; user is a plain User below
```

A branch that doesn't happen is also nil (ADR 0012, Ruby's rule, typed):
a branchless `if` whose condition is false, a finished `while`, and a
call ended by `break` all produce maybes. The dividing rule: could the
expression have produced a value? Then the absence of one is nil. Could
it never (`puts`)? Then using it is a compile error.

Absent is not empty: `""` and `[]` are *present* values with nothing in
them. `empty?` answers emptiness, `nil?` answers absence, never each
other's question. (Rails' `blank?`/`present?` conflation is deliberately
not reproduced.)

## Migration

- **Compiles verbatim, same meaning:** `return if x.nil?` (and it narrows
  — below the guard, `x` is a plain value), `x == nil`, `x&.y`,
  `x || default`, `in nil` patterns.
- **Loud compile errors:** `if user` truthiness (see
  [truthiness](truthiness.md)), `nil.to_s` and any other method on nil,
  `x.nil?` where `x` can never be absent (dead guard), `NilClass`.
- Ruby cannot represent a nested optional (`nil` doesn't nest), so the
  polyfill covers the whole unnested surface and the linter flags nested
  sites as fix-at-flip-time.

The presence partner is **`some`** / **`some?`** (ADR 0009) — unclaimed in
Ruby core, Rails, and Hanami; precedented with exactly this meaning by
dry-monads' `Maybe#some?`. Auto-wrapping means `some(x)` is written only
in the rare nested case (`some(nil)`: present-but-absent-inside), where a
bare `nil` would be a never-guess error.
