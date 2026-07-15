# Memory model: MTE-backed safety, no GC pause, no borrow checker

The hardware bet: Memory Tagging Extension gives memory safety with zero annotations. PAC everywhere.

## Tasks

- [ ] Survey what MTE actually provides on current A/M-series (which chips, granule size, sync vs async modes)
- [ ] Allocation strategy: what runs on non-MTE hardware (M1–M3)? Graceful degradation or hard floor?
- [ ] Ownership story for the compiler itself: what invariants does the compiler prove vs the hardware catch?
- [ ] Reference-counting / arena / escape-analysis mix — "no GC pause" needs a real design, not vibes
- [ ] PAC integration for control-flow integrity
