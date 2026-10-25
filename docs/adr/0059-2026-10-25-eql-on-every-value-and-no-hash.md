# 0059 — `eql?` on every value; `hash` stays unexposed

- **Status:** Accepted (ruled by the deciding user 2026-10-25 on the recommendation in [#154](https://github.com/portlandlang/portland/issues/154); `eql?` built in the seed and the trio, differentially pinned, 2026-10-25)
- **Date:** 2026-10-25
- **Issue:** [#154](https://github.com/portlandlang/portland/issues/154)

## Context

Ruby has two equalities. `==` is the one code writes: `1 == 1.0` is true, and a collection asks `==` of what it holds. `eql?` is the one a hash key matches by: same kind and same value, no promotion between numbers, so `1.eql?(1.0)` is false and `{1 => :x}` is not `{1.0 => :x}`. Beside them sits `hash`, an integer digest whose only promise is that two `eql?` values digest alike — and which Ruby seeds afresh every process, so the number means nothing outside the run that made it.

Portland's runtime already had both equalities: `==` as Ruby's, and `eql?` as the equality its hash keys matched by, unexposed. Seven ruby/spec files (`core/{array,hash,range,struct,integer,float,string}/eql_spec.rb`) and seven `hash_spec.rb` files waited on whether to name them.

## Decision

1. **`eql?` is a method on every value, as Ruby's.** Same kind and same value: `1.eql?(1)`, never `1.eql?(1.0)`; `0.0.eql?(-0.0)` as `==` has it, NaN never; strings by canonical text, as `==`; arrays, hashes, ranges, and structs element by element through `eql?`, a hash's order no part of its identity; nil, booleans, and symbols by themselves. Tie goes to Ruby (principle 2), and it is one behavior with one name, not a synonym of `==` (principle 3).
1. **`hash` is not exposed.** A value is a hash key by `eql?`, and the runtime keeps whatever digest it uses to itself. Exposing it would invite code to persist a number Ruby itself regenerates per process; nothing in application-shaped Ruby calls it. `value.hash` refuses naming this. It can be built later, compatibly, the day a real gem needs it.
1. **`equal?` (identity) does not exist.** Values never mutate ([ADR 0015](0015-2026-07-23-values-never-mutate.md)), so identity is not observable and the question has no answer to give.

## Consequences

- `eql?` costs nothing: it is the runtime's own `PartialEq`, now callable.
- The checker types `eql?` as a boolean on every receiver, a struct's included.
- The seven `eql_spec` files port; the seven `hash_spec` files are recorded as decided out.
- Recorded in [equality.md](../ruby/equality.md).
