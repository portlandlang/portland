# Report: the decisions ahead

Written 2026-07-22, at the close of the optionals arc (ADRs 0005–0011,
seed rungs 1–6, trio rungs 1–6, #21 complete). This is a working map, not
a spec: every open decision we know about, its options and tradeoffs,
what deciding it unlocks, and a recommended order and answer for each.

**The governing tiebreaker** (user, 2026-07-22): for any decision that
affects end users' usage, *matching Ruby is the preferred answer* unless
it costs a penalty against Portland's design principles. Tie goes to
Ruby. Each recommendation below says whether it's a Ruby-match, and if
not, what penalty justified diverging.

---

## The decisions, one by one

### 1. #22 — The value of a branchless `if` (and friends)

**The question.** What does `x = if cond then 5 end` produce when `cond`
is false? Also: `while`'s value, and a call broken out of.

**Options.**

- **A — it's a maybe.** No else → the expression is `Integer?`; using it
  means handling the nil case with the normal toolkit.

  ```ruby
  greeting = if morning? then "gm" end   # String?
  puts greeting or "hello"               # the toolkit just works
  ```

- **B — "no value" is a static condition.** Using a branchless `if` as a
  value is a compile error ("add an else or don't use the result"), the
  way `puts`'s result already is.

**Tradeoffs.** A is the Ruby answer (Ruby yields nil) and is consistent
with ADR 0010's one rule — partiality returns maybes. B is stricter and
arguably more honest (a missing branch is a code shape, not data), but
it invents a second absence-like concept ("no value") that we just spent
six ADRs unifying away, and it breaks working Ruby code that A accepts.

**Unlocks.** The evaluator's `Outcome` conflation (empty slot vs nil)
becomes principled; `while`/`break value` semantics; removes the seed's
"produced no value" panic family.

**Recommendation: A — Ruby-match.** No penalty: the maybe is typed, so
no ambient nil sneaks back; the strictness B wanted arrives anyway via
"unhandled maybe is a compile error." Keep `puts` as B-style (its result
is *never* meaningful — that's not partiality, that's a statement).

### 2. #20 — The `case/in` spec

Five sub-decisions. Pattern matching is load-bearing (ADRs 0005/0008/0009
lean on it), so this is the highest-value session on the board.

**2a. Exhaustiveness.** Options: compile-checked (a `case/in` over a
maybe missing `in nil` refuses to build) vs Ruby's runtime
`NoMatchingPatternError`. **Recommend: compile-checked.** Diverges from
Ruby, justified: this *is* the optionals safety story, and the
divergence is loud (compile error), never silent. The migration promise
holds.

**2b. `when` without `===`.** Ruby's `when` power is monkeypatchable
`===`. Options: (i) `===` becomes an ordinary, statically resolved
method — classes/structs define it, `when` calls it, no runtime magic;
(ii) `when` is plain `==` plus grammar-special class/range forms;
(iii) something new. **Recommend (i) — Ruby-match** in behavior, static
in mechanism. `when Integer`, `when 1..9`, `when /re/` all keep working;
the only thing lost is redefining `===` at runtime, which is cut-list
territory anyway.

**2c. The bare-lowercase capture trap.** `in none` silently captures
(matched everything — the bug that killed the word `none`). Options:
keep Ruby's capture-by-default vs require a marker on captures.
**Recommend: keep Ruby's semantics — Ruby-match**, because the polyfill
depends on identical pattern behavior, and Portland already has the
mitigations Ruby lacks: the no-shadow rule (a capture colliding with any
existing name is an error) and 2a's exhaustiveness (a stray catch-all
makes later arms unreachable → compile error). The trap is real in Ruby;
with those two rules it stops being reachable in Portland.

**2d. Pattern grammar edges.** Pin (`^variable`), guards
(`in x if x > 0`), alternatives (`in 1 | 2`), one-line forms
(`expr => pattern`, `expr in pattern`), find pattern (`in [*, x, *]`).
**Recommend: all in except find-pattern (defer until pulled)** — each is
Ruby-match and each already earns its keep in compiler-shaped code. Note
the one-line `=>` doubles as rightward destructuring, which may quietly
answer the destructuring-assignment question ADR 0011 left unentangled.

**2e. Struct patterns.** `in Token(kind: "integer")`. **Recommend: in —
Ruby-match** (class patterns with keyword deconstruction), and the trio
is the first customer.

