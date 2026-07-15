# Define the Stage 0 subset

The tiny language the disposable Rust seed compiler must handle — just enough to write a compiler in.

## Tasks

- [ ] List the minimum features: methods, blocks, pattern matching?, strings, arrays, hashes, optionals, what else?
- [ ] Explicitly list what Stage 0 _omits_ (macros? `together`? GPU dispatch? — almost certainly yes, all deferred)
- [ ] Write a few sample Portland programs in the subset to pressure-test it
- [ ] The test: could we rewrite the seed compiler itself in this subset? (Stage 1 depends on it)

## Notes

The seed is crude and disposable on purpose. It never has to be good — it has to exist.
