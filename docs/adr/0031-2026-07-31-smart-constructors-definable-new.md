# 0031 — Smart constructors: `new` is definable, and `fields` is the raw constructor

- **Status:** Accepted (chosen for [#61](https://github.com/portlandlang/portland/issues/61) job 3 over the `def self.`-only and namespace-coexistence drafts, whose PRs closed unmerged with their design records; a decision, not a build — the build is its own arc, 2026-07-31)
- **Date:** 2026-07-31

## Context

Job 3 of [#61](https://github.com/portlandlang/portland/issues/61): Ruby's `initialize` validated, derived, and defaulted; Portland's `new` is kwargs-in-fields-out with no room for a line of logic. ADR 0027 already decided the failure shape; the open question was where construction logic lives.

The competing drafts relocated the logic behind a new name — `Token.of(text)` — and kept `new` raw. But call sites outnumber definitions, and the migrating Rubyist's default spelling of construction is `Token.new(...)`: renaming the entry point taxes every caller to spare the one definition. Ruby even has an idiom in this exact shape — override `self.new`, do the work, reach the machinery beneath — because `initialize` was never the only way to own construction.

The structural crux any logic-in-`new` design must answer: Portland has no ivars, so a defined `new` must call *something* that actually produces the value. Ruby hides that layer inside `allocate`; Portland has to spell it.

## Decision

**`def self.name(...)` is legal inside a struct body** — a type function, registered as `Token::from_pair`, called through the namespace rung dispatch already owns, no `self` value inside. **And `new` is one of the definable names.** Defining it replaces the raw constructor everywhere; inside its body — and only there — **`fields(...)` is the raw constructor**, kwargs-in-fields-out, the `new` every struct had before this ADR.

<!-- not-portland: decided syntax, not yet built -->

```ruby
struct Token
  kind
  text

  def self.new(raw)
    return failure("a token needs text") if raw.empty?
    fields(kind: "word", text: raw)
  end
end

token = Token.new("rose") or panic "a token was required"
```

The rules:

- **A struct that defines no `new` keeps the raw one.** Plain structs change in nothing.
- **Replacement, not shadowing.** Once defined, `Token.new` means the definition everywhere — there is no position where the spelling means the raw constructor instead. The body reaches the raw layer only through `fields`, a different spelling, so no name in the program has two readings. (An earlier sketch had the definition shadowing raw `new` everywhere *except* the body; that is one spelling with position-dependent meanings — the shape the no-shadow rule exists to prevent — and this decision removes it rather than excusing it.)
- **`fields(...)` is legal only inside `def self.new`.** Anywhere else it refuses with the rewrite named. Inside, it is unambiguous *by construction*: no instance exists there, so no field or local can claim the name — and a program-defined `fields` function is refused in that one position, naming the collision.
- **The defined `new` takes any signature** — positional `Token.new("rose")` included, which the raw kwargs form never could. A fallible one returns the value or a `failure` (ADR 0027), handled with the toolkit like any call.
- **The ladder stays no-shadow:** defining `new` twice refuses; `with` is not declarable; traits may not define `self.` anything (ADR 0028).
- **In a module body, `def self.name` is accepted and identical to plain `def`** (settled in review, 2026-07-31). Ruby reaches `Statistics.mean` only through `def self.mean`; Portland's module `def` already means that (ADR 0021) — so the marker is redundant there, and accepting it is a non-difference that lets migrating modules compile unchanged instead of refusing over a prefix that changes nothing.

## The word

`fields` names the shape, not an action: the value with these fields, no logic ran. The roads not taken, each rejected in review:

- **`super`** — Ruby's word for MRO-climbing in a language with no MRO: a fake-hierarchy connotation, plus bare `super`'s implicit argument forwarding, which against a kwargs-only raw layer would be quietly different behavior (principle 5).
- **`build`** — FactoryBot's and Rails' word for the *smart outer* layer, here naming the dumb inner one; and one of Ruby's most-claimed method names, so reserving it would fire refusals on real migrating code constantly.
- **`allocate`** — Ruby-spelling reuse with different behavior: Ruby's sets no state, this fills every field.
- **`raw`** — honest but less self-documenting; nothing else recommended it over `fields`.

## Consequences

- **Call sites migrate for free.** `Token.new(...)` — the spelling in essentially all migrating construction — is a deliberate non-difference. The definition site is the mechanical move: `initialize`'s body becomes `self.new`'s, the `@field = value` tail becomes the `fields(field: value)` line, and a migrating `def self.new ... super` gets one edit — `super` to `fields(...)` — with the refusal naming it.
- **Invariants hold by construction.** Outside the body there is no spelling of the raw constructor, so a validating struct cannot be constructed around — enforcement the `def self.of` shape could offer only as convention, and without waiting on the parked visibility question.
- **No Ruby word is reused.** `super` stays unspellable (inheritance is declined, ADR 0028), and `fields` arrives with no Ruby meaning to diverge from.
- **The one hole this shares with every shape considered:** `token.with(text: "!")` rebuilds field-wise without re-running the defined `new`. Recorded here rather than hidden; if real code trips on it, that is the pull for a `with`-routes-through-`new` decision later.
- **A cost, honestly:** `Token.new` no longer tells the reader at the call site whether it can fail; the type does, and #9's checker is what will make an unhandled failure loud at build time.
- **The ledger entry** — `docs/ruby/classes.md`, the single biggest thing a migrating Rubyist greps for — is due when job 3 builds, complete rather than promissory, mapping all four of `class`'s jobs per #61.
- **Job 4 stays parked** on #11, as #61 records.
