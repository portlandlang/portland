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

Arithmetic is `+ - * / % **` with unary minus and parens. `+` concatenates strings and arrays; `*` repeats them. Integer division and modulo are **floored, exactly as in Ruby** (ADR 0018) — `-7 / 2` is `-4`, and `-7 % 2` is `1`. Exponent is `**` or its named twin `pow` ([ADR 0033](adr/0033-2026-08-11-exponent-is-starstar-and-pow.md)): right-associative, above `*`, floats through the host, and a minus before `**` applies last — `-2 ** 2` is `-4`, Ruby's answer and mathematics'. The one refusal is the chained negative literal (`-5.abs ** 2`), where Ruby's own rules contradict each other. A negative integer exponent and a past-i64 result refuse with their rewrites named, since Portland has neither rationals nor bignums.

Comparison is `== != < <= > >=`. Equality works across all types, with mixed types simply unequal; ordering is integers-only for now.

**Strings** are double-quoted, with `\n` `\t` `\"` `\\` `\#` escapes and `#{...}` interpolation that auto-`to_s`es and nests.

**Heredocs** are squiggly only — `<<~SQL`, SCREAMING_CAPS terminator (ADR 0020). `<<~'SQL'` suppresses interpolation. Indented terminators, attached method calls, and several per line all work. `<<EOS` and `<<-EOS` do not exist, and that is what lets `<<` stay unambiguously the append operator with no disambiguation rule anywhere.

**Arrays** are `[1, "two", [3]]`. **Hashes** are `{"key" => value}`, insertion-ordered, any type of key. `%w[rose city]` builds a word array, with Ruby's content rules ([ADR 0030](adr/0030-2026-07-27-word-array-contents.md)): `\]` escapes a bracket, balanced brackets nest, `rose\ city` keeps its space, and whitespace runs split once. The rest of the `%` family waits on #29.

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

Value equality, definition-ordered fields, capitalized names. Struct bodies take fields first, then methods. Inside a method, bare names resolve locals → the receiver's fields → its own methods → top-level methods, with no-shadow enforced across every layer; `with`, `nil?`, and `some?` are reserved. `self` is the receiver, and exists for the pass-myself-along case.

**Construction logic lives in type functions** ([ADR 0031](adr/0031-2026-07-31-smart-constructors-definable-new.md)): `def self.name` in a struct body declares a function on the type — `Token.of(raw)` — with no instance and no `self` value inside, bare names resolving outward from the struct's namespace. `new` is one of the definable names: defining it replaces the raw constructor everywhere, takes any signature (positional included), and returns the value or a `failure` (ADR 0027); inside it — and only there — `fields(kind: "word", text: raw)` is the raw kwargs-in-fields-out layer every struct starts with. One namespace serves instances and the type, so the same name on both sides refuses naming both. In a module body, `def self.` is accepted as a plain `def` — Ruby's spelling of the meaning Portland's module `def` already has.

```ruby
struct Badge
  label

  def self.new(raw)
    return failure("a badge needs text") if raw.empty?
    fields(label: raw)
  end
end

badge = Badge.new("rose") or panic "a badge was required"
```

Types nest in types (`Invoice::Line`). Modules inside structs are an error.

**Traits carry shared behavior** (ADR 0028): a `trait` is a bundle of methods with no state, and a struct `include`s it —

```ruby
trait Sexpable
  def sexp_list(nodes)
    nodes.map { it.sexp }.join(" ")
  end
end

struct ArrayNode
  elements

  include Sexpable

  def sexp
    "(array #{sexp_list(elements)})"
  end
end
```

The verb is Ruby's and the safety is in the noun: `trait` and `module` are distinct declarations, so `include Statistics` — a namespace — is a refusal with the rewrite named, and Ruby's two meanings of `include` cannot be spelled. Trait methods merge into the struct at its declaration, reach the carrier's fields bare, and sit in the same no-shadow ladder as everything else — every collision (two traits, a trait and an own method, a trait method and a field, even a trait method's *parameter* against a carrier's field) is a refusal naming both owners, never Ruby's silent last-include-wins. There is no inheritance and no `class` — declined, not deferred. The compiler's own AST nodes carry `trait Sexp`, which is the dogfood.

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

Bang methods (`upcase!`, `push`) do not exist and will not — `!` belongs to call sites as unwrap-or-propagate (ADR 0027), and `def save!` is a refusal naming the rewrite. Rebinding spells the mutation Ruby's bangs meant: `word = word.upcase`.

