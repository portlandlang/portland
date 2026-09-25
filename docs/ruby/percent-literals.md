# The `%` literal family

**Summary:** `%w[]` and `%i[]` survive, with `[]`, `()`, or `{}`; `%q`, `%Q`, `%()`, `%s`, `%W`, `%I`, and `%x` refuse by name; `%r` waits on the regex decision.

**Status:** decided member by member ([ADR 0051](../adr/0051-2026-09-24-the-percent-literal-family.md), 2026-09-24), built in both oracles the same day; `%w[]`'s content rules are [ADR 0030](../adr/0030-2026-07-27-word-array-contents.md)'s.

## Ruby

Ten literals share the `%` prefix: `%w`/`%W` word arrays, `%i`/`%I` symbol arrays, `%q`/`%Q`/bare `%` strings, `%s` a symbol, `%r` a regex, `%x` a shell command — the capital letter meaning "interpolates" in each pair. Any non-alphanumeric delimiter is legal, bracket pairs balance, and a backslash escapes the delimiter in use.

What RubyGems.org actually writes, from every gem on it (195,390 gems, July 2026 snapshot):

| Member | Gems | % of all gems | Share of `%`-using gems, pre-2015 | 2020+ |
|---|---:|---:|---:|---:|
| `%w` | 42066 | 21.5% | 69.8% | 66.6% |
| `%` | 12714 | 6.5% | 25.2% | 17.4% |
| `%r` | 15994 | 8.2% | 25.9% | 29.6% |
| `%i` | 10991 | 5.6% | 0.7% | 38.8% |
| `%Q` | 6487 | 3.3% | 16.3% | 5.0% |
| `%q` | 2750 | 1.4% | 6.5% | 2.5% |
| `%W` | 2626 | 1.3% | 4.7% | 3.6% |
| `%x` | 2257 | 1.2% | 4.9% | 2.0% |
| `%I` | 124 | 0.1% | 0.0% | 0.4% |
| `%s` | 44 | 0.0% | 0.1% | 0.1% |

Delimiters: `()` 41.5% of sites, `[]` 29.0%, `{}` 23.4%, everything else under 2.5% combined. `%w` splits `[]` 46% / `()` 43%; `%i` is `[]` 90%; `%r` is `{}` 61%.

## Portland

**In:** `%w[]` and `%i[]`, each opening with `[`, `(`, or `{`. The pair the literal opened with is the one that balances and the one a backslash escapes; every other character is a word's own.

```ruby
%w[rose city]         # ["rose", "city"]
%w(a (b) c \) d)      # ["a", "(b)", "c", ")", "d"]
%i[paid pending]      # [:paid, :pending]
%i{odd\ name x}       # [:"odd name", :x]
```

**Out**, each with its rewrite in the refusal:

| Ruby | refusal says | write instead |
|---|---|---|
| `%q(it's)` | `'%q(' is not a Portland literal — write a quoted string or a heredoc` | `"it's"` |
| `%Q(say "hi")`, `%(say "hi")` | same | `"say \"hi\""`, or `<<~TEXT` for a body full of quotes |
| `%s(name)` | `'%s(' is not a Portland literal — write :name, or :"odd name" for a name with spaces` | `:name` |
| `%W[#{env}-web db]` | `'%W[' is not a Portland literal — write %w[] when no word interpolates, or ["#{a}", "b"] when one does` | `["#{env}-web", "db"]` |
| `%I[#{env}_admin]` | `'%I[' is not a Portland literal — write %i[]; a symbol does not interpolate` | symbols do not interpolate ([symbols.md](symbols.md)) |
| `%x(ls)` | `'%x(' is not a Portland literal — there is no shell execution` | nothing yet — the OS surface is [#78](https://github.com/portlandlang/portland/issues/78) |
| `%w|a b|` | `'%w|' is not a Portland delimiter — write %w[], %w(), or %w{}` | `%w[a b]` |

**Deferred:** `%r{...}` refuses with `'%r{' is not a Portland literal — there is no regex yet` until [#74](https://github.com/portlandlang/portland/issues/74) decides regex.

A bare `%` is read as the string form only where Ruby reads it so — after a space and glued to its opener, `a %(b)` — so `a % (b)` and `a%(b)` are modulo, as they always were.

## Migration

Every rewrite above is mechanical; the linter ([#36](https://github.com/portlandlang/portland/issues/36)) can apply each from the refusal's own sentence. The one that costs a human moment is a `%Q` or `%()` body dense with double quotes, where the honest rewrite is a heredoc rather than a backslash forest.
