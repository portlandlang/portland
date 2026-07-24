# 0021 — Namespaces: `module`, `::` for names, always qualified

- **Status:** Accepted (not yet built)
- **Date:** 2026-07-24
- **Issue:** [#27](https://github.com/portlandlang/portland/issues/27) (carved out of the object-model umbrella)

## Context

Portland had **no namespacing at all**: `::` did not parse, `module` was not
a keyword, and `MAX = 5` was an ordinary binding. Everything lived in one
flat top-level namespace.

That is the largest gap in the object model by usage volume. In the
ruby_research corpus (n=500), `constant_path_node` — the `Foo::Bar` form —
appears **44,284 times across 78.0% of gems**, and `module_node` appears in
**87.4%**, edging out `class_node` at 84.8%. By contrast `super_node` is a
**19.2%** minority: namespacing is far more load-bearing in real Ruby than
inheritance is.

The demand is concrete rather than speculative:

- **Enums need a home.** `Purchase`'s status vocabulary must be
  distinguishable from `User`'s. This blocks enums, which block symbols,
  which block `{name: "pdx"}` hash shorthand — table stakes.
- **`parser.pdx` already crowds the top level** with ~40 node structs.
- **Any package ecosystem** needs collision-free names.

## Decision

### 1. `module` is a namespace keyword — namespace only

```ruby
module Statistics
  struct Summary
    mean
    median
  end

  def mean(values)
    values.sum / values.length
  end
end
```

Ruby's `module` does two unrelated jobs: **namespace** and **mixin**. Only
the first is demanded by anything currently blocked, so Portland takes the
namespace job now and defers mixins entirely to the object-model session.

That split is a dividend of deciding in this order, and it is one Ruby
cannot retrofit. Ruby has **no way to bring a name into scope without mixin
machinery** — `include Math` to avoid typing `Math.` recruits the ancestor
chain to solve a namespace problem, and `module_function` exists to paper
over the mismatch. So a Ruby reader seeing `include Foo` cannot tell
`include Comparable` (real behavior inheritance) from `include Math`
(typing convenience). Portland's mixin keyword, whenever it arrives, will be
a different word by construction.

### 2. `::` names, `.` invokes

```ruby
Shapes::Circle                   # naming — reach into a namespace
Statistics::PI                   # naming — a constant
Statistics.mean(readings)        # invoking — call it
Shapes::Circle.new(radius: 5)    # both, in order
```

The line is **naming versus invoking**, not compile-time versus runtime —
a module function call is statically resolved in Portland too, so that
framing would have been imprecise.

This is Ruby's own convention (`Math::PI` vs `Math.sqrt(4)`), which
Rubyists apply reflexively. Ruby leaves it as style and permits
`Foo::bar()`; Portland makes it a rule, so `Statistics::mean(x)` is a
never-guess error naming the fix. It is also already consistent with
Portland as built: `Token.new(kind:, text:)` uses `.` today.

### 3. Names are always fully qualified

`Statistics.mean(data)` at every call site. There is **no import
mechanism**, no namespace aliasing, and no way to inject names into scope.

Lexical nesting is the only shortening: inside `module Statistics`, its own
names are written bare. This is Ruby's answer to verbosity too.

Rejected, and why:

- **Namespace aliasing** (`import numpy as np`) — the abbreviation hides
  where a name lives.
- **Wholesale injection** (`use Statistics` making `mean` bare) — recreates
  exactly the question never-guess exists to kill: *where did this name come
  from?* It also fights no-shadow, since injected names collide with locals.
- **Selective import** (`import Statistics::mean`) — same objection, smaller
  blast radius. Ruby has no equivalent at all.

Evidence that qualified paths are not felt as painful: 44,284 `::` paths
across 78% of gems, with `include`-for-convenience now dated style.

**Asymmetry noted deliberately:** an import can be added later if the
`evaluator.pdx` case (≈40 node types from `parser.pdx`) genuinely hurts. It
cannot be removed once code depends on it. Start restrictive.

### 4. Both declaration forms, semantically identical

```ruby
module Portland
  module Compiler
    struct Token
      kind
    end
  end
end

module Portland::Compiler       # identical meaning
  struct Token
    kind
  end
end
```

Ruby offers both and they **behave differently** — a genuine footgun:

```ruby
module A
  LIMIT = 10
  module B
    LIMIT      # 10 — nesting is [A::B, A]
  end
end

module A::B
  LIMIT      # NameError — nesting is only [A::B]
end
```

Portland allows both spellings and gives them **one behavior**: lexical
scope always includes every enclosing level, however the namespace was
declared. Ruby's `Module.nesting` asymmetry is an artifact of its
constant-lookup implementation, not a semantic worth reproducing.

Keeping the path form matters because **namespace depth is forced, not
chosen**. Every library must namespace under its own name or collide —
`Nokogiri::XML::Document`, `Faraday::Adapter::NetHttp`,
`Aws::S3::Client`, `RSpec::Core::ExampleGroup`. Two to three levels is the
floor for ecosystem code. Block-form-only would cost three indents before
any code in every file of such a library; the path form exists precisely to
answer that, and the trap it carries in Ruby is separable from the syntax.

(An earlier draft justified block-form-only by asserting Portland's
namespaces would be shallow. That was a prediction used as a reason, and it
was wrong: the flat examples were all *stdlib*, which is privileged to claim
top-level names. Third-party packages never are.)

### 5. Types nest in types; modules do not nest in types

```ruby
struct Purchase
  status
  total

  enum Status              # sketch — enums are the next session
    :pending
    :paid
  end
end

struct Invoice
  lines

  struct Line              # Ruby allows this; so does Portland
    description
    amount
  end
end
```

One rule, no carve-out — "only enums may nest" would be a special case to
remember for no gain. A module inside a struct is a **compile error**:
modules group things, a struct is a thing.

### 6. Falls out

- **Files and modules are unrelated.** `require_relative` loads a file,
  exactly as in Ruby. No path-to-namespace convention — Rails only gets that
  from Zeitwerk, a library.
- **No root module.** Top level is top level; there is no `Object`
  equivalent to reach through.
- **Constants need no new concept.** In Ruby, constants exist as a distinct
  thing *because variables are mutable by default*. ADR 0001 made everything
  immutable by default, so `MAX = 5` is already unrebindable. What remained
  was only *where the name lives* — which is what this ADR answers.

## Consequences

- Build: `module` keyword, `::` as a two-character token, namespace
  resolution in the seed, then threaded through the trio. The trio's own
  ~40 node structs become the first real test.
- `Statistics::mean(x)` and modules-inside-structs join the never-guess
  error family.
- Unblocks the enum session, which unblocks symbols, which unblocks
  `{name: "pdx"}`.
- Deliberately **not** decided here: mixins, inheritance, visibility,
  `class << self`, classes-vs-structs (the object-model session, which wants
  the class-shape census), and generics.
- Migration: `Foo::Bar` and `Foo.bar()` compile verbatim when they follow
  Ruby's own convention. `include Foo` for namespace convenience has no
  Portland equivalent — the fix is to qualify. `module Foo::Bar` compiles
  verbatim *and* behaves less surprisingly than it does in Ruby.
- Ledger: `docs/ruby/namespaces.md`.
