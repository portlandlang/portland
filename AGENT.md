# Working brief

**For:** any contributor working on this Portland project, whether a coding agent or a human. See also: [README](README.md).

This file is orientation and conventions only. It deliberately summarizes nothing — a summary is a second home for a fact, and the ones that used to live here went stale within days.

## Read in this order

1. **[docs/principles.md](docs/principles.md)** — the rules that settle arguments, in precedence order. Read before deciding anything. Several exist because we broke them once, and the file says which.
1. **[ROADMAP.md](ROADMAP.md)** — where we are and what's next.
1. **[docs/language.md](docs/language.md)** — what Portland speaks.
1. **[docs/architecture.md](docs/architecture.md)** — the seed, the compiler, and the contract between them.
1. **[docs/ruby/](docs/ruby/)** — what each divergence costs a Rubyist.
1. **[docs/adr/](docs/adr/)** — the decision log: why each ruling was made, when it was made.

**The reference docs are principles.md, language.md, architecture.md, the docs/ruby/ ledger, and this file.** Where one of them and an ADR disagree, the reference doc wins: a later ruling can override an ADR without marking it, so never cite a single ADR as how something works ([ADR 0089](docs/adr/0089-2026-11-18-adrs-are-history.md)). Each ADR's banner links the reference sections that cite it.

[docs/history/](docs/history/) is frozen writing. Read it for reasoning, never for status.

## Conventions

**Decisions become ADRs**, one file per decision, `NNNN-YYYY-MM-DD-slug.md`, never renumbered, Status of Accepted / Tentative / Superseded by NNNN. An issue comment is not a decision record. Issues discuss; ADRs decide. **A new ADR lands with its edit to the reference doc** it changes, in the same commit; `script/docs/generate` then writes the ADR's banner from that citation, and `script/docs/check generated` fails for an ADR no reference doc cites.

**Every Ruby-divergent ADR updates [docs/ruby/](docs/ruby/)** in the same breath — a file per difference, with the Ruby behavior, the Portland behavior, why, and what happens to migrating code. If a decision is a non-difference, say so explicitly; `script/docs/check` enforces the pair.

**Never hard-wrap markdown prose.** One line per paragraph, bullet, or table row — let the editor wrap. Hard wrapping makes diffs lie: changing three words reflows every line after them, so a one-clause edit reads as a rewritten paragraph. Code blocks keep their own line breaks, since there the breaks are the content. No check enforces this: mdl's line-length rule is a ceiling, and it has no rule for a line broken too early.

**Every commit gets a CHANGELOG entry.** `## Unreleased` is strictly newest-first: insert **directly under the `## Unreleased` header**, never anchored on a nearby existing bullet — that silently drifts as the file grows, and it has already scrambled the order once.

**One small logical change per commit**, tests green at each. For work that can't stay green across a boundary, branch and merge when it's green again — no PR needed for solo work.

**Never hand-write expected output.** The seed is the oracle; tests assert that the compiler agrees with it.

**Tick issue-body checklists as parts land**, so an issue's state is readable without reading its comments.

