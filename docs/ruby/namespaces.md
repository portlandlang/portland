# Namespaces and modules

**Status:** decided
([ADR 0021](../adr/0021-2026-07-24-namespaces-and-modules.md)), not yet
built.

## Ruby

`module` does **two unrelated jobs**: it is a namespace *and* a mixin. There
is no way to bring a name into scope without mixin machinery — `include
Math` just to type `sqrt` instead of `Math.sqrt` recruits the ancestor chain
to solve a naming problem, and `module_function` exists to paper over the
mismatch.

The consequence is that `include Foo` is ambiguous to a reader:

```ruby
include Comparable   # real mixin — you want <=> to power < > between?
include Math         # namespace convenience — you just didn't want to type Math.
```

Same keyword, unrelated intents, and you have to know what's inside the
module to tell which.

Ruby also offers two ways to declare a nested namespace, and **they behave
differently**:

```ruby
module A
  LIMIT = 10
  module B
    LIMIT      # 10 — lexical nesting is [A::B, A]
  end
end

module A::B
  LIMIT      # NameError — lexical nesting is only [A::B]
end
```

And `::` versus `.` is convention rather than rule — Ruby permits
`Foo::bar()` to call a method.

## Portland

**`module` is namespace-only.** Mixins are a separate, later decision and
will get a different keyword, so the two intents can never be confused at a
call site.

**`::` names, `.` invokes** — as a rule, not a style:

```ruby
Shapes::Circle                   # naming
Statistics::PI                   # naming
Statistics.mean(readings)        # invoking
Shapes::Circle.new(radius: 5)    # both, in order
```

`Statistics::mean(x)` is a compile error naming the fix. This is Ruby's own
convention (`Math::PI` vs `Math.sqrt(4)`) promoted from habit to law.

**Names are always fully qualified.** There is no import, no aliasing, and
no way to inject names into scope. Lexical nesting is the only shortening —
inside `module Statistics`, its own names are bare, exactly as in Ruby.

**Both declaration forms exist and mean the same thing.** `module A::B` is
identical to nested blocks, including lexical visibility of `A`'s names.
Ruby's `Module.nesting` asymmetry is not reproduced.

**Types nest in types; modules do not nest in types.**

```ruby
struct Invoice
  lines

  struct Line
    description
    amount
  end
end

struct Foo
  module Helpers   # compile error — modules group things, a struct is a thing
  end
end
```

**Files and modules are unrelated**, as in Ruby: `require_relative` loads a
file and implies no namespace. (Rails' path-to-constant convention comes
from Zeitwerk, a library, not the language.)

**Constants need no separate concept.** In Ruby they exist because variables
are mutable by default; under [mutability](mutability.md) everything is
already unrebindable, so `MAX = 5` was always constant — it only lacked a
place to live.

## Migration

- **`Foo::Bar` and `Foo.bar()`** — compile verbatim wherever they follow
  Ruby's own convention, which is nearly everywhere. Free tier.
- **`module Foo::Bar`** — compiles verbatim *and* behaves less surprisingly
  than in Ruby, since outer names stay visible. Free tier, and it silently
  fixes a bug class.
- **`Foo::bar()` used to call** — compile error, fix is `Foo.bar()`. Free
  tier autocorrect; the distinction is mechanical.
- **`include Foo` for namespace convenience** — no equivalent. The fix is to
  qualify the call sites. Loud, mechanical, but real work in code that leans
  on it.
- **`include Foo` for a genuine mixin** — waits on the object-model session;
  no answer yet either way.
- **Deeply nested namespaces** — unaffected. Depth is forced by collision
  avoidance in any ecosystem (`Nokogiri::XML::Document`,
  `Aws::S3::Client`), and Portland keeps the path form precisely so those
  libraries do not pay three indents per file.
