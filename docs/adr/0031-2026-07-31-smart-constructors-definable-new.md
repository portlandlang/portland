# 0031 — Smart constructors: `new` is definable, and `super` reaches the fields

- **Status:** Proposed (variant 3 of 3 for [#61](https://github.com/portlandlang/portland/issues/61) job 3, competing with the `def self.` and namespace-coexistence drafts — and subsuming the first; a decision artifact, nothing built)
- **Date:** 2026-07-31

## Context

Job 3 of [#61](https://github.com/portlandlang/portland/issues/61): Ruby's `initialize` validated, derived, and defaulted; Portland's `new` is kwargs-in-fields-out with no room for a line of logic. ADR 0027 already decided the failure shape; the open question is where construction logic lives.

The first two variants relocate the logic behind a new name — `Token.of(text)` — and keep `new` raw. But call sites outnumber definitions, and the migrating Rubyist's default spelling of construction is `Token.new(...)`: renaming the entry point taxes every caller to spare the one definition. Ruby even has the exact idiom this variant blesses — override `self.new`, do the work, `super` down to the machinery beneath — because `initialize` was never the only way to own construction.

The structural crux any logic-in-`new` design must answer: Portland has no ivars, so a defined `new` must call *something* that actually produces the value. Ruby hides that layer inside `allocate`; Portland has to spell it.

## Decision

**`def self.name(...)` is legal inside a struct body** — everything in the variant-1 draft holds: a type function, registered as `Token::from_pair`, no `self` value inside, the no-shadow ladder extended. **And `new` is one of the definable names.** Defining it replaces the raw constructor everywhere; inside its body — and only there — **`super(...)` is the raw constructor**, kwargs-in-fields-out, the `new` every struct had before this ADR.

<!-- not-portland: proposed syntax, not yet built -->

```ruby
struct Token
  kind
  text

  def self.new(raw)
    return failure("a token needs text") if raw.empty?
    super(kind: "word", text: raw)
  end
end

token = Token.new("rose") or panic "a token was required"
```

The rules:

- **A struct that defines no `new` keeps the raw one.** Plain structs change in nothing.
- **Replacement, not shadowing.** Once defined, `Token.new` means the definition everywhere — there is no position where the spelling means the raw constructor instead. The body reaches the raw layer only through `super`, a different spelling, so no name in the program has two readings. (An earlier sketch had the definition shadowing raw `new` everywhere *except* the body; that is one spelling with position-dependent meanings — the shape the no-shadow rule exists to prevent — and this decision removes it rather than excusing it.)
- **`super` is legal only inside `def self.new`.** Anywhere else it refuses with the rewrite named. There is no chain to climb (ADR 0028 declined inheritance); this is a fixed one-rung reach to the field-filler, not Ruby's open ladder.
- **The defined `new` takes any signature** — positional `Token.new("rose")` included, which the raw kwargs form never could. A fallible one returns the value or a `failure` (ADR 0027), handled with the toolkit like any call.
- **The ladder stays no-shadow:** defining `new` twice refuses; `with` is not declarable; traits may not define `self.` anything (ADR 0028).

## Consequences

- **Call sites migrate for free.** `Token.new(...)` — the spelling in essentially all migrating construction — is a deliberate non-difference. The definition site is the mechanical move: `initialize`'s body becomes `self.new`'s, the `@field = value` tail becomes the `super(field: value)` line, and a Rubyist who already wrote `def self.new ... super` may move nearly unchanged.
- **Invariants hold by construction.** Outside the body there is no spelling of the raw constructor, so a validating struct cannot be constructed around — the guarantee variant 1 offers only as convention, and without waiting on the parked visibility question.
- **`super` re-enters the language narrowed.** Ruby's `super` climbs a hierarchy; Portland's reaches the fields, one rung, constructor-only. The ledger entry (`docs/ruby/classes.md`, due when job 3 builds) records the narrowing loudly.
- **The one hole every variant shares:** `token.with(text: "!")` rebuilds field-wise without re-running the defined `new`. Recorded here rather than hidden; if real code trips on it, that is the pull for a `with`-routes-through-`new` decision later.
- **A cost, honestly:** `Token.new` no longer tells the reader at the call site whether it can fail; the type does, and #9's checker is what will make an unhandled failure loud at build time.
- **Job 4 stays parked** on #11, as #61 records.
