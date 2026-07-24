# Types

**Status:** locked direction (design brief); the inference design itself
is open ([#9](https://github.com/portlandlang/portland/issues/9)).

## Ruby

Dynamically typed; every check happens at runtime or never. Duck typing —
"if it quacks" — discovered in production. Sorbet/RBS exist as bolt-on
annotation systems because the pain is real, but they trade Ruby's clean
page for ceremony.

## Portland

Statically typed, **inferred, not written** — the lean is bidirectional
inference with local generalization (#9), not Hindley-Milner purity.
Types are present (the safety) but invisible (the joy). The page looks
like Ruby; the compiler knows everything anyway.

```ruby
def find_user(id)     # no annotations anywhere
  ...
end                   # inferred: takes an Integer, returns a User?
```

Type names appear in exactly three places:

1. **Error messages** — where they do their best work.
2. **Public boundary annotations**, optional, as documentation.
3. **Design conversations.**

Duck typing becomes **structural typing**: "responds to `quack`" is still
the contract, but checked at compile time. No nominal-interface ceremony;
the duck test just stops being administered in production.

## Migration

- Idiomatic Ruby mostly *is* well-typed already — it compiles unchanged
  and gains the checking silently.
- Code that gives one variable two types over its life, or builds
  heterogeneous ad-hoc structures, gets loud inference errors asking for
  clarity it should have had anyway.
- Sorbet/RBS annotations have no Portland equivalent to migrate to —
  delete them; the compiler infers what they declared.
