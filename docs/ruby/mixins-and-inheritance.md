# Sharing behavior: single inheritance, nothing else

**Summary:** `struct Child < Parent` survives with Ruby's spelling; mixins, `super`, runtime subclassing, and `class << self` do not.

**Status:** tentative ([ADR 0027](../adr/0027-2026-07-27-object-model-single-inheritance.md), one of two competing drafts for [#27](https://github.com/portlandlang/portland/issues/27)).

## Ruby

<!-- not-portland: Ruby, shown for contrast -->

```ruby
class ArrayNode < Node
  include Sexpable        # mixins interleave into the ancestor chain

  def sexp
    super + suffix        # super, with invisible implicit arguments
  end
end
```

## Portland (this draft)

<!-- not-portland: proposed syntax from a tentative ADR; nothing here is built -->

```ruby
struct ArrayNode < Node
  elements

  def sexp
    "(array #{sexp_list(elements)})"
  end
end
```

One parent, fields concatenated, overrides replace. `in Node` matches any descendant, and hierarchies are closed per program, so exhaustiveness over descendants is checkable where Ruby's open classes made it meaningless.

## Migrating

- `class Child < Parent` — becomes `struct Child < Parent`, shape intact; this is the 17/50's direct port.
- `include`/`extend`/`prepend` — gone; capability modules become parent structs, top-level functions, or duplication.
- `super` — gone; call the parent's method by qualified name, arguments written out. The implicit-arguments form has no equivalent, deliberately.
- Runtime subclassing (`Class.new`) and reopening — gone with the rest of runtime metaprogramming.
- `class << self` — becomes module functions, as today.
- Equality is type-strict across the hierarchy: a child never equals its parent, which Ruby's `==` sometimes allowed by accident.