**Single quotes for a string that interpolates nothing and escapes only a quote.** Ruby's own preference, and the specs are where it shows: an expected value that is itself quoted text — JSON, an `inspect` rendering, a refusal's sentence — reads `'"sym"'`, never `"\"sym\""`. Double quotes stay for interpolation and for the escapes single quotes cannot spell (`\n`, `\t`). The repo has no `rubocop.yml`; this list is the style guide, and this is its first entry about how Portland itself is written ([#206](https://github.com/portlandlang/portland/issues/206)).

**An implementation file is in a fixed order**, in `compiler/` and `core/`: its header comment, its requires, its constants, its traits, its structs, its defs, and then what it runs, with each kind alphabetical, the methods inside a struct or a trait too. A def used by one other def is sorted with the rest. Order is kept only where it means something: a constant after the one it is built from, a trait ahead of the struct that includes it, the arms of a `case`, a struct's fields. A program, a spec, or an example reads top to bottom as a story. `script/lint` holds the implementation to it.

**Adding, removing, or renaming a builtin in the seed changes the method table.** The checker's shape 2 and contract check read each builtin receiver's method set from the table in `compiler/inference.pdx`, which is derived from the seed's behavior, not from its Rust: run `script/probe_builtin_methods`, paste its lines over the table, and commit the two together. The seed reads the same eight lines at build (`seed/src/builtins.rs`) for its own wordings, so the table has one home; `script/check_builtin_table` diffs it against a fresh probe and runs in CI whenever the seed, the table, or the probe changed. The table retires with the seed at Stage 3, when the stdlib becomes Portland source.

## Writing that stays true

Prose goes stale when it says something the code later stops doing and nothing notices. These rules apply to docs, code comments, spec headers, manifest reasons, issues, and PR bodies ([#288](https://github.com/portlandlang/portland/issues/288)).

**Never name a count or an exact timing.** Not lines of code, spec examples, spec files, open issues, manifest entries, or checks, and not "0.02s" or "15ms". A count is true for one commit. An approximate timing that makes a point ("about a minute") is fine. `script/docs/check counts_and_timings` flags a written-out number of files, lines, examples, rows, issues, or entries, and an exact timing; spelled-out numbers are left to you.

**No time-relative words.** "today", "currently", "now", "for now", "so far", "yet", "not yet", "recently", and "current" all age silently. Say what is true without a clock, and tie a pending state to the issue that ends it: "refuses; #N tracks it", with the issue's number and link, not "doesn't work yet". `script/docs/check time_words` flags these in the docs and manifest, and the unambiguous ones in comments, unless the clause names an issue.

**Before closing an issue, run `script/mentions N`** and bring every claim that waited on it up to date in the same change.

**Comments say what the code does and why, never what it used to do.** History lives in git and the CHANGELOG; a comment that narrates the past ("once typed it as…", "no longer…") is a second, decaying record of it. A regression test may name the bug it pins, since that is why it exists; implementation code states its reason in the present. No check enforces this one (`script/docs/check`'s header says why).

**Pin behavior in a spec, not in prose.** A spec under `spec/` runs on both implementations, so a claim it holds cannot drift; a doc sample is only parsed (`script/docs/check code_samples`), so the answer in its comment can. Where a doc states what something does, a spec should say it too.

**One home per fact.** The reference docs hold how things work ([Read in this order](#read-in-this-order)); everything else links to them rather than restating them.

## Running things

|                        |                                                                                 |
| ---------------------- | ------------------------------------------------------------------------------- |
| `script/test`          | `cargo fmt --check`, clippy `-D warnings`, the whole suite, `script/lint`, then the doc checks; `--fast` skips the two whole-suite tests (about 15s instead of a minute) |
| `script/lint [file…]`  | Portland's own linter, written in Portland, over `compiler/` and `core/` — or only the files named; `script/test` runs it |
| `script/report [concern] [file…]` | A program's own loud choices read back to it (#30): each `panic` with its reason, each `mutable` name with the lines that rebind it, each `together`; it judges nothing and never fails |
| `script/console`       | the REPL; `script/console file.pdx` runs a file                                 |
| `script/docs/check`    | the doc checks in `script/docs/checks/`; name one to run it alone               |
| `script/docs/generate` | rewrite generated index sections; name one generator to run it alone            |
| `script/spec [path…]`  | the language spec in `spec/`, on both oracles — or only the files and directories named |
| `script/ruby_spec_dashboard PATH` | the spec dashboard's JSON, from a ruby/spec checkout and `spec/ruby_spec.yml` |
| `script/mentions N`    | every place this repo and its sibling checkouts cite issue N; run it before closing N |
| `script/bootstrap`     | first-time setup, installs the git hooks                                        |

**Importing from ruby/spec updates `spec/ruby_spec.yml`** in the same commit: each upstream file taken in gets an entry naming its pdx specs, and a partial or a skip says why, with the ledger page or issue. The dashboard at portlandlang.com is drawn from it — refresh it with `script/ruby_spec_dashboard ~/Developer/ruby/spec > ../portlandlang.com/_data/ruby_spec.json`. Every file not taken in has a generated stub under `spec/ruby/` (`script/ruby_spec_stubs ~/Developer/ruby/spec`): its upstream titles as pendings, parsed by the gate but never run. Importing a file means deleting its stub and regenerating.

Hooks are tracked in `script/hooks/` and installed via `core.hooksPath`. There is one, `pre-push`, the fast gate (`script/test --fast`); it takes `--no-verify`. Commits run nothing. The whole suites and the builtin method table are CI's.

Rust lives at `/opt/homebrew/opt/rustup/bin`, which is not on the default PATH. Repo layout is in [architecture.md](docs/architecture.md#repo-layout).

## Standing cautions

- **Verify Portland's behavior by running it.** Writing a plausible Portland snippet from memory has already produced three constructs that do not exist (endless methods, a one-line `if/then/else`, a ternary).
- **The REPL's multi-line detection is a string match on the parse error message.** Any new multi-line construct must be added there or the REPL silently breaks. Heredocs and brace blocks both did.
- **The crates.io `portland` crate is a name squat.** Do not suggest publishing or maintaining it.
- **Domains are the user's job.** Don't track them, don't nag.
