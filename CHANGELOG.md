# Changelog

## Unreleased

- Ledger: heredocs get their own file, `docs/ruby/heredocs.md` — the `<<`-collision reasoning and the two distinct autocorrect tiers (lowercase terminators are free, `<<`/`<<-` are unsafe) are substantive enough to read as a difference in their own right rather than two bullets in a removals list. `removed-syntax.md` keeps a one-line pointer, matching how truthiness, nil, and bitwise operators are already handled.

- Ledger audit: every ADR is now reflected in `docs/ruby/`, and the two that aren't cited are deliberate — 0018 (numbers) is a non-difference since Portland matches Ruby's floored division exactly, and 0014's kwargs match Ruby 3 exactly. The gap the audit found was **splats**, which appeared nowhere despite giving migrants a parse error today; removed-syntax.md gains a **"Deferred, not removed"** section covering splats, floats, ranges, brace blocks, `it`, and tabled symbols — because "not yet buildable" and "ruled out" should not look the same to someone porting code.

- ADR 0020: heredocs decided — **squiggly `<<~` only**; `<<EOS` and `<<-EOS` are out. Beyond one-way-to-do-it, this dissolves a collision: ADR 0015 made `<<` the append operator, and Ruby tells the two apart by asking whether the receiver is a local or a method (verified — `x << EOS` appends, `puts <<~EOS` opens a heredoc), which is exactly the guessing no-shadow exists to kill. With squiggly as the only opener, `<<` is always append and `<<~` is always a heredoc, so no disambiguation rule is needed at all. No space between `<<~` and the terminator (already a Ruby syntax error), and terminators must be **SCREAMING_CAPS** — Ruby accepts any identifier (`<<~sql`, `<<~_x`, even `<<~puts` parse), but the convention is universal and RuboCop's `Naming/HeredocDelimiterCase` already defaults to enforcing it, so the migration is free-tier. Indented terminators, `<<~'EOS'`, attached method calls, and multiple heredocs per line all work as in Ruby. The `<<`/`<<-` rewrite is an unsafe autocorrect — switching openers strips common indentation and changes the string's value. Supersedes issue #6's "all flavors" scope.

- Symbols: the core question is **decided** — `:foo` exists as a general type, checked for membership wherever a declared closed vocabulary is expected and free elsewhere; `{name: "pdx"}` is symbol-keyed and ships as table stakes. The reframe that unlocked it: Ruby's problem with `status = :pending` was never the syntax, it's that the set is *open*, so `:pendign` is a valid symbol — declaring the set fixes that without touching the spelling. Accepted tradeoffs are recorded explicitly in the [session notes](docs/reports/2026-07-23-symbols-first-pass.md), along with the lint that could later recover checking for hash keys. Capitalized cases were rejected for glance-ambiguity with structs; Swift's leading dot was rejected on technical grounds (Portland has leading-dot method chaining, so `.pending` collides with a chained call). ADR waits on the enum shape — payloads and enum-type naming are still open, and the latter makes #27's object model the keystone.

- Symbols, first pass — ~~tabled, no decision~~ superseded by the entry above ([report](docs/reports/2026-07-23-symbols-first-pass.md)). The audit found that Portland's existing ADRs have already reassigned every job symbols do except one: kwarg and struct-pattern labels are compile-time (0013/0014), metaprogramming is dropped, `&:sym` becomes `{ it.upcase }` (0017), hashes take string keys. The residue is **enum-ish values** — so symbols are entangled with the undecided enum/sum-type question, now listed on the roadmap in its own right. Both of Ruby's *reasons* for symbols are already dead in Portland: they're the interpreter's identifier table exposed (a bare 4.0.6 interns 3,599 before user code runs), and a workaround for mutable strings (ADR 0015 made strings immutable). What survives is the semantic "this is a name, not data" — so the real question is what carries that: a distinct type, Swift-style typed enum cases, or OCaml-style inferred variants. Corpus queries to settle it are listed in the report.

