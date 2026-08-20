# 0041 — Return annotations are arrow comments

- **Status:** Accepted (syntax ratified 2026-08-19; recognition and verification land *together* with the inference increment that can check them — never a window where an annotation can lie)
- **Date:** 2026-08-19
- **Issue:** [#9](https://github.com/portlandlang/portland/issues/9) — decision 4 of the design ladder, the output half; input annotations stay deliberately TBD

## Context

ADR 0040 deferred annotation syntax until intuition existed to decide with. The intuition arrived by lineage study rather than by waiting: four generations of Ruby's own annotation systems, each keeping the previous one's good half — YARD (types belong in comments, but unchecked ones rot), Sorbet (checked or it lies, but the `sig` ceremony was the price nobody wanted to pay), RBS (the right grammar — `->` for returns, `?` for nilable — in the wrong place, a parallel file tree that drifts), and rbs-inline (the community's convergence: RBS's grammar back in the comments, checked, including a trailing form on the `def` line itself: `def find_user(id) #: User?`).

The deciding user proposed the next step on that exact line, motivated independently by aesthetics (fewer positive pixels — the single-quote argument) and by Ruby's `#=>`-shows-the-result documentation convention.

## Decision

**A method's return type may be annotated by a trailing arrow comment on its `def` line:**

<!-- not-portland: `...` is a placeholder body, not Portland -->

```ruby
def find_user(id) # -> User?
  ...
end
```

1. **The marker is `# ->` trailing a `def` signature line, followed by one type expression.** rbs-inline's placement with the arrow as the marker itself — one glyph swapped from `#:`, and the honest one: in RBS's own grammar the arrow already *means* "returns," where `#=>` conventionally introduces a **value** (`3 + 4 #=> 7`). `def handle #=> String` would read "evaluates to the constant String" — a small lie; `# -> String` reads "returns a String" — the true sentence.
1. **Position-restricted, strictly.** The form is live *only* trailing a `def` signature line. `# ->` anywhere else is an ordinary comment, and `#=>` is an ordinary comment *everywhere* — pasted irb transcripts (`total = 3 + 4 #=> 7`) stay the inert prose they always were. One position, one meaning (principle 3).
1. **The annotation trails the line where the parameter list ends — every shape.** *Ruled twice the same day: a first ruling chose the def's opening line (stable anchor, return-beside-the-name), was accepted on a misread, and re-ruled to the deciding user's actual preference once the multiline shapes were drawn out.* "Where the parens close" is conceptual, not literal — for a paren-less list it means where the args end. One rule, four shapes, ruled ahead of the grammar (none of the multiline or paren-less shapes parse today):

   <!-- not-portland: signature-shape sketches, bodies elided -->

   ```ruby
   def foo(bar, baz) # -> String

   def foo(bar,
           baz) # -> String

   def foo bar, baz # -> String

   def foo bar,
           baz # -> String

   def foo = @bar # -> String
   ```

   For an endless def the parameter list and body share the one line, and a comment runs to end-of-line, so the annotation trails the whole line. The accepted costs, named: appending a parameter touches the annotation's line, and in a long list the return type reads at the bottom rather than beside the name. The gains: the arrow lands where every function-type notation puts it — after the inputs, `(args) -> Return` — and for paren-less multiline lists the annotation *may* additionally serve the parser as an args-end anchor, though the trailing-comma continuation rule already decides that without it. Exactly one home; `# ->` always speaks for the def, never for a parameter.
1. **The type grammar is the debug dump's, round-trippable.** Bare names (`String`, `Token`), trait names as shapes (ADR 0040 §2), `?` for maybes (`User?` = `Maybe[User]`), `[Integer]` for arrays, `{Symbol => Integer}` for hashes — exactly what `types.pdx` renders, so an annotation is literally *writing down what the dump would have said*. The vocabulary grows with the rendering, one decision covering both.
1. **Annotations are checked docs — they can never lie.** The compiler verifies each annotation against the inferred return type; a mismatch refuses (wording owed to the error-voice decision, per ADR 0034 §1). Because of that, **recognition ships with verification**: until the inference increment that can compute method returns, the form is an inert comment — YARD-tier, explicitly temporary — rather than a parsed-but-unchecked promise.
1. **Input annotations stay open, unblocked.** Return types are the highest-value annotation (what a README reader wants; the direction inference is worst at communicating) and this form needs no parameter story — the kwarg-colon collision that entangled the inline options never arises. If input annotations ever earn their way in, nothing here constrains their shape.

## Consequences

- **Migration tier: _free_ — the only annotation form that could be.** A trailing comment is valid Ruby today; annotated Portland runs under Ruby unchanged, and a pre-flip linter gem could check the annotations inside plain Ruby. Every inline alternative was locked-until-flip.
- The build, when inference reaches it: both lexers learn the def-trailing scan, `DefNode` carries the declared return, and the checker compares it against synthesis — refusing mismatches with error-voice wordings.
- Ledger: [types.md](../ruby/types.md) carries the shape; the "annotations at public boundaries, optional, as documentation" sentence now has its spelling.
- The `#=>` value-convention stays available for a possible future *example* annotation (`#=> "@veganstraightedge"`) — noted, not designed, not promised.
