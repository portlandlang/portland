# 0047 — The error voice: structured refusals, one renderer, five wordings

- **Status:** Accepted (rulings and wordings ratified 2026-09-22; built the same day — the renderer, shapes 1, 2 (struct receivers), 4, 5, and arity; shape 3 followed with 3d narrowing, and shape 2 on builtins with the method table, 2026-09-23)
- **Date:** 2026-09-22
- **Issue:** [#9](https://github.com/portlandlang/portland/issues/9) — decision 6 of the design ladder, the session ADR 0040 held the first refusals for

## Context

ADR 0040 held every inference-backed refusal until concrete draft errors existed to react to, because inference-heavy languages get un-joyous exactly here. Increments 3a and 3b landed silent; the dump was the only place a type was visible. This session drafted the five commonest failure shapes from the corpus and ratified them one at a time, with the rulings that govern all future wordings decided ahead of the first sentence.

Two findings shaped the session. First, the seed's runtime wordings for these exact shapes were never designed — they leak Rust's debug rendering (`got String("shane")`, `cannot apply Add to String("a") and Integer(1)`). ADR 0034 §2's byte-for-byte rule assumed a designed sentence to move; here there was none. Second, a plain method on a maybe runs fine at runtime (`users.first.upcase` prints `A`), failing only on the nil path — so the maybe check is the one shape with no runtime twin on the happy path, and it is ADR 0005's promise made static.

The deciding user's own requirement, raised before the wordings: the voice must be changeable later without editing scattered strings.

## Decision

### The rulings

1. **Type spelling in errors is the dump's rendering.** `String`, `User?`, `[Integer]`, `{Symbol => Integer}`, trait names as shapes. Errors and annotations (ADR 0041) share one spelling; the vocabulary grows in one place.
1. **The sentence shape is fact, dash, next step.** `<what is wrong> — <what to do>`, as every existing refusal already reads. A hint or rewrite appears only where exactly one exists; where two or more exist, the sentence ends at the fact (principle 3, never guess). The renderer owns the join.
1. **No did-you-mean.** An edit-distance hint is a guess by construction, and the house has no precedent for one. Deferred; revisited when the corpus shows how often the near-miss is the only near-miss.
1. **Refusals are structured, rendered once.** The checker never calls `panic` with a sentence. It produces a tagged refusal — `["no_method", receiver, type, name]` and kin — and one renderer in the trio turns refusals into text. This is the #39 move (`render`, `render_inspect`) applied to errors: the representation stays private, the voice lives in one file, and pin tests assert the renderer's output. Rulings 1–3, 5, and 6 are renderer rules, each one line to change. The renderer builds *before* the first refusal, so no inference error ever exists as a bare string.
1. **Source spellings are single-quoted; types and values are bare.** Keywords, operators, method names, quoted expressions, and rewrites — anything the reader could type — render inside single quotes: `'if'`, `'&&'`, `'user.some?'`, `'knd'`. Type names stay bare (capitalized proper nouns already). Runtime values render through inspect, which already quotes strings, so they are never double-wrapped. `true` and `false` are values and stay bare. Ruby 3.4 made the same move, from the old `` `foo' `` mix to `'foo'` — tie goes to Ruby. Backticks never appear in a refusal; the six existing seed wordings that use them migrate when the seed's wordings get their module.
1. **Long values and expressions truncate.** The renderer cuts inspect output and quoted source expressions past a limit (the number is picked at build time), with the ellipsis inside the quotes so a string's closing mark survives: `"Lorem ipsum dolor si…"`. Ruby 3.4 dropped the receiver's value from `NoMethodError` entirely because inspect output could be enormous or leak secrets; Portland keeps the value, shortened. The secrets half is a known trade, not solved — a real leak pulls for a redaction rule.

### Where the seed's wording was undesigned, the session's sentence wins, and the seed adopts it

ADR 0034 §2 stands: a check that moves a *designed* runtime refusal to build time keeps the seed's words. Where the seed's words were Rust debug leakage, the sentence is designed once here and the seed is brought to it — same sentence, different nouns. The checker says `got String`; the runtime says `got "shane"`. Shapes 1, 2, and 4 below carry both rows.

### The wordings, pinned

Each fires only when every type it names is known. `unknown` on any side is silence (ADR 0034 §4).

**Shape 1 — a non-boolean condition** (Ruby truthiness, the commonest migration reflex). The construct names itself as the seed already does — `'if'`, `'while'`, `'unless'`, `'&&'`, `'||'`; the ternary desugars to `if` and says so.

```text
'if' condition must be true or false, got String
'while' condition must be true or false, got Integer
'&&' needs true or false, got String
```

A maybe has exactly one honest boolean question, so it alone gets the hint:

```text
'if' condition must be true or false, got User? — write 'user.some?'
```

Runtime, same sentence: `'if' condition must be true or false, got "shane"`.

**Shape 2 — a method the type does not have.** One sentence for builtins and structs alike; the seed's separate `has no field` sentence retires into it, since the reader does not care whether `kind` was a field or a def. The receiver is named as written, then its type, then the miss, so the reader has the particular value in question — the context the bare `Token has no method` lacked.

```text
'token' is a Token, which has no method 'knd'
'"Shane"' is a String, which has no method 'knd'
'users.first' is a User, which has no method 'knd'
```

Runtime: `"Shane" is a String, which has no method 'knd'`.

**Shape 3 — a maybe used as plain.** The headline check; it rhymes with the runtime nil sentence on purpose. **Lands with 3d narrowing, never before** — without the seven narrowing forms it would refuse every guarded program.

```text
'users.first' is a String? — handle the nil case before '.upcase'
```

Runtime, on the nil path: `nil has no method 'upcase' — handle the nil case first`.

**Shape 4 — an operator on mismatched types.** The seed's sentence, with the glyph and type names in place of the debug rendering. No hint: `to_s` and interpolation are two rewrites. Recorded flag: Ruby migrants hit this line constantly, and if corpus pain argues for one rewrite, interpolation is the house-preferred spelling.

```text
cannot apply '+' to String and Integer
```

Runtime: `cannot apply '+' to "total: " and 3`.

**Shape 5 — an annotation that lies** (ADR 0041's owed wording). No runtime analog. Exactly one of the two is wrong and the checker cannot know which, so the hint is honest.

```text
'handle' answers Integer, not the annotated String — change one
```

**ADR 0035's tripwire, ratified the same day once narrowing was built:** a `case/in` over a maybe-typed subject with no `else` must take both cases — nil by an `in nil` arm or a bare capture, the present case by a bare capture or a pattern that matches every present value. ADR 0035's own sentence shape, with the case named:

```text
case/in does not cover nil — add the arm, or an else
case/in does not cover the present case — add the arm, or an else
```

**A sixth that needed no session:** arity at call sites already has a runtime wording, `greet expects 1 argument(s), got 0`. It moves to build time as is, except the `argument(s)` hedge becomes a real plural while both oracles are being touched.

### Below the sentence

The renderer prints the offending source line and its location beneath every refusal, Elm-style, if the trio's nodes carry a line — verified at build time, never assumed. Not a wording; recorded here because it is what stops the quoted expression from carrying the whole context load, for every future shape too.

**Ruled and built (2026-09-24, [#89](https://github.com/portlandlang/portland/issues/89)):** a node's line is the line of the first token that begins it — the `if` keyword, a binary's left operand, the `def` — one rule, no per-node taste. The shape beneath the sentence is the number, a bar, and the line as written, indented two spaces:

```text
'name' is a String, which has no method 'knd'
  7 | puts name.knd
```

It is one function in the renderer, so the shape is one edit. Tokens carry their line (heredoc bodies padded back in so the count holds), the thirteen node kinds a refusal can point at carry theirs, and a node built by a desugar with no token in hand prints no location rather than a wrong one. The rule bends in one place: a dotted call takes its name's line rather than its receiver's start, since the receiver's start is the caller's and `.knd` is where a reader looks in a chain anyway.

**Build note (2026-09-22):** verified — the trio's nodes carried no line ([#89](https://github.com/portlandlang/portland/issues/89)), so the quoted spelling carried the context alone until the ruling above. The build also fixed one thing and filed four: the first refusal fired on the evaluator's own source and exposed a 3b inference bug (`mutable result = nil` rebound inside an `each` read as Nil), fixed in inference so every walk forgets a nested body's rebindings; shape 2's builtin half and the seed's runtime row for it wait on a method table ([#88](https://github.com/portlandlang/portland/issues/88)); the hosted runtime's own wording for a missing method ([#90](https://github.com/portlandlang/portland/issues/90)) and its missing argument count ([#91](https://github.com/portlandlang/portland/issues/91)) are pre-existing; and `failure?` refusing on a plain struct ([#92](https://github.com/portlandlang/portland/issues/92)) is why shape 2 counts it among the names every struct answers.

## Consequences

- The build order for the first refusal increment: the renderer, then shapes 1, 2, and 4 (types known after 3b), then shape 5 (annotations, ADR 0041's teeth), then arity. Shape 3 waits for 3d.
- The seed's runtime sites for shapes 1, 2, and 4 are edited by hand to the ratified sentences when each lands — the differential harness pins the change.
- The seed's remaining ~200 scattered `panic!` strings are not owed by this ADR. A behavior-preserving move into a wordings module is separate work, safe because every one is already pinned.
- Ledger: [static-checks.md](../ruby/static-checks.md) carries the third family and cites this ADR; [truthiness.md](../ruby/truthiness.md) and [types.md](../ruby/types.md) are unchanged in substance — the sentences here are what they already promised.
- ADR 0034 §1 stands unchanged: the checker remains the trio's alone; this ADR is the oracle for the words, and the pin tests assert the renderer's output rather than a literal at each site.
