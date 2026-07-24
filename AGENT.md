# Working brief

_For: the coding agent working on Portland. Humans want [README](README.md)._

This file is orientation and conventions only. It deliberately summarizes nothing — a summary is a second home for a fact, and the ones that used to live here went stale within days.

## Read in this order

1. **[docs/principles.md](docs/principles.md)** — the eleven rules that settle arguments, in precedence order. Read before deciding anything. Several exist because we broke them once, and the file says which.
1. **[ROADMAP.md](ROADMAP.md)** — where we are and what's next.
1. **[docs/language.md](docs/language.md)** — what Portland speaks today.
1. **[docs/architecture.md](docs/architecture.md)** — the seed, the trio, and the contract between them.
1. **[docs/adr/](docs/adr/)** — the decision log. ADRs beat everything except a newer ADR.
1. **[docs/ruby/](docs/ruby/)** — what each divergence costs a Rubyist.

[docs/history/](docs/history/) is frozen writing. Read it for reasoning, never for status.

## Conventions

**Decisions become ADRs**, one file per decision, `NNNN-YYYY-MM-DD-slug.md`, never renumbered, Status of Accepted / Tentative / Superseded by NNNN. An issue comment is not a decision record. Issues discuss; ADRs decide.

**Every Ruby-divergent ADR updates [docs/ruby/](docs/ruby/)** in the same breath — a file per difference, with the Ruby behavior, the Portland behavior, why, and what happens to migrating code. If a decision is a non-difference, say so explicitly; `script/docs/check` enforces the pair.

**A new ledger file needs an H1 and an italic summary line under it**, then `script/docs/generate`. That summary is what the index is built from, so the index cannot drift from it — do not hand-edit between the `generated` markers.

**Never hard-wrap markdown prose.** One line per paragraph, bullet, or table row — let the editor wrap. Hard wrapping makes diffs lie: changing three words reflows every line after them, so a one-clause edit reads as a rewritten paragraph. Code blocks keep their own line breaks, since there the breaks are the content. Enforcement rides with [#31](https://github.com/portlandlang/portland/issues/31).

**Every commit gets a CHANGELOG entry.** `## Unreleased` is strictly newest-first: insert **directly under the `## Unreleased` header**, never anchored on a nearby existing bullet — that silently drifts as the file grows, and it has already scrambled the order once.

**One small logical change per commit**, tests green at each. For work that can't stay green across a boundary, branch and merge when it's green again — no PR needed for solo work.

**Never hand-write expected output.** The seed is the oracle; tests assert that the trio agrees with it.

**Tick issue-body checklists as parts land**, so an issue's state is readable without reading its comments.

## Running things

| | |
|---|---|
| `script/test` | `cargo fmt --check`, clippy `-D warnings`, the whole suite, then the doc checks |
| `script/console` | the REPL; `script/console file.pdx` runs a file |
| `script/docs/check` | the doc checks in `script/docs/checks/`; name one to run it alone |
| `script/docs/generate` | rewrite generated index sections; name one generator to run it alone |
| `script/bootstrap` | first-time setup, installs the git hooks |

Hooks are tracked in `script/hooks/` and installed via `core.hooksPath`. `pre-commit` is the fast gate; `pre-push` runs everything. Both take `--no-verify`.

Rust lives at `/opt/homebrew/opt/rustup/bin`, which is not on the default PATH. Repo layout is in [architecture.md](docs/architecture.md#repo-layout).

## Standing cautions

- **Verify Portland's behavior by running it.** Writing a plausible Portland snippet from memory has already produced three constructs that do not exist (endless methods, a one-line `if/then/else`, a ternary).
- **The REPL's multi-line detection is a string match on the parse error message.** Any new multi-line construct must be added there or the REPL silently breaks. Heredocs and brace blocks both did.
- **The crates.io `portland` crate is a name squat.** Do not suggest publishing or maintaining it.
- **Domains are the user's job.** Don't track them, don't nag.
