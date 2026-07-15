# Port Prism's lexer from C to Rust

Prism's C source is the textbook for Ruby's lexical hell. Port the hard bits; don't fork-and-prune.

## The hard bits (the months-of-tedium list)

- [ ] Heredocs (all flavors, squiggly, interpolating)
- [ ] Regex-vs-division disambiguation
- [ ] String interpolation lexing
- [ ] `%w[]` / `%i[]` / percent-literal family
- [ ] Paren-less method call lexing feedback

## Notes

- `ruby_prism` crate is parse-only FFI — no lexer API, no grammar hooks. Use only as a reference oracle during dev.
- Ours lexes Portland, not Ruby: skip tokens for cut features (globals, `for`, perlisms) as we go.