- ADR 0019: ranges decided — three answers. (1) Range patterns **count toward exhaustiveness**: sorted integer ranges with a beginless first, an endless last, and no gaps prove a `case` total with no `else`; gaps error, overlap stays legal (the cascading `..10 / ..100 / ..1000` idiom depends on first-match-wins), fully-shadowed arms were already errors, arm order is a lint someday. The rejected alternative would have forced `else panic "unreachable"` on total cases — corroding `grep panic` as a crash audit, since an empty `else` yields nil and infects the result type. (2) **Slices are collections, never maybes** — `words[99..]` is `[]`, not nil; the start clamps the way Ruby already clamps the end, while `array[1]` stays `T?` (ADR 0010 untouched: one element has an honest absence, a sub-collection of nothing is `[]`). (3) **A range spans a newline only where one reading exists** — slices and patterns stay paren-free (`array[1..]`, `in ..10`), while both ambiguous shapes error and name their readings: a trailing `1..` at end of line, and a line-initial `..4` after a complete expression. Ruby 4.0.6 verified throughout, and it is stranger than its reputation: slicing is asymmetric (`[4..]` nil but `[3..]` empty), a trailing `..` reaches forward across a newline, a leading `..` does not reach back — even inside parens, where `y = (1` / `..4)` binds `y` to `..4` — yet a leading `.` does. Three parser accidents, replaced by one rule. Ledger: ranges.md.

- ADR 0018 + seed: numbers decided, and the oldest crude-on-purpose call retired — integer `/` and `%` now **floor like Ruby** instead of truncating like Rust (`-7 / 2` is `-4`, `-7 % 2` is `1`; expectations verified against real Ruby, all four sign combinations pinned, plus a differential test proving the trio inherits it). Floats ride along decided but unbuilt: IEEE doubles, Ruby's printing, mixed-arithmetic promotion, `div`/`fdiv` on demand. Python's always-real `/` was rejected as a silent divergence; a never-guess error on integer `/` as too heavy for arithmetic this common. No ledger file: this is a deliberate non-difference.

- Fixed collateral damage from an earlier mechanical sweep: three comments and one test's string data read `'word` where they meant `'s` (`"it'word escaped"`). The test asserted its own corrupted value on both sides, so it passed while checking the wrong thing.

- ADR 0017: `it` decided — an ordinary binding under the no-shadow rule, not Ruby's soft keyword: `{ it.sexp }` works; contesting the name anywhere in reach is a rename-one error; nested `it` is a shadow, so it errors ("name your parameters"); no mixing with `|parameters|`; wherever `it` compiles it has exactly one meaning. Numbered parameters `_1`–`_9` are out (polyfill autocorrects `_1 → it` free). Ledger: parentheses.md + removed-syntax.md. Build lands with brace blocks.

