# Ranges

**Status:** decided
([ADR 0019](../adr/0019-2026-07-23-ranges.md)), not yet built. Ruby
behavior below was verified against Ruby 4.0.6.

## Ruby

`1..5` inclusive, `1...5` exclusive, first-class objects, usable as
patterns and as slice indices. Two edges are sharper than their
reputation:

- **Slicing is asymmetric.** The end clamps freely, the start does not:
  `[1,2,3][1..99]` is `[2,3]`, but `[1,2,3][4..]` is `nil` and
  `[1,2,3][-99..]` is `nil`. And `[1,2,3][3..]` is `[]` — start *equal*
  to length is a valid boundary, start past it is not.
- **An endless range swallows the next line.** `x = 1..` followed by a
  line containing `5` is `1..5`, indented or not.

Ruby does not check `case` exhaustiveness at all, so range arms carry no
coverage obligation.

## Portland

### Slices are collections, never maybes

```ruby
words[1..3]      # Array — no unwrap, ever
words[99..]      # []
"hello"[9..]     # ""
```

The start clamps the way Ruby already clamps the end. `[4..]` and
`[-99..]` yield empty rather than nil.

**`array[1]` is unchanged — still `T?`** (see [lookups](lookups.md)).
The two are different questions: one *element* has an honest absence
answer; a *sub-collection* of nothing is `[]`, a perfectly good
collection. Ruby's nil-for-out-of-range slice is the nil/empty
conflation Portland exists to kill.

### Range patterns count toward exhaustiveness

```ruby
case score
in ..0    then "none"
in 1..9   then "some"
in 10..   then "lots"
end                      # proven total — no else needed
```

The checker sorts the integer ranges and requires no gaps, a beginless
first, an endless last. Gaps are compile errors. Overlap stays legal —
first-match-wins is real semantics, and the cascading `..10 / ..100 /
..1000` idiom depends on it — while an arm *entirely* covered by earlier
arms is already an unreachable-arm error
([pattern matching](pattern-matching.md)). Arm order carries no meaning
when arms are disjoint, so it is a lint someday, never a compile error.

### Endless ranges close on a token that can't continue them

`array[1..]`, `slice(1.., 2..)`, and `in 10.. then` need no parens. Only
where the next token could be an operand is there an error:

```ruby
span = 1..
p span
# error: endless range at end of line — does it continue?
#   the range, closed here:  span = (1..)
#   a range up to p span:    span = 1..p(span)
```

That is the one spot Ruby resolves by silently continuing — so the error
lands exactly where Ruby is a footgun.

## Migration

- Range literals, range patterns, and in-bounds slices — compile
  verbatim, same meaning.
- Out-of-range slices change `nil` to empty. Loud where it matters: code
  checking `slice.nil?` hits the never-absent-left-side error, `if
  array[4..]` is already an error (no truthiness), and unchecked code
  would have crashed in Ruby on `nil.each`.
- A trailing endless range in an assignment needs its parens —
  free-tier polyfill autocorrect, since `(1..)` is valid Ruby too.
- Exhaustiveness gains: `case` chains over ranges that were total in
  Ruby lose their now-unnecessary `else` — optional cleanup, not a
  requirement.

**Open to revision on evidence.** "Almost no real code relies on
nil-from-slice" is a prior, not a measurement — a good ruby_research
query when the full corpus lands.
