module

public import Aeneas
public import Bijou.Bytes
public import Mathlib.Tactic

@[expose] public section

/-!
# Big-endian bytes: Aeneas's `toBEBytes` vs the model's `beBytes`

Aeneas defines `to_be_bytes` bit by bit (`BitVec.toBEBytes`). These
lemmas give each byte's numeric value and connect a shifted payload's
leading bytes to `Bijou.beBytes`, the SPEC model's payload encoding.
They hold for any width of `n` bytes, so `u32`, `u64` and `u128` share them.
-/

open Aeneas Aeneas.Std

namespace BijouAeneas

theorem toLEBytes_getElem!_toNat {w : ℕ} (x : BitVec w) (i : ℕ) :
    (x.toLEBytes[i]!).toNat = x.toNat / 256 ^ i % 256 := by
  apply Nat.eq_of_testBit_eq
  intro k
  by_cases hk : k < 8
  · have h := BitVec.toLEBytes_getElem!_testBit x i k hk
    simp only [Byte.testBit] at h
    rw [h, BitVec.getElem!_eq_testBit_toNat]
    rw [show (256 : ℕ) = 2 ^ 8 by norm_num, ← pow_mul, Nat.testBit_mod_two_pow,
      Nat.testBit_div_two_pow]
    simp [hk, Nat.add_comm, Nat.mul_comm]
  · have h256 : (256 : ℕ) ≤ 2 ^ k := by
      calc (256 : ℕ) = 2 ^ 8 := by norm_num
        _ ≤ 2 ^ k := Nat.pow_le_pow_right (by norm_num) (by omega)
    have hl : (x.toLEBytes[i]!).toNat < 2 ^ k :=
      lt_of_lt_of_le (BitVec.isLt _) (by simpa using h256)
    have hr : x.toNat / 256 ^ i % 256 < 2 ^ k := lt_of_lt_of_le (Nat.mod_lt _ (by norm_num)) h256
    rw [Nat.testBit_lt_two_pow hl, Nat.testBit_lt_two_pow hr]

theorem toBEBytes_getElem!_toNat {n : ℕ} (x : BitVec (8 * n)) (j : ℕ) (hj : j < n) :
    (x.toBEBytes[j]!).toNat = x.toNat / 256 ^ (n - 1 - j) % 256 := by
  have hlen : x.toLEBytes.length = n := by simp; omega
  have hj' : j < x.toLEBytes.reverse.length := by simp; omega
  rw [BitVec.toBEBytes, getElem!_pos x.toLEBytes.reverse j hj', List.getElem_reverse,
    ← getElem!_pos x.toLEBytes (x.toLEBytes.length - 1 - j) (by omega),
    toLEBytes_getElem!_toNat, hlen]

open Bijou

/-- Digit `m` (from the right) of `y % 256^t` is digit `m` of `y`, for `m < t`. -/
theorem mod_pow_digit (y t m : ℕ) (hm : m < t) :
    y % 256 ^ t / 256 ^ m % 256 = y / 256 ^ m % 256 := by
  have hsplit : (256 : ℕ) ^ t = 256 ^ m * 256 ^ (t - m) := by
    rw [← pow_add]; congr 1; omega
  rw [hsplit, Nat.mod_mul_right_div_self]
  apply Nat.mod_mod_of_dvd
  exact dvd_pow_self 256 (by omega)

theorem beBytes_getElem! (t y j : ℕ) (hy : y < 256 ^ t) (hj : j < t) :
    (beBytes t y)[j]! = y / 256 ^ (t - 1 - j) % 256 := by
  induction t generalizing y j with
  | zero => omega
  | succ t ih =>
    rcases j with _ | j
    · have hlt : y / 256 ^ t < 256 := by
        rw [Nat.div_lt_iff_lt_mul (by positivity), mul_comm, ← pow_succ]; exact hy
      simp only [beBytes, List.getElem!_cons_zero, show t + 1 - 1 - 0 = t by omega,
        Nat.mod_eq_of_lt hlt]
    · have hmod : y % 256 ^ t < 256 ^ t := Nat.mod_lt _ (by positivity)
      simp only [beBytes, List.getElem!_cons_succ]
      rw [ih (y % 256 ^ t) j hmod (by omega), mod_pow_digit y t (t - 1 - j) (by omega),
        show t + 1 - 1 - (j + 1) = t - 1 - j by omega]

/-- Shifting a `t`-byte value up by `n - t` bytes moves its digits up unchanged. -/
theorem shifted_digit (n y t j : ℕ) (ht : t ≤ n) (hj : j < t) :
    y * 256 ^ (n - t) / 256 ^ (n - 1 - j) % 256 = y / 256 ^ (t - 1 - j) % 256 := by
  rw [show n - 1 - j = (n - t) + (t - 1 - j) by omega, pow_add, mul_comm y,
    Nat.mul_div_mul_left _ _ (by positivity)]

