module

public import BijouAeneas.Generated.Funs

@[expose] public section

/-!
# BijouAeneas: the real Rust refines the SPEC model

`BijouAeneas.Generated.*` is the pure Lean model of `bijoux` produced by
Charon + Aeneas (`nix run .#aeneas-translate`); it is regenerated, not
edited. The refinement theorems relating it to `Bijou.Family` (the SPEC
model in ../lean) live alongside it.
-/
