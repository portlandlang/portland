# 0031 — Smart constructors live in the struct body, spelled `def self.`

- **Status:** Proposed (variant 1 of 2 for [#61](https://github.com/portlandlang/portland/issues/61) job 3, competing with the namespace-coexistence draft; a decision artifact, nothing built)
- **Date:** 2026-07-31

## Context

Ruby's `class` did four jobs, and ADR 0028 re-homed two ([#61](https://github.com/portlandlang/portland/issues/61) keeps the ledger). Job 3 — construction logic — is unanswered and pull-ready: Ruby's `initialize` validates, derives, and defaults, while Portland's `new` is kwargs-in-fields-out with no room for a line of logic.

The *shape* was already decided by ADR 0027: a fallible constructor is a function returning the value or a `failure(reason)` — `Token.of(text)` handled with the unwrap toolkit, `Token.of!(text)` to propagate, `or` to fall back, patterns to destructure. The only open question is **where the function lives**, because `module Token` beside `struct Token` collides today: the struct's name shadows the namespace, so `Token.of(...)` dispatches against the struct and never finds the function.

## Decision

**`def self.name(...)` is legal inside a struct body, and declares a type function.** It registers as `Token::of` — a plain function in the struct's namespace — and is called as `Token.of(raw)` through the namespace rung dispatch already owns.

<!-- not-portland: proposed syntax, not yet built -->

```ruby
struct Token
  kind
  text

  def self.of(raw)
    return failure("a token needs text") if raw.empty?
    Token.new(kind: "word", text: raw)
  end
end

token = Token.of("rose") or panic "a token was required"
```

The rules, each an old rule applied:

- **There is no instance and no `self` value inside.** The `self.` in the header is position, not a value — it marks "on the type, not on instances." The body is a plain function whose bare names resolve outward from `Token::` (ADR 0021's rule); construction spells `Token.new(...)` explicitly, because there is nothing implicit to receive it.
- **The no-shadow ladder extends one rung** (ADR 0028's pattern): a type function may not share a name with a field or an instance method of its struct — the refusal names both owners. `new` and `with` are not declarable — refused with the rewrite named.
- **Traits may not declare `def self.`** — a trait is a stateless bundle of instance behavior (ADR 0028), and a refusal says where the function belongs instead.
- **Nothing else changes meaning.** Bare `Token` stays the struct it is today; modules keep holding plain functions as they always have.

## Consequences

- **The spelling is a deliberate non-difference.** `def self.of` inside a Ruby class body moves to Portland unchanged, and the `.of`/`.for`/`.from` constructor idiom the corpus already uses arrives with its muscle memory intact. The migration is a rename: `initialize`'s body becomes `of`'s body ending in `Token.new(...)`. Recorded in `docs/ruby/classes.md` when job 3 builds — the ledger debt #61 already tracks.
- **No name gains a second meaning.** `Token` answers one question (which struct?) before and after — the ambiguity the competing draft must manage never exists. Inference (#9) types the name once.
- **Dispatch grows one small rung** in both oracles: a type-name receiver checks `new`, `with`, then type functions. The seed's structs and methods are already separate tables; the trio's is one lookup with a qualified name.
- **Job 4 stays parked.** A type function is stateless like everything else; the parser-with-a-position story still waits on #11, as #61 records.
