# 0014 — Keyword arguments are Ruby 3's; splats stay out

- **Status:** Accepted (splat deferral revisitable on demand)
- **Date:** 2026-07-22

## Decision

Regular methods take keyword parameters exactly as Ruby 3 does:

```ruby
def greet(name:, greeting: "hi")   # label: required, label: default optional
  "#{greeting} #{name}"
end

greet(name: "pdx")                 # paren and command calls both take labels
greet greeting: "yo", name: "pdx"
```

- Keywords are strictly separate from positionals (Ruby 3's rule — no
  hash-to-kwargs autoconversion, which was Ruby 2's famous migration
  wound; Portland starts on the right side of it).
- Keyword parameters follow positionals in a definition; labels follow
  positionals at a call site. Defaults may reference earlier parameters.
- Missing required labels and unknown labels are named errors.

**Splats (`*args`, `**kwargs`) stay out**, deferred rather than rejected:
they fight arity clarity and inference, and nothing in the compiler-shaped
corpus has pulled for them. When something does, they get their own
decision.

## Consequences

- Built same-day in the seed and the trio (differentially pinned);
  `new`/`with` were already keyword-only, so the language now has one
  argument story.
- Migration: Ruby 3 kwarg code compiles verbatim; splat uses get a clean
  parse error (loud), and the linter can flag them pre-flip.
