# Regex fuzz batches

Generated cases for the seed's regex engine, each with the answer Ruby 4.0.9 gave and the matcher steps the seed took when it was recorded. `seed/tests/onigmo_fuzz.rs` replays them on every `script/test`, without Ruby: a case fails when its answer differs from Ruby's or its steps grow more than a quarter past the record, and the run fails when the total grows more than a twentieth.

| File | Shape |
|---|---|
| `general-1.tsv`, `general-2.tsv` | Random patterns across the whole of Onigmo's Ruby syntax |
| `cache-7.tsv` | Ambiguous alternations, repeats, and lookarounds over longer texts, where the match cache turns on |
| `grapheme-11.tsv` | `\X` and `\R` over emoji, flags, Hangul, Indic conjuncts, and combining marks |
| `start-13.tsv` | Searches from a position, with `\G`, `\A`, `^`, `\b`, lookbehind, and `\K` |
| `fold-17.tsv`, `fold-23.tsv` | Case-insensitive patterns over the characters whose folds change length or cross ASCII |
| `bench.tsv` | Benchmark patterns over `bench_text.txt`, a generated 128 KB English-like text |

The number in each name is the generator's seed. `script/regex_fuzz` makes a batch, records the steps, and times the benchmarks beside Ruby's; its header gives the commands. A case that runs past 250,000 steps while being recorded is left out, 20,000,000 for the benchmarks, which keeps the replay to about a second. The steps count the matcher's work only: a benchmark whose literal is absent takes no steps, since the search scans the text before any attempt runs, and its scanning is not guarded.

The format is in the header of `seed/tests/onigmo_fuzz.rs`: one case per line, tab-separated, `id start steps pattern text answer`, with backslash escapes for tabs, newlines, and invisible characters, so a diff reads as the patterns themselves.
