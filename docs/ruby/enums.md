# Enums

**Summary:** Ruby has none; Portland's are closed vocabularies of symbol cases, with keyword payloads and checked exhaustiveness.

**Status:** decided ([ADR 0022](../adr/0022-2026-07-25-enums-with-payloads.md)), not yet built — symbols land first, since `:foo` does not lex today.

## Ruby

Ruby has no enum. A closed vocabulary is spelled by convention and enforced by nothing:

<!-- not-portland: Ruby, shown for contrast -->

```ruby
STATUSES = [:pending, :paid, :refunded].freeze

purchase.status = :pendign          # perfectly valid; fails somewhere later, or never
```

Rails adds `enum`, but it is a different feature: **column sugar**, not a type.

<!-- not-portland: Ruby on Rails, shown for contrast -->

```ruby
class Purchase < ApplicationRecord
  enum :status, [:pending, :paid]
end

purchase.paid!                      # generated bang setter
purchase.paid?                      # generated predicate
purchase.status                     # => "paid" — a String, not the symbol you wrote
Purchase.statuses                   # => {"pending" => 0, "paid" => 1}
```

It must live inside a model because it maps to a column; it carries no payloads; and you read back a different type than you wrote. Most Rubyists' entire experience of "enum" is this, which is why the standalone kind — `Result`, `Ordering` — reads as unfamiliar rather than missing.

## Portland

A declared type, nested when owned and top-level when not, whose cases may carry keyword payloads:

<!-- not-portland: enums are decided but unbuilt — `:foo` does not lex yet -->

```ruby
enum Ordering
  :less
  :equal
  :greater
end

struct Purchase
  amount
  status

  enum Status
    :pending
    :paid(on:)
    :refunded(on:, reason:)
  end
end

case purchase.status
in :pending           then "not paid yet"
in :paid(on:)         then "paid #{on}"
in :refunded(reason:) then "refunded — #{reason}"
end
```

Four differences from every Ruby spelling of the idea:

- **Membership is checked.** `:pendign` is a compile error where a vocabulary is declared, which is the whole point — Ruby's problem with `status = :pending` was never the syntax, it was that the set is open.
- **The set is closed, so `case/in` is exhaustive.** Drop an arm and it will not build; add a case to the enum and every `case` over it fails until handled. ADR 0013 specced this; enums are the first thing closed enough to check.
- **Payloads live with the case that owns them.** `reason` exists inside the `:refunded` arm and nowhere else, so a paid purchase has no `reason` field to read by mistake.
- **You read back what you wrote.** `purchase.status` is `:paid`, not `"paid"` — no indifferent access, because there is nothing to be indifferent between.

## Migration

- **`STATUSES = [...].freeze` plus a symbol field** becomes an `enum` — a gem-supplied autocorrect at best, since the constant array carries no information about which field it governs. The vocabulary transfers verbatim; the spellings do not change.
- **Rails' `enum :status, [...]`** becomes a nested `enum Status` plus an ordinary `status` field. Two lines where Rails had one, deliberately (ADR 0022 §4).
- **Generated predicates do not port.** `purchase.paid?` has no equivalent; the rewrite is `case/in`, or `purchase.status == :paid` for a single test. This is the largest ergonomic loss for Rails code, and it is deliberate: a predicate chain is not exhaustiveness-checked (ADR 0022 §5).
- **Bang setters do not port.** `purchase.paid!` is Rails writing to a column. Portland has no mutation; the rewrite is `purchase.with(status: :paid)`.
- **`Purchase.statuses` does not port.** The enum is a type, not a lookup table (ADR 0022 §6).
- **Reading back a String does not port** — and this one is loud rather than silent, since comparing `:paid` to `"paid"` is a type error rather than Ruby's silent `false`.
