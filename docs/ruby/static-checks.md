# Static checks: the build refuses what Ruby ships

**Summary:** Ruby checks everything at runtime or never; Portland's compiler refuses code that is written wrong, whether or not it would ever run.

**Status:** begun ([ADR 0034](../adr/0034-2026-08-11-the-checker-and-the-oracle-succession.md)); two families are built in the compiler's checker — enum vocabularies, and coverage where the arms establish the set ([ADR 0035](../adr/0035-2026-08-12-exhaustiveness-over-what-the-arms-reveal.md)). The third family, inference-backed refusals, has its voice and its first five wordings decided ([ADR 0047](../adr/0047-2026-09-22-the-error-voice.md)) and builds next. The rest of [#9](https://github.com/portlandlang/portland/issues/9)'s inventory (flow narrowing, the full exhaustiveness rule) lands here as it arrives.

## Ruby

Every check happens when the line runs, so a typo in a rarely-taken branch ships and detonates in production — or never detonates, and the branch silently does nothing forever. The second case is the crueler one: `in :payed` in a `case/in` is not an error in Ruby's spirit (a symbol that matches nothing), it just never fires, and nothing ever says so.

## Portland

The compiler runs a checker before evaluating (the seed deliberately never will — it is the disposable half). First family, from ADR 0022's closed vocabularies:

- A payload-carrying construction must name a declared case, agree with every declaration, and bring exactly the declared labels — refused at build with the same words Ruby-style runtime checking used to say, only earlier: `no enum declares a case :shipped`, `` `:paid` takes (on:) ``.
- A pattern naming an undeclared case — the silent-forever branch — refuses with the diagnostic Ruby never had: `in :payed can never match — no enum declares a case :payed`.

Second family, from [ADR 0035](../adr/0035-2026-08-12-exhaustiveness-over-what-the-arms-reveal.md) — a `case/in` that leaves part of its subject unhandled, where the arms themselves say what the whole is. Ruby answers this at runtime with `NoMatchingPatternError`, on the one input that happened to reach it:

- **Enum cases**, when a payload-carrying arm establishes which enum is being matched: `case/in does not cover :refunded — add the arm, or an else`.
- **The integers**, when every arm is a range: ADR 0019's proof (beginless first, endless last, no gaps) decides it, and a failure names the hole — `case/in leaves 10..19 uncovered — add the arm, or an else`.
- **Arms that can never fire** (ADR 0013 §3): one below a bare capture, or a case a higher arm already matches whole — `in :paid can never match — an arm above already matches :paid`.
- **A maybe's two cases** (ADR 0035's tripwire, fired by [ADR 0047](../adr/0047-2026-09-22-the-error-voice.md) once narrowing existed): over a subject inference knows to be a maybe, `case/in does not cover nil — add the arm, or an else`, and `the present case` likewise. Ruby's `case/in` over a nilable value raises `NoMatchingPatternError` on the input that happened to arrive.

Third family, from [ADR 0047](../adr/0047-2026-09-22-the-error-voice.md) — refusals inference makes possible, decided before any was built and landing one at a time. Every refusal is structured and rendered by one walker, so the voice lives in one file; the sentence is fact, dash, next step, with a hint only where exactly one exists; source spellings are single-quoted, types and values bare; beneath the sentence, the line's number and the line as written (`7 | puts name.knd`, indented), from the first token that begins the node. Each fires only where the type is fully known — an unknown anywhere is silence. The five wordings, each pinned to the ADR's text:

- **A non-boolean condition** (built) — Ruby's truthiness reflex, refused at build: `'if' condition must be true or false, got String`; a maybe gets the one honest hint, `— write 'user.some?'`. `while`, `&&`, and the right side of a boolean `||` say the same.
- **A method the type does not have** (built) — `'token' is a Token, which has no method 'knd'`, `'name' is a String, which has no method 'knd'`, the receiver named as written so the reader has the particular one in question; a struct's set is its declaration, a builtin's the table `script/probe_builtin_methods` derives from the seed. No did-you-mean, on purpose.
- **A maybe used as plain** (built) — `'users.first' is a String? — handle the nil case before '.upcase'`; Ruby raises `NoMethodError` only when the array happens to be empty. The seven narrowing forms of ADR 0040 §3 — the exit guard, `some?`/`nil?` branches, the or-guard, `or` with a plain default, `== nil` branches, `in nil` above a capture, and `&.` — are how a program says it handled nil, and a guarded use is never refused.
- **An operator on mismatched types** (built) — `cannot apply '+' to String and Integer`, where Ruby says `no implicit conversion of Integer into String` at runtime. The legal pairs are the seed's own; a maybe, struct, or enum operand declines.
- **An annotation that lies** (built, top-level defs) — `'handle' answers Integer, not the annotated String — change one`; Ruby has nothing to say, since the annotation is a comment.
- **An argument the def cannot use** (built, [ADR 0048](../adr/0048-2026-09-23-contracts-the-rulings.md)) — `'token' is a Token, but 'greet' needs '.upcase'`, at the call rather than inside the def where Ruby's `NoMethodError` would land; a trait's name stands in when the whole contract is exactly its set, `but 'show' needs Describable`.
- **The wrong number of arguments** (built, bare calls to top-level defs) — the seed's own runtime sentence, moved earlier with its plural fixed: `greet expects 1 to 2 arguments, got 0`. Ruby raises `ArgumentError` when the call runs.

The checker declines wherever it cannot tell (principle 4): unknown constructs pass through, bare symbols are never checked, guarded arms sit outside the arithmetic in both directions, and each future check fires only where the tree alone proves it applies. Coverage refuses only what it can *disprove* — a `case/in` it cannot reason about yet is silence, not a demand for `else`, and inference is what widens that later.

## Migration

- Correct Ruby is untouched — the checker only refuses programs whose written code contradicts its own declarations.
- Dead-code typos that Ruby shipped become build errors here; the fix is the fix the code always needed.
- There is no way to suppress a check, and none is planned: a refusal wrong enough to need suppressing is a checker bug to report, not a wart to silence.
