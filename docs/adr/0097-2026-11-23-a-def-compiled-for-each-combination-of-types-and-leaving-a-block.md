# 0097 — A def compiled for each combination of types, and leaving a block

<!-- generated: do not edit by hand — script/docs/generate -->

> [!IMPORTANT]
> **A decision record, not a reference.** It says why a ruling was made; a later ruling may have changed it without marking this file. See also: [architecture.md: The backend — designed, not built](../../docs/architecture.md#the-backend--designed-not-built).

<!-- /generated -->

- **Status:** Accepted (ruled by the deciding user 2026-11-23, one question at a time; tracked on [#491](https://github.com/portlandlang/portland/issues/491) and [#511](https://github.com/portlandlang/portland/issues/511)).
- **Date:** 2026-11-23
- **Issue:** [#491](https://github.com/portlandlang/portland/issues/491)

## Context

The native build holds blocks, arrays, and maybes ([ADR 0096](0096-2026-11-23-native-blocks-arrays-and-maybes.md)). What stands between it and a method written in the core is two things that ADR left undecided, and the core's own source shows both:

```ruby
def find
  self.each do |element|
    found = yield(element)
    return element if found
  end
  nil
end
```

The first is that `find` is one piece of source called on an array of Integers at one place and an array of Strings at another. The native build gives every value one type, and since the spike ([ADR 0094](0094-2026-11-22-a-native-spike-through-llvm-ir-text.md)) it has done so by requiring every call of a def to agree: `twice(1)` beside `twice("ab")` refuses. Under that rule a second call of `map` would refuse the build.

The second is the `return` inside the block. A native block is its own function, called from inside the walk's loop, and a plain return there leaves only the block.

The native build also had nothing it was for. It was aimed at a benchmark and at the core's methods, and neither is a program anyone runs.

## Decision

1. **A def is compiled once for each combination of types it is called with**: what its parameters are, and where it yields, what its block is handed and answers. Two calls with the same types share one function. This holds for every def, a program's own and the core's alike, so `twice(1)` and `twice("ab")` both build, as two functions. It replaces the rule of ADR 0094 that every call of a def must agree.
1. **One function over tagged values was declined.** It would test a tag on every operation when the line runs, and it is the layout already declined for an array's elements (ADR 0096): a function cannot serve two element types at once when an array is laid out by its one. The cost taken instead is a larger binary and a longer build.
1. **`return` and `break` written inside a block have Ruby's meanings.** `return` leaves the method the block was written in, with its value. `break` leaves the call the block was handed to, which then answers the break's value.
1. **They are built by passing a flag up.** The block stores the answer in its writer's frame and returns saying it is leaving. Each caller between tests that after the call, releases what it holds, and returns the same in turn, until the frame the leaving is aimed at. Every return is an ordinary one, so every counted value is released by the code that already releases it.
1. **Jumping straight out was declined as the first form.** It is what CRuby does, with a `throw` instruction and a jump across its C frames, and CRuby can because a collector frees what the skipped frames held. Counted references owe a release in every frame skipped, which a jump would need tables to honour. The flag costs one test after each call of a block, and `clang` removes it wherever the block is written inline and cannot leave.
1. **The native build is aimed at the linter** ([#510](https://github.com/portlandlang/portland/issues/510)) compiled natively ([#511](https://github.com/portlandlang/portland/issues/511)). It is a program with a user, it needs the speed, and it is mostly the compiler's own lexer and parser. After methods on a type, what is built next is what the linter's build refuses next.
1. **Symbols and Floats are set aside**, which the linter does not need. This changes the order ADR 0096 gave.

## Not decided here

- A value that may be one of several types. A syntax tree is one by nature, so the linter calls for it; ADR 0096 left it for a later ruling, and it is still that.
- How a method on a type is compiled, beyond that it is compiled once for each combination of types like any def.

## Consequences

- A **deliberate non-difference** from Ruby and from the two interpreters: a program inside the slice prints the same bytes natively as on the seed, a `return` or a `break` inside a block among them.
- A program that calls one def with two types builds where it refused, so the native build accepts more of what the interpreters accept.
- A def called with many combinations of types becomes that many functions.
- The order of the native build's work is read off a build that fails, and is no longer set in advance.
