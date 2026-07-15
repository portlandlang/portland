# Define the Stage 0 subset

The tiny language the disposable Rust seed compiler must handle — just enough to write a compiler in.

## Tasks

- [x] List the minimum features — documented from the working seed in [docs/STAGE0.md](../docs/STAGE0.md)
- [x] Explicitly list what Stage 0 _omits_ — same doc
- [x] Sample programs — `seed/tests/fixtures/*.pdx` (hello, fizzbuzz, blocks, showcase)
- [ ] The test: could we rewrite the seed compiler itself in this subset? (Not yet — needs at least hashes/structs-of-some-kind, `return`/`break`, and string building. Grow the subset against this question.)

## Notes

The seed is crude and disposable on purpose. It never has to be good — it has to exist.
