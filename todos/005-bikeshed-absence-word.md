# Bikeshed: the absence word

No ambient nil — absence is one explicit case of an optional (`User?`). The _spooky_ nil is gone; the word may survive.

## Candidates

- `nil` — Ruby lineage, warm, but drags Ruby's baggage
- `none` — reads well in pattern matching
- `empty` — collides with `Array#empty?` mental space

## Tasks

- [ ] Pick the word
- [ ] Spec how it appears in pattern matching (`in none then …`?)
- [ ] Spec the unwrap ergonomics — the beautiful line must also be the safe line
