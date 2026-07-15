# Design the compile pipeline inside the Rust floor

The first real technical drill-down, still open: lexer → parser → inference → MLIR → CPU/GPU/SME.

## Tasks

- [ ] Map the stages and their intermediate representations (tokens → AST → typed IR → MLIR dialects → LLVM)
- [ ] Decide which MLIR dialects we use vs define (a `portland` dialect? lower to `linalg`/`gpu`/`llvm`?)
- [ ] Where does the implicit-parallelism decision live (which stage decides `.map` goes to GPU)?
- [ ] Incremental compilation story — `irb`-fast feedback loop is a joy goal
- [ ] Draw the primitive boundary: the smallest set of operations that genuinely need Rust
