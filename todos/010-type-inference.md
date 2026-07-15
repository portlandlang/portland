# Type inference design

Hindley-Milner-style: types present (safety) but invisible (joy). Annotations only at public boundaries, as docs.

## Tasks

- [ ] HM core: how far does vanilla HM get us; where does Ruby-shaped syntax strain it?
- [ ] Structural typing — "if it quacks," checked at compile time; spec the protocol/shape mechanism
- [ ] Optionals in the type system (`User?` as sugar for what?)
- [ ] Boundary annotation syntax — readable-as-docs, never ceremony
- [ ] Error messages when inference fails — this is where inference-heavy languages get un-joyful; budget real design time
