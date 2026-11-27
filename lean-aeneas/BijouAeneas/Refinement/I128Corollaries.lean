module

public import BijouAeneas.Refinement.I128

@[expose] public section

/-!
# Properties of the real `bijoux::i128`, derived from the refinement specs

`Refinement/I128.lean` proves that each Rust function returns exactly what the
signed SPEC model prescribes. Combining those equalities with the model's
signed theorems (`lean/Bijou/Signed.lean`) gives the format properties as
statements about the Rust functions themselves:

- `decode_encoded` / `decode_after_encode` — round-trip.
- `decode_canonical` — anything `decode` accepts is the encoding of its result.
- `decode_consumed_from_tag` — consumed length depends only on the first byte.
- `encode_injective` — distinct values never share an encoding.
- `encode_lex_iff` — byte order of encodings is *zigzag* order
  (`0, -1, 1, -2, 2, …`), not numeric order.
- `encoded_len_eq_appended` — `encoded_len` is the number of bytes `encode`
  appends.

The proofs are `I64.lean`'s with the bit width substituted
(`scripts/gen_widths.py`). Change `I64.lean` first, then regenerate
this file rather than editing it by hand.
-/

open Aeneas Aeneas.Std Result
open BijouRust

namespace BijouAeneas.I128

/-- Every `i128` is in the model's signed value domain `[-half, half)`. -/
theorem val_in_half (v : Std.I128) :
    -(Bijou.bijou128.half : ℤ) ≤ v.val ∧ v.val < Bijou.bijou128.half := by
  have h := v.hBounds
  have : (Bijou.bijou128.half : ℤ) = 2 ^ 127 := by norm_num [Bijou.Family.half, Bijou.bijou128]
  rw [this]; simpa using h

theorem decode_encoded (v : Std.I128) (s : Slice Std.U8) (rest : List ℕ)
    (hs : s.val.map (·.val) = Bijou.bijou128.encodeS v.val ++ rest) :
    i128.decode s ⦃ r => ∃ n : Std.Usize,
      r = .Ok (v, n) ∧ n.val = Bijou.bijou128.encodedLenS v.val ⦄ := by
  apply WP.spec_mono (decode_spec s)
  intro r hr
  obtain ⟨h1, h2⟩ := val_in_half v
  rw [hs, Bijou.bijou128.decodeS_encodeS h1 h2 rest] at hr
  match r with
  | .Ok (w, n) =>
    simp only [toModelS, Except.ok.injEq, Prod.mk.injEq] at hr
    exact ⟨n, by rw [IScalar.eq_of_val_eq hr.1], hr.2⟩
  | .Err .BufferTooShort => simp [toModelS] at hr
  | .Err .Overflow => simp [toModelS] at hr

/-- Canonicality: whenever `decode` accepts an input, the value is in range
and the bytes it consumed are exactly the encoding of that value. -/
theorem decode_canonical (s : Slice Std.U8) :
    i128.decode s ⦃ r => ∀ (v : Std.I128) (n : Std.Usize), r = .Ok (v, n) →
      n.val = Bijou.bijou128.encodedLenS v.val ∧
      (s.val.map (·.val)).take n.val = Bijou.bijou128.encodeS v.val ⦄ := by
  apply WP.spec_mono (decode_spec s)
  intro r hr v n hrv
  subst hrv
  simp only [toModelS] at hr
  have hbytes : ∀ b ∈ s.val.map (·.val), b < 256 := by
    intro b hb
    simp only [List.mem_map] at hb
    obtain ⟨x, _, rfl⟩ := hb
    have := x.hBounds
    simpa using this
  have := Bijou.bijou128.decodeS_ok hbytes hr.symm
  exact ⟨this.2.1, this.2.2⟩

/-- Framing: on success, the number of bytes consumed is determined by the
first byte alone (`lenFromTag`), as for the unsigned format. -/
theorem decode_consumed_from_tag (s : Slice Std.U8) (tag : ℕ) (rest : List ℕ)
    (hs : s.val.map (·.val) = tag :: rest) :
    i128.decode s ⦃ r => ∀ (v : Std.I128) (n : Std.Usize), r = .Ok (v, n) →
      n.val = Bijou.bijou128.lenFromTag tag ⦄ := by
  apply WP.spec_mono (decode_spec s)
  intro r hr v n hrv
  subst hrv
  simp only [toModelS, Bijou.Family.decodeS] at hr
  rw [hs] at hr
  split at hr
  · rename_i z m hd
    simp only [Except.ok.injEq, Prod.mk.injEq] at hr
    rw [hr.2]
    exact Bijou.bijou128.decode_consumed_from_tag hd
  · simp at hr

