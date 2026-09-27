# `class`: four jobs, four homes

**Summary:** `class` is a spelling of `struct`, and its four jobs have homes: data-with-behavior is a `struct` (either word), shared behavior is a `trait`, construction logic is `def self.` on the type with `new` definable, and stateful objects are an open question.

**Status:** decided ([ADR 0028](../adr/0028-2026-07-27-object-model-structs-and-traits.md), [ADR 0031](../adr/0031-2026-07-31-smart-constructors-definable-new.md), and [ADR 0056](../adr/0056-2026-09-27-class-is-a-spelling-of-struct.md), which brought the keyword back on 2026-09-27 — built the same day); jobs 1–3 built on both implementations, differentially pinned; job 4 open ([#103](https://github.com/portlandlang/portland/issues/103)). The ledger of the four jobs is [#61](https://github.com/portlandlang/portland/issues/61).

## Ruby

One keyword does everything:

<!-- not-portland: Ruby, shown for contrast -->

```ruby
class Token < Node                 # data + a hierarchy
  include Comparable               # mixed-in behavior

  def initialize(text)             # construction logic
    raise ArgumentError if text.empty?
    @kind = "word"
    @text = text
  end

  def self.of(text) = new(text)    # a type-level function

  def advance!                     # state that moves
    @position += 1
  end
end
```

## Portland

Each job has its own declaration, and the page still reads like Ruby:

```ruby
struct Token
  kind
  text

  include Comparable

  def self.of(text)
    Token.new(text)
  end

  def self.new(text)
    return failure("a token needs text") if text.empty?
    fields(kind: "word", text: text)
  end
end
```

1. **Data with behavior** — `struct`, the only concrete type: value equality, `with` for updated copies, methods after the fields (ADR 0028).
1. **Shared behavior** — a `trait`, included with Ruby's verb; hierarchies flatten, and [the mixins entry](mixins-and-inheritance.md) owns that story.
1. **Construction logic** — `def self.name` declares a type function; `new` is one of the definable names and replaces the raw constructor everywhere, any signature, value-or-`failure` (ADRs 0027, 0031). Inside it — and only there — `fields(...)` is the raw kwargs-in-fields-out layer, so a validating struct cannot be constructed around: the raw constructor has no spelling outside the body.
1. **Stateful objects** — the parser-with-a-position, the connection, the cache — have no home yet. Values thread through returns today (the compiler's own Outcome pattern). This was parked until `together` had semantics; it has them now (ADR 0029), and the question is open as [#103](https://github.com/portlandlang/portland/issues/103).

`class` itself is a second spelling of `struct` ([ADR 0056](../adr/0056-2026-09-27-class-is-a-spelling-of-struct.md)): `class Token` and `struct Token` declare the same type. Inheritance, `class Token < Node`, refuses with the trait rewrite named; `class << self` refuses naming `def self.`; and an instance variable, `@text`, refuses naming the bare field until #103 decides what instance state is.

## Migration

- `class Token` holding data keeps its `class` line (ADR 0056); the field list becomes explicit, and each `@text` read becomes a bare `text`.
- `initialize` becomes `def self.new`: its body moves whole, the `@field = value` tail becomes one `fields(field: value)` call, and `raise` becomes a returned `failure` the caller handles with the toolkit. Call sites do not change: `Token.new(...)`, positional arguments included, is a deliberate non-difference.
- A `def self.new ... super` override needs one edit — `super` to `fields(...)` — and `super` anywhere refuses with the rewrite named (there is no hierarchy to climb).
- `def self.of`-style constructors move unchanged, in structs and in modules both — a module-body `def self.` is accepted as a plain `def`.
- `class << self` blocks flatten to one `def self.` per function.
- Stateful classes are the honest cost today: rewrite as values threaded through returns, or wait for #103's answer before migrating that code.
