# Principles

**For:** anyone deciding something — a feature, a spelling, a diagnostic, a sentence of documentation.

These are the rules that settle arguments. They are not aspirations; each one has already overruled something we wanted to do, and several were written down only after we got them wrong once. Where two principles collide, the one earlier in this file wins.

[ADRs](adr/) record what was decided. This file records _how_ deciding works.

## How Portland is designed

### 1. The beautiful line must also be the safe, fast line

Programmer happiness is job one. Safety and performance are job 1.1 — not tradeoffs _against_ joy but contributors to it, and only when you never feel them. The moment they show up as bookkeeping, they have failed the test even if they are correct.

So the bar every feature must clear: **does this make the beautiful line also the safe, fast line — or does it force a different, uglier line to get safe and fast?** Reject the latter. Immutability earns its place here twice: it is what makes `photos.map { it.thumbnail }` safe to spread across cores, and the line did not have to change to get that.

### 2. Tie goes to Ruby

For anything an end user types, matching Ruby is the preferred answer unless it costs a design principle above it. Not because Ruby is always right, but because muscle memory is real and the migration path is a standing design criterion — Portland is greenfield, and cold-start adoption is the problem greenfield has instead of Rubinius's.

Every divergence must name the principle that justified it. "It seemed cleaner" is not one.

### 3. Never guess

Where one spelling has two genuine readings, Portland refuses and names both readings with their rewrites. It does not pick.

`puts -1` is an argument or a subtraction; `Boolean?` under `or` is two different "no"s; a bare `{` after a paren-less call could be a hash, the inner call's block, or the outer call's. Each is a compile error that shows you the menu. Ruby resolves these with whitespace heuristics and local-versus-method lookups, and the resulting rules are the ones nobody can recite. The enabling rule is **no shadowing**: a name is a local or a method, never both, so a bare name is always unambiguous.

Where the menu can be trimmed honestly, trim it — the parser peeks to drop the hash reading when a `|` rules it out. Peek to shorten the question, never to answer it.

