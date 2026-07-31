# 0031 — Smart constructors live in a namespace sharing the struct's name

- **Status:** Proposed (variant 2 of 2 for [#61](https://github.com/portlandlang/portland/issues/61) job 3, competing with the struct-body `def self.` draft; a decision artifact, nothing built)
- **Date:** 2026-07-31

## Context

Job 3 of [#61](https://github.com/portlandlang/portland/issues/61): Ruby's `initialize` validated, derived, and defaulted; Portland's `new` is kwargs-in-fields-out. ADR 0027 already decided the shape — a fallible constructor is a function returning the value or a `failure(reason)` — leaving only the question of where the function lives. Today `module Token` beside `struct Token` collides: the struct's name shadows the namespace, so `Token.of(...)` never finds the function.

## Decision

**A `module` and a `struct` may share a name.** The module holds plain functions; the struct stays data plus methods; the collision refusal is deleted and dispatch is ordered instead.

<!-- not-portland: proposed semantics — the sharing this shows is the thing being decided -->

```ruby
struct Token
  kind
  text
end

module Token
  def of(raw)
    return failure("a token needs text") if raw.empty?
    Token.new(kind: "word", text: raw)
  end
end

token = Token.of("rose") or panic "a token was required"
```

The rules:

- `Token.name(...)` resolves: `new`, `with`, then the namespace's functions — the rung ADR 0021 built, reopened for names that are also structs.
- Bare `Token` in expression position stays the struct; the namespace has no value form, exactly as namespaces never had.
- No new syntax exists. The whole feature is one deleted refusal plus a resolution order — modules, functions, and ADR 0027 do the rest.
- Cohesion is by adjacency, not containment: the module sits beside its struct in the file, by convention rather than grammar.

## Consequences

- **This is not a non-difference — Ruby cannot spell it at all.** `module Token` after `class Token` is a `TypeError` in Ruby, so the migrating Rubyist must move `def self.of` out of the class body into a sibling module: a real rewrite at every constructor, in the locked-until-flip tier, and the ledger entry (`docs/ruby/classes.md`) must say so plainly.
- **`Token` comes to name two declarations.** Every future feature must answer for both — inference (#9) first: what is the type of the bare name `Token`? The ordering rule answers dispatch, but the never-guess principle (3) will be asked again each time a new construct meets a doubled name. That standing pressure is this draft's real price.
- **Nothing is added to struct bodies.** Structs stay pure data-plus-behavior declarations; everything callable without an instance is uniformly a namespace function, one mechanism instead of two.
- **Job 4 stays parked** on #11, as #61 records.
