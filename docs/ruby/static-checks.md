# Static checks: the build refuses what Ruby ships

**Summary:** Ruby checks everything at runtime or never; Portland's compiler refuses code that is written wrong, whether or not it would ever run.

**Status:** begun ([ADR 0034](../adr/0034-2026-08-11-the-checker-and-the-oracle-succession.md)); the first check family — enum vocabularies, static — is built in the trio's checker. The rest of [#9](https://github.com/portlandlang/portland/issues/9)'s inventory (exhaustiveness, flow narrowing, unhandled maybes) lands here as it arrives.

## Ruby

Every check happens when the line runs, so a typo in a rarely-taken branch ships and detonates in production — or never detonates, and the branch silently does nothing forever. The second case is the crueler one: `in :payed` in a `case/in` is not an error in Ruby's spirit (a symbol that matches nothing), it just never fires, and nothing ever says so.

## Portland

The trio runs a checker before evaluating (the seed deliberately never will — it is the disposable half). First family, from ADR 0022's closed vocabularies:

- A payload-carrying construction must name a declared case, agree with every declaration, and bring exactly the declared labels — refused at build with the same words Ruby-style runtime checking used to say, only earlier: `no enum declares a case :shipped`, `` `:paid` takes (on:) ``.
- A pattern naming an undeclared case — the silent-forever branch — refuses with the diagnostic Ruby never had: `in :payed can never match — no enum declares a case :payed`.

The checker declines wherever it cannot tell (principle 4): unknown constructs pass through, bare symbols are never checked, and each future check fires only where the tree alone proves it applies.

## Migration

- Correct Ruby is untouched — the checker only refuses programs whose written code contradicts its own declarations.
- Dead-code typos that Ruby shipped become build errors here; the fix is the fix the code always needed.
- There is no way to suppress a check, and none is planned: a refusal wrong enough to need suppressing is a checker bug to report, not a wart to silence.
