# The language

**For:** anyone reading or writing Portland — what it is, what the rules are, and how it's meant to be written.

Everything here **runs today**, unless a section says otherwise. Run it with `script/console a_file.pdx`, or line by line with `script/console` and no arguments.

How it's built underneath is [architecture.md](architecture.md). Why each rule is the way it is lives in [ADRs](adr/); what it costs a Rubyist lives in [docs/ruby/](ruby/).

## A taste

```ruby
struct Token
  kind
  text

  def integer?
    kind == "integer"
  end
end

def describe(token)
  return "an empty token" if token.text.empty?

  case token
  in Token(kind: "integer", text:) then "the number #{text}"
  in Token(kind: "word", text:)    then "the word #{text}"
  else
    "a #{token.kind}"
  end
end

tokens = %w[42 pdx].map { Token.new(kind: "word", text: it) }
tokens.each do |token|
  puts describe(token)
end

puts tokens.first&.integer?
```

```text
the word 42
the word pdx
false
```

## The rules that govern everything

Four rules do most of the work. They are not features; they are the reasons other features look the way they do.

1. **No shadowing.** A name is a local or a method, never both. Assigning `greet = 1` where a method `greet` exists is an error. This is what makes a bare name unambiguous, which is what makes paren-less calls safe.
1. **Never guess.** Where one spelling has two genuine readings, Portland refuses and shows you both with their rewrites. It never picks.
1. **Immutable by default.** Bare binding is immutable; `mutable` is the marked, rare case. Values never mutate at all — names do.
1. **No truthiness, no ambient nil.** Conditions take booleans and nothing else. Absence is an explicit, typed maybe rather than a value's secret.

## Values

**Integers** are `i64`. **Floats** are IEEE doubles, printed Ruby's way, and mixed arithmetic promotes. A dot is only a float when a digit follows it, which is what keeps `1..5` a range.

Arithmetic is `+ - * / %` with unary minus and parens. `+` concatenates strings and arrays; `*` repeats them. Integer division and modulo are **floored, exactly as in Ruby** (ADR 0018) — `-7 / 2` is `-4`, and `-7 % 2` is `1`.

Comparison is `== != < <= > >=`. Equality works across all types, with mixed types simply unequal; ordering is integers-only for now.

**Strings** are double-quoted, with `\n` `\t` `\"` `\\` `\#` escapes and `#{...}` interpolation that auto-`to_s`es and nests.

**Heredocs** are squiggly only — `<<~SQL`, SCREAMING_CAPS terminator (ADR 0020). `<<~'SQL'` suppresses interpolation. Indented terminators, attached method calls, and several per line all work. `<<EOS` and `<<-EOS` do not exist, and that is what lets `<<` stay unambiguously the append operator with no disambiguation rule anywhere.

