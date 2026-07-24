# Portland

A joyous programming language for Apple silicon.

_Close to the metal, on Metal._

**Status:** Stage 0 done, Stage 1 begun. A Rust seed interprets a real slice of the language; Portland's own lexer, parser, and evaluator are written in Portland, and the parser parses itself. There is no compiler yet — see [ROADMAP](ROADMAP.md) for the honest burn-down.

## The question

> What could a programming language be if it ran _only_ on Apple silicon — and wasn't Swift?

Every mainstream language is conservative because it has to run on hardware it has never seen. Drop portability and the constraints turn into features: unified memory means there is no host/device distinction to model, so the same `.map` line can be one core or a GPU dispatch. Swift specifically _can't_ take these bets — it targets Linux, Windows, and embedded, and it's ABI-frozen.

Locking to one vendor's hardware is the feature.

## The soul is Ruby's

Programmer happiness first. Job one is the joy of reading and writing the code — and safety and performance are not traded against that, they are built so you never feel them. The bar every feature has to clear: **the beautiful line must also be the safe, fast line.** If going fast forces a different, uglier line, the feature is wrong.

Most of Ruby's felt joy is its _surface_ — blocks as prose, everything-is-an-expression, implicit returns, no ceremony — and that part survives static compilation untouched. Most of Ruby's pain lives in its _runtime_. So: keep how it reads, replace what it does underneath.

This runs today:

```ruby
def greeting(name)
  return "hello, stranger" if name.empty?
  "hello, #{name}!"
end

puts greeting("portland")

cities = {"pdx" => "portland", "sea" => "seattle"}
cities.each do |code, city|
  puts("#{code.upcase} is #{city}") unless city == "seattle"
end
```

## And here's what's different

```ruby
settings = {"theme" => "teal"}
theme = settings["mode"] or "dark"    # a missing key is a maybe, not a landmine
puts theme                            # => dark

mutable total = 0                     # bare bindings can't be rebound at all
[1, 2, 3].each { total += it }

case total
in 0    then puts "nothing"
in 1..9 then puts "#{total} — single digit"
else
  puts "big"
end
```

- **No ambient nil.** Absence is an explicit, typed maybe — never a value's secret. `[].first` hands you something you must handle, and the only crash in a Portland program is one you typed: `or panic "why"`.
- **Immutable by default, and values never mutate.** `mutable` marks the rare exception, and it governs _names_, not values — so two names can never spook each other through a shared buffer.
- **No truthiness.** Conditions take booleans. There is no nil to be falsy, so there is nothing to be clever about.
- **Never guess.** Where one spelling has two honest readings, Portland refuses and shows you both with their rewrites, rather than resolving it with a whitespace rule nobody can recite.
- **Concurrency you don't manage.** Declare independence and the runtime spreads work across P and E cores, the GPU, and the matrix unit — safe _because_ values are immutable. (Designed; not yet built.)
- **Self-hosted early.** A tiny Rust seed, deleted on purpose once Portland can compile itself.

## Where to go from here

| | |
|---|---|
| [**The language**](docs/language.md) | what it is — syntax, rules, style |
| [**Architecture**](docs/architecture.md) | how it's built: the seed, the trio, the road to a real compiler |
| [**Principles**](docs/principles.md) | the rules that settle arguments |
| [**Portland for Rubyists**](docs/ruby/) | every difference from Ruby, and what it costs to migrate |
| [**ADRs**](docs/adr/) | the decision log, one file per decision |
| [**ROADMAP**](ROADMAP.md) | what's done, what's dropped, what's coming |
| [**CHANGELOG**](CHANGELOG.md) | what changed, newest first |
| [**History**](docs/history/) | dated writing, frozen — never current, never a source of truth |

Try it with `script/console a_file.pdx`, or `script/console` alone for a REPL. Portland targets **macOS 26+ on Apple silicon** and nothing else, on purpose.

## The name

**Portland**, extension **`.pdx`** — the keep-it-weird, craft-over-scale ethos, and a faint Rose City → Ruby lineage echo. `.pdx` is an airport code and a quiet signature on every file.

Companions: [ruby_research](https://github.com/portlandlang/ruby_research) (corpus evidence over rubygems.org, which is how features earn their way in) and [zed-portland](https://github.com/portlandlang/zed-portland) (editor support).
