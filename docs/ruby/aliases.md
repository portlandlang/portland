# Aliases

**Summary:** The famous twins ship with Ruby's meanings; the long tail of Ruby's remaining aliases refuses by naming the surviving spelling.

**Status:** decided ([ADR 0036](../adr/0036-2026-08-18-the-famous-twins-ship.md)) — the six families the ruby/spec import met are built in both oracles; the closed edge is the standing policy for everything beyond them.

## Ruby

Method aliases everywhere, made with `alias`/`alias_method` machinery: `size`/`length`, `map`/`collect`, `reduce`/`inject`, `find`/`detect`, `key?`/`has_key?`/`include?`/`member?`, and a long tail (`entries`, `to_ary`, `detect`'s own kin). Which name a codebase uses is house style; both always work.

## Portland

Synonyms are welcome — one behavior may have many spellings; one spelling never has two behaviors (principle 3). What Portland declines is the _runtime machinery_, not the twins: there is no `alias` keyword today and no runtime aliasing ever, so each shipped twin is a plain builtin sharing its survivor's implementation, drift impossible by construction. (Whether a *static* user-facing `alias` someday earns a keyword is [#81](https://github.com/portlandlang/portland/issues/81).)

The line is categorical (ADR 0036, revised the day it landed): **every pure synonym whose survivor exists ships as a twin.** Shipped: `size` everywhere `length` answers, `collect`, `inject` (with `reduce`'s explicit initial), `detect`, `filter` and `find_all` beside `select`, `collect_concat` beside `flat_map`, `entries` beside `to_a`, `each_pair` beside Hash's `each`, `magnitude` beside `abs`, `member?` on arrays, and Hash's whole membership family — `has_key?`, `include?`, `member?` all asking about keys, as in Ruby. `succ`/`pred`/`next` shipped earlier on the same philosophy. A pure synonym missed by the enumeration refuses plainly and ships on notice.

What refuses **by naming the real spelling** is the false-alias set: Ruby's implicit-conversion protocol (`to_ary`, `to_int`, `to_str`) looks like conversion spellings but carries duck-typing semantics Portland does not have, so `to_int is spelled to_i here` points the migrating author at what they actually want. Aliases of unbuilt survivors refuse as undefined until the survivor exists; the mutator family's refusals belong to [mutability](mutability.md), not here.

## Migration

- The six families migrate verbatim, same meanings — _free_, and they are the aliases half of Ruby types on reflex.
- `inject(:+)` and seedless `inject` are not the alias question: symbol-to-block rides the proc question ([#77](https://github.com/portlandlang/portland/issues/77)).
- An alias outside the shipped set fails loudly with the surviving spelling in the message; the linter can autocorrect these in valid Ruby before the flip — _gem-supplied_.
- `alias_method` is gone with the rest of the metaprogramming surface ([metaprogramming.md](metaprogramming.md)) — computed names at runtime. `alias` the keyword is a different animal: its Ruby hazards were all redefinition-shaped (it *snapshots* the method at that point in class-body execution, which is what powered alias-method-chain monkeypatching), and redefinition does not exist here — so a static `alias` would be innocent compile-time machinery, one body with two names. It is unbuilt because nothing has pulled for it and `def fu = foo` already says it; whether the sugar earns a keyword is [#81](https://github.com/portlandlang/portland/issues/81).
