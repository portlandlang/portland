# Truthiness

**Status:** locked by the design brief (strict booleans follow from no
ambient nil); the rewrites below are the decided unwrap toolkit
([ADR 0008](../adr/0008-2026-07-22-unwrap-toolkit.md)).

## Ruby

Everything is truthy except `nil` and `false`. `if user` quietly means
"if user isn't nil," and whole idiom families lean on it.

## Portland

Conditions take booleans. Nothing else. There is no truthiness because
there is no ambient nil to be falsy — the two cuts are one cut.

`if user` where `user` is a maybe is a **compile error**, and the error
suggests the rewrite. What it becomes depends on what the Ruby line meant:

```ruby
# "do this only if it's there" — predicate + narrowing
if user.some?                    # user narrows to a plain User inside
  greet(user)
end

# "bail early if missing" — the or-guard
user = find_user(id) or return
puts "hi #{user.name}"           # plain User below

# "it was a chain link" — safe navigation, kept from Ruby
name = user&.nickname or "friend"

# both branches deserve real code — pattern match
case user
in User(name:) then greet(name)
in nil         then puts "who?"
end
```

If the condition was a plain boolean all along, nothing changes — strict
booleans only reject non-booleans.

## Migration

This is the single biggest mechanical migration cost, accepted knowingly:
every `if x` / `unless x` doing nil-work needs one of the rewrites above.
All failures are loud (compile errors with suggestions), never behavioral.
The polyfill linter can flag and autocorrect most cases in Ruby before the
flip.
