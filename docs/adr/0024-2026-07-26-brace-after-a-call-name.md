# 0024 — Brace blocks attach wherever `do` blocks do

- **Status:** Accepted (built in the seed and the trio, differentially pinned, 2026-07-26)
- **Date:** 2026-07-26

## Context

[ADR 0016](0016-2026-07-23-brace-blocks-never-guess-owner.md) promised that `{ ... }` and `do ... end` are **dead-identical** — no precedence split, no binding split — and that the one position where readings collide is a compile error naming each rewrite. Both halves turn out to be untrue, and the second is the serious one.

**Braces only attach to dot calls.** `nodes.map { it.sexp }` works. Nothing else does:

<!-- not-portland: none of these parse today, which is the bug this ADR fixes -->

```ruby
twice { puts "again" }      # error: `{` after a paren-less call …
twice() { puts "again" }    # error: expected a newline after statement, got `{`
render(config) { |x| x }    # error: expected a newline after statement, got `{`
```

So a brace block cannot be given to a user-defined method at all, however it is spelled. `do ... end` can, in every one of those positions.

**Which makes the never-guess menu unactionable.** ADR 0016's worked example offers three rewrites; two of them do not parse. Measured, not reasoned:

| the menu's suggestion | parses? |
| --- | --- |
| `render config({ "a" => 1 })` | yes |
| `render(config { "a" => 1 })` | **no** — `expected closing paren after arguments` |
| `render(config) { "a" => 1 }` | **no** — `expected a newline after statement` |

Principle 3 says the error "shows you the menu … the error is one round, never a dialogue." A menu whose entries are dead ends is worse than a dialogue: the author copies the rewrite the error told them to use and gets a second, unrelated error. This has shipped for days, in the ADR, in [parentheses](../ruby/parentheses.md), and in both oracles' diagnostics.

**And the beautiful line is the one being refused.** The pull that surfaced all this was a spec DSL:

<!-- not-portland: the line this ADR allows, which does not parse until it is built -->

```ruby
twice { greet name: "pdx" }
```

Unambiguous to any reader — `{ greet name: "pdx" }` is not a hash, and the parser knows it, since `config = { greet name: "pdx" }` is `expected => in hash literal`. Until 2026-07-26 the peek called it ambiguous anyway, because it scanned the whole body for a `label:` and found the keyword argument. That is fixed. What remains is that the line is still refused, and the rewrite it is pushed toward is `twice() { greet name: "pdx" }`, which does not parse — and would be ugly if it did. Principle 1 is the one that settles this: **the beautiful line must also be the safe, fast line**, and here safety was being bought with a line nobody would write.

## Decision

**A brace block attaches to a call in exactly the positions a `do` block does. The never-guess error survives only where readings genuinely collide, and every rewrite it names must parse.**

Three parts, in dependency order:

1. **Parenthesized calls take a brace block.** `twice() { ... }`, `render(config) { ... }`, `render(config { ... })`. There is no ambiguity to speak of — the parens closed the argument list — and ADR 0016 already assumed this worked when it wrote those rewrites into the menu. Pure repair.

1. **A paren-less call with no arguments takes a brace block where the peek leaves one reading.**

   <!-- not-portland: half of these are the refusals, so the block cannot parse whole -->

   ```ruby
   twice { puts "again" }            # accepted — no `=>`, no leading label
   twice { greet name: "pdx" }       # accepted — the first element is not a pair
   twice { |item| item }             # accepted — a `|` cannot open a hash
   twice {{name: 1}}                 # accepted — a `{` cannot open a pair either

   twice {}                          # error, two readings — empty hash, or empty block?
   twice { "a" => 1 }                # error, two readings — a pair, or a match assertion?
   twice {name: 1}                   # error, one reading — a hash; write twice({name: 1})
   ```

1. **A paren-less call with arguments is unchanged.** `render config { ... }` keeps ADR 0016's three-way menu, because three readings really are available there. Its rewrites become reachable via part 1.

The peek that decides part 2 now decides *what compiles* rather than which error prints, so its rule belongs in this ADR rather than in the source. It answers **three** ways, not two — "could be a hash" hides a distinction that matters, and collapsing it invents ambiguities:

| the braces | reading | because |
| --- | --- | --- |
| `{ \|item\| … }` | block | a `\|` cannot start a pair |
| `{ greet name: 1 }` | block | the first element is not a pair, however pair-like the rest looks |
| `{{name: 1}}` | block | the first element is a `{`, which cannot start a pair |
| `{name: 1}` | **hash** | a label is not a statement, so there is no block body to read it as |
| `{"a" => 1, "b" => 2}` | **hash** | a pair *list* is not a statement either |
| `{}` | either | an empty hash, or an empty block |
| `{"a" => 1}` | either | a lone pair is also a one-line match assertion (ADR 0013) |

Judged from the **first pair position only**: a hash's first element is `label: value` or `value => value`, so anything else rules the hash out. The rows are not guesses — each was checked by asking the seed whether the body is a statement.

**One reading does not always mean accept.** Where the single reading is a *block*, the braces attach. Where it is a *hash*, the call is still refused, and told rather than asked: `` `{` after a paren-less call is a hash, not a block — a block body cannot start with a label. Write foo({ ... }) ``. Refused because accepting would make `foo { … }` mean a block for most bodies and a hash argument for pair-shaped ones — the unrecitable rule this ADR exists to avoid. Nothing is lost: `foo name: 1` already passes keyword arguments and `foo({name: 1})` a literal hash.

Rejected, one line each:

- **Refuse always** (the status quo). Refuses `twice { |item| item }`, which has one reading, while `nodes.map { |item| item }` one token to the left is fine — a rule nobody could recite, which is the disease ADR 0016 was treating.
- **Accept only on `{ |`.** Safe and tiny, and it keeps the peek out of the grammar, but it refuses `twice { greet name: "pdx" }` — the exact line that prompted this. Principle 1 rejects it.
- **Accept always, Ruby's rule verbatim.** Makes `twice {}` a block and `twice {name: 1}` a block whose body then fails to parse; Ruby's guess imported wholesale, which ADR 0016 considered and rejected on principle 3.

## Consequences

- **One residual divergence, and it is in the ledger:** `foo {}` is valid Ruby — a call handed an empty block — and does not compile here, because `{}` is equally an empty hash argument. A refusal naming both rewrites, never a silent difference, so promise 1 holds. Recorded in [parentheses](../ruby/parentheses.md).
- Polyfill tiers: every accepted form is **free** (already valid Ruby, same meaning). `foo {}` is **gem-supplied** — a linter rewrites it to `foo() {}`, and an empty block has nothing for the rewrite to alter.
- ADR 0016's menu becomes true. Its three rewrites all parse once part 1 lands, and the two broken entries stop being documented as advice.
- **The peek becomes load-bearing, which is the cost to weigh.** Today it only chooses which error prints, so imprecision is a wording bug. After this it chooses accept versus refuse, so imprecision is a compatibility bug — and any hash syntax added later must be taught to the peek in the same commit, or code that compiled stops compiling. That is why its rule is specified above rather than left to the implementation, and why the specification is written in terms of *pair position* rather than a token scan.
- The seed threads this through its existing `command_arguments` flag; the trio, holding no parser state, needs the brace path in `parse_command` and the same first-position peek it already has. Both must land together — a divergence in *what parses* is worse than one in wording.
- `{` and `do` become genuinely interchangeable for the first time, which is what ADR 0016 said and this ADR is what makes it so.
