# Randomness

**Summary:** `rand`, `srand`, `Random.new(seed)`, `sample`, and `shuffle` are Ruby's, and the generator is Ruby's step for step, so a seeded sequence is the same number in both languages.

**Status:** decided ([ADR 0064](../adr/0064-2026-10-25-randomness-rubys-both.md), 2026-10-25, out of [#169](https://github.com/portlandlang/portland/issues/169)). Built in the seed and the compiler, differentially pinned against CRuby's sequences.

## Ruby

`rand(6)`, `rand`, `rand(1..6)`, `[1, 2, 3].sample`, `list.shuffle`, `srand(42)`; and the explicit `Random.new(42)` with `rand`, `seed`, `sample(random:)`, `shuffle(random:)`. The generator is MT19937; `Random.new(42).rand(10)` is 6.

## Portland

The same, with the same numbers:

```ruby
rand               # a float in [0, 1)
rand(6)            # 0 to 5
rand(1..6)         # 1 to 6
srand(42)          # reseeds the ambient generator; answers the seed before
dice = Random.new(42)
dice.rand(10)      # 6 — Ruby's 6
dice.seed          # 42
%w[a b c].shuffle(random: Random.new(1))   # ["a", "c", "b"], as in CRuby
[1, 2, 3, 4, 5].sample(2, random: Random.new(42))   # [4, 1]
```

- **`Random#rand` is restrictive**: `rand(0)` and a negative bound refuse as `invalid argument - 0`; **bare `rand` is Kernel's**: `rand(0)` is a float, `rand(-5)` draws below 5, `rand(1.5)` truncates to `rand(1)`.
- **A single `sample` is a maybe**: `[].sample` is nil, `[4].sample` is 4 under the or-guard like any lookup.
- **`srand(0)` means a fresh random seed**, as Ruby's does.
- **A generator moves.** It is the one value that does, by nature; two are equal by seed and position, and `inspect(Random.new(42))` is `Random.new(42)`.
- **Not here:** `Random.bytes`, `urandom`, `Random::Formatter` ([#119](https://github.com/portlandlang/portland/issues/119)); `Random::DEFAULT`, which the bare functions are; `random: Random` the class; float ranges ([#149](https://github.com/portlandlang/portland/issues/149)); `sample`'s `random:` object that is not a Random.

## Migration

- `rand`, `sample`, `shuffle`, `srand`, `Random.new(seed)` — port verbatim, same numbers for the same seed.
- `random: Random` — drop the keyword; the ambient generator is what it named.
- `Random::DEFAULT.rand` — `rand`.
