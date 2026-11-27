module

public import BijouAeneas.Refinement.U128

@[expose] public section

/-!
# Properties of the real `bijoux::u128`, derived from the refinement specs

`Refinement/U128.lean` proves that each Rust function returns exactly what the
SPEC model prescribes (`encoded_len_spec`, `encode_spec`, `decode_spec`).
Combining those equalities with theorems already proven about the model
(`lean/Bijou/*.lean`) gives the familiar format properties as statements about
the Rust functions themselves:

- `decode_encoded` / `decode_after_encode` — round-trip.
- `decode_canonical` — anything `decode` accepts is the encoding of its result.
- `decode_consumed_from_tag` — consumed length depends only on the first byte.
- `encode_injective` — distinct values never share an encoding.
- `encode_lex_iff` — byte order of encodings equals numeric order.
- `encoded_len_eq_appended` — `encoded_len` is the number of bytes `encode`
  appends.

These are consequences of the specs, which remain the primary (and stronger)
results: each corollary covers one property, while a spec pins down the
function's entire behaviour.

Generated from `U64Corollaries.lean` by `scripts/gen_widths.py`: change that
file, then regenerate, rather than editing this one.
-/

open Aeneas Aeneas.Std Result
open BijouRust

namespace BijouAeneas.U128

/-- Every `u128` is in the model's value domain. -/
theorem val_lt_pow (v : Std.U128) : v.val < 256 ^ Bijou.bijou128.tiers := by
  have h := v.hBounds
  have : (256 : ℕ) ^ Bijou.bijou128.tiers = 2 ^ 128 := by norm_num [Bijou.bijou128]
  rw [this]; simpa using h

/-- Round-trip: `decode` on bytes that start with the encoding of `v` returns
`v` and the encoding's length, whatever follows. -/
theorem decode_encoded (v : Std.U128) (s : Slice Std.U8) (rest : List ℕ)
    (hs : s.val.map (·.val) = Bijou.bijou128.encode v.val ++ rest) :
    u128.decode s ⦃ r => ∃ n : Std.Usize,
      r = .Ok (v, n) ∧ n.val = Bijou.bijou128.encodedLen v.val ⦄ := by
  apply WP.spec_mono (decode_spec s)
  intro r hr
  rw [hs, Bijou.bijou128.decode_encode (val_lt_pow v) rest] at hr
  match r with
  | .Ok (w, n) =>
    simp only [toModel, Except.ok.injEq, Prod.mk.injEq] at hr
    exact ⟨n, by rw [UScalar.eq_of_val_eq hr.1], hr.2⟩
  | .Err .BufferTooShort => simp [toModel] at hr
  | .Err .Overflow => simp [toModel] at hr

/-- Canonicality: whenever `decode` accepts an input, the bytes it consumed
are exactly the encoding of the value it returned. There is no other way to
spell a value that the decoder would accept. -/
theorem decode_canonical (s : Slice Std.U8) :
    u128.decode s ⦃ r => ∀ (v : Std.U128) (n : Std.Usize), r = .Ok (v, n) →
      n.val = Bijou.bijou128.encodedLen v.val ∧
      (s.val.map (·.val)).take n.val = Bijou.bijou128.encode v.val ⦄ := by
  apply WP.spec_mono (decode_spec s)
  intro r hr v n hrv
  subst hrv
  simp only [toModel] at hr
  have hbytes : ∀ b ∈ s.val.map (·.val), b < 256 := by
    intro b hb
    simp only [List.mem_map] at hb
    obtain ⟨x, _, rfl⟩ := hb
    have := x.hBounds
    simpa using this
  have := Bijou.bijou128.decode_ok hbytes hr.symm
  exact ⟨this.2.1, this.2.2⟩

/-- Framing: on success, the number of bytes consumed is determined by the
first byte alone (`lenFromTag`), without looking at the payload. -/
theorem decode_consumed_from_tag (s : Slice Std.U8) (tag : ℕ) (rest : List ℕ)
    (hs : s.val.map (·.val) = tag :: rest) :
    u128.decode s ⦃ r => ∀ (v : Std.U128) (n : Std.Usize), r = .Ok (v, n) →
      n.val = Bijou.bijou128.lenFromTag tag ⦄ := by
  apply WP.spec_mono (decode_spec s)
  intro r hr v n hrv
  subst hrv
  simp only [toModel] at hr
  rw [hs] at hr
  exact Bijou.bijou128.decode_consumed_from_tag hr.symm

