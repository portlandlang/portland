# Types

**Summary:** Static and inferred, written only at public boundaries; duck typing becomes structural.

**Status:** design core decided ([ADR 0040](../adr/0040-2026-08-19-inference-the-design-core.md)): parameter contracts are structural from the body — the duck test at compile time, as promised — _sharpened_ by concrete types flowed from call sites; traits double as names for shapes; `Maybe[T]` is a wrapper, never a union, with the narrowing forms inventoried and built. Contracts have their rulings ([ADR 0048](../adr/0048-2026-09-23-contracts-the-rulings.md)): a contract is spelled `needs '.upcase', '.length'`, is the body's own demand with nothing inherited, and a trait's name may stand in for it in errors on an exact match; caller-flow sharpening waits for a whole-program checker. Return annotations are decided ([ADR 0041](../adr/0041-2026-08-19-return-annotations-are-arrow-comments.md)) and checked; the error voice is decided ([ADR 0047](../adr/0047-2026-09-22-the-error-voice.md)) and its refusals built; generics wait deliberately; increments build in [#9](https://github.com/portlandlang/portland/issues/9).

## Ruby

Dynamically typed; every check happens at runtime or never. Duck typing — "if it quacks" — discovered in production. Sorbet/RBS exist as bolt-on annotation systems because the pain is real, but they trade Ruby's clean page for ceremony.

## Portland

Statically typed, **inferred, not written** — the lean is bidirectional inference with local generalization (#9), not Hindley-Milner purity. Types are present (the safety) but invisible (the joy). The page looks like Ruby; the compiler knows everything anyway.

<!-- not-portland: `...` is a placeholder body, not Portland -->

```ruby
def find_user(id)     # no annotations anywhere
  # ...
end                   # inferred: takes an Integer, returns a User?
```

Type names appear in exactly three places:

1. **Error messages** — where they do their best work.
1. **Public boundary annotations**, optional, as documentation — spelled as a trailing arrow comment, `def find_user(id) # -> User?` ([ADR 0041](../adr/0041-2026-08-19-return-annotations-are-arrow-comments.md)): rbs-inline's placement with RBS's own returns-arrow as the marker, valid Ruby verbatim (the one annotation form that is polyfill-_free_), and checked against inference the moment the checker can — an annotation can never lie. Return types only; input annotations are deliberately TBD.
1. **Design conversations.**

Duck typing becomes **structural typing**: "responds to `quack`" is still the contract, but checked at compile time. No nominal-interface ceremony; the duck test just stops being administered in production. Written down, a contract is what the body demands, spelled as the source spells it — `needs '.quack', '.waddle'`, an operator bare of the dot ([ADR 0048](../adr/0048-2026-09-23-contracts-the-rulings.md)); `types.pdx` shows one per parameter, and an error names a matching trait instead when one fits exactly.

## Migration

- Idiomatic Ruby mostly _is_ well-typed already — it compiles unchanged and gains the checking silently.
- Code that gives one variable two types over its life, or builds heterogeneous ad-hoc structures, gets loud inference errors asking for clarity it should have had anyway.
- Sorbet/RBS annotations have no Portland equivalent to migrate to — delete them; the compiler infers what they declared.
