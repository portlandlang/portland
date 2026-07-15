# Portland

A joyous programming language for Apple silicon.

Close to the metal, on Metal.

> **Status: early design.** No compiler yet. Portland is in the brainstorming-to-scaffolding stage.
> See [`AGENT.md`](AGENT.md) for the working brief and
> [`docs/DESIGN.md`](docs/DESIGN.md) for the full design rationale.

## The idea

What could a language be if it ran _only_ on Apple silicon (A-series / M-series) — and wasn't Swift? Locking to one vendor's hardware is the feature: it lets Portland make assumptions general, portable languages can't.

The soul is Ruby's: programmer happiness first. Safety and performance aren't traded _against_ joy — they're built so you never feel them. The bar every feature must pass: the beautiful line should also be the safe, fast line.

"Ruby, the good parts"

- **Ruby's ergonomic surface, kept** — blocks, expressions, pattern matching, no ceremony.
- **No ambient nil** — absence is an explicit optional, never a value's secret.
- **Immutable when shared, mutable when local** — which is also what makes parallelism safe.
- **Concurrency you don't manage** — declare independence; the runtime spreads work across P/E cores, the GPU, and the matrix unit over unified memory.
- **Self-hosted early** — a tiny Rust seed, then Portland written in Portland.

## File extension

File extension is `.pdx`.