**Unlocks.** Rewriting the trio's `case/when`-on-kind dispatch as real
patterns; exhaustive handling of every maybe; #22's implementation
surface; the `some(nil)` nested-case story completing; eventually the
Stage 1 compiler being written the way Portland wants to be written.

### 3. Keyword arguments on regular methods

Not yet an issue — the seed supports kwargs only on `new`/`with`.
Almost pure build: Ruby's semantics (`key:` required, `key: default`
optional) are the spec. One real sub-decision: splats (`*args`,
`**kwargs`) — **recommend deferring splats entirely** (they fight
inference and arity clarity; revisit with evidence). **Ruby-match
otherwise.** Unlocks: every struct-heavy call site in the trio getting
labels; Stage 1 API ergonomics.

### 4. #10 — Mutable values and `<<` (mutability act two)

**The question bundle.** Do in-place mutators exist (`push`, `upcase!`)?
How is value-mutability marked relative to `mutable` (which gates only
rebinding)? What happens at a sharing boundary? And does `<<` append
return?

**Options.**

- **A — real in-place mutation**, frozen at share boundaries. Maximum
  Ruby familiarity; but reintroduces aliasing spookiness (two names, one
  buffer — Ruby's classic action-at-a-distance bug) and makes the
  share-boundary mechanics (freeze? copy? error?) load-bearing and
  intricate.
- **B — values stay immutable; mutation is rebinding.** The accumulator
  idiom is `mutable list = []` + `list += [item]`. Semantically clean;
  cost is O(n²) copying *unless* the runtime is smart.
- **C — B, plus `<<` as a rebinding operator** (compound assignment,
  like `+=`):

  ```ruby
  mutable line = ""
  line << word          # sugar for: line = line + word
  ```

  Reads exactly like Ruby at the call site. Aliases can't be spooked —
  `other = line; line << "!"` leaves `other` untouched (in Ruby it
  doesn't!). And the performance story is the runtime's, not the
  semantics': the seed's values are already `Rc`-shared, and the classic
  refcount-1 trick (mutate in place when nobody else holds the value)
  makes rebuild-append O(1) amortized without any semantic mutation.

**Tradeoffs.** C is the beautiful line being the safe line: Ruby
fingers, functional soul. Its price: `<<` re-enters the grammar, and
Ruby's lexer pileup (shift vs append vs heredoc opener) is exactly what
ADR 0003 celebrated escaping — though as a *compound-assignment-shaped*
operator it can live at statement/assignment level, which is a far
smaller lexical footprint than Ruby's anywhere-operator. Divergence
warning for the ledger: migrated code that *relied* on aliased mutation
changes behavior — that must be a loud lint, not silent (the polyfill
can detect many cases; the rest is the `docs/ruby/` ledger's job).

**Recommendation: C.** Ruby-match at the reading level, divergence at
the aliasing level justified by the tier-1 parallelism thesis (immutable
values are *why* `.map` can spread across cores). Bang methods
(`upcase!`) stay out — rebinding spells it (`word = word.upcase`).

**Unlocks.** String/array building in the trio (the evaluator's
pair-list hash workaround, guest hash indexing, mini_lexer's `text +=`
patterns get honest); buffer-shaped Stage 1 compiler code; unblocks the
`together`-capture story (ADR 0001's promised compile error needs the
value-mutability line drawn).

### 5. The object model — methods on structs (needs an issue)

The biggest unnamed decision. STAGE0 deliberately has "no methods in
struct bodies yet." Questions: methods in `struct ... end` bodies; is
there a `class` at all or is Portland structs-all-the-way; modules as
namespaces and/or mixins; `self`; visibility; constants.

Too big for this report to answer responsibly. **Recommend:** open the
issue, and take one increment *before* the full session: **methods in
struct bodies** (Ruby-match in surface, obvious semantics, no
inheritance questions), because Stage 1 compiler code is begging for
`token.integer?` over `token_integer?(token)`. The full object-model
session (inheritance? mixins? nothing?) comes later with that evidence
in hand.

### 6. Error handling (needs an issue)

Also unnamed, also large: what does recoverable error handling look
like? `panic` is deliberately unrecoverable (ADR 0010). Options sketch:
Ruby's `begin/rescue` exceptions (max migration comfort, but invisible
control flow and un-typed escape hatches), typed results
(`Result`/maybe-shaped returns + pattern matching — pairs beautifully
with what we just built, but ceremony risk), or a hybrid (typed errors
with sugar that reads like rescue). This interacts with inference (#9)
and `together` (#11 panic propagation), so it should be decided before
both. **No recommendation yet** — needs its own evidence file and
session; the exhibits will come from `read_file`/IO in the trio, the
first genuinely fallible operations.

### 7. #11 — `together` semantics

Five questions, all small, analyzed in-session 2026-07-22: task-line
contents (recommend: assignments only, to start), outer writes from
tasks (recommend: banned), plain lines interleaved (recommend: allowed,
per ADR 0004's design), panic propagation (recommend: first panic wins,
siblings complete-or-cancel, block re-panics at the join — structured
concurrency's rule), effect ordering (recommend: **explicitly
unspecified**; the serial seed is one legal schedule, documented as
such so the differential harness doesn't entrench it). Nesting:
allowed. Then a serial implementation in seed + trio is an afternoon of
rungs; true parallelism is deliberately not the seed's job.

**Unlocks.** The concurrency vocabulary becoming real; the polyfill
gem's `together` spec; evidence for #13's implicit tier.

### 8. #9 — Type inference (the real compiler's front door)

The deep one. Sub-decisions: Hindley-Milner vs bidirectional/local
inference (recommend leaning **bidirectional with local
generalization** — better error messages, plays well with structural
typing and future macros; HM purity is not a goal, joy of errors is);
annotation syntax at public boundaries (undesigned; must feel like
docs, not ceremony); structural typing mechanics; the formal narrowing
rules (ADR 0008's list becomes a spec); where `Boolean?` never-guess
lives. Everything in the optionals static half lands here. Should come
*after* the surface decisions above so it type-checks the real
language, not a draft.

### 9. #5 — The compile pipeline (with #12/#13 riding on it)

Lexer → parser → inference → MLIR → CPU/GPU/SME. The road to deleting
the seed. Design session, heavily entangled with #9, #12, #13. The
Apple-silicon-specific excitement lives here — after the language
surface stops moving under it.

### 10. Build-when-pulled (no decisions needed)

Heredocs (#6 — the Prism-textbook homework), symbols, floats, ranges
(surface is Ruby-match by default; floats need one honest talk about
division semantics — the seed's truncating division is already flagged),
`%()`-family literals, blocks on paren-less commands (one small
never-guess call). Any of these can fill a build day without a design
session.

---

## Recommended order

The principle: **surface before depth** — every decision that changes
what programs look like should land before the machinery (#9, #5) that
must analyze them; and each decision should feed evidence to the next.

1. **#22** branchless-if value — smallest, closes an ADR 0010 loose end
   (one conversation).
2. **#20** `case/in` spec — highest leverage per hour; unlocks idiomatic
   trio rewrites that generate evidence for everything else.
3. **Kwargs on methods** — build, near-zero decisions.
4. **Methods in struct bodies** (open the object-model issue; take just
   this increment).
5. **#10** mutable values + `<<` — with the trio's accumulated
   pure-threading pain as the evidence file.
6. **Error handling** (open the issue; design session) — before #11 and
   #9 need its answers.
7. **#11** `together`, serial implementation.
8. **The full object-model session.**
9. **#9** inference → **#5** pipeline → **#12/#13** hardware. 🎉
10. Heredocs/symbols/floats/ranges interleaved as palate cleansers.

## Recommended answers, one line each

| Decision | Recommendation | Ruby-match? |
|---|---|---|
| #22 branchless if | A maybe (`if` w/o else is `T?`) | ✅ yes |
| #20 exhaustiveness | compile-checked | ❌ loud, justified |
| #20 `when`/`===` | `===` as statically resolved method | ✅ behaviorally |
| #20 captures | Ruby's, guarded by no-shadow + exhaustiveness | ✅ yes |
| #20 edges | pin, guards, alternatives, one-line in; find later | ✅ yes |
| kwargs | Ruby's, no splats for now | ✅ mostly |
| #10 mutability | `<<` as rebinding append; no in-place mutators | ✅ reads-as, ❌ aliasing (justified by tier-1 parallelism) |
| object model | methods in struct bodies first; rest later | ✅ (increment) |
| errors | undecided — needs its own session | — |
| #11 together | assignments-only tasks, banned outer writes, unspecified ordering, structured panics | — (new ground) |
| #9 inference | lean bidirectional; decide in session | — |

---

*Everything here is a proposal. ADRs are where decisions become real.*
