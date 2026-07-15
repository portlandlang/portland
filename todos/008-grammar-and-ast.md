# Grow the grammar and AST fresh in Rust

Hand-written recursive descent against Portland's own AST — what every language that cares about errors does (Rust, Clang, Go, Prism itself).

## Tasks

- [ ] Design the AST node set (start from Prism's shape for inspiration, subtract the cut list, add optionals / `together` / macros)
- [ ] Recursive descent parser over the ported lexer
- [ ] Error recovery strategy — great error messages are a joy feature, design them in from day one
- [ ] Grammar sketch document for the Stage 0 subset first, full surface later

## Not

- No PEG / parser-combinator libraries (wrong fit for context-sensitive lexing, weaker errors)
- No forking Prism's grammar — grow our own
