# 0027 — The object model: structs gain single inheritance

- **Status:** Tentative (one of two competing drafts for [#27](https://github.com/portlandlang/portland/issues/27) — this and structs-and-traits; the branches not merged close unmerged)
- **Date:** 2026-07-27

## Context, shared by both drafts

Structs have fields, methods, value equality, `with`, and nesting — and no way to **share behavior**. The pull is the trio's own code: every AST node struct renders itself, and the helpers they share (`sexp_list`, the escape logic) sit at the top level because there is nowhere else. The full-session questions: inheritance or not, mixins, everything-is-a-struct vs classes, visibility, constants, and a migration story for `class << self` (17/50 gems).

Both drafts answer the same worked example — the trio's nodes sharing their rendering helpers — so they read side by side.

## Decision (this draft)

**Structs gain single inheritance and nothing else.** A struct may name one parent; it gains the parent's fields and methods, may add its own, and may override. No mixins, no `class` keyword, no multiple anything — Ruby's spelling for the one relationship, with the parts that made Ruby's version treacherous cut at the door.

<!-- not-portland: this draft's proposed syntax; nothing here is built -->

```ruby
struct Node
  position

  def sexp_list(nodes)
    nodes.map { it.sexp }.join(" ")
  end
end

struct ArrayNode < Node
  elements

  def sexp
    "(array #{sexp_list(elements)})"
  end
end

struct HashNode < Node
  pairs

  def sexp
    "(hash #{sexp_list(pairs)})"
  end
end
```

The rules, and what each cuts from Ruby:

- **Fields concatenate, parent's first**; `new` takes them all as keywords, `with` still copies. A parent may be field-less (relaxing today's at-least-one-field rule for parents only).
- **`in Node` matches any descendant** — the type pattern gains ancestry, which is what makes an array of nodes dispatchable. Exhaustiveness over a parent's *known* descendants becomes checkable (#9), because the hierarchy is closed per program — there is no runtime subclassing.
- **No `super`.** An override replaces; a method wanting the parent's version calls it by qualified name (`Node.sexp_list(...)`-shaped, exact spelling open). `super`'s implicit-arguments trick — Ruby's spookiest call — does not port.
- **Value equality includes the concrete type**: an `ArrayNode` never equals a `HashNode` however their fields align, and never equals a bare `Node`.
- **One parent, structs only** — no mixins in this draft; capability-modules migrate as parent structs or stay top-level functions.

## The trade, stated plainly

**Bought:** subtype polymorphism — the thing the trio's evaluator fakes with a 30-arm `case/in` today. An array of `Node`s is a real concept, `in Node` dispatches it, and the 17/50 gems with real hierarchies port their shape directly. Ruby's spelling (`<`) reads at sight.

**Spent:** the flat world — reading a struct now means reading its chain; value semantics carry a type-identity subtlety (`with` on a child stays a child, equality is type-strict); and the no-shadow ladder gains a rung *per ancestor*, which is exactly the readability tax the language has spent 24 ADRs refusing. The draft caps it — single parent, no super, closed hierarchies — but the tax exists at every depth.

## Interactions

- **#9 inference:** ancestry-aware type patterns and closed-hierarchy exhaustiveness are genuinely new inference surface — the biggest #9 cost of the two drafts.
- **#38 (`inspect`) / #28 (errors):** a base `Error` struct gives rescue-by-pattern something ancestry-shaped to filter after all — this draft quietly re-opens that door in whichever #28 variant lands.
- **The trio:** the worked example flattens its helper soup into a `Node` parent; the evaluator's dispatch `case` could shrink arm by arm as behavior moves onto nodes — the larger payoff, and the larger rewrite.
