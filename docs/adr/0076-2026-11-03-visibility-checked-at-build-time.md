# 0076 — Visibility: `private`, `protected`, and `public` are Ruby's, checked at build time

- **Status:** Accepted (ruled by the deciding user 2026-11-03, accepting the recommendation in [#133](https://github.com/portlandlang/portland/issues/133); not built — the build is [#189](https://github.com/portlandlang/portland/issues/189); the edges named below are expected to sharpen in the build)
- **Date:** 2026-11-03
- **Issue:** [#133](https://github.com/portlandlang/portland/issues/133)

## Context

Visibility was undecided on purpose — [language.md](../language.md) listed it under "not yet designed" since [#27](https://github.com/portlandlang/portland/issues/27) closed — waiting to be pulled for. The pull is the corpus: 55% of gems in a 300-gem sample write `private`, `protected`, or `public`, so the keywords stand between most gems and running. Today `private` in a type body is read as a field named `private`, which is a bug wearing the undecided question's clothes.

Ruby checks visibility at the call, at runtime: `box.secret` raises `NoMethodError: private method 'secret' called`. The information is all static — which methods a section marks, who the receiver is — so Portland can refuse the call at build time, where every other wrong call is refused ([ADR 0034](0034-2026-08-11-the-checker-and-the-oracle-succession.md)).

## Decision

1. **`private`** makes the methods after it, to the end of the body or the next visibility word, callable only without an explicit receiver: from the struct's own methods, and from the methods of a trait it includes. **`private def name`** marks one method. A call with a receiver from outside — `box.secret` — refuses at build time, naming the method and its type: `'secret' is private to Box`.
1. **`protected`** is `private` plus a call on another value of the same type from inside the type's own methods — `other.secret` inside `==` or `<=>`. Ruby's rule, and the one Portland's value-equality code wants.
1. **`public`** ends a `private` or `protected` section. Methods before any visibility word are public, as Ruby's are.
1. **`private_constant :NAME`** makes a module's or struct's constant ([ADR 0053](0053-2026-09-24-constants.md)) unreadable from outside its namespace; the read refuses at build time.
1. **`module_function` refuses**, naming `def self.`: a module's `def` is already a module function ([ADR 0021](0021-2026-07-24-namespaces-and-modules.md)).
1. **Type functions (`def self.`) are public.** Ruby's `private_class_method` is not taken until pulled for.
1. **No runtime cost.** Visibility is a build-time fact about a declaration; the seed, which has no checker, accepts every call, and the compiler refuses — the oracle succession's shape, as with enum membership.

## Consequences

- The parser on both implementations reads `private`, `protected`, and `public` in a type body as section markers, and `private def` as a one-method marker; the field-named-`private` bug ends here.
- The checker gains a visibility pass over method calls with receivers; its refusals are in [ADR 0047](0047-2026-09-22-the-error-voice.md)'s voice and pinned.
- Where a trait's method calls a private method of the including struct, it is inside the type (ruling 1); where a private method is called through `self.secret`, Ruby allows it since 2.7 and so does this.
- Ledger: a new [visibility.md](../ruby/visibility.md); `language/private_spec.rb`'s manifest row moves from the undecided reason to the build issue.
