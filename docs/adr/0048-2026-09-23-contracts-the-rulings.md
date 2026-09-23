# 0048 — Contracts: the spelling, traits as names, no inheritance, agreement sharpens

- **Status:** Accepted (rulings ratified 2026-09-23; built the same day — contracts in the dump, and the call-site refusal for struct arguments; caller flow waits for a whole-program checker, see Consequences)
- **Date:** 2026-09-23
- **Issue:** [#9](https://github.com/portlandlang/portland/issues/9) — increment 3c, the rulings ADR 0040 §1's hybrid model left open

## Context

ADR 0040 §1 decided the model: a parameter's type is a structural contract computed from what the body demands of it, sharpened by concrete types flowed from call sites. It left the spelling, the reach, and the refusals for the increment that builds them. The brief for that increment laid a four-rung ladder, ratified one rung at a time.

Working out how the second half — caller flow — would compute exposed one thing the model assumed: "every reachable call passes a Token" is knowable only with every caller in view, and the checker checks one file. The compiler's own files require one another, so a def in `parser.pdx` has callers in `evaluator.pdx` the checker never sees when checking `parser.pdx`. Sharpening on in-file callers and refusing inside the body would be a refusal built on an assumption (principle 4).

## Decision

1. **A contract is spelled `needs '.upcase', '.length'`** — the word, then each demand as the source spells it: a dotted call with its dot, quoted (ADR 0047 §5); an operator bare of the dot, quoted, since `n + 1` demands `'+'`. The dot does the work the noun "method" does in shape 2's sentence, exactly as shape 3's `before '.upcase'` already does. The renderer applies the quotes; the contract is a plain list of names. This spelling is the dump's today and the error vocabulary the day call-site refusals ship (ADR 0047 §1 made those one vocabulary), and the form a person could type into an annotation without wincing, should input annotations ever arrive (ADR 0041 §6).
1. **A trait's name may stand in for the set — in errors only, on an exact match only.** ADR 0040 §2 lets an error say `Describable` for that method set as a shape; it does so when the demand set equals a declared trait's method names, never on a partial overlap (`needs Describable` would then claim methods the body never used), and never in the dump, which stays the raw list so the reader can always see what the body asked for.
1. **A contract is the body's own demand; nothing is inherited.** A body that hands its parameter to another def does not inherit that def's contract. Transitive demand is the whole-program road ADR 0040 named as Crystal's cost, and where errors surface far from their cause. Deferred, not refused.
1. **Agreeing call sites sharpen; disagreeing ones decline.** When every reachable call passes one known type, the body reads its parameter at that type and the ordinary shapes fire inside it. When callers disagree, the parameter stays at its contract — unknown to shapes 1–4 — and nothing refuses. Refusing the call that fails the contract is the call-site wording's job, a later session.

## The call-site wording

Ratified 2026-09-23, once step one was in. A call that hands a def something its body cannot use refuses at the call — the place the mistake was made, where the runtime would fail inside the def:

```text
'token' is a Token, but 'greet' needs '.upcase'
'box' is a Box, but 'show' needs Describable
```

The argument as written, its type, the def, and what it wanted. No hint: fixing the argument and fixing the def are two rewrites. Only the demands the argument lacks are named — the whole contract is in the dump; a trait's name stands in when the whole contract is exactly its method set (ruling 2). Struct arguments by their declaration, builtin arguments by the method table ([#88](https://github.com/portlandlang/portland/issues/88), built the same day); an argument the renderer cannot spell declines.

## Consequences

- **Step one builds now, silent:** each def's parameters and their contracts appear in `types.pdx` — `def greet(name: needs '.upcase') # -> String`; a parameter with a default shows the default's type; one the body never touches shows `needs nothing`. Zero refusals.
- **Caller flow (ruling 4's sharpening half) waits for a whole-program checker.** Per-file checking cannot establish "every reachable call". The ruling stands as the rule for the day the pipeline ([#5](https://github.com/portlandlang/portland/issues/5)) or a whole-program door gives the checker every file at once; it is tracked as [#94](https://github.com/portlandlang/portland/issues/94) rather than built on in-file callers.
- **Call-site contract refusals are sound today for struct arguments** — the argument's type, the struct's declared method set, and the body's demand all come from the file in hand — and wait only for their wording session; builtin arguments wait for the method table ([#88](https://github.com/portlandlang/portland/issues/88)) besides.
- Ledger: [types.md](../ruby/types.md) gains the contract spelling beside its structural-typing sentence; a contract is how "responds to `quack`" is written down.