Parameters are binding sites too, so `def f(mutable position)` works. Blocks rebind outer mutables — the accumulator pattern — refuse outer immutables with the fix named, and their own fresh locals die at `end`. Loop iterations are fresh scopes for their own locals, which is the block rule applied to `while`.

Constants needed no new concept: immutability already makes `MAX = 5` unrebindable. All that was missing was a place for the name to live, and that is what namespaces are for.

## Absence

The headline feature (ADRs 0005–0010, 0012). There is no ambient nil. Absence is one explicit case of a maybe, and it is typed.

`nil` is a keyword literal with no methods — `nil.upcase` refuses and tells you to handle the nil case first, `puts nil` refuses, and it is not falsy because nothing is. `nil?` and `some?` work on every value; they are the one dispatch a maybe allows.

**Partial operations return maybes** instead of panicking: `[].first`, `last`, `min`, `max`, out-of-range array and string indexing, and missing hash keys. `fetch` retired, because all three of its arities are the or-guard with lazy evaluation for free.

The unwrap toolkit is five things, and deliberately nothing else:

```ruby
name = lookup(id) or "anonymous"        # or — lazy, typed
user = find(id) or return               # or-guard: return / break / next
config = load_config or panic "no key"  # the only crash you can write
title = article&.headline               # &. — absent receiver short-circuits
content = read_file!(path)              # ! — unwrap-or-propagate (failures, below)
case value
in nil then "nothing"                 # case/in matches payload or nil
in found then found.upcase
end
```

There is **no `if let`** and **no force-unwrap operator**. `or panic "why"` is the single crash spelling, which makes `grep panic` an audit of every accepted risk in a codebase.

`or`, `and`, and `not` are **dead-identical** to `||`, `&&`, and `!` — same precedence, same semantics (ADR 0007). Ruby's secret `and`/`or` precedence is gone, so `x = nil or 7` binds the `or` first. `or` is _typed_: booleans get logical or, a maybe gets unwrap-or-else.

`some(x)` is written only in genuinely nested cases. It is identity on plain values and a real box only around `nil` or another `some`, so `[nil].first` and `[].first` differ, and a stored hash nil beats an or-guard default — Ruby's `fetch` rule, preserved.

**Failure is absence with a reason** (ADR 0027). A fallible operation returns its value or a `failure(reason)` — always a real box, since its whole job is marking the sad path — and the toolkit above handles it unchanged: `or` takes the fallback, `case/in` reaches *through* the box to the reason (a failure is transparent to patterns, where a some-box stays opaque), and `failure?` joins `nil?`/`some?` as a universal dispatch. `!` is the propagation: `read_file!(path)` yields the content or returns the failure from the enclosing method — one greppable character per frame a failure may cross, unwinding to its write site like any `return` (ADR 0025). There is no `raise` and no `rescue`: a failure never crosses a frame that did not mark it. `puts` refuses a failure as it refuses nil; `p` and `inspect` render it. `read_file` and `write_file` are the first fallible operations, and a method can never be *named* with `!` — `def save!` is a refusal pointing at the call site.

The **static** half of this — flow narrowing, unhandled-maybe compile errors, the `Boolean?` never-guess, dead right-hand sides — is the checker's job (#9). Today those surface as runtime panics.

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

