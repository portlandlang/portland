# 0074 — `value.class` answers the type

- **Status:** Accepted (ruled by the deciding user 2026-11-03, accepting the recommendation in [#148](https://github.com/portlandlang/portland/issues/148); built on both implementations the same day under [#187](https://github.com/portlandlang/portland/issues/187); one consequence found in the build: a type's bare name, `Token` or `Integer` on its own, is the type value too, since ruling 3's `value.class == Foo` needs it)
- **Date:** 2026-11-03
- **Issue:** [#148](https://github.com/portlandlang/portland/issues/148)

## Context

`self.class` and `value.class` refuse today — `expected method name after dot, got Keyword`, since `class` is a keyword — and that stops **5,021 gems** in the level-0 census. The uses are few and idiomatic: `self.class.name` in a message or an `inspect`, `value.class == Foo` as a type test, and `self.class.new(...)` to build another of the same type inside a method.

Portland already has a type as a name a program can use: `Token.new`, `Token.of(...)` ([ADR 0031](0031-2026-07-31-smart-constructors-definable-new.md)), `in Token` and `when Integer` as patterns, `Array.new` ([ADR 0058](0058-2026-10-16-builtin-types-carry-type-functions.md)). What it does not have is a class *object* — nothing to `allocate`, no `ancestors`, no `instance_methods` — and [metaprogramming.md](../ruby/metaprogramming.md) says why.

## Decision

1. **`value.class` answers the type**, the same thing the name `Token` means in `Token.new`: for a struct its type, for a builtin value `Integer`, `Float`, `String`, `Symbol`, `Array`, `Hash`, `Range`, `Set`, `Random`, `Boolean`. `nil.class` refuses as every method on nil does ([ADR 0006](0006-2026-07-22-absence-word-nil.md)).
1. **A type answers what it can answer statically.** `.name` is its qualified spelling, `"Token"` or `"Shop::Token"`; `.new(...)` and the type functions work as they do on the written name; `==` between types is identity; `inspect` is the name. Nothing else: no `ancestors`, `instance_methods`, `superclass`, `allocate`, or `method_defined?` — those are the reflection a closed runtime has none of, and they refuse by name.
1. **`value.class == Foo` works**, and the ledger names `value in Foo` as the Portland spelling without refusing the Ruby one.
1. **The checker knows the answer.** `value.class` has a static type everywhere `value` does, so `self.class.new(...)` inside a struct's method is checked as `Token.new(...)` is.
1. **Rejected:** refusing `.class` as metaprogramming. Nothing here can be modified at runtime, so the refusal buys nothing and costs 5,021 gems.

## Consequences

- The parser on both implementations takes `class` after a dot as a method name; the keyword stays a keyword everywhere else.
- Types become first-class values in a narrow way — the same way they already were as `Token` in `Token.new` — and the ledger says so in one place.
- Ledger: [classes.md](../ruby/classes.md) and [metaprogramming.md](../ruby/metaprogramming.md).
