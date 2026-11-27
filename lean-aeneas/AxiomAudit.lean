import Bijou
import BijouAeneas

/-!
# Axiom audit

Fails unless every theorem in the model (`Bijou.*`) and the refinement proofs
(`BijouAeneas.*`) depends only on Lean's three standard axioms.

`lake build --wfail` already rejects `sorry`, but not the other ways a proof can
quietly trust more than the kernel: `native_decide` and `bv_decide` (which add
`Lean.ofReduceBool` / `_native` axioms, trusting compiled code), or a stray
`axiom`. Run by `nix run .#ci-aeneas`:

```sh
lake env lean AxiomAudit.lean
```
-/

open Lean Elab Command

run_cmd do
  let env ← getEnv
  let standard := #[``propext, ``Classical.choice, ``Quot.sound]
  let inScope (n : Name) := (`Bijou).isPrefixOf n || (`BijouAeneas).isPrefixOf n
  let isThm : ConstantInfo → Bool
    | .thmInfo _ => true
    | _ => false
  let theorems := env.constants.fold (init := #[]) fun acc decl info =>
    if inScope decl && !decl.isInternal && isThm info then acc.push decl else acc
  let mut offenders : Array MessageData := #[]
  for decl in theorems do
    let axioms ← liftCoreM <| collectAxioms decl
    let extra := axioms.filter fun a => !standard.contains a
    unless extra.isEmpty do
      offenders := offenders.push m!"{decl}: {extra.toList}"
  unless offenders.isEmpty do
    throwError m!"{offenders.size} theorem(s) use non-standard axioms:\n" ++
      MessageData.joinSep offenders.toList "\n"
  logInfo m!"axiom audit: {theorems.size} theorems, standard axioms only"
