# Redefinition

**Summary:** A top-level name is defined once — a second `def`, `struct`, `enum`, `trait`, `alias`, or constant of the same name refuses, and a builtin's name is never taken.

**Status:** decided ([ADR 0052](../adr/0052-2026-09-24-one-definition-per-name.md), 2026-09-24), built in both oracles the same day; the REPL is the one place redefining is allowed.

## Ruby

Everything is open. A second `def foo` replaces the first without a word (a warning only under `-w`); a class or module reopens anywhere; core classes can be patched from any file; and load order decides which definition wins. Monkeypatching, alias-method-chain, and `prepend` all lean on this.

## Portland

One definition per name, across the whole program — the entry file and everything it requires:

```ruby
def apply = 1
def apply = 2        # 'apply' is already defined — rename one

struct Token
  kind
end
struct Token         # 'Token' is already defined — rename one
  kind
  text
end

def panic(reason)    # 'panic' is a builtin — rename yours
  puts reason
end
```

The checker says it at build with the line beneath; the seed and the hosted runtime say it when the second definition runs. A module may be reopened to add *new* names — `module A` twice composes, as ADR 0021's nesting forms already do — but `A::x` defined twice refuses like any name.

What this protects: [ADR 0010](../adr/0010-2026-07-22-partial-operations-return-maybes.md)'s "the only crash is one you typed" is enforceable, since no program can define its way out of `panic`; and the audit report (#30) reads true.

## Migration

- **Monkeypatching and reopening core classes** were already gone with runtime metaprogramming ([metaprogramming.md](metaprogramming.md)); this closes the accidental door the seed had left open.
- **Alias-method-chain** (`alias old_foo foo; def foo`) refuses at the redefinition; the replacement is a new name.
- **A duplicated def from a paste** refuses where Ruby would silently take the second — the case the corpus actually contains.
- The REPL redefines freely, since a session is not a program.
