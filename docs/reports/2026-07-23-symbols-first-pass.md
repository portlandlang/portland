# Symbols: first pass

**Status:** the core question is **decided** (symbols exist — see below);
the enum mechanism they lean on is still being designed, so no ADR yet.
Corpus evidence is still wanted to affirm or overturn the priors marked as
such. This file is the session record.

## Decided 2026-07-23: symbols exist, as a general type

`:foo` is an ordinary symbol literal. Where a **declared closed vocabulary**
is expected, the compiler checks membership; everywhere else it is just a
symbol value:

```ruby
purchase.with(status: :paid)         # checked — must be a case of that enum
purchase.with(status: :pendign)      # compile error

config = {name: "pdx", port: 8080}   # ordinary symbol keys, Ruby-verbatim
config[:name]                        # works
config["name"]                       # compile error — string key, symbol-keyed hash
```

The reframe that unlocked this: **Ruby's problem with `status = :pending`
was never the syntax — it is that the set is open**, so `:pendign` is a
perfectly valid symbol. Declaring the set fixes that without touching the
spelling. "Typed" does not have to mean "not a symbol"; the type can be
declared and checked while the values stay spelled `:like_this`.

`{name: "pdx"}` is therefore **symbol-keyed**, Ruby-verbatim, and ships
regardless — it is table stakes (user's call: hashrockets are nobody's
favorite, inside Ruby or out).

### Tradeoffs accepted, explicitly

1. **Two string-like types.** Portland will have both `String` and `Symbol`,
   so users must choose between them — the Ruby wart this session opened by
   trying to avoid. It is *mitigated, not eliminated*: static types turn the
   classic mismatch into a compile error instead of Ruby's silent nil, so
   `HashWithIndifferentAccess` has nothing to paper over. But the choice
   still exists at every hash-key site.
2. **Open positions stay unchecked.** A symbol where no closed vocabulary is
   expected is not checked. `config[:nmae]` is a valid symbol that simply
   misses, yielding a maybe per ADR 0010. The uncheckedness is confined to
   exactly where Ruby is also unchecked, and ADR 0010 already forces the
   miss to be handled — but a typo there is still not a compile error.
3. **Rails' indifferent-access habits do not port.** Code that writes a
   symbol key and reads a string one (or vice versa) becomes a compile
   error rather than working. Loud, and mechanically fixable, but real
   migration work for Rails-shaped codebases.
4. **The "one job" purity is given up.** The audit below argued symbols
   would be a type with no unique remaining job. They now have two — enum
   case spelling and hash keys/labels — which is less pure and more Ruby.
   Deliberate.
5. **Rejected alternative:** symbols existing *only* as enum members
   (option A), which buys total checking at the cost of `{name: "x"}`
   meaning string keys.

### The escape hatch, for later

Tradeoff 2 looks recoverable for the common case without giving up
ergonomics. Where a hash literal's keys are statically visible, a lint (or
later, a type refinement in the TypeScript-object-type direction) could flag
a literal-key lookup that cannot match any key the hash was built with:

```ruby
config = {name: "pdx", port: 8080}
config[:nmae]      # lintable: no such key in a hash built right here
```

That recovers most of option A's safety while keeping option B's syntax.
Explicitly **not** part of this decision — recorded because it is the
direction to explore if tradeoff 2 bites.

### Rejected syntaxes for enum cases, and why

- **Capitalized cases** (`in Pending`) — makes enum cases visually
  indistinguishable from structs (and later classes) at a glance, when
  capitalization should tell you what kind of thing you are looking at.
- **Swift's leading dot** (`in .pending`) — rejected on a *technical*
  ground, not taste: Portland supports leading-dot method chaining across
  lines (verified: `"portland"` / `.upcase` / `.reverse` works today), so
  `.pending` occupies the same visual slot as a chained call whose receiver
  is on a previous line. Swift can afford that syntax because Swift has no
  leading-dot chaining.
- **`:symbol` cases** — chosen. The sigil distinguishes them from captures
  (`in pending` binds anything), cannot be confused with a method call, and
  reads as Ruby because it is Ruby.

## Why this is hard

Symbols look like an easy keep — they're beloved, and the corpus backs the
intuition: `symbol_node` appears in **80.6% of 500 sampled gems, 208,911
occurrences** (ruby_research `reports/latest/feature_usage.md`), 4th most
prevalent node type.

But that number counts every symbol use undifferentiated, and Portland's
existing ADRs have already reassigned nearly all of them.

## The audit: what job would a symbol still do?