- ADR 0016: brace blocks decided — `{ ... }` and `do ... end` mean exactly the same thing (Ruby's braces-bind-tight precedence guess dies); the one colliding position, a bare `{` after a paren-less command call, is a compile error naming each reading with its rewrite (up to three: hash argument, inner call's block, outer call's block — the parser peeks to shrink the menu, never to pick). Ledger: parentheses.md. Build pending; `it` and `&:symbol` deliberately undecided.

- Trio, `<<` sweep: the deferred taste pass — parser.pdx and evaluator.pdx retire ~45 `X += [item]` / `text += part` appends to `<<` (ADR 0015 sugar, matching lexer.pdx). Counters keep `+=`; genuine array concatenations (`captures += sub_captures`) stay `+=` because `<<` appends exactly one element.

- Trio, guest `with` (#27): the evaluator builds an updated copy of a tagged-array struct — field order kept, replaced labels take the new value, checked before any dispatch like the seed; the struct-methods differential now exercises single, chained, and original-untouched `with`. The trio's last flagged #27 gap outside the full object-model session.

- Trio, #27 taste payoff (third slice): Token grew the six kind predicates (`identifier?`, `integer?`, `keyword?`, `newline?`, `operator?`, `string?`) — the issue's own motivating example — and parser.pdx's 29 `token.kind == "..."` string comparisons became predicate calls; the index-assignment head check became type patterns.

- Trio, #27 taste payoff (second slice): the evaluator's `node.kind == "..."` string checks became type patterns — `argument in KeywordArgumentNode`, `(node in SafeMethodCallNode)`, and friends. One `kind` read remains (the cannot-evaluate error message).

- Trio, #27 taste payoff: every AST node struct renders its own S-expression via a `sexp` method — the per-node `*_sexp` helpers moved into their struct bodies (fields read bare, children recurse with `child.sexp`, `MethodCallNode`/`SafeMethodCallNode` share `method_call_sexp(self, dot)`), and the 40-branch `case node.kind` dispatcher dissolved into polymorphic `node.sexp`. Shared renderers (`headed_sexp`, `sexp_list`, `branch_sexp`, `encode_string`) stay top-level. Output byte-identical; the `kind` field survives only for the evaluator's few remaining shape checks.

- Trio, #27 increment: struct methods threaded (StructDefNode carries methods; `self` keyword; own-method bare calls via a `__self__` binding; dispatch before builtins and fields) and builtin type patterns bootstrap on the seed's own (`subject in Integer` answers the guest's `in Integer`; the struct-shape probe became a pattern too) — differentially pinned. Known gaps: guest `with`, method/field collision checks (seed is the oracle).

- Seed, #27 increment: methods in struct bodies — fields first then `def`s, dispatched on instances before field access; bare names resolve locals → fields → own methods → top-level (no-shadow enforced across layers, `new`/`with`/`nil?`/`some?` reserved); `self` is the receiver and that's its whole job; kwargs/guards/defaults all work; top-level bodies clear the receiver. Plus builtin type patterns: `in String` / `Integer` / `Array` / `Hash` / `Boolean` — the type predicate, pattern-flavored, no reflection API.

- ROADMAP: the two missing design issues opened — #27 object model (struct-methods increment first, type predicates included) and #28 error handling (exceptions vs typed results, `!` decided with it).

- Accumulator retirement: lexer.pdx and mini_lexer.pdx speak `<<` (`text << character`, `tokens << Token.new(...)`); STAGE0's variables section became the bindings section; ledger and ROADMAP record #10 complete.

- Trio: `<<` and `[]=` threaded (AppendNode/IndexUpdateNode, sexps, evaluator handlers) — and the sugar retires the pair-list era: evaluate_hash builds **host hashes** with `built[key] = value`, so guest hash indexing, `p hash`, and hash `each` all match the seed byte-identically; trio `[]=` is hash-only for now (array index assignment waits for type predicates); differential pinned.

- Seed: `<<` and `[]=` as rebinding sugar (ADR 0015 §2) — `line << word` concatenates strings and appends one array element; `hash[k] = v` / `array[i] = v` are functional updates rebound on the name (arrays replace in range or append at the end); both gate on `mutable` and cannot spook aliases (tested). Statement position only; `<<` enters the lexer append-only.

- ADR 0001 implemented (the `mutable` branch): `mutable name = ...` declares the one rebindable kind of name; bare assignment creates immutables or rebinds mutables; compound assignment gates on `mutable`; the three closure rules (accumulators licensed, outer-immutable rebinds refused with the fix named, fresh block-locals die at `end`); `mutable` parameters (`def f(mutable position)` — a parameter is a binding site); loop iterations are fresh scopes for their own locals (the block rule applied to `while`); pattern captures follow assignment rules (immutable clash errors, suggesting the pin) and failed guards roll their captures back. The whole codebase took its own medicine: seed tests, fixtures, and the trio are fully migrated; the trio parses `mutable` as syntax (the seed is the enforcement oracle — documented crude divergence).

- ADR 0015 (#10): values never mutate; names do — no in-place mutators ever (securing RC-exactness, #12); `<<` and `[]=` return as rebinding sugar in the `+=` family, `mutable`-gated (the loudness mechanism for the aliasing change); the share-boundary question dissolves; bang methods out with `!` semantics deferred (leading candidate: rebinding sugar). Ledger updated.

- Trio, case/in final sliver (#26): the one-line forms — `expr in pattern` and `expr => pattern` — threaded through parser.pdx and evaluator.pdx, differentially pinned. #26 complete; hash and find patterns stay deliberately deferred.

- Trio, case/in payoff + fixture (#26): the evaluator's dispatchers (`evaluate_statement`, `evaluate_expression`, `match_pattern` itself) are rewritten as `case/in` struct patterns — Portland matching on Portland's own AST, byte-identical throughout; nil-subject guards close the wrong-shape gap for the common case; patterns.pdx joins the fixture suite (direct and hosted); STAGE0 and the ledger record the runtime half as built.

- Trio, case/in rung 5 (#26): the pattern surface threaded through the trio — lexer.pdx (`in`, `^`), parser.pdx (pattern grammar with literals-as-nodes, captures, alternatives, pin, guards, array and keyword-only struct patterns; `field:` desugars to a capture at parse time; sexps), evaluator.pdx (guest matcher over tagged-array structs) — differentially pinned against the seed. Known crude gap noted: no type predicates yet, so wrong-shaped subjects panic instead of missing.

- Seed, case/in rung 4 (#26): the one-line forms — `expr in pattern` is a boolean test (binds captures on a hit, reads as a condition: `if node in BreakNode`), and `expr => pattern` is rightward destructuring that panics on mismatch (`pair => [a, b]` — the pattern-grammar answer to multiple assignment).

- Seed, case/in rung 3 (#26): pin `^variable` (compare, don't capture; `^` enters the lexer as pin-only — xor stays out per ADR 0003), guards (`in x if x > 10` — bind first, guard sees captures, false falls through), and array patterns (`[a, b]` exact, `[first, *rest]` / `[first, *]` with a trailing splat; suffix-after-splat waits with the find pattern).

- Seed, case/in rung 2 (#26): struct patterns, keyword-only (ADR 0013 §5) — `in ReturnNode(value: nil)` refines by field, `in Token(kind:)` binds shorthand, `in Token(kind: k)` binds named, bare `in BreakNode` matches by type; positional fields refuse with the keyword-only error; sub-patterns recurse (alternatives and literals nest).

- Seed, case/in rung 1 (#26): `in` branches on `case` — literal patterns (integers, strings, booleans, nil), captures (bind and persist, no-shadow-fenced), `|` alternatives, `then` one-liners, and the runtime exhaustiveness preview (no match + no else = a panic naming the subject). Struct patterns, pin, and guards are later rungs.

- parser.pdx: pure-refactor alphabetization — structs, builders, sexp helpers, and the sexp dispatcher's branches sort alphabetically (the precedence ladder keeps its narrative order); two stale exhibit comments updated.

- Trio: keyword arguments threaded through parser.pdx (`KeywordParameterNode`, label-aware command arguments) and evaluator.pdx (label binding with defaults), differentially pinned; ADR 0014 records the decision (Ruby 3 semantics, splats deferred); STAGE0/ROADMAP synced.

- Seed: keyword arguments on regular methods, Ruby 3 style — `label:` required, `label: default` optional, strictly separate from positionals, kwargs on paren and command calls, defaults may reference earlier parameters; missing/unknown labels are named panics. Splats stay out (deferred, per the open-decisions report).

- ADR 0013 (#20): the `case/in` spec — compile-checked exhaustiveness (`else` opts out), `===` as a statically resolved method, Ruby captures fenced by no-shadow + unreachable-arm errors + an unused lint, pin/guards/alternatives/one-line forms in (tentative), find pattern deferred, struct patterns keyword-only. Ledger: docs/ruby/pattern-matching.md; building is #26.

- AGENT.md rewritten against reality: status (seed + trio built, optionals shipped), the working method (ADRs, ledger, Ruby-match tiebreaker, never-guess, demand-driven, differential harness), decided-so-far, and pointers to ROADMAP/report. DESIGN.md marked as the historical capture it is (ADRs win on disagreement).

- ADR 0012 (#22): a branch that doesn't happen produces nil — branchless `if`, finished `while`, and broken-out calls are maybes (Ruby-match, typed); `puts` stays valueless by the could-it-ever rule. Built in seed + trio, differentially pinned; STAGE0's "where nil would have been" ledger closes.

- ROADMAP.md: Zed v0 above the line — zed-portland shipped and verified; #24's remainder is the grammar fork and registry.

- ROADMAP.md: Tooling section (next up) — Zed extension (#24, escalated: .pdx is un-highlighted today) and benchmarks (#25).

- ROADMAP.md: the one-page burn-down — done 🎉 above the line, dropped ❌, coming ⬜ below; linked from the README.

- seed/tests/fixtures/optionals.pdx: the absence story as a living-documentation fixture, output pinned in the end-to-end suite.

- todos/ deleted — the migration table served its purpose; GitHub issues are the backlog (original files in git history).

- docs/reports/: 2026-07-22 open-decisions report — every known pending decision with options, tradeoffs, unlocks, recommended order and answers; records the Ruby-match-preferred tiebreaker.

- Trio, optionals rung 6 (#21): Exhibit B resolved — an absent `else` (and `case` else, and the empty half of a postfix-guard desugar) is nil, distinct from a present empty one; absent branches produce no value in the evaluator.

- Trio, optionals rung 5 (#21): Exhibit C resolved — a method call's block is nil or a BlockNode, no longer a zero-or-one array; the evaluator asks `node.block.some?`.

- Trio, optionals rung 4 (#21): Exhibit A resolved — `ReturnBareNode` is gone; bare `return` is a `ReturnNode` whose value is nil, exactly the shape the Rust seed always had. Seed fix along the way: the maybe predicates fall through struct field access like `to_s` does.

- Trio, optionals rung 3 (#21): evaluator.pdx speaks optionals, differentially tested byte-identical to the seed — nil literal, `nil?`/`some?`, typed or with or-guard divergers riding the signal machinery, `&.`, `some`/`panic` builtins, min/max. The slot pattern grew raw extractors (`slot_value`/`value_at` via `each`) because host lookups now lift stored nils — the wrapper touching its own implementation.

- Trio, optionals rung 2 (#21): parser.pdx parses the optionals surface — `nil` literal, `or`/`and`/`not` normalized to their sigils in the tree (dead-identical by construction), or-guard right sides (`return`/`break`/`next`/command-form `panic`), and `&.` as `SafeMethodCallNode`; still parses the whole compiler including itself.

- Trio, optionals rung 1 (#21): lexer.pdx learns the optionals surface — `nil`/`and`/`or`/`not` keywords and the `&.` operator.

- docs: STAGE0.md records the optionals runtime half as built; ledger statuses synced.

- Seed, optionals rung 6: `some(x)` and the nested case (ADR 0005/0009) — identity on plain values, a real box only around nil/Some; lookups lift found values with it, so `[nil].first` ≠ `[].first` and a stored hash nil beats the or-guard default (fetch semantics, ADR 0010).

- Seed, optionals rung 5: safe navigation `&.` (ADR 0008) — an absent receiver short-circuits to nil without evaluating arguments; chains compose with the or-guard (`hash["key"]&.upcase or "ROSE"`).

- Seed, optionals rung 4: the panic frontier flips (ADR 0010) — `[].first`/`last`/`min`/`max`, out-of-range array and string indexing, and missing hash keys return nil instead of panicking; lookups compose with the or-guard (`config["theme"] or "teal"`).

- Seed, optionals rung 3c: the or-guard divergers (ADR 0008) — `x = f() or return [value]` / `or break` / `or next` skip the binding and unwind, and `panic "why"` arrives as the language's only crash spelling (ADR 0010), including the paren-less command form in or-guard position.

- Seed, optionals rung 3b: `or` / `and` / `not` word forms, dead-identical to `||` / `&&` / `!` (ADR 0007) — same precedence (`x = nil or 7` binds the or first, unlike Ruby), same typed semantics.

- Seed, optionals rung 3a: typed `||` (ADR 0007) — nil left unwraps-or-else (short-circuit, lazy right), booleans stay logical, `&&` refuses nil; the static halves (dead right sides, `Boolean?` never-guess) stay flagged as out of the seed's reach.

- Seed, optionals rung 2: `nil?` and `some?` on every value (ADR 0009) — the one dispatch that works on nil; every other method on nil panics with "handle the nil case first".

- Seed, optionals rung 1: the `nil` literal (ADR 0006) — lexes as a keyword, `Value::Nil`, equality across types, `p nil` renders `nil`, and `puts nil` panics as the crude preview of the future compile error.

- docs/adr/ 0011: `together` has one register — named-at-site only, no positional form; destructuring assignment stays unentangled. Closes the last open item on #3.

- docs/adr/ 0010 + docs/ruby/lookups.md: partial operations return maybes — the only crash is one you typed (`or panic "why"`); `fetch` retires (all three arities are the or-guard, lazy for free).

- docs/adr/ 0009: the presence word is `some`/`some?` — ecosystem-verified unclaimed (dry-monads precedent, with our exact meaning); ledger synced.

- docs/adr/ 0008: the unwrap toolkit — flow narrowing, the or-guard, `&.` (flattening), `case/in`; deliberately no `if let` and no force-unwrap operator (`or panic "why"` is the hatch).

- docs/ruby/: the Ruby → Portland difference ledger — one file per difference (nil/optionals, truthiness, word operators, mutability, metaprogramming, types, concurrency, parentheses, bitwise, removed syntax), README with the two migration promises (loud-never-silent divergence, the polyfill test).

- docs/adr/ 0005–0007: optionals are a wrapper with a collapsed-feeling surface; the absence word is `nil` (partner word open); `or`/`and`/`not` dead-identical with their sigils, `or` typed with never-guess on `Boolean?`.

- docs/adr/: architecture decision records, one per file — 0001 `mutable`, 0002 the `~` task sigil, 0003 bitwise-out (tentative, `<<` append TBD).

- evaluator.pdx rung 7 + stretch (#19): structs (tagged-array instances, kwargs construction, field access) — and mini_lexer.pdx now runs on the Portland evaluator byte-identical to the seed. Every rung of #19 is complete.

- evaluator.pdx rung 6 + SUMMIT (#19): collections and blocks (guest values are host values, so host dispatch does the work) — and the whole fixture suite (hello, arithmetic, fizzbuzz, showcase, blocks, tour) now runs on the Portland evaluator byte-identical to the seed, CI-pinned. Structs (rung 7) remain for the mini_lexer stretch.

- evaluator.pdx rung 5 (#19): methods — one namespace for locals and methods (the no-shadow rule makes it sound), recursion, defaults, fresh method-only scopes, return unwinding.

- evaluator.pdx rung 4 (#19): control flow — if/unless/while/case, guards, and break/next/return as threaded signals.

- evaluator.pdx rung 3 (#19): string interpolation moved to parse time in parser.pdx (desugared to + chains with .to_s, like the seed); string nodes now hold decoded values, sexps re-encode them.

- evaluator.pdx rung 2 (#19): variables — assignment and lookup over prepend-to-shadow assoc bindings.

- evaluator.pdx rung 1 (#19): operators — host operators do the work since guest values are host values; differential harness extracted.

- evaluator.pdx rung 0 (#19): Portland running Portland — Outcome-threaded evaluation of literals and puts/p, run.pdx driver, differential-tested byte-identical against the seed.

- parser.pdx rungs 6 + summit (#18): command calls with never-guess errors, array/hash/%w literals — and the flag: parser.pdx parses lexer.pdx, tokenize.pdx, parse.pdx, and itself with zero error nodes, CI-pinned.
- Seed: Value::Array/Hash payloads are now Rc-shared — immutability makes sharing invisible, and the self-parse dropped from 7 minutes to under 8 seconds. lexer.pdx tokens carry leading_space for the parser's ambiguity rules.

- parser.pdx rung 5 (#18): definitions — def with defaults, struct, do-blocks with parameters, keyword arguments.

- parser.pdx rung 4 (#18): control flow — if/elsif/else, unless (desugared), while, case/when with then one-liners, and postfix guards.

- parser.pdx rung 3 (#18): statements — assignment, compound assignment (desugared), return/break/next, and error recovery that skips to the next line instead of cascading.

- parser.pdx rung 2 (#18): postfix — dot calls, indexing, paren call arguments, leading-dot chains across newlines, and negative literals binding Ruby-style (-5.abs).

- parser.pdx rung 1 (#18): the full expression precedence ladder — logicals, comparisons, arithmetic, unary, parens — with identifiers, strings, and booleans as operands.

- parser.pdx rung 0 (#18): Step-threaded recursive descent skeleton, sexp printer, parse.pdx driver — integers parse, unknown tokens become error nodes.

- `require_relative`: multi-file Portland programs, Ruby-style (resolved against the requiring file, `.pdx` implied, loads once). `compiler/lexer.pdx` is now a library; `compiler/tokenize.pdx` is the command-line driver.
- Paren-less calls, the Portland way: command calls at statement position (`puts "hello"`) and bare zero-argument calls (`ready?`) — powered by two new rules instead of Ruby's whitespace guessing: shadowing a method with a local is an error, and ambiguous forms (`puts -1`, `puts [1]`, `puts (1)`) are clean parse errors that show both readings. The lexer now records leading whitespace to detect them.
- `return`/`break`/`next` now work inside blocks with Ruby semantics: `next` skips the iteration, `break` stops it (the call produces no value), `return` unwinds through the block to the enclosing method — guard-search idioms like `return number if number.even?` inside `each` work.
- Single-character variable names renamed away everywhere (`character` not `c`, `index` not `i`), per style.
- MIT license (`LICENSE.md`); the crate's dual-license placeholder becomes plain MIT on its next publish.
- `compiler/lexer.pdx`: Portland's lexer, written in Portland — tokenizes the full token set (strings with escapes and interpolation, `%w[]`, two-character operators, `?`/`!` names) and lexes its own source with zero error tokens. Step one of Stage 1.
- Recursion depths measured (and the macOS-26 hang-on-overflow discovered): the seed now runs on a 512 MB-stack thread with explicit depth guards that fail as clean Portland errors instead of hanging.
- Structs: `struct Name ... end` immutable records with kwargs-only construction (`Token.new(kind: ...)`), field access, `.with(...)` updated copies, and value equality. First user: `mini_lexer.pdx`, now hash-free.

- Design docs, todos, and namespace squats (GitHub orgs `portlandlang` + `pdxlang`, crates.io `portland` v0.0.0).
- Todos migrated to GitHub issues #1–#17; `todos/` now holds only the mapping.
- Cargo workspace: `crate/` (the published placeholder, eventually the real compiler) + `seed/` (Stage 0, never published), with `script/test` (fmt + clippy + tests).
- Seed lexer: integer literals, identifiers with `?`/`!` suffixes, double-quoted strings (no escapes/interpolation yet), newline tokens, space/tab skipping.
- Seed lexer: `def`/`do`/`end` keywords (lookalikes like `def?`/`ending` stay identifiers) and single-character punctuation (`(` `)` `,` `.` `=` `+`).
- Seed AST + recursive descent parser: integer and string literals, left-associative `+`, parenthesized grouping. `1 + 2` now means something.
- Seed parser, statement level: newline-separated programs, variable references, assignment, method calls with parenthesized arguments, and `def ... end` with parameters and body.
- Seed interpreter (tree-walking reference semantics): literals, arithmetic (`+ - * / %`, unary minus), string concatenation, comparisons, strict-boolean `if`/`elsif`/`else` expressions, `while` loops, assignment, and user-defined methods with fresh scopes.
- `puts` builtin with pluggable output; builtins produce no value (a seed-level preview of "no ambient nil").
- `pdx` binary: runs `.pdx` files (fixture-tested end to end, fizzbuzz included) and opens a REPL when run bare — multi-line definitions buffer, errors report and continue.
- Comments (`#` to end of line).
- String escape sequences (`\n` `\t` `\"` `\\`), decoded in the parser.
- Dot method calls, chainable, with read-only builtin value methods: `length`, `upcase`, `downcase`, `reverse`, `empty?` on strings; `abs`, `zero?`, `positive?`, `negative?` on integers; `to_s` on everything. `-5` is a negative literal, so `-5.abs == 5`.
- Arrays: literals, indexing (negative indices; out of range panics — no nil), `+` concatenation, `length`/`first`/`last`/`empty?`/`join`.
- Blocks: `do |x| ... end` on `each`, `map`, and `times`. Blocks are closures over the enclosing scope; parameters are block-local.
- `return` (exits the enclosing method, unwinding through loops) and `break` (exits the enclosing `while`); misuse panics, unsupported-in-blocks stated honestly.
- `docs/STAGE0.md`: the seed subset documented as built, including what's deliberately out.
- Hashes: `{"key" => value}` literals (insertion-ordered, duplicate keys last-wins), lookup by any value (missing key panics — no nil), `length`/`empty?`/`key?`/`keys`/`values`.
- Stdlib breadth: string `chars`/`split`/`include?`/`start_with?`/`end_with?` and `[index]`; integer `even?`/`odd?`; array `include?`/`sum`/`min`/`max`.
- `unless` (block and postfix) and postfix `if` — guard clauses (`return 0 if n < 0`) work.
- Short-circuiting `&&`/`||` and `!`, strict booleans.
- String interpolation `"#{...}"`, desugared to concatenation with auto-`to_s`; lexer keeps token boundaries honest through nested strings and braces.
- Compound assignment (`+= -= *= /= %=`), `next` in `while` loops, `hash.each do |key, value|`.
- `case/when` with equality matching, multiple values per `when`, and aligned `when x then y` one-liners.
- `tour.pdx` fixture: the full Stage 0 surface through the real binary.
- `p` builtin and `inspect` rendering (strings keep quotes; the REPL uses it, like irb).
- Single-quoted literal strings, `%w[]` word arrays, `*` repetition for strings and arrays.
- Default parameter values (trailing only, bound left to right so defaults can reference earlier parameters).
- `select`/`reject`/`reduce(initial)` blocks and `String#to_i`.
- Crude IO builtins (`argv()`, `read_file`, `write_file`) — names are placeholders; unblocks real programs. `word_count.pdx` fixture is a working `wc`.
- `each_with_index`, `sort` (integer arrays), `slice(start, length)` on strings and arrays; bare `puts()` prints a blank line.
- `mini_lexer.pdx` fixture: a lexer written in Portland, tokenizing Portland-ish source — first compiler work in the language itself.
- `each_with_index`, `upto`/`downto`, leading-dot method chains across newlines, duplicate-parameter rejection, REPL buffers multi-line strings.
- `script/bootstrap` and `script/cibuild`; `todos/018` gap analysis toward Stage 1 self-hosting.
