# Symbols: first pass (tabled)

**Status:** no decision. Tabled 2026-07-23 pending corpus evidence and the
enum/sum-type question it entangles with. This file is the session notes so
the next pass starts from here instead of from scratch.

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

## The reframe

The session opened with the wrong question ("keep or kill symbols?") and
found a better one:

> Portland has already killed both of Ruby's *reasons* for symbols. The
> semantic reason survives — so what carries it? A distinct interned type
> (Ruby's answer), a typed enum with symbol-ish syntax (Swift's), or
> inferred variants (OCaml's)?

## Entanglement

This is **entangled with the enum/sum-type question, which is undecided**
and probably belongs with the #27 object-model session. Portland already has
exhaustiveness machinery (ADR 0013, extended to ranges in ADR 0019), so a
real enum would slot into it; a symbol never can, since the set of symbols
is open. Deciding symbols before enums risks answering the smaller question
first.

## What the corpus should measure next

1. **The residue** — symbol occurrences that are *not* hash keys, kwarg
   labels, metaprogramming DSL arguments, or `&:sym`. If near zero, symbols
   are confirmed redundant. If large, inspect what it actually is before
   ruling.
2. **`&:sym` prevalence** — sizes the `{ it.method }` rewrite (ADR 0017).
3. **Symbol-keyed vs string-keyed hash literals** — sizes the migration if
   `{name: "x"}` comes to mean string keys.
4. **Bare symbols as arguments/returns** — the enum-shaped usage,
   specifically `{:ok, ...}`-style tagging, which feeds the #28 error
   session too.

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

- If symbols go: does `{name: "x"}` hash shorthand arrive, meaning *string*
  keys? (Kills the `HashWithIndifferentAccess` papercut; `{name: "x"}["name"]`
  would work where Ruby returns nil — and `[:name]` would be a parse error,
  so the divergence is loud.)
- If symbols stay: what stops them being a second string type users must
  choose between — Ruby's actual wart?
- `%i[a b c]` rides on whatever is decided (flagged in ruby_research's
  `PORTLAND_DECISION_CANDIDATES.md` under the `%` literal zoo).
