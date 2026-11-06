# Visibility

**Summary:** `private`, `protected`, and `public` mean what Ruby's mean, and a call that breaks them refuses at build time instead of raising at the call; `module_function` refuses naming `def self.`.

**Status:** decided ([ADR 0076](../adr/0076-2026-11-03-visibility-checked-at-build-time.md), 2026-11-03, out of [#133](https://github.com/portlandlang/portland/issues/133)); **built** on both implementations 2026-11-06 under [#189](https://github.com/portlandlang/portland/issues/189). The checker refuses where the receiver's type is known — a local bound to a `Box`, a `Box.new(...)` — which is where a static check can reach; a receiver whose type it cannot see passes, as any other method check does.

## Ruby

`private` makes the methods after it callable only without an explicit receiver (or with `self.` since 2.7); `protected` also allows a receiver of the same class from inside the class; `public` ends a section; `private def name` marks one method; `private_constant :NAME` hides a constant; `module_function` makes a module's methods callable on the module. Every check is at the call, at runtime: `box.secret` raises `NoMethodError: private method 'secret' called for an instance of Box`.

## Portland

The same words, the same sections, the same rule about receivers:

```ruby
struct Box
  contents

  def open = reveal

  private

  def reveal = contents
end

Box.new(contents: 1).open      # 1
Box.new(contents: 1).reveal    # refuses to build: 'reveal' is private to Box
```

- **`private`** — methods after it, to the end of the body or the next visibility word, are callable only without a receiver or through `self.`: from the struct's own methods and from the methods of a trait it includes. `private def name` marks one method.
- **`protected`** — `private` plus a receiver of the same type from inside the type's methods, which is what `def ==(other)` and `def <=>(other)` want.
- **`public`** — ends a section; everything before any visibility word is public.
- **`private_constant :NAME`** — a module's or struct's constant unreadable from outside its namespace.
- **Type functions (`def self.`) are public.** `private_class_method` waits for a pull.
- **`module_function` refuses**, naming `def self.`: a module's `def` is already a module function ([namespaces.md](namespaces.md)). So does a bare `private` in a module body, which has no instance methods to mark — a mixin is a trait, where `private` marks its methods — and `private :name`, which marks a method after its def: write `private` above the def, or `private def name`.

The difference is *when*. Which methods a section marks and who the receiver is are static facts, so the refusal is the build's, in [the error voice](../adr/0047-2026-09-22-the-error-voice.md), and no program ever reaches a `NoMethodError` for visibility. The seed, which has no checker, accepts every call; the compiler refuses ([static-checks.md](static-checks.md)).

## Migration

- `private`, `protected`, `public`, `private def`, and `private_constant` port verbatim; the one change is that a violation stops the build instead of a test run. _Free._
- `module_function` becomes `def self.` on each method. _Mechanical._
- `private_class_method` has no spelling yet; the method stays public until a file pulls for it.
