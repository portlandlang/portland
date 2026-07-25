# 0022 — Enums: closed vocabularies of symbol cases, with keyword payloads

- **Status:** Accepted (not yet built)
- **Date:** 2026-07-25

## Context

Portland could describe a value's shape but never its **set**. Structs give named records with payloads, but a struct is an open universe: nothing declares that a purchase's status is one of exactly three things. ADR 0013 promised compile-checked exhaustiveness for `case/in`, and then had nothing closed to check it against.

Symbols were tabled on 2026-07-23 for precisely this reason ([session notes](../history/2026-07-23-symbols-first-pass.md)). That audit found Portland's existing ADRs had already reassigned every job Ruby's symbols do — kwarg labels are compile-time (0014), struct-pattern labels likewise (0013), metaprogramming is dropped, `&:sym` becomes `{ it.upcase }` (0017) — leaving exactly one residue: **enum-ish values**. It also found the blocker: naming an enum type from outside the context that implies it needs namespacing, and Portland had none. [ADR 0021](0021-2026-07-24-namespaces-and-modules.md) removed that blocker.

**The frame worth naming, because it misleads.** Rails' `enum` is not what Rust and Swift call an enum. Rails' is *column sugar* — `enum :status, [:active, :archived]` declares a column's vocabulary, always inside a model, reading back a String, with no payloads and nothing you can pass around. A Rubyist arriving from Rails has never wanted a standalone enum because Rails offers no way to write one; the gap is filled with a frozen constant array or a value object instead.

Portland's own compiler shows both shapes, which is what settled the nesting question:

- `Token`'s kinds — `identifier`, `integer`, `keyword`, `operator`, `string`, `float`, `newline`, `error`, `word_array` — belong to `Token` and nothing else. The Rails-shaped case.
- `parser.pdx` carries **45 node kinds across 46 structs**. That vocabulary discriminates *which node type you have*; it spans every struct and is owned by none. There is no model to nest it in.

## Decision

**`enum` declares a closed vocabulary. Its cases are lowercase symbols, and a case may carry a keyword-only payload.**

<!-- not-portland: enums are decided but unbuilt — `:foo` does not lex yet -->

```ruby
enum Ordering              # top-level: owned by no type
  :less
  :equal
  :greater
end

struct Purchase
  amount
  status

  enum Status              # nested: owned by Purchase (ADR 0021, types nest in types)
    :pending
    :paid(on:)
    :refunded(on:, reason:)
  end
end

purchase = Purchase.new(amount: 40, status: :paid(on: "tuesday"))

case purchase.status
in :pending           then "not paid yet"
in :paid(on:)         then "paid #{on}"
in :refunded(reason:) then "refunded — #{reason}"
end
```

### 1. Payloads are in, and they are keyword-only

A case may carry data of its own: `:refunded(on:, reason:)`. Positional payloads are refused — struct construction is already kwargs-only and struct patterns are already keyword-only, so this introduces no new rule, only a new thing for `case/in` to match. A positional form would be the one place in Portland where you count arguments to learn what they mean.

The payload-free case is the degenerate form and costs nothing: `:pending` reads exactly as it would in a design without payloads. `Ordering` above is payload-free throughout.

