module

public import BijouAeneas.Generated.FunsExternal
public import Mathlib.Tactic.IntervalCases

@[expose] public section

/-!
# Specs for `core`/`alloc` operations used by the translated `bijoux` code

`bijoux` is `no_std`, but uses `alloc` (`Vec`) and `core` (slices, integer
intrinsics). Width-independent specs: `Vec::extend_from_slice` on bytes,
our `Vec::truncate` model (`Generated/FunsExternal.lean`), and list
bookkeeping for the fixed-size encode buffer.
-/

open Aeneas Aeneas.Std Result

namespace BijouAeneas

@[step]
theorem extend_u8_spec (v : alloc.vec.Vec Std.U8) (s : Slice Std.U8)
    (h : v.length + s.length ≤ Usize.max) :
    alloc.vec.Vec.extend_from_slice core.clone.CloneU8 v s ⦃ w => w.val = v.val ++ s.val ⦄ := by
  unfold alloc.vec.Vec.extend_from_slice
  simp only [h, _root_.dite_true]
  step with List.clone_spec (by intro x _; rfl)
  simp_all

@[step]
theorem truncate_spec {T : Type} (A : Type) (v : alloc.vec.Vec T) (n : Std.Usize) :
    alloc.vec.Vec.truncate A v n ⦃ w => w.val = v.val.take n.val ⦄ := by
  simp [alloc.vec.Vec.truncate, WP.spec_ok]

theorem four_elems {α : Type} (l : List α) (h : l.length = 4) :
    [l[0], l[1], l[2], l[3]] = l := by
  apply List.ext_getElem
  · simp [h]
  · intro j h1 h2
    simp at h1
    interval_cases j <;> rfl

theorem eight_elems {α : Type} (l : List α) (h : l.length = 8) :
    [l[0], l[1], l[2], l[3], l[4], l[5], l[6], l[7]] = l := by
  apply List.ext_getElem
  · simp [h]
  · intro j h1 h2
    simp at h1
    interval_cases j <;> rfl

theorem sixteen_elems {α : Type} (l : List α) (h : l.length = 16) :
    [l[0], l[1], l[2], l[3], l[4], l[5], l[6], l[7], l[8], l[9], l[10], l[11], l[12], l[13], l[14], l[15]] = l := by
  apply List.ext_getElem
  · simp [h]
  · intro j h1 h2
    simp at h1
    interval_cases j <;> rfl

theorem take_assemble {α β : Type} (f : α → β) (buf : List α) (tag : α) (pb : List α)
    (t : ℕ) :
    ((buf ++ tag :: pb).take (buf.length + t + 1)).map f =
      buf.map f ++ f tag :: (pb.take t).map f := by
  rw [List.take_append, List.take_of_length_le (by omega),
    show buf.length + t + 1 - buf.length = t + 1 by omega]
  simp

@[step]
theorem split_first_spec {T : Type} (s : Slice T) :
    core.slice.Slice.split_first s ⦃ o => match o with
      | none => s.val = []
      | some (x, r) => s.val = x :: r.val ⦄ := by
  unfold core.slice.Slice.split_first
  split <;> simp_all

@[step]
theorem first_chunk_spec {T : Type} (N : Std.Usize) (s : Slice T) :
    core.slice.Slice.first_chunk N s ⦃ o => match o with
      | none => s.val.length < N.val
      | some a => N.val ≤ s.val.length ∧ a.val = s.val.take N.val ⦄ := by
  unfold core.slice.Slice.first_chunk
  split <;> simp_all

end BijouAeneas
