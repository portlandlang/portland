# 0056 — `class` is a spelling of `struct`

- **Status:** Accepted (ruled by the deciding user 2026-09-27; built in the seed and the trio, differentially pinned, 2026-09-27, [#127](https://github.com/portlandlang/portland/issues/127)) — supersedes [ADR 0028](0028-2026-07-27-object-model-structs-and-traits.md) on the keyword only
- **Date:** 2026-09-27
- **Issue:** [#61](https://github.com/portlandlang/portland/issues/61), with job 4 split out as [#103](https://github.com/portlandlang/portland/issues/103)

## Context

ADR 0028 made structs the only concrete type and traits the carriers of shared behavior, and in passing removed the `class` keyword; the ledger's [classes.md](../ruby/classes.md) called that "declined, not deferred". Reviewing it on 2026-09-27 while scoping how to assess a gem for Portland readiness, the deciding user ruled that permanent removal was never the intent: with `class` gone, essentially no Ruby file runs unedited, whatever else Portland supports. The keyword is the entry point for nearly every gem.

The object model itself holds up. What removed the word was a sentence, not an argument: every job `class` did that Portland supports, `struct` already does.

## Decision

1. **`class Name ... end` is `struct Name ... end`.** Same body grammar — fields listed bare, methods, `include` of a trait, `def self.new` with `fields(...)`, nested types — and the same value it builds. Both parsers read `class` into the node `struct` produces, so everything downstream (the checker, inference, both evaluators, the S-expression dump) sees one construct under two spellings. This is principle 3's many-names-one-behavior, the same shape as `and`/`&&` and the builtin twins.
1. **Inheritance still refuses.** `class Token < Node` has no Portland meaning — hierarchies flatten into traits (ADR 0028, [mixins-and-inheritance.md](../ruby/mixins-and-inheritance.md)) — so the parser refuses it with a sentence naming the trait rewrite, in ADR 0047's voice. The wording is settled when it is built.
1. **Instance state is not decided here.** A class whose methods assign instance variables — `@position += 1` — is job 4 of `class`, which has had no open issue since #11 closed on 2026-07-27. It is [#103](https://github.com/portlandlang/portland/issues/103) now. Until #103 decides, an instance variable refuses, and the refusal points at the ledger page.
1. **ADR 0028 stands otherwise.** Structs remain the only concrete type, traits the only shared behavior, and there is still no subtyping. Only the sentence removing the keyword is superseded.

## Consequences

- Value-shaped Ruby classes — data plus methods, built once and not mutated — keep their `class` line when migrated; the migration linter (#36) no longer rewrites the keyword.
- The two walls between most real Ruby classes and Portland are now named and tracked in one place each: inheritance (declined, trait rewrite) and mutable instance state (#103, open).
- A per-gem readiness assessment can count `class` as runs-as-is for value-shaped classes and route the rest to exactly those two reasons.
- Built (#127): both lexers treat `class` as a keyword and both parsers read it into the struct node; `spec/struct/class_spec.pdx` pins `class` and `struct` answering the same values. The refusals' wordings, settled at build, pinned on both implementations:
  - `'class Token < Node' inherits, and Portland has no inheritance — move Node's shared methods into a trait and 'include' it` (spelled with `struct` when `struct` declared it);
  - `'class << self' has no Portland meaning — write each method as 'def self.name' in the type's body`;
  - `'@count' is an instance variable, which Portland does not have — a field is read by its bare name, 'count'`;
  - `'@@total' is a class variable, which Portland does not have — a value lives in a local, a field, or a constant`.
- Ledger: [classes.md](../ruby/classes.md) says the keyword is a spelling of `struct` and points job 4 at #103.
