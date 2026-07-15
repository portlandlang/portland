# Compile-time macros

The replacement for all of Ruby's runtime metaprogramming (monkeypatching, `method_missing`, `define_method`, `eval` — all cut). The metaprogramming joy without the runtime mystery or cost.

## Tasks

- [ ] Survey prior art: Rust macros (declarative + proc), Elixir macros, Crystal macros, Swift macros
- [ ] What do the top Ruby metaprogramming use-cases (ActiveRecord-style DSLs, attr_*, delegation) look like as Portland macros?
- [ ] Hygiene rules
- [ ] Where macros run in the pipeline (pre- or post-inference?)
- [ ] The joy test: does the macro _call site_ read like Ruby prose?
