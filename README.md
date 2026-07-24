# Portland

A joyous programming language for Apple silicon.

Close to the metal, on Metal.

> **Status: Stage 0 seed, Stage 1 begun.** A tiny Rust interpreter (`seed/`) runs a
> real slice of Portland — see [`docs/STAGE0.md`](docs/STAGE0.md) for exactly what.
> The first self-hosted piece exists: [`compiler/lexer.pdx`](compiler/lexer.pdx) is
> Portland's lexer, written in Portland, and it tokenizes its own source.
> See [`ROADMAP.md`](ROADMAP.md) for where this is going and how close it is,
> [`AGENT.md`](AGENT.md) for the working brief,
> [`docs/DESIGN.md`](docs/DESIGN.md) for the full design rationale,
> and the [issues](https://github.com/portlandlang/portland/issues) for what's next.

## A taste

This runs today, via `script/console the_file.pdx` (or line by line in the REPL,
`script/console` with no arguments):

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

squares = [1, 2, 3].map do |number|
  number * number
end
puts(squares.join(" + ") + " = #{squares.sum}")
```

## The idea

What could a language be if it ran _only_ on Apple silicon (A-series / M-series) — and wasn't Swift? Locking to one vendor's hardware is the feature: it lets Portland make assumptions general, portable languages can't.

The soul is Ruby's: programmer happiness first. Safety and performance aren't traded _against_ joy — they're built so you never feel them. The bar every feature must pass: the beautiful line should also be the safe, fast line.

"Ruby, the good parts"

- **Ruby's ergonomic surface, kept** — blocks, expressions, pattern matching, no ceremony.
- **No ambient nil** — absence is an explicit optional, never a value's secret.
- **Immutable when shared, mutable when local** — which is also what makes parallelism safe.
- **Concurrency you don't manage** — declare independence; the runtime spreads work across P/E cores, the GPU, and the matrix unit over unified memory.
- **Self-hosted early** — a tiny Rust seed, then Portland written in Portland.

## File extension

File extension is `.pdx`.