| symbol's job in Ruby | Portland status |
|---|---|
| kwarg labels — `foo(name: "x")` | ADR 0014: compile-time labels, no symbol |
| struct pattern labels — `in Token(kind:)` | ADR 0013: compile-time |
| `send`, `respond_to?`, `define_method`, `attr_accessor`, `alias_method` | dropped forever (runtime metaprogramming) |
| `&:upcase` | deferred by ADRs 0016/0017 — `{ it.upcase }` covers it |
| hash keys — `{name: "x"}` | Portland hashes are `{"key" => v}`; string keys already work |
| **enum-ish values — `status = :pending`** | **unsolved** |

Every row but the last is spoken for. The residue is the enum job.

## Why Ruby has symbols — two rationales, both dead in Portland

**1. Symbols are the interpreter's identifier table, exposed as values.**
A bare Ruby 4.0.6 interpreter has **3,599 symbols interned before any user
code runs** (`Symbol.all_symbols.length`) — its own method, ivar, and
constant names. `send(:foo)` works because `:foo` *is* the key in the method
table. That is why symbols and metaprogramming are inseparable in Ruby.
Portland has no runtime method table, so this rationale has no referent.

**2. Interning works around mutable strings.** Verified on 4.0.6:
`:foo.object_id == :foo.object_id` is true; `"foo".object_id ==
"foo".object_id` is false; string literals are still not frozen by default.
Symbols give identity comparison, one allocation, and a key that can't
mutate after insertion. Portland's strings are already immutable (ADR 0015)
and an AOT compiler interns literals invisibly, so this rationale is also
gone.

**3. What survives: the semantic rationale.** A symbol says *"this is a
name, not data."* `"pending"` is text — it might be user input, displayed,
concatenated. `:pending` is an identifier, a member of a closed vocabulary
the program controls. This is the use–mention distinction, it's real, it's
language-independent, and it is almost certainly the source of the
affection.

## Prior art

- **Lisp/Scheme, Smalltalk** — foundational; symbols are what identifiers
  *are* in the AST / selector table. Ruby's lineage.
- **Erlang/Elixir atoms** — the closest sibling, and they carry exactly the
  job Portland has unsolved: `{:ok, value}` / `{:error, reason}` tagging is
  the core idiom. Note the cost of an open, untyped vocabulary: Erlang's
  atom table isn't garbage collected, making atom exhaustion a DoS vector.
- **JavaScript `Symbol()`** — false cognate. Unique opaque property keys,
  not interned names.
- **Rust, Swift, Haskell, OCaml, Go, Java, C#** — no symbols at all. Not an
  oversight: enums/sum types cover the closed-vocabulary job *and* add
  exhaustiveness, while interning becomes an invisible optimization.

**The middle ground worth studying:**

- **Swift's leading dot** — `status = .pending`. Symbol ergonomics (terse,
  no ceremony, reads as a name), but a typed enum case with exhaustiveness;
  the type comes from context so you never write `Status.pending`.
- **OCaml polymorphic variants** — `` `Pending ``, written with no prior
  declaration, and the type system *infers* the set. The closest existing
  thing to "symbols that typecheck."

OCaml's is the one to look hard at, because Portland already has inferred
types (#9): `status = :pending` with no declaration, vocabulary inferred,
exhaustiveness in `case/in` for free.

## How Rails does enums (ActiveRecord 8.1.3, read from source)

```ruby
class Conversation < ApplicationRecord
  enum :status, [:active, :archived]
end

