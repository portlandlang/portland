# 0028 — The object model: structs stay the only type; traits carry shared behavior

- **Status:** Accepted (chosen for [#27](https://github.com/portlandlang/portland/issues/27) over single inheritance, whose draft closed unmerged; amended in review — the verb is `include`. A decision — the build follows as its own arc.)
- **Date:** 2026-07-27

## Context, shared by both drafts

Structs have fields, methods, value equality, `with`, and nesting — and no way to **share behavior**. The pull is the trio's own code: every AST node struct renders itself, and the helpers they share (`sexp_list`, the escape logic) sit at the top level because there is nowhere else. The full-session questions: inheritance or not, mixins, everything-is-a-struct vs classes, visibility, constants, and a migration story for `class << self` (17/50 gems).

Both drafts answer the same worked example — the trio's nodes sharing their rendering helpers — so they read side by side.

## Decision (this draft)

**Structs stay the only concrete type. Shared behavior lives in traits: bundles of methods a struct includes.** There is no inheritance, no `class`, no subtyping — a struct is exactly its declaration, plus the methods its traits contribute. ADR 0021 §5 already promised the load-bearing half of this shape: mixins were deferred with the note that they would get their own **declaration** keyword, so `module` (a namespace, nothing else) can never blur into a mixin the way Ruby's one keyword serves both masters.

**The verb is Ruby's `include`, and that is safe *because* the noun is new.** Ruby's confusion was never the word `include` — it was that `include Comparable` (mix in behavior) and `include Math` (inject a namespace) are the same operation on the same kind of thing. With `trait` and `module` as distinct declarations, the never-guess check moves to the operand and the confusion cannot be spelled: `include Describable` mixes in a trait; `include Statistics` is a refusal with the rewrite named — a namespace is never injected, write `Statistics.mean(...)`. Forty years of muscle memory kept, at the cost of nothing.

<!-- not-portland: this ADR's decided syntax; nothing here is built -->

```ruby
trait Sexpable
  def sexp_list(nodes)
    nodes.map { it.sexp }.join(" ")
  end
end

struct ArrayNode
  elements

  include Sexpable

  def sexp
    "(array #{sexp_list(elements)})"
  end
end

struct HashNode
  pairs

  include Sexpable

  def sexp
    "(hash #{sexp_list(pairs)})"
  end
end
```

The rules, each an application of an existing one:

- **A trait has methods only** — no fields, no state; a struct's shape stays entirely in its own declaration, so `with`, value equality, and construction are untouched.
- **Trait methods resolve after the struct's own** — locals → fields → own methods → trait methods → top-level, extending the existing ladder by one rung; a collision between two carried traits is a **no-shadow refusal naming both**, never Ruby's silent last-include-wins.
- **`include` is a declaration, not a call** — it appears in struct bodies only, it is not a runtime operation, and there is no `Module.ancestors` to inspect: which traits a struct includes is compile-time information, like its fields. Same spelling as Ruby; none of the mechanics.
- **Type patterns stay concrete** — `in ArrayNode` works as today; whether `in Sexpable` (match by capability) exists is deferred until #9 can check it, noted rather than smuggled.
- `trait` as the declaration word is settled with the shape; it reads right and no Ruby meaning collides with it.

## The trade, stated plainly

**Bought:** the flat world survives — every struct's full method set is readable from its declaration plus a finite trait list, no ancestor chains to walk, no `super`, no diamond. Value semantics stay trivially correct because nothing about representation is shared. This is the draft that changes the *least* language.

**Spent:** no subtype polymorphism — an array of "anything Sexpable" is just an array, and dispatch is structural (`case/in` on concrete types) or trait-method calls on values you already know carry it. `class << self` migrates to module functions (namespaces already do this work); Ruby's `Comparable`/`Enumerable` migrate as traits naturally, but Ruby *inheritance* hierarchies must flatten — the 17/50 census understates that cost because it counts singleton classes, not subclassing.

## Interactions

- **#9 inference:** trait-carrying is a static fact; "does this value's type carry Sexpable" is answerable without runtime tags, and exhaustiveness over concrete structs keeps working.
- **#38 (`inspect`) and #28 (errors):** both fit as traits the way `Comparable` does — a default carried rendering, a `Failure` trait — which is why those deferrals pointed here.
- **The trio:** the worked example is its own code; adopting this draft turns its top-level helper soup into two or three traits with no semantic change, a mostly mechanical pass.
