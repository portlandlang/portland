# Sharing behavior: traits, not modules or superclasses

**Summary:** `include` survives — but it takes a `trait`, a stateless method bundle, never a module or a superclass; collisions are refused by name, and `extend`/`prepend`/subclassing are gone.

**Status:** decided ([ADR 0028](../adr/0028-2026-07-27-object-model-structs-and-traits.md)). Built in the seed and the compiler, differentially pinned — see [the language](../language.md#values).

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

One keyword (`module`) serves namespacing and mixing-in, so `include Comparable` (behavior) and `include Math` (namespace injection) are the same operation; method conflicts resolve silently by ancestor order; `class << self` opens a singleton for good measure.

## Portland

```ruby
trait Sexpable
  def sexp_list(nodes)
    nodes.map { it.sexp }.join(" ")
  end
end

struct ArrayNode
  elements

  include Sexpable
end
```

The verb is Ruby's; the safety is in the noun. `trait` and `module` are distinct declarations, so `include Statistics` — a namespace — is a refusal with the rewrite named: namespaces are never injected, write `Statistics.mean(...)`. Ruby's two meanings of `include` cannot be spelled.

## Migrating

- `include Comparable`-style capability modules — become traits, often **zero-character** at the include site: the module body moves under `trait`, and the `include` line does not change. The shim gem ([#36](https://github.com/portlandlang/portland/issues/36)) can even rehearse the strictness in Ruby: `extend Portland::Trait` on a module makes collisions raise at class-definition time.
- Modules with state (`@memo` in a mixin) — do not port; state belongs to the struct that declares it.
- `include Math`-style namespace injection — a refusal; write the qualified call.
- Subclassing — flattens: shared behavior into traits, shared *shape* written out per struct. This is the real cost, and ADR 0028 pays it on purpose.
- `class << self` (17% of gems at n=50) — becomes module functions; namespaces already invoke with `.`.
- Conflicting methods from two mixins — no longer silent: including both is a refusal naming the collision.