`return` exits the enclosing method, unwinding through loops _and_ blocks — the method the `return` was **written** in, so a helper that merely `yield`ed to the block is unwound through, never answered (ADR 0025, Ruby's rule). `break` and `next` control the enclosing `while` or block iteration. After a dot, though, `next` is a method name — `page.next` is `page.succ` — because loop control can never stand where a message goes, so the two readings never meet; `next` is the only keyword admitted there, on pull ([#79](https://github.com/portlandlang/portland/issues/79)), not keywords wholesale.

## Pattern matching

Load-bearing, not a corner feature (ADR 0013). The compiler dispatches on its own AST with it.

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

No match and no `else` panics today. That is the runtime preview of what the checker will enforce statically: exhaustiveness. Range patterns count toward it — sorted integer ranges with a beginless first, an endless last, and no gaps are total, and need no `else`. Overlap is legal, first match wins, and gaps are an error.

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

**Paren-less calls** work at statement position — `puts "hello"`, `shout word, other` — and bare zero-argument calls work anywhere: `ready?`, `pdx`. **Dot calls take paren-less arguments too** ([ADR 0032](adr/0032-2026-08-11-dotted-calls-take-paren-less-arguments.md)): `expect(x).to eq(y)`, `words.join "-"`, `"portland".slice 0, 4` — adjacency nests to the innermost call, commas make siblings, a `do` past the arguments belongs to the outermost call, and a bare command as an argument (`.to eq y`) stays unparsed on purpose. Two rules replace Ruby's whitespace heuristics: no shadowing makes the bare name unambiguous, and never-guess turns the genuinely ambiguous forms into errors. `puts -1`, `puts [1]`, and `puts (1)` each say _ambiguous without parens_ and show both readings, dotted or bare. `foo - 1` stays subtraction.

## Blocks

`do |item| ... end` and `{ |item| ... }` are **dead-identical** (ADR 0016) — Ruby's braces-bind-tighter rule is gone. They work on `each` (arrays, and hashes with `|key, value|`), `each_with_index`, `map`, `select`, `reject`, `reduce(initial)`, `times`, `upto`, `downto`. Blocks are closures; their parameters are block-local.

Naming **`it`** declares the implicit parameter (ADR 0017):

```ruby
squares = numbers.map { it * it }
```

`it` is an ordinary binding under no-shadow, not a soft keyword. Every collision is a shadow and every shadow is an error: declared parameters, a nested `it`, or a local of that name. `_1`–`_9` do not exist; `it` and named parameters cover the space.

A user-defined method takes a block too, reached with `yield`, and a paren-less call may be handed one:

```ruby
describe "Array#first" do
  specify "comes back absent when the array was empty" do
    expect([].first).to(be_nil)
  end
end
```

`do ... end` there belongs to the **outermost** call, always — `outer inner do ... end` hands the block to `outer` — which is Ruby's rule and needs no menu, because there is only one reading to have.

**`{ ... }` attaches wherever `do ... end` does** (ADR 0024) — dot calls, parenthesized calls, and paren-less calls alike:

```ruby
repeat { greet name: "pdx" }
```

A bare `{` is refused only where the readings genuinely collide. After an argument there are three — hash argument, the inner call's block, the outer call's block — and directly after a name there are two, hash argument or block. The parser peeks at the **first pair position** to trim that menu: a hash's first element is `label: value` or `value => value`, so `{ greet name: "pdx" }` cannot be a hash however hash-like it looks further in. Where the menu trims to one reading there is nothing to guess and the braces are simply that call's block; where two survive — `{}`, `{name: 1}`, `{ "a" => 1 }` — the error names each with a rewrite that parses.

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

`module` is a namespace and **nothing else** (ADR 0021). Mixins are decided and unbuilt: a `trait` is its own declaration kind, and `include` takes only traits (ADR 0028) — so `include Comparable` and `include Math` can never be confused the way they are in Ruby, because including a namespace is a refusal with the rewrite named.

**`::` names, `.` invokes** — a rule, not a convention. `Statistics::mean(x)` is a never-guess error.

Names are **always fully qualified**: no import, no aliasing, no injection, with lexical nesting the only shortening. `module A::B` and nested blocks are semantically identical, which drops Ruby's `Module.nesting` trap. Bare names resolve outward from where they were _written_, not from where they are called.

## Symbols and enums

A symbol is a name rather than data (ADR 0023). `:paid`, `:paid?`, and `:"odd key"` when the name is not identifier-shaped. Comparison is equality only; a symbol is not a String, and `:paid == "paid"` is `false` here where the checker will refuse it.

Hash keys use the shorthand, and it is the only spelling of a symbol key:

```ruby
config = {name: "pdx", port: 8080}
config[:name]                        # => "pdx"
mixed = {"string" => 1, sym: 2}      # the rocket stays for other keys
```

`{:name => "pdx"}` is refused with the rewrite named. **There is no `String#to_sym`** — a symbol built from a string at runtime could never be checked against a vocabulary, so cutting it is what makes checking possible at all. Operator symbols (`:+`, `:[]`) went with the `send` and `&:` family that needed them.

An **enum** declares a closed vocabulary of symbol cases, and a case may carry a keyword-only payload (ADR 0022):

```ruby
enum Ordering            # top-level: owned by no type
  :less
  :equal
  :greater
end

struct Purchase
  amount
  status

  enum Status            # nested: owned by Purchase
    :pending
    :paid(on:)
    :refunded(on:, reason:)
  end
end

purchase = Purchase.new(amount: 40, status: :paid(on: "tuesday"))

case purchase.status
in :pending           then "not paid yet"
in :paid(on:)         then "paid #{on}"
in :refunded(reason:) then "refunded — #{reason}"
end
```

Nesting follows the rule that nests types in types, so `Purchase::Status` names the type while `purchase.status` reads the value. A payload-free case is simply a symbol — nothing about `:pending` needs to exist at runtime. A payload-carrying one destructures in `case/in` by label, exactly as a struct pattern does.

There are no generated predicates and no `Status.all`: an enum is a type, not a lookup table.

**What the seed does not do yet.** Membership (`:pendign` is not a case of that enum) and exhaustiveness over cases are *static* — they need to know which enum a position expects, and the seed has no types. Both wait for [#9](https://github.com/portlandlang/portland/issues/9). The one check a runtime can make is here: a payload must name the labels its case declared, so `:paid(wrong: 1)` says `` `:paid` takes (on:) ``.

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

**`inspect(value)`** is `p` without the printing — the source-shaped rendering as a string, for composing into messages. A function rather than Ruby's method, because absence is its most important input and nil has no methods (ADR 0026); `inspect(nil)` is `"nil"`.

Comments are `#` to end of line. Names are `snake_case`, and `?`/`!` suffixes are part of the name.

## Style

Emerging rather than settled — this section describes how the compiler is actually written, and it will grow as more of the language exists.

- **`it` for a single block parameter**, named parameters otherwise. `nodes.map { it.sexp }` reads better than naming a variable you use once. Nesting forces names anyway, since a nested `it` is a shadow.
- **`<<` to accumulate, `+=` to concatenate or count.** Building a collection an element at a time is `tokens << token`; adding two whole things is `+=`. The compiler converted about forty-five sites to this rule and kept `+=` where it was genuinely concatenation.
- **Leading dot for chains that break across lines.**
- **Keyword arguments at struct construction**, which the language requires, and increasingly at call sites where a bare positional would be a mystery at the call site.
- **Ask, don't guess.** When you write a construct with two readings, the compiler will make you disambiguate. Reach for the parens rather than learning which way it would have gone — there is no which-way to learn.

Open, and not yet ruled on: whether `or`/`and` or `||`/`&&` is preferred prose when they are dead-identical, and how much to lean on postfix guards. Both wait on more real Portland to look at.

## Concurrency

Three tiers, and you will live almost entirely in the first: implicit parallelism, safe _because_ values are immutable, where `photos.map { it.thumbnail }` will spread across cores when it is worth it and you never asked. **Tier two runs today** — serially, with its semantics decided and pinned, so the eventual scheduler must match it (ADR 0029):

```ruby
together do
  meanwhile user = fetch_user(id)
  ~ orders = recent_orders(id)      # ~ and meanwhile are dead-identical
end
render(user, orders)                # plain values after end
```

The rules are the language's rules, applied: task names bind **at the `end`** and not before, so nothing can read a result that is not there yet; a task cannot see a sibling's name, because independence is the declaration; a failed task binds its `failure` to its name, handled after the join with the toolkit — siblings always run to completion; nothing unwinds across the join (`return`, `break`, `!` inside a task refuse); a task cannot rebind an outer `mutable`, which is safe-because-immutable as a rule rather than a slogan; and `together` produces nothing, its answer being the names. Plain lines interleave freely, their locals dying at `end`. Cross-task effect ordering is deliberately unpromised — that is the room the scheduler will live in.

Tier three — cancellation, timeouts, racing — is future work, rare by design.

## Decided, not yet built

- **Bitwise operators are out** (ADR 0003, tentative), with named methods instead.

## Not yet designed

Visibility, compile-time macros (#14), regex, the `%` literal zoo (#29), string semantics beyond what Rust's choices gave the seed by accident, and types themselves (#9). Inheritance is not deferred but **declined** (ADR 0028 chose traits over it), and error handling stopped being on this list on 2026-07-27 (ADR 0027).

## Gone, on purpose, forever

Monkeypatching and open classes. `method_missing`. Runtime `define_method`. `eval`. Globals and `$specials`. Truthiness. Ambient nil and `NilClass`. The GIL and the `Thread` API. `for`, `BEGIN`/`END`, flip-flops, and the `and`/`or` precedence trick. `fetch`. Force-unwrap operators and `if let`-style binding conditionals. Numbered block parameters. In-place mutators.

Runtime metaprogramming's replacement is compile-time macros, which are undesigned. Everything else on this list is simply not coming back — see [docs/ruby/removed-syntax.md](ruby/removed-syntax.md) for what each one costs a migrating codebase, and how loudly it fails.
