# The superset

**Summary:** The inventory of Portland grammar Ruby cannot parse — the locked-until-flip tier, in one place.

**Status:** an index, verified by running `ruby -c` on Ruby 4.0.6 (2026-08-19) rather than by intuition — which the probe promptly corrected twice, see the last section. Each entry's story lives in its owner doc; this page only points (no fact has two homes).

## Why this list matters

The [polyfill promise](README.md) grades every difference: _free_ (valid Ruby, same meaning), _gem-supplied_ (valid Ruby grammar, a gem adds the behavior), _locked-until-flip_ (grammar Ruby cannot parse). This page is the third tier's roll call: the forms below are the exact places a Portland file stops being loadable by a Ruby interpreter, and therefore the cost of the flip. Everything not listed here parses under Ruby today.

## Grammar Ruby refuses, Portland speaks

- **`struct` declarations** — data-with-behavior, fields first ([classes.md](classes.md), ADR 0028/0031).
- **`trait` declarations** — behavior bundles carried by `include` ([mixins-and-inheritance.md](mixins-and-inheritance.md), ADR 0028).
- **`enum` declarations** — closed symbol vocabularies, payload cases included ([enums.md](enums.md), ADR 0022).
- **Payload-carrying case construction** — `:paid(on: "tuesday")` ([enums.md](enums.md)).
- **Payload-carrying case patterns** — `in :paid(on:)` ([pattern-matching.md](pattern-matching.md)).
- **The one-line `case/in`** — `case status in :paid then "receipt" end`; Ruby's subject eats the `in` as a match test and errors ([pattern-matching.md](pattern-matching.md), ADR 0042).

Committed but unbuilt forms stay off this list until they exist; when the ternary (#83) lands it will be Ruby-parity, and the `# -> Type` annotations (ADR 0041) were chosen precisely because they never join this page.

## Two forms that surprisingly do NOT belong here

Both were assumed superset and probed otherwise — Ruby parses each as an ordinary method-call reading, which is what makes their polyfills possible:

- **`mutable count = 0`** parses in Ruby as `mutable(count = 0)` — a method call taking an assignment. Gem-supplied tier: a polyfill `mutable` can accept and unwrap exactly that ([mutability.md](mutability.md)).
- **`together do ~ a = 1 end`** parses in Ruby as a `together` block whose lines call unary `~` on assignments. Gem-supplied tier, remarkably ([concurrency.md](concurrency.md)).

## Migration

- A codebase that avoids the six forms above runs under both interpreters for its whole migration; each form adopted is a one-way door through the flip.
- The list can only be *appended to knowingly* — any ADR adding grammar Ruby cannot parse should add its line here, which keeps the flip cost a maintained number rather than an archaeology project.