conversation.active!         # bang setter
conversation.active?         # => true
conversation.status          # => "active"  ← a String, not a symbol
conversation.status = 1      # the DB integer also works
Conversation.active          # scope
Conversation.statuses        # => {"active" => 0, "archived" => 1}
```

Four things that shaped this session:

1. **Cases are declared as lowercase symbols** — `:active`, never `Active`.
   The Ruby-flavored spelling is lowercase, which is what killed the
   capitalized-case sketch.
2. **You read back a String.** Rails writes symbols, reads strings, accepts
   either plus the integer — the indifferent-access fudge.
3. **The dominant ergonomic is the generated predicate**, not case dispatch.
   Real Rails code branches on `conversation.active?`. Whether Portland
   generates predicates is a separate decision (user's call), but it is the
   part Rubyists actually love.
4. **It is not a type.** It is a column with sugar — no payloads, nothing
   you can pass around. `Conversation.statuses` is a hash on the class.
5. **It is scoped to the model.** A `Purchase` status and a `User` status
   coexist without collision. This killed the global `enum Status` sketch:
   "status" is exactly the kind of generic word many concepts want, and
   Portland's namespace is completely flat.

## The reframe

The session opened with the wrong question ("keep or kill symbols?"), found
a better one ("what carries the semantic job?"), and then found that the
framing itself was wrong. The error was treating *typed* as precluding
symbol **syntax** — it does not. See the Decided section above.

## Entanglement, and what is still open

The enum mechanism symbols lean on is **still being designed**. Open:

- **Payloads.** Rails-style enums are payload-free named values. But
  `Ok(value)` / `Error(reason)` for #28 needs payloads, and `:paid` reads
  beautifully payload-free while `:active(since: "today")` starts looking
  like a method call. **One feature or two?** — the live question.
- **Where enums are declared.** The Rails answer is "inside the concept
  that owns it," which for Portland means nested in a `struct`. That works
  for the common case, where every use site knows the expected type — but
  naming one from outside (`def notify(status:)` — what type?) needs
  namespacing, which Portland has none of (`::` does not parse, `module` is
  not a keyword, `MAX = 5` is an ordinary binding).
- Enum case *scoping* needs no new machinery — it is member access, the
  same shape `token.kind` already has. Enum **type naming** does. That
  distinction was gotten wrong once in session and is worth keeping
  straight.

This makes **#27's object model the keystone**: hash shorthand waits on
symbols, symbols wait on enums, and enums touch namespacing. Pointing the
class-shape census at #27 unblocks the most.

## What the corpus should measure next

Now that symbols are in, these affirm-or-overturn the priors rather than
decide the question:

1. **The residue** — symbol occurrences that are *not* hash keys, kwarg
   labels, metaprogramming DSL arguments, or `&:sym`. Sizes how much real
   code is doing the closed-vocabulary job that enums will now check.
2. **`&:sym` prevalence** — sizes the `{ it.method }` rewrite (ADR 0017).
3. **String-vs-symbol key mixing** — how often a codebase writes a symbol
   key and reads a string one (or vice versa). This directly sizes accepted
   tradeoff 3, since every such site becomes a compile error.
4. **Hash-literal-then-literal-lookup shape** — how often a hash is built
   with visible literal keys and then read with literal keys. Sizes the
   lint in "The escape hatch," i.e. how much of tradeoff 2 is recoverable.
5. **Bare symbols as arguments/returns** — the enum-shaped usage,
   specifically `{:ok, ...}`-style tagging, which feeds #28 too.
6. **Enum-ish vocabularies per class** — how many distinct closed
   vocabularies a typical class carries, and whether they are payload-free
   (Rails-shaped) or payload-carrying. Decides the one-feature-or-two
   question above.

## Where the implementation actually stands (verified 2026-07-23)

Tested against the seed, so the next pass starts from facts:

| form | status |
|---|---|
| `p(:foo)` | parse error — `:` lexes as a bare `Colon`; no symbol literal exists |
| `{:name => "pdx"}` | parse error, same reason |
| `{name: "pdx"}` | parse error — "expected => in hash literal" |
| `{"name" => "pdx"}` | works — string keys, `=>` required |
| `greet(name: "pdx")` | works |

Symbols do not exist anywhere in Portland today. Keyword arguments work but
involve no symbol: `name:` parses as an identifier plus a `Colon` token and
becomes a label at parse time.

**`{name: "pdx"}` hash shorthand is a parse error today**, which is
arguably a larger migration issue than symbols — it is the most common hash
literal form in modern Ruby. It has to be built either way; whether it means
*symbol* keys (if symbols exist) or *string* keys (if they don't) is
downstream of this decision.

## Open sub-questions for the next pass

- **Payloads: one feature or two?** (see Entanglement) — the blocker.
- **Where enums are declared, and how their type is named from outside** —
  needs #27's namespacing answer.
- **Rails-style generated predicates** (`purchase.paid?`) — the part of
  Rails enums people actually love. User flagged this as its own decision,
  deliberately deferred.
- **Does `T?` become a built-in enum?** One mechanism or two. The surface
  stays `nil`/`some` either way (ADRs 0005–0009 are settled), but the
  underlying model could unify — optionals are already a sum type in a
  wrapper's clothes.
- `%i[a b c]` is now buildable, since symbols exist (flagged in
  ruby_research's `PORTLAND_DECISION_CANDIDATES.md` under the `%` literal
  zoo).
- **Symbol interning and equality** — unstated so far. Compile-time interned
  with identity comparison is the obvious answer; worth writing down when
  the ADR lands.
