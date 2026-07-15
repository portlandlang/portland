# Heterogeneous dispatch: one `.map`, many execution units

UMA means data lives once — the same `.map` line is a GPU dispatch on a big array, one core on a small one. The runtime places work across P/E cores, Metal GPU, SME.

## Tasks

- [ ] Cost model: when is GPU dispatch worth it? (size thresholds, kernel launch overhead on Apple GPUs)
- [ ] Lowering path to Metal: MLIR `gpu` dialect → AIR? Or Metal shading language codegen? Investigate what's actually open
- [ ] SME (M4+) path via LLVM intrinsics — which patterns lower to matrix ops
- [ ] QoS / energy as a type-level effect (`@ecore @background`) — spec the effect syntax
- [ ] Honest-limit doc: Neural Engine and old AMX are reached via CoreML/Accelerate/MPS only; never claim direct NPU codegen