The rule is directional, and the direction matters: **one spelling never has two behaviors; one behavior may have many spellings.** Synonyms are a good part of Ruby that Portland keeps on purpose — sometimes the natural sentence says `total + 1`, sometimes `page.succ` — and that expressiveness is part of the joy this language is for, and a migration path besides (principle 2). What Ruby got wrong is the other direction: one word or one syntax that behaves one of many ways depending on context, resolved at runtime, and that is what Portland avoids where it can. "One name per job" is **not** a house principle and never was; it was once written down as if it were (in [#79](https://github.com/portlandlang/portland/issues/79)'s first framing and a CHANGELOG entry), and this paragraph is the correction. Which of Ruby's alias families ship is [#79](https://github.com/portlandlang/portland/issues/79)'s decision, family by family; whether synonyms are allowed at all is not a question.

### 4. Never guess, in the implementation either

The same rule, turned inward, and the harder half.

The Portland compiler often cannot see what the Rust seed can — its parser is functional, so it holds no frames. Where it cannot tell, it **declines to check** rather than checking wrongly. `mixed_it_error` skips any block body that opens a block of its own, because a flat token scan cannot tell whose `it` it is.

A wrong diagnostic is worse than a missing one: a gap leaves the compiler incomplete, a false positive makes it unusable. The seed is the oracle either way, so the cost of declining is bounded. Every such gap is recorded rather than left silent.

### 5. Divergence is loud, never silent

Two promises govern the [Ruby ledger](ruby/):

1. Where Ruby code compiles in Portland, it means the same thing. Where the semantics differ, it fails to compile, with an explanation and a suggested rewrite. **No spelling is reused with quietly different behavior.**
1. **The polyfill test.** A future gem plus a linter should be able to teach each Portland idiom inside valid Ruby before a codebase flips. Differences are graded: _free_ (already valid Ruby, same meaning), _gem-supplied_ (valid grammar, the gem adds the method or lint), _locked-until-flip_ (new grammar Ruby cannot parse).

The corollary that bites hardest is about documentation, not code: when recording a divergence, **do not flatten levels**. "Silently accepts" is materially worse than "detects but reports differently," and a bullet that merges them hides exactly the thing this principle exists to prevent. Learned by writing such a bullet, and by being asked about it.

### 6. Features are built when a real file pulls for them

Demand-driven. Issues are commitments, not wishes. A feature earns its place by being needed to write the compiler, or by showing up in the corpus — [ruby_research](https://github.com/portlandlang/ruby_research) runs re-runnable reports over rubygems.org, and its numbers have already reordered the plan once: `constant_path_node` appears in 78% of gems and `super_node` in 19%, so namespacing is far more load-bearing than inheritance. That is the opposite of intuition, and intuition was going to build inheritance first.

## How we work

### 7. The seed is the oracle

The Rust seed defines behavior; the Portland compiler must match it byte-identically. **Never hand-write expected output.** This covers error wording as well as results: where the compiler can diagnose at all, it must say exactly what the seed says, and a test pins it.

The succession, begun with the checker ([ADR 0034](adr/0034-2026-08-11-the-checker-and-the-oracle-succession.md)): where the compiler refuses a program the seed cannot — a build-time check with no runtime analog — no oracle has a wording to match, so the full rule is *never hand-write what an oracle can produce; where no oracle exists, the deciding ADR is the oracle, and the test pins its exact words.* A check that merely moves a runtime refusal to build time keeps the seed's wording byte for byte; only the timing moves.

Corollary, learned the hard way: **green is not covered.** The differential suite stayed green through an entire batch of new syntax the compiler did not understand, because no test exercised it. Ship the hosted test _with_ the feature, not after.

### 8. Verify, don't remember

Run the thing. Ruby's edges were checked by executing Ruby 4.0.6, not by recalling them — and the system Ruby, 2.6, would have given confidently wrong answers about beginless ranges and `case/in`. Ruby's slicing is asymmetric in a way nobody remembers correctly (`[3..]` is `[]` but `[4..]` is `nil`).

The same applies to our own work: **check our own ADRs before reasoning about interactions between our own features.** A `~task` collision was once invented out of thin air and reasoned about at length; ADR 0002 had already specified `~` as a statement-line marker with a following space, so there was no collision to solve.

And: **a prediction is not a reason.** "Portland's namespaces will be shallow" was used to justify a decision; it was a guess, and it was wrong, because the flat examples it rested on were all standard library — ecosystem code is forced two and three deep by collision avoidance.

### 9. Recommend, don't enumerate

Presenting a three-by-two-by-two matrix of options is not neutrality, it is offloading. Bring a recommendation to accept or reject, with the reasoning visible and the rejected paths named in one line each. The person deciding should be able to say "yes" or "no", not "hold on, let me build a table."

### 10. Reach for prior art before inventing

Especially for nuanced domains. `ruby/spec` and Ruby-implemented-in-Ruby come before invented semantics; Prism's C lexer is the textbook for the hard lexical parts; MLIR exists for precisely our heterogeneous-compute thesis. Inventing is the fallback, not the default.

### 11. Communication is a feature

Portland's whole premise is that how something reads is not cosmetic. That applies to the project as much as to the language — a decision nobody can find, a rationale nobody can follow, and a status nobody can trust are all bugs.

Four rules, all checkable:

- **Every doc names its reader**, in a line under the title. A doc written for everyone is written for nobody.
- **No fact has two homes.** Where a doc needs a fact it does not own, it links. Every duplicated sentence is a future contradiction, and this repo has already produced several.
- **High level outside, detail as you zoom in.** README is the widest view; each link goes one level deeper and never repeats the level above at the same detail.
- **A doc that is not current says so, at the top.** Frozen writing is safe to keep — it can only be old, never wrong. Frozen writing linked as if it were a plan is neither.

Decisions get an ADR, not a paragraph in a brief. Issues discuss; ADRs decide. Changes get a CHANGELOG entry the day they land.

## Revising these

Everything here is revisable before 1.0, but the burden is on the challenger, and a revision that touches a decision gets a new ADR that supersedes the old one. The mechanically-checkable half of these rules lives in `script/docs/check`, which runs in `script/test`; the judgment half — a prediction used as a reason, a bullet that flattens distinct cases — stays the author's job, which is why it is written down here.
