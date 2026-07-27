# Sharing behavior: traits, not modules or superclasses

**Summary:** `include`/`extend`/`prepend` and subclassing are gone; a struct `carries` traits — method bundles with no state, resolved after the struct's own methods, collisions refused by name.

**Status:** tentative ([ADR 0027](../adr/0027-2026-07-27-object-model-structs-and-traits.md), one of two competing drafts for [#27](https://github.com/portlandlang/portland/issues/27)).

## Ruby

<!-- not-portland: Ruby, shown for contrast -->

```ruby
module Sexpable
  def sexp_list(nodes) = nodes.map(&:sexp).join(" ")
end

class ArrayNode < Node
  include Sexpable
end
```

One keyword (`module`) serves namespacing and mixing-in; `include` is a runtime call; method conflicts resolve silently by ancestor order; `class << self` opens a singleton for good measure.

## Portland (this draft)

<!-- not-portland: proposed syntax from a tentative ADR; nothing here is built -->

```ruby
trait Sexpable
  def sexp_list(nodes)
    nodes.map { it.sexp }.join(" ")
  end
end

struct ArrayNode
  elements

  carries Sexpable
end
```

Namespaces and mixins can never be confused, because they never shared a keyword (ADR 0021 kept `module` a namespace and nothing else, promising mixins their own word).

## Migrating

- `include Comparable`-style capability modules — become traits, nearly verbatim; methods-only modules port cleanly.
- Modules with state (`@memo` in a mixin) — do not port; state belongs to the struct that declares it.
- Subclassing — flattens: shared behavior into traits, shared *shape* into composition or duplication. This is the real cost, and this draft pays it on purpose.
- `class << self` (17/50 gems) — becomes module functions; namespaces already invoke with `.`.
- Conflicting methods from two mixins — no longer silent: carrying both is a refusal naming the collision.
