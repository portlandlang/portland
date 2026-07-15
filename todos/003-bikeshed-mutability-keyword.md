# Bikeshed: the mutability keyword

Bare binding is immutable (`foo = "abc"`); mutability is the marked, rare case. Pick the marker.

## Candidates

- `let` — collides with other languages' _immutable_ connotation (Swift, Rust); risky
- `var` — familiar, reads as "variable = varies"
- `mut` — explicit, Rust-flavored; maybe too much ceremony-smell

## Constraint

Ceremony goes on the rare thing. The marker must read as prose, not annotation. Decide alongside [005](005-bikeshed-absence-word.md) and [004](004-bikeshed-concurrency-sigil.md) so the keyword set feels like one voice.
