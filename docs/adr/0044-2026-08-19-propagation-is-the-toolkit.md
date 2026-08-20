# 0044 — Propagation is the toolkit, and `!` goes back to being a name

- **Status:** Accepted (built in both oracles, 2026-08-19) — supersedes ADR 0027's `!` clause; the failure *model* (typed results, absence with a reason, the unwrap toolkit) is untouched and remains 0027's
- **Date:** 2026-08-19

## Context

ADR 0027 settled two things at once: errors are typed results (`failure(reason)`, handled by the same toolkit as absence), and `!` at call sites means unwrap-or-propagate. The first has been load-bearing and uncontested. The second came out of an overnight three-draft session with a quick morning sign-off — and the written record shows it overrode the deciding user's own recorded position: **ADR 0015 §5 (2026-07-23) names "bang as rebinding sugar — `word.upcase!` ≡ `word = word.upcase`, `mutable`-gated" as the user's leading candidate**, with "`!` = may panic" as the competing one and the note that *neither may claim `!` without revisiting this note*. The overnight session took the competing branch. When it resurfaced three weeks later the user rejected it on review, consistent with their July position: `!` claimed for propagation locks out Ruby's bang-method naming convention whole, and the flavor it generalized (raise-on-failure) is a Rails-ism where core Ruby's dominant bang flavor is the mutate family. The corpus footprint of the feature after four weeks: two live call sites, both in its own spec.

The replacement was chosen from a presented field: the **explicit toolkit** (chosen — "especially if it Just Works™ today," and it did, modulo two hosted gaps the probe flushed out), a **guard word** (`or propagate` — later-maybe, gated on [ruby_research#4](https://github.com/portlandlang/ruby_research/issues/4)'s unclaimed-name census), **`try`** (refused — reads as a method call taking the rest of the line, which in Ruby's grammar is exactly what it would be, plus the JS/Java feel), and **postfix `?`** (refused — the glyph is spent twice, names and the committed ternary).

## Decision

**1. Propagation is the explicit toolkit, and nothing else.** A failure crosses a frame by being returned at a spelled-out guard:

<!-- not-portland: sketch; read_file's failure shape elided -->

```ruby
def load_settings(path)
  content = read_file(path)
  return content if content.failure?
  parse_settings(content)
end
```

Every crossing is written, greppable (`grep failure?`), and made of pieces that already existed — the postfix guard, the `failure?` predicate, ADR 0025's return aim. Nothing new to learn, which was 0027's own selling point, now applied to its own operator.

**2. `!` reverts to Ruby's role: part of a method's name.** `def save!` is legal; `note.shout!` calls the method named `shout!`; `alias holler! shout!` aliases it. What the suffix *means* on a name is the author's convention, as in Ruby — with one standing carve-out: **no builtin bang methods exist yet**, and the candidate rebind sugar (`name.upcase!` desugaring to `name = name.upcase`, the `<<` precedent) is deliberately not decided here — it waits on [ruby_research#3](https://github.com/portlandlang/ruby_research/issues/3)'s mutate-vs-raise census and its own ruling.

**3. Bindings still refuse the suffix, because Ruby refuses it too:** `content! = 1` is not an lvalue in either language — the wording drops its dead reference (`` `!` belongs to a method's name — a binding cannot take it ``), pinned.

**4. The guard word stays on the menu.** If real Portland finds the two-line guard heavy, `x = risky(flag) or propagate` — a fourth or-guard action returning the failure itself — is the recorded next step: zero new operators, an extension of machinery every reader knows. Its *name* awaits the census; its need awaits real code.

## Consequences

- The unwind removed the `Propagate` node from both oracles, freed bang names at defs, calls, dot-calls, and aliases, and rewrote the propagate spec into the toolkit spelling — 824 spec runs green, byte-identical.
- Two pre-existing hosted gaps surfaced by probing option C are fixed and pinned: the compiler never dispatched `failure?` (unreachable behind `!`), and failure values fell through pattern-transparency into struct-method lookup — failures are now opaque to dispatch except their predicate, matching the seed.
- The `grep '!'` audit property transfers to `grep failure?`; the deep-unwind ceremony 0027 bought off with one character returns as visible guards, accepted knowingly by the deciding user.
- Ledger: [errors.md](../ruby/errors.md) rewritten; the bang-methods question in [mutability.md](../ruby/mutability.md)'s orbit reopens pending the census.