**Arrays** are `[1, "two", [3]]`. **Hashes** are `{"key" => value}`, insertion-ordered, any type of key. `%w[rose city]` builds a word array — with one live bug: it cannot contain a `]`, because the lexer scans to the first one (#29).

**Ranges** are `1..5` and `1...5`, with endless and beginless forms (ADR 0019). Integer bounds only. `(1..n)` iterates via `each`/`map`/ `sum`/`to_a`, `include?` answers without walking, and range patterns match by membership.

A range spans a newline **only where one reading exists.** Both ambiguous shapes are errors: a trailing `1..` at end of line, and a line-initial `..4` after a complete expression. Ruby's line-crossing behavior here is three unrelated accidents — trailing `..` reaches forward, leading `..` does not reach back even inside parens, but a leading `.` does. Portland has one rule and names both readings.

**Structs** are immutable named records:

```ruby
struct Token
  kind
  text
end

token = Token.new(kind: "integer", text: "42")   # kwargs only, all fields required
token.kind                                       # field access
token.with(text: "43")                           # updated copy; nothing mutates
```

Value equality, definition-ordered fields, capitalized names. Struct bodies take fields first, then methods. Inside a method, bare names resolve locals → the receiver's fields → its own methods → top-level methods, with no-shadow enforced across every layer; `new`, `with`, `nil?`, and `some?` are reserved. `self` is the receiver, and exists for the pass-myself-along case.

Types nest in types (`Invoice::Line`). Modules inside structs are an error.

## Names and binding

```ruby
count = 1                  # immutable
mutable total = 0          # the one rebindable kind of name
total += 5
```

`mutable` is fused to first assignment and gates **rebinding only** (ADRs 0001, 0015). Compound assignment (`+= -= *= /= %=`), the append `line << word`, and index assignment (`hash[key] = value`, `array[index] = value`) all require it.

Values never mutate. `<<` and `[]=` are rebinding sugar, not mutation, so aliases can never be spooked:

```ruby
mutable numbers = [1]
others = numbers
numbers << 2
others.length              # 1 — in Ruby this would be 2
```

Bang methods (`upcase!`, `push`) do not exist and will not. Rebinding spells it: `word = word.upcase`.

Parameters are binding sites too, so `def f(mutable position)` works. Blocks rebind outer mutables — the accumulator pattern — refuse outer immutables with the fix named, and their own fresh locals die at `end`. Loop iterations are fresh scopes for their own locals, which is the block rule applied to `while`.

Constants needed no new concept: immutability already makes `MAX = 5` unrebindable. All that was missing was a place for the name to live, and that is what namespaces are for.

## Absence

The headline feature (ADRs 0005–0010, 0012). There is no ambient nil. Absence is one explicit case of a maybe, and it is typed.

`nil` is a keyword literal with no methods — `nil.upcase` refuses and tells you to handle the nil case first, `puts nil` refuses, and it is not falsy because nothing is. `nil?` and `some?` work on every value; they are the one dispatch a maybe allows.

**Partial operations return maybes** instead of panicking: `[].first`, `last`, `min`, `max`, out-of-range array and string indexing, and missing hash keys. `fetch` retired, because all three of its arities are the or-guard with lazy evaluation for free.

The unwrap toolkit is four things, and deliberately nothing else:

```ruby
name = lookup(id) or "anonymous"        # or — lazy, typed
user = find(id) or return               # or-guard: return / break / next
config = load_config or panic "no key"  # the only crash you can write
title = article&.headline               # &. — absent receiver short-circuits
case value
in nil then "nothing"                 # case/in matches payload or nil
in found then found.upcase
end
```

There is **no `if let`** and **no force-unwrap operator**. `or panic "why"` is the single crash spelling, which makes `grep panic` an audit of every accepted risk in a codebase.

`or`, `and`, and `not` are **dead-identical** to `||`, `&&`, and `!` — same precedence, same semantics (ADR 0007). Ruby's secret `and`/`or` precedence is gone, so `x = nil or 7` binds the `or` first. `or` is _typed_: booleans get logical or, a maybe gets unwrap-or-else.

`some(x)` is written only in genuinely nested cases. It is identity on plain values and a real box only around `nil` or another `some`, so `[nil].first` and `[].first` differ, and a stored hash nil beats an or-guard default — Ruby's `fetch` rule, preserved.

The **static** half of this — flow narrowing, unhandled-maybe compile errors, the `Boolean?` never-guess, dead right-hand sides — is the real compiler's job (#9). Today those surface as runtime panics.

## Control flow

Everything is an expression.

```ruby
greeting = if morning?
  "good morning"
else
  "hello"
end

puts "quiet" unless loud?
return 0 if count < 0
```

There is no ternary operator — `?` is not an operator at all, it is part of a name. `then` belongs to `case`, not to `if`.

`if` / `elsif` / `else` / `end`, `unless`, `while`, `case/when` with aligned `when x then y` one-liners, and postfix guards. Conditions are strict booleans — there is no truthiness, because there is no nil to be falsy.

**A branch that doesn't happen is nil** (ADR 0012): a branchless `if`, a finished `while`, and a call broken out of all produce nil, typed. `puts` alone produces _nothing_ — using its result is an error — because it could never have had an answer. That is the dividing rule: could this ever have produced a value?

`return` exits the enclosing method, unwinding through loops _and_ blocks. `break` and `next` control the enclosing `while` or block iteration.

## Pattern matching

Load-bearing, not a corner feature (ADR 0013). The trio dispatches on its own AST with it.

```ruby
case node
in Node(kind: "integer", text:)  then text.to_i
in [first, *rest]                then first
in 1 | 2 | 3                     then "small"
in ^expected                     then "matched the pinned value"
in count if count > 10           then "many"
in 1..9                          then "single digit"
in nil                           then "absent"
end
```

Struct patterns are **keyword-only**, and `text:` on its own binds a name of the same name. Captures bind and persist as in Ruby, fenced by no-shadow. One-line forms: `expression in pattern` is a boolean that binds on a hit, and `expression => pattern` destructures rightward and panics on mismatch — which is also the answer to destructuring assignment.

Builtin type patterns are `in String`, `Integer`, `Array`, `Hash`, `Boolean`. **The type predicate is a pattern, not a reflection API** — `is_a?` and `.class` do not exist.

No match and no `else` panics today. That is the runtime preview of what the real compiler will check statically: exhaustiveness. Range patterns count toward it — sorted integer ranges with a beginless first, an endless last, and no gaps are total, and need no `else`. Overlap is legal, first match wins, and gaps are an error.

Hash patterns and the find pattern (`in [*, x, *]`) are deliberately not built; they wait to be pulled for.

## Methods and calls

```ruby
def greet(name, greeting: "hello", loud: false)
  message = "#{greeting}, #{name}"
  return message.upcase if loud
  message
end
```

Implicit return of the last expression, arity-checked calls, trailing default parameter values that may reference earlier parameters, and Ruby 3 keyword parameters (ADR 0014) — `label:` required, `label: default` optional, strictly separate from positionals, accepted on both paren and command calls. Splats are deferred. Method bodies get a fresh scope and see no outer locals, which is Ruby's rule, kept.

**Paren-less calls** work at statement position — `puts "hello"`, `shout word, other` — and bare zero-argument calls work anywhere: `ready?`, `pdx`. Two rules replace Ruby's whitespace heuristics: no shadowing makes the bare name unambiguous, and never-guess turns the genuinely ambiguous forms into errors. `puts -1`, `puts [1]`, and `puts (1)` each say _ambiguous without parens_ and show both readings. `foo - 1` stays subtraction.

## Blocks

`do |item| ... end` and `{ |item| ... }` are **dead-identical** (ADR 0016) — Ruby's braces-bind-tighter rule is gone. They work on `each` (arrays, and hashes with `|key, value|`), `each_with_index`, `map`, `select`, `reject`, `reduce(initial)`, `times`, `upto`, `downto`. Blocks are closures; their parameters are block-local.

Naming **`it`** declares the implicit parameter (ADR 0017):

```ruby
squares = numbers.map { it * it }
```

`it` is an ordinary binding under no-shadow, not a soft keyword. Every collision is a shadow and every shadow is an error: declared parameters, a nested `it`, or a local of that name. `_1`–`_9` do not exist; `it` and named parameters cover the space.

A bare `{` after a paren-less command call is a never-guess error that names each reading — hash argument, the inner call's block, the outer call's block — and the parser peeks to _trim_ that menu when a `|` rules the hash out, never to pick.

## Namespaces

```ruby
module Statistics
  LIMIT = 10

  struct Summary
    mean
  end

  def mean(values)
    values.sum / values.length
  end
end

Statistics.mean(readings)              # invoking → .
Statistics::LIMIT                      # naming → ::
Statistics::Summary.new(mean: 2)       # both, in order
```

`module` is a namespace and **nothing else** (ADR 0021). Mixins are deferred and will get a different keyword, so `include Comparable` and `include Math` can never be confused the way they are in Ruby.

**`::` names, `.` invokes** — a rule, not a convention. `Statistics::mean(x)` is a never-guess error.

Names are **always fully qualified**: no import, no aliasing, no injection, with lexical nesting the only shortening. `module A::B` and nested blocks are semantically identical, which drops Ruby's `Module.nesting` trap. Bare names resolve outward from where they were _written_, not from where they are called.

## Multi-file programs

`require_relative "lexer"` — resolved against the requiring file's directory, `.pdx` implied, loaded once and returning false on a repeat.

## The library, so far

Read-only, and small on purpose.

| Type       | Methods                                                                                                          |
| ---------- | ---------------------------------------------------------------------------------------------------------------- |
| String     | `length upcase downcase reverse empty? chars split include? start_with? end_with? to_i to_f slice` and `[index]` |
| Integer    | `abs zero? positive? negative? even? odd? to_f`                                                                  |
| Float      | `abs to_i`                                                                                                       |
| Array      | `length first last empty? join include? sum min max sort slice` and `[index]`, negative indices included         |
| Hash       | `length empty? key? keys values` and `[key]`                                                                     |
| Everything | `to_s`, `nil?`, `some?`                                                                                          |

Method chains continue across newlines with a leading dot.

**IO** is `puts` (one line per argument, produces no value), `p` (prints `inspect` renderings and returns its argument, like Ruby), and crude file access: `argv()`, `read_file(path)`, `write_file(path, content)`. Those three are placeholder names until the real object model exists.

Comments are `#` to end of line. Names are `snake_case`, and `?`/`!` suffixes are part of the name.

## Style

Emerging rather than settled — this section describes how the trio is actually written, and it will grow as more of the language exists.

- **`it` for a single block parameter**, named parameters otherwise. `nodes.map { it.sexp }` reads better than naming a variable you use once. Nesting forces names anyway, since a nested `it` is a shadow.
- **`<<` to accumulate, `+=` to concatenate or count.** Building a collection an element at a time is `tokens << token`; adding two whole things is `+=`. The trio converted about forty-five sites to this rule and kept `+=` where it was genuinely concatenation.
- **Leading dot for chains that break across lines.**
- **Keyword arguments at struct construction**, which the language requires, and increasingly at call sites where a bare positional would be a mystery at the call site.
- **Ask, don't guess.** When you write a construct with two readings, the compiler will make you disambiguate. Reach for the parens rather than learning which way it would have gone — there is no which-way to learn.

Open, and not yet ruled on: whether `or`/`and` or `||`/`&&` is preferred prose when they are dead-identical, and how much to lean on postfix guards. Both wait on more real Portland to look at.

## Decided, not yet built

- **Concurrency** (ADRs 0002, 0004, 0011 — tentative). Three tiers, and you live almost entirely in the first: implicit parallelism, safe _because_ values are immutable, where `photos.map { it.thumbnail }` spreads across cores when it is worth it and you never asked. Tier two declares independence:

  ```ruby
  together do
    meanwhile user = fetch_user(id)
    ~ orders = recent_orders(id)      # ~ and meanwhile are dead-identical
  end
  render(user, orders)                # plain values after end
  ```

  Results are named at the task site — there is no positional register. Tier three is explicit control: cancellation, timeouts, racing. Rare. Semantics are #11.

- **Symbols.** The core question is decided: `:foo` exists as a general type, checked for membership where a closed vocabulary is declared, and `{name: "pdx"}` is symbol-keyed and ships. The ADR waits on the enum shape.
- **Bitwise operators are out** (ADR 0003, tentative), with named methods instead.

## Not yet designed

Enums and sum types (next up, now that namespaces exist to hold them), the rest of the object model — mixins, inheritance, visibility (#27), error handling and the deferred `!` (#28), compile-time macros (#14), regex, the `%` literal zoo (#29), string semantics beyond what Rust's choices gave the seed by accident, and types themselves (#9).

## Gone, on purpose, forever

Monkeypatching and open classes. `method_missing`. Runtime `define_method`. `eval`. Globals and `$specials`. Truthiness. Ambient nil and `NilClass`. The GIL and the `Thread` API. `for`, `BEGIN`/`END`, flip-flops, and the `and`/`or` precedence trick. `fetch`. Force-unwrap operators and `if let`-style binding conditionals. Numbered block parameters. In-place mutators.

Runtime metaprogramming's replacement is compile-time macros, which are undesigned. Everything else on this list is simply not coming back — see [docs/ruby/removed-syntax.md](ruby/removed-syntax.md) for what each one costs a migrating codebase, and how loudly it fails.
