module

public import BijouAeneas.Generated.Funs
public import BijouAeneas.Refinement.Bytes
public import BijouAeneas.Refinement.CoreAlloc
public import BijouAeneas.Refinement.FunsExternalChecks
public import BijouAeneas.Refinement.I128
public import BijouAeneas.Refinement.I128Corollaries
public import BijouAeneas.Refinement.I32
public import BijouAeneas.Refinement.I32Corollaries
public import BijouAeneas.Refinement.I64
public import BijouAeneas.Refinement.I64Corollaries
public import BijouAeneas.Refinement.U128
public import BijouAeneas.Refinement.U128Corollaries
public import BijouAeneas.Refinement.U32
public import BijouAeneas.Refinement.U32Corollaries
public import BijouAeneas.Refinement.U64
public import BijouAeneas.Refinement.U64Corollaries

@[expose] public section

/-!
# BijouAeneas: the real Rust refines the SPEC model

`BijouAeneas.Generated.*` is the pure Lean model of `bijoux` produced by
Charon + Aeneas (`nix run .#aeneas-translate`); it is regenerated, not
edited. The refinement theorems relating it to `Bijou.Family` (the SPEC
model in ../lean) live alongside it.
-/
