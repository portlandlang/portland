# 0075 — `attr_reader` is a spelling of the field list

- **Status:** Accepted (ruled by the deciding user 2026-11-03, accepting the recommendation in [#134](https://github.com/portlandlang/portland/issues/134); not built — the build is [#188](https://github.com/portlandlang/portland/issues/188))
- **Date:** 2026-11-03
- **Issue:** [#134](https://github.com/portlandlang/portland/issues/134)

## Context

A Portland struct lists its fields bare, one name per line. Ruby's `attr_reader :size, :label` in a class body declares readers for instance variables, and 36% of gems in a 300-gem sample write it; today it fails with an undesigned parse error, and the retired-spelling table says `'attr_reader' has no Portland meaning — a field is read by its bare name`, which is true of the *read* and says nothing about the *declaration*.

[ADR 0056](0056-2026-09-27-class-is-a-spelling-of-struct.md) made `class` a spelling of `struct` on the principle that one behavior may have many spellings (principle 3). This is the same move one line down.

## Decision

1. **`attr_reader :size, :label` in a struct or class body declares the fields `size` and `label`**, exactly as the two bare lines would. The two spellings may be mixed in one body; a field named twice refuses as it does today.
1. **`attr_writer` and `attr_accessor` stay out.** They make setters, and a setter mutates; where Ruby's mutable instance state goes is [#103](https://github.com/portlandlang/portland/issues/103)'s. Their refusals keep naming that.
1. **A field is a constructor keyword**, whichever way it is spelled. Ruby's `attr_reader` declares no `initialize` parameter; Portland's field is always one, so a class that read `attr_reader :size` and set `@size` in `initialize` migrates as [classes.md](../ruby/classes.md) already says every class does: `initialize` becomes `def self.new` with `fields(...)`.

## Consequences

- The parser on both implementations reads `attr_reader` in a type body as a field list; outside one it keeps refusing by name.
- The retired-spelling table's `attr_reader` row changes from "has no Portland meaning" to the field-declaration reading.
- Ledger: [classes.md](../ruby/classes.md).
