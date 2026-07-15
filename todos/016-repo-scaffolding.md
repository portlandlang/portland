# Repo scaffolding

Once the GitHub org exists. This design repo plus the future compiler repo.

## Tasks

- [x] `git init` this design repo, first commit — push to org still pending
- [x] Decide repo layout: monorepo (design + seed + compiler in one repo, cargo workspace at root)
- [x] Seed crate via `cargo new` (`seed/` = `portland-seed`, `publish = false`)
- [ ] Scripts to Rule Them All: `script/test` done; `script/bootstrap`, `script/build`, `script/cibuild` pending
- [x] CHANGELOG.md
- [ ] License decision (placeholder crate shipped as `MIT OR Apache-2.0`; confirm or change)
- [ ] Migrate these `todos/` files into GitHub issues once the repo is up