**Why one feature rather than two.** Portland can already express payload-carrying variants — `struct Ok` / `struct Error` plus `case/in` struct patterns runs today. What it cannot express is *closedness*, so nothing checks that both were handled. Building the exhaustiveness checker for names-only would mean building it a second time the first time anything wants an exhaustive set of payload-carrying cases — `Result` in [#28](https://github.com/portlandlang/portland/issues/28) being the obvious customer. The checker gets written once.

Note the asymmetry, which mirrors ADR 0021's import argument: deciding payloads **in** does not decide #28 — exceptions remain available, and `Result` is then simply an enum nobody reaches for. Deciding payloads **out** partly forecloses it, because the typed-result option would arrive without exhaustiveness, which is the guarantee that makes it worth having.

### 2. Where an enum lives follows ADR 0021, with nothing new

Nested when the vocabulary is owned by one concept (`Purchase::Status`), top-level when it is not (`Ordering`). This is the existing rule for types nesting in types, unchanged — `Purchase::Status` is `Invoice::Line` with a different payload.

`::` names, `.` invokes, exactly as before. `Purchase::Status` names the type; `purchase.status` reads the value. `purchase::Status` is meaningless, since `::` addresses types and namespaces while `purchase` is a value.

In practice `Purchase::Status` is rarely written. Day-to-day code says `purchase.status` and never names the enum at all; the type surfaces only where it must be named, such as a parameter annotation at a public boundary (#9).

### 3. Cases are constructed bare, with the type from context

`Purchase.new(status: :paid(on: "tuesday"))` — not `Purchase::Status.paid(...)`. The expected type comes from the field, so the case reads as a name rather than a constructor call.

This is the ergonomic Swift gets from its leading dot (`status = .paid`), reached with a symbol instead. The session notes rejected the leading dot on a technical ground rather than taste: Portland has leading-dot method chaining across lines, so `.paid` occupies the same visual slot as a chained call whose receiver sits on the previous line.

### 4. Declaration is explicit; there is no one-line form

`enum Name ... end` declares the vocabulary. The field is declared separately, as an ordinary struct field.

Rails collapses the two — `enum :status, [:active, :archived]` declares column and vocabulary together — and a Portland equivalent (`enum status: [:pending, :paid]`) is deliberately **not** taken in this ADR. It is sugar over the explicit form, and sugar can be added later while the general form cannot be removed. It would also need a derive rule turning a field named `status` into a type named `Status`, which is a convention the compiler would have to guess at; and it has no way to express `Ordering`, so the explicit form would have to exist underneath it regardless.

### 5. No generated predicates

Rails generates `conversation.active?`, and it is the ergonomic Rubyists reach for most. Portland does not generate it, in the first cut.

The reason is exhaustiveness. `case/in` is the *checked* path: add a case to an enum and every `case` over it fails to build until the new case is handled. A predicate chain is unchecked — `if purchase.paid?` ... `elsif purchase.pending?` silently keeps compiling when a fourth case appears. Generating predicates would hand users an ergonomic alternative that opts out of the guarantee the feature exists to provide.

Revisitable as sugar once there is real Portland code to look at. This is the call in this ADR held with least confidence.

### 6. The enum's name is a type, not a value

There is no `Status.all`, no `Status.cases`, and no `Purchase.statuses` lookup hash. Rails has those because its enum is a column with a mapping table behind it; Portland's is a type. Enumerating a type's cases at runtime is `is_a?`-adjacent, and ADR 0013 already established that type predicates are patterns rather than a reflection API.

If iterating cases turns out to be wanted, it returns as a deliberate feature with its own reasoning, not as a side effect of declaring an enum.

## Consequences

- **Symbols are unblocked.** The core symbol question was decided on 2026-07-23 and left without an ADR pending this shape; that ADR can now be written. `{name: "pdx"}` hash shorthand follows it, which retires the most common modern Ruby hash form currently being a parse error.
- **ADR 0013's exhaustiveness gets its first real customer.** Until now no vocabulary was closed, so the promised check had nothing to check. Enums are the closed set the checker was specced against.
- **#28 keeps both doors open.** `Result` as `:ok(value:)` / `:error(reason:)` is expressible if the error-handling session wants typed results; exceptions remain equally available. The relevant corpus signal, from the 500-gem census, is that Ruby's exceptions are swallowed roughly 2:1 over being re-raised — 1,172 sites across 28.8% of gems — which is a "loud, never silent" violation at scale, and the strongest argument for the typed option. That decision is #28's, not this one's.
- **Nothing here is built.** The lexer has no symbol literal — `:foo` lexes as a bare `Colon` today — so symbols land first. Exhaustiveness checking is static and belongs to #9; the seed will preview it as a runtime panic on an unmatched case, exactly as `case/in` does now.
- **Deferred by name:** generated predicates (§5), the Rails one-line declaration (§4), iteration over cases (§6), and whether an enum case is assignable where a plain symbol is expected — a widening question that belongs to inference (#9).
- **Evidence caveat.** The corpus queries the symbols notes asked for — the symbol residue, enum-ish vocabularies per class, payload-free versus payload-carrying shape — have not been run; a full-corpus re-run is in flight. Ruby has no enums, so those queries can only measure proxies (`[:ok, value]` tuple returns, symbol-headed arrays in `case/in`) and would inform rather than decide the payload question. Recorded as a prior, in ADR 0019's sense, open to revision on evidence.
- Ledger: [`docs/ruby/enums.md`](../ruby/enums.md).