/-- The bytes `encode v` appended after `buf`. -/
theorem appended_eq (v : Std.U128) (buf out : alloc.vec.Vec Std.U8)
    (h : out.val.map (·.val) = buf.val.map (·.val) ++ Bijou.bijou128.encode v.val) :
    (out.val.map (·.val)).drop buf.val.length = Bijou.bijou128.encode v.val := by
  rw [h]
  simp

/-- Injectivity: encoding two values into the same buffer yields the same
bytes only if the values are equal. -/
theorem encode_injective (v w : Std.U128) (buf : alloc.vec.Vec Std.U8)
    (hbuf : buf.length + 17 ≤ Usize.max) :
    u128.encode v buf ⦃ o₁ => u128.encode w buf ⦃ o₂ => o₁.val = o₂.val → v = w ⦄ ⦄ := by
  apply WP.spec_mono (encode_spec v buf hbuf)
  intro o₁ h₁
  apply WP.spec_mono (encode_spec w buf hbuf)
  intro o₂ h₂ heq
  have h : Bijou.bijou128.encode v.val = Bijou.bijou128.encode w.val := by
    have := congrArg (List.map (·.val)) heq
    rw [h₁, h₂] at this
    exact List.append_cancel_left this
  exact UScalar.eq_of_val_eq (Bijou.bijou128.encode_injective (val_lt_pow v) (val_lt_pow w) h)

/-- Order: the bytes appended by `encode` compare lexicographically exactly as
the values compare numerically, so encodings sort like the integers. -/
theorem encode_lex_iff (v w : Std.U128) (buf : alloc.vec.Vec Std.U8)
    (hbuf : buf.length + 17 ≤ Usize.max) :
    u128.encode v buf ⦃ o₁ => u128.encode w buf ⦃ o₂ =>
      (v.val < w.val ↔ Bijou.Lex ((o₁.val.map (·.val)).drop buf.val.length)
        ((o₂.val.map (·.val)).drop buf.val.length)) ⦄ ⦄ := by
  apply WP.spec_mono (encode_spec v buf hbuf)
  intro o₁ h₁
  apply WP.spec_mono (encode_spec w buf hbuf)
  intro o₂ h₂
  rw [appended_eq v buf o₁ h₁, appended_eq w buf o₂ h₂]
  exact Bijou.bijou128.encode_lex_iff (val_lt_pow v) (val_lt_pow w)

/-- `encoded_len v` is exactly the number of bytes `encode v` appends. -/
theorem encoded_len_eq_appended (v : Std.U128) (buf : alloc.vec.Vec Std.U8)
    (hbuf : buf.length + 17 ≤ Usize.max) :
    u128.encoded_len v ⦃ n => u128.encode v buf ⦃ out =>
      out.val.length = buf.val.length + n.val ⦄ ⦄ := by
  apply WP.spec_mono (encoded_len_spec v)
  intro n hn
  apply WP.spec_mono (encode_spec v buf hbuf)
  intro out hout
  have := congrArg List.length hout
  simp only [List.length_map, List.length_append, Bijou.Family.encode_length] at this
  rw [this, hn]

/-- Encode then decode: decoding a freshly encoded buffer returns the value. -/
theorem decode_after_encode (v : Std.U128) (buf : alloc.vec.Vec Std.U8) (hb : buf.val = []) :
    u128.encode v buf ⦃ out => u128.decode (alloc.vec.Vec.deref out) ⦃ r =>
      ∃ n : Std.Usize, r = .Ok (v, n) ∧ n.val = Bijou.bijou128.encodedLen v.val ⦄ ⦄ := by
  have hbuf : buf.length + 17 ≤ Usize.max := by
    have : buf.length = 0 := by simp [hb]
    rw [this]; scalar_tac
  apply WP.spec_mono (encode_spec v buf hbuf)
  intro out hout
  apply decode_encoded v _ []
  rw [hb] at hout
  simpa [alloc.vec.Vec.deref] using hout

end BijouAeneas.U128