/-- The first `t` big-endian bytes of an `n`-byte integer holding
`y * 256^(n-t)` are exactly the model's `beBytes t y`. -/
theorem payload_bytes {n : ℕ} (x : BitVec (8 * n)) (y t : ℕ) (ht : t ≤ n) (hy : y < 256 ^ t)
    (hx : x.toNat = y * 256 ^ (n - t)) :
    (x.toBEBytes.take t).map (·.toNat) = beBytes t y := by
  apply List.ext_getElem
  · simp; omega
  · intro j h1 h2
    simp only [List.length_map, List.length_take, BitVec.toBEBytes_length] at h1
    rw [List.getElem_map, List.getElem_take, ← getElem!_pos x.toBEBytes j (by simp; omega),
      toBEBytes_getElem!_toNat x j (by omega), hx, shifted_digit n y t j ht (by omega),
      ← getElem!_pos (beBytes t y) j h2, beBytes_getElem! t y j hy (by omega)]

/-! ## The other direction: `from_be_bytes` vs the model's `fromBe` -/

/-- Little-endian value of a list of byte values. -/
def leVal : List ℕ → ℕ
  | [] => 0
  | b :: bs => b + 256 * leVal bs

theorem leVal_lt (l : List ℕ) (h : ∀ b ∈ l, b < 256) : leVal l < 256 ^ l.length := by
  induction l with
  | nil => simp [leVal]
  | cons b bs ih =>
    have hb := h b (by simp)
    have := ih (fun x hx => h x (by simp [hx]))
    simp only [leVal, List.length_cons, pow_succ]
    omega

theorem fromLEBytes_toNat (l : List Byte) :
    (BitVec.fromLEBytes l).toNat = leVal (l.map BitVec.toNat) := by
  induction l with
  | nil => simp [BitVec.fromLEBytes, leVal]
  | cons b l ih =>
    rw [BitVec.fromLEBytes, BitVec.toNat_or, BitVec.toNat_setWidth, BitVec.toNat_shiftLeft,
      BitVec.toNat_setWidth, ih]
    have hb : b.toNat < 2 ^ 8 := b.isLt
    have hL : leVal (l.map BitVec.toNat) < 2 ^ (8 * l.length) := by
      have := leVal_lt (l.map BitVec.toNat) (fun x hx => by
        simp only [List.mem_map] at hx
        obtain ⟨y, _, rfl⟩ := hx
        exact y.isLt)
      rw [pow_mul]; simpa using this
    have hW : (2 : ℕ) ^ (8 * (b :: l).length) = 2 ^ (8 * l.length) * 2 ^ 8 := by
      rw [List.length_cons, mul_add, mul_one, pow_add]
    simp only [leVal, List.map_cons]
    generalize leVal (l.map BitVec.toNat) = L at hL ⊢
    have h8 : (2 : ℕ) ^ 8 ≤ 2 ^ (8 * l.length) * 2 ^ 8 :=
      Nat.le_mul_of_pos_left _ (by positivity)
    have hLW : L < 2 ^ (8 * l.length) * 2 ^ 8 :=
      lt_of_lt_of_le hL (Nat.le_mul_of_pos_right _ (by positivity))
    have hsh : L * 2 ^ 8 < 2 ^ (8 * l.length) * 2 ^ 8 :=
      Nat.mul_lt_mul_of_pos_right hL (by positivity)
    rw [hW, Nat.mod_eq_of_lt (lt_of_lt_of_le hb h8), Nat.mod_eq_of_lt hLW, Nat.shiftLeft_eq,
      Nat.mod_eq_of_lt hsh, ← Nat.shiftLeft_eq, Nat.or_comm,
      ← Nat.shiftLeft_add_eq_or_of_lt hb, Nat.shiftLeft_eq]
    ring

theorem leVal_append_single (l : List ℕ) (b : ℕ) :
    leVal (l ++ [b]) = leVal l + 256 ^ l.length * b := by
  induction l with
  | nil => simp [leVal]
  | cons x xs ih => simp only [List.cons_append, leVal, ih, List.length_cons, pow_succ]; ring

theorem fromBe_eq_leVal_reverse (l : List ℕ) : Bijou.fromBe l = leVal l.reverse := by
  induction l with
  | nil => simp [Bijou.fromBe, leVal]
  | cons b bs ih =>
    rw [Bijou.fromBe, List.reverse_cons, leVal_append_single, ← ih, List.length_reverse]
    ring

theorem fromBEBytes_toNat (l : List Byte) :
    (BitVec.fromBEBytes l).toNat = Bijou.fromBe (l.map BitVec.toNat) := by
  rw [BitVec.fromBEBytes, BitVec.toNat_cast, fromLEBytes_toNat, fromBe_eq_leVal_reverse,
    List.map_reverse]

theorem fromBe_zeros (k : ℕ) (l : List ℕ) :
    Bijou.fromBe (List.replicate k 0 ++ l) = Bijou.fromBe l := by
  induction k with
  | zero => simp
  | succ k ih => simp [List.replicate_succ, Bijou.fromBe, ih]

end BijouAeneas