/-- The bytes `encode v` appended after `buf`. -/
theorem appended_eq (v : Std.I128) (buf out : alloc.vec.Vec Std.U8)
    (h : out.val.map (·.val) = buf.val.map (·.val) ++ Bijou.bijou128.encodeS v.val) :
    (out.val.map (·.val)).drop buf.val.length = Bijou.bijou128.encodeS v.val := by
  rw [h]
  simp

/-- Injectivity: encoding two values into the same buffer yields the same
bytes only if the values are equal. -/
theorem encode_injective (v w : Std.I128) (buf : alloc.vec.Vec Std.U8)
    (hbuf : buf.length + 17 ≤ Usize.max) :
    i128.encode v buf ⦃ o₁ => i128.encode w buf ⦃ o₂ => o₁.val = o₂.val → v = w ⦄ ⦄ := by
  apply WP.spec_mono (encode_spec v buf hbuf)
  intro o₁ h₁
  apply WP.spec_mono (encode_spec w buf hbuf)
  intro o₂ h₂ heq
  have h : Bijou.bijou128.encodeS v.val = Bijou.bijou128.encodeS w.val := by
    have := congrArg (List.map (·.val)) heq
    rw [h₁, h₂] at this
    exact List.append_cancel_left this
  obtain ⟨hv1, hv2⟩ := val_in_half v
  obtain ⟨hw1, hw2⟩ := val_in_half w
  exact IScalar.eq_of_val_eq (Bijou.bijou128.encodeS_injective hv1 hv2 hw1 hw2 h)

/-- Order: the bytes appended by `encode` compare lexicographically in
*zigzag* order — by absolute value, and a negative value just before its
positive counterpart (`0, -1, 1, -2, 2, …`) — not in numeric order. -/
theorem encode_lex_iff (v w : Std.I128) (buf : alloc.vec.Vec Std.U8)
    (hbuf : buf.length + 17 ≤ Usize.max) :
    i128.encode v buf ⦃ o₁ => i128.encode w buf ⦃ o₂ =>
      (Bijou.Lex ((o₁.val.map (·.val)).drop buf.val.length)
          ((o₂.val.map (·.val)).drop buf.val.length) ↔
        v.val.natAbs < w.val.natAbs ∨ (v.val.natAbs = w.val.natAbs ∧ v.val < w.val)) ⦄ ⦄ := by
  apply WP.spec_mono (encode_spec v buf hbuf)
  intro o₁ h₁
  apply WP.spec_mono (encode_spec w buf hbuf)
  intro o₂ h₂
  rw [appended_eq v buf o₁ h₁, appended_eq w buf o₂ h₂]
  obtain ⟨hv1, hv2⟩ := val_in_half v
  obtain ⟨hw1, hw2⟩ := val_in_half w
  exact Bijou.bijou128.encodeS_lex_iff hv1 hv2 hw1 hw2

/-- `encoded_len v` is exactly the number of bytes `encode v` appends. -/
theorem encoded_len_eq_appended (v : Std.I128) (buf : alloc.vec.Vec Std.U8)
    (hbuf : buf.length + 17 ≤ Usize.max) :
    i128.encoded_len v ⦃ n => i128.encode v buf ⦃ out =>
      out.val.length = buf.val.length + n.val ⦄ ⦄ := by
  apply WP.spec_mono (encoded_len_spec v)
  intro n hn
  apply WP.spec_mono (encode_spec v buf hbuf)
  intro out hout
  have := congrArg List.length hout
  simp only [List.length_map, List.length_append, Bijou.Family.encodeS_length] at this
  rw [this, hn]

/-- Encode then decode: decoding a freshly encoded buffer returns the value. -/
theorem decode_after_encode (v : Std.I128) (buf : alloc.vec.Vec Std.U8) (hb : buf.val = []) :
    i128.encode v buf ⦃ out => i128.decode (alloc.vec.Vec.deref out) ⦃ r =>
      ∃ n : Std.Usize, r = .Ok (v, n) ∧ n.val = Bijou.bijou128.encodedLenS v.val ⦄ ⦄ := by
  have hbuf : buf.length + 17 ≤ Usize.max := by
    have : buf.length = 0 := by simp [hb]
    rw [this]; scalar_tac
  apply WP.spec_mono (encode_spec v buf hbuf)
  intro out hout
  apply decode_encoded v _ []
  rw [hb] at hout
  simpa [alloc.vec.Vec.deref] using hout

end BijouAeneas.I128
