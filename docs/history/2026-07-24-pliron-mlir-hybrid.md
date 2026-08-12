# pliron / MLIR hybrid — a backend architecture sketch

**Summary:** A candidate #5 architecture: pliron hosts the Portland-specific middle IR in native Rust, real MLIR runs below a textual seam as pinned stock binaries and owns the hardware arrows.

**Status:** exploration, not a decision. #5 (the compile pipeline) is
undesigned and owns every call sketched here. Captured from a design
conversation, 2026-07-24, so the thinking isn't lost.

**Context:** [pliron](https://github.com/pliron-org/pliron) is an
extensible compiler IR framework — "Programming Languages Intermediate
RepresentatiON" — MLIR's architecture (dialects, ops, types, attributes,
passes) reimplemented natively in safe Rust, no C++ underneath. It ships
an LLVM dialect real enough to have compiled bzip2 and Lua. The other
Rust route to MLIR is [melior](https://github.com/mlir-rs/melior),
bindings over the real C++ MLIR. This report assumes pliron keeps its
momentum and delivers its roadmap; if it stalls, the fallback is bespoke
Rust structs for the same layer (the rustc approach), and the
architecture below survives unchanged.

## The observation that unlocks the split

The backend was never one question. Everything Portland-specific —
desugaring `together`, inserting RC and eliding it via escape analysis,
compiling optionals and narrowing into branches, mutability enforcement,
the *decision* half of placement — lives in dialects Portland must write
itself under any framework. Upstream MLIR contributes nothing to that
layer. Upstream's enormous value sits *below* it, in the standard
parallel-compute soup:

- `linalg` / `vector` — structured ops that keep "this is a parallel
  map/matmul/reduction" legible late, with mature tiling, fusion, and
  vectorization transforms. The representation a placement decision
  wants to operate on.
- `arm_sme` — contributed substantially by ARM's own engineers: lowering
  to SME outer-product instructions, streaming-mode entry/exit, and
  allocation of the scarce ZA tile registers. Siblings `arm_sve`,
  `arm_neon`. Rebuilding this means re-deriving streaming-mode
  discipline from the architecture manual, without ARM reviewing the
  patches.
- `gpu` — portable kernel modeling (launches, thread indices,
  host/device memory ops) and battle-tested kernel-outlining passes:
  exactly the "your block becomes the SIMT program" transformation,
  subtle around captures, memory placement, and barriers.
- The substrate — bufferization, canonicalization, CSE, pass manager,
  verification, dialect conversion. Unglamorous, Apple-agnostic, years
  of accumulated correctness.

One honest caveat: **there is no public AIR or Metal backend in
LLVM/MLIR** — AIR lives in Apple's private fork; the public paths to
the GPU are MSL source or Metal's binary toolchain. The final GPU
emission step is custom work under *any* framework. MLIR's advantage on
that arrow is the 90% before the last step.

So "pliron vs MLIR" was the wrong question. The right one: who hosts
the Portland-specific middle, and who hosts the hardware bottom.

## The hybrid

pliron hosts the Portland dialects — high- and mid-level IR, all the
language's own analyses and transforms, in safe Rust, no C++ in the
daily loop. At the bottom of that stack, a printer emits **only
upstream MLIR dialects** (`func`, `scf`, `arith`, `linalg`, `vector`,
`gpu`) as textual IR, and the MLIR side runs as **stock prebuilt
binaries** — an `mlir-opt`-shaped pipeline invoked as a subprocess, the
way compilers shell out to `llc`. No melior FFI in the core, no C++
build; a version-pinned binary dependency.

Three things make this attractive rather than merely possible:

1. **It's the normal architecture.** rustc: HIR/MIR in native Rust,
   hand off to LLVM. Swift: SIL, then LLVM. Own the language-specific
   middle in your own language; delegate the hardware bottom to
   external infrastructure. The only novelty is a framework (pliron)
   instead of bespoke structs for the middle — buying a verifier, pass
   manager, and printing for free. And since pliron is MLIR-shaped, the
   seam is a pretty-printer, not a semantic translation.
1. **It matches the roadmap's own staging.** #11 wants `together`
   serial first; Stages 1–2 are CPU-only. That whole stretch is
   plausibly carried end-to-end by pliron's LLVM dialect with no MLIR
   anywhere (NEON auto-vectorization comes from LLVM regardless). Real
   MLIR joins exactly when #13 does, below a seam designed for it. The
   thesis is never bet on pliron — only the warm-up act is.
1. **A textual seam is differential-harness food.** A boundary that
   emits `.mlir` files can be pinned, diffed, golden-tested, and
   bisected. In character for a project whose verification culture is
   byte-identical oracle diffing.

## Conditions and costs

- pliron's core (verifier, pass infra, printing) must harden as its
  momentum implies. Watch its LLVM dialect's completeness.
- The seam emits *only* upstream dialects — no custom C++ dialect
  registration — so the MLIR side stays stock binaries.
- **MLIR's textual format and dialects churn between releases.** The
  seam printer targets one pinned version and is re-verified (golden
  tests over `.mlir` output) at every version bump. Never free, but
  tractable.
- Debug info / source locations must survive the crossing.
- **Stage 3 tension:** a pliron-hosted IR layer makes the Rust floor
  fatter, while the Rubinius creed wants it thin and descending.
  Probable resolution: the pliron layer is Stage 1–2 scaffolding that a
  later Portland rewrite absorbs; the MLIR binaries remain permanent
  external tooling, as LLVM is for rustc.

## Distribution (who installs what)

1. **Compiler developers** — pinned `llvm` in the Brewfile for
   `mlir-opt` / `mlir-translate`. The only place Homebrew appears.
1. **Portland programmers** — a self-contained toolchain: the pinned
   binaries vendored inside the distribution next to `pdx`, invoked by
   path, never resolved from the user's environment (rustup ships
   rustc's LLVM; Julia bundles its own; Xcode carries Swift's).
   Vendoring is what makes the version pin real — the seam cannot
   tolerate whatever MLIR a user happens to have installed.
1. **People running Portland-compiled programs** — nothing. MLIR is
   compile-time machinery.

Wrinkle in audience 2, from the GPU path rather than MLIR: if AIR
emission leans on Apple's Metal toolchain (`xcrun metal`), Portland
programmers may need Xcode Command Line Tools — not vendorable, since
it's Apple's. Ubiquitous on dev Macs and #13-era anyway, but it should
be a decision, not a surprise.

## Open question for the #5 session

Where does the placement *decision* (#13's brain, versus its lowering
arms) live — above the seam in pliron, where Portland's own analyses
(immutability, block-shape divergence heuristics) can inform it, or
below in MLIR, near the cost models? Instinct: above — placement is
language semantics, not lowering. Undecided on purpose.
