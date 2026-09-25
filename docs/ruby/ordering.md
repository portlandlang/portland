# Ordering

**Summary:** `<=>` and `include Comparable` migrate unchanged; strings order by canonical text, and `Comparable` is a builtin trait that needs your `<=>` to exist.

**Status:** decided ([ADR 0054](../adr/0054-2026-09-25-ordering.md), 2026-09-25), built in both oracles the same day.

## Ruby

`<=>` answers -1, 0, 1, or `nil` when the two are not comparable. `include Comparable` is a module that derives `<`, `<=`, `>`, `>=`, `==`, `between?`, and `clamp` from it. Strings compare by bytes. `sort`, `min`, and `max` call `<=>`, and a missing one surfaces as `comparison of X with Y failed` at runtime.

## Portland

```ruby
struct Version
  major
  minor

  include Comparable

  def <=>(other) = [major, minor] <=> [other.major, other.minor]
end

Version.new(major: 1, minor: 2) < Version.new(major: 1, minor: 10)   # true
[newer, older].sort                                                  # by <=>
"apple" < "banana"                                                   # true
5.clamp(1, 3)                                                        # 3
```

The differences, each small:

- **`<=>` never answers `nil`.** Two values it cannot order refuse, `cannot apply '<=>' to 1 and "a"`, since Portland has no ambient nil.
- **Strings order by canonical text**, not bytes — a decomposed `é` equals the composed one ([characters.md](characters.md)).
- **`Comparable` does not derive `==`.** Struct equality is already field-by-field; a `<=>` that disagrees with it would make two definitions of "equal".
- **`include Comparable` without a `<=>` refuses at the include**, not on the first comparison: `Box includes Comparable but defines no '<=>' — define it`. And `a < b` on a struct without the include names the fix: `— include Comparable in Box`.
- **`<=>` is the one operator a method may be named by.** `def +(other)` and friends are not Portland.

## Migration

- A `def <=>` and `include Comparable` port verbatim, as do callers reading the answer as an integer.
- A `<=>` that returns `nil` for "not comparable" refuses at that call; the rewrite is to not ask.
- `sort { |a, b| ... }` with a comparator block, `min_by`, and `max_by` are not built yet; `sort_by` covers most of them.
