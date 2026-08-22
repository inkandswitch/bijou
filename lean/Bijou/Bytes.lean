/-!
# Big-endian byte strings

Fixed-width big-endian encoding of natural numbers, its inverse, and a
strict lexicographic order on byte strings.

bijou payloads are big-endian so that lexicographic byte comparison
agrees with numeric comparison; the key fact is `beBytes_lex_beBytes`.

Bytes are modelled as `Nat` with explicit `< 256` hypotheses where
required, which keeps every side condition within `omega`'s reach.
-/

namespace Bijou

/-- The `w`-byte big-endian representation of `n`, most significant byte
first. Truncates if `n ≥ 256 ^ w`; callers establish the bound. -/
def beBytes : Nat → Nat → List Nat
  | 0, _ => []
  | w + 1, n => n / 256 ^ w :: beBytes w (n % 256 ^ w)

/-- Interpret a byte string as a big-endian natural number. -/
def fromBe : List Nat → Nat
  | [] => 0
  | b :: bs => b * 256 ^ bs.length + fromBe bs

@[simp]
theorem beBytes_length (w n : Nat) : (beBytes w n).length = w := by
  induction w generalizing n with
  | zero => rfl
  | succ w ih => simp [beBytes, ih]

theorem beBytes_lt_256 {w n : Nat} (h : n < 256 ^ w) :
    ∀ b ∈ beBytes w n, b < 256 := by
  induction w generalizing n with
  | zero => simp [beBytes]
  | succ w ih =>
    intro b hb
    have hpos : 0 < 256 ^ w := Nat.pow_pos (by omega)
    simp only [beBytes, List.mem_cons] at hb
    cases hb with
    | inl hhead =>
      subst hhead
      have hpow : 256 ^ (w + 1) = 256 ^ w * 256 := by rw [Nat.pow_succ]
      exact Nat.div_lt_of_lt_mul (by omega)
    | inr htail => exact ih (Nat.mod_lt n hpos) b htail

/-- Decoding inverts encoding: `fromBe ∘ beBytes w` is the identity on
`[0, 256 ^ w)`. -/
theorem fromBe_beBytes {w n : Nat} (h : n < 256 ^ w) :
    fromBe (beBytes w n) = n := by
  induction w generalizing n with
  | zero =>
    simp only [beBytes, fromBe]
    omega
  | succ w ih =>
    have hpos : 0 < 256 ^ w := Nat.pow_pos (by omega)
    simp only [beBytes, fromBe, beBytes_length, ih (Nat.mod_lt n hpos)]
    rw [Nat.mul_comm]
    exact Nat.div_add_mod n (256 ^ w)

theorem fromBe_lt {bs : List Nat} (h : ∀ b ∈ bs, b < 256) :
    fromBe bs < 256 ^ bs.length := by
  induction bs with
  | nil => simp [fromBe]
  | cons b bs ih =>
    have hb : b < 256 := h b (List.mem_cons_self ..)
    have hr : fromBe bs < 256 ^ bs.length :=
      ih fun x hx => h x (List.mem_cons_of_mem _ hx)
    have hpow : 256 ^ (bs.length + 1) = 256 ^ bs.length * 256 := by rw [Nat.pow_succ]
    have hmul : (b + 1) * 256 ^ bs.length ≤ 256 * 256 ^ bs.length :=
      Nat.mul_le_mul (by omega) (Nat.le_refl _)
    simp only [fromBe, List.length_cons]
    have hsucc : (b + 1) * 256 ^ bs.length = b * 256 ^ bs.length + 256 ^ bs.length := by
      rw [Nat.succ_mul]
    omega

/-- Encoding inverts decoding: every `w`-byte string is the encoding of
its value. This is the structural-canonicality workhorse. -/
theorem beBytes_fromBe {bs : List Nat} (h : ∀ b ∈ bs, b < 256) :
    beBytes bs.length (fromBe bs) = bs := by
  induction bs with
  | nil => rfl
  | cons b bs ih =>
    have hr : fromBe bs < 256 ^ bs.length :=
      fromBe_lt fun x hx => h x (List.mem_cons_of_mem _ hx)
    have hpos : 0 < 256 ^ bs.length := Nat.pow_pos (by omega)
    have hdiv : (fromBe bs + b * 256 ^ bs.length) / 256 ^ bs.length = b := by
      rw [Nat.add_mul_div_right _ _ hpos, Nat.div_eq_of_lt hr, Nat.zero_add]
    have hmod : (fromBe bs + b * 256 ^ bs.length) % 256 ^ bs.length = fromBe bs := by
      rw [Nat.add_mul_mod_self_right, Nat.mod_eq_of_lt hr]
    simp only [List.length_cons, beBytes, fromBe, Nat.add_comm (b * 256 ^ bs.length),
      hdiv, hmod, ih fun x hx => h x (List.mem_cons_of_mem _ hx)]

theorem take_append_length (as bs : List α) : (as ++ bs).take as.length = as := by
  induction as with
  | nil => rfl
  | cons a as ih => simp [List.take_succ_cons, ih]

theorem mem_of_mem_take {l : List α} {n : Nat} {a : α} (h : a ∈ l.take n) : a ∈ l := by
  induction l generalizing n with
  | nil => simp at h
  | cons x xs ih =>
    cases n with
    | zero => simp at h
    | succ n =>
      simp only [List.take_succ_cons, List.mem_cons] at h
      cases h with
      | inl h => simp [h]
      | inr h => exact List.mem_cons_of_mem _ (ih h)

/-- Strict lexicographic order on byte strings. -/
inductive Lex : List Nat → List Nat → Prop
  | nil {b : Nat} {bs : List Nat} : Lex [] (b :: bs)
  | head {a b : Nat} {as bs : List Nat} : a < b → Lex (a :: as) (b :: bs)
  | tail {a : Nat} {as bs : List Nat} : Lex as bs → Lex (a :: as) (a :: bs)

theorem Lex.irrefl : ∀ (bs : List Nat), ¬Lex bs bs := by
  intro bs h
  induction bs with
  | nil => cases h
  | cons b bs ih =>
    cases h with
    | head h => omega
    | tail h => exact ih h

theorem Lex.asymm {as bs : List Nat} (h : Lex as bs) : ¬Lex bs as := by
  induction h with
  | nil => intro h2; cases h2
  | head h =>
    intro h2
    cases h2 with
    | head h2 => omega
    | tail _ => omega
  | tail h ih =>
    intro h2
    cases h2 with
    | head h2 => omega
    | tail h2 => exact ih h2

/-- `Lex` is transitive. With `irrefl` and `asymm`, this makes it a
strict order — so sorting byte strings by `Lex` is well-defined. -/
theorem Lex.trans {as bs cs : List Nat} (h₁ : Lex as bs) : Lex bs cs → Lex as cs := by
  induction h₁ generalizing cs with
  | nil => intro h₂; cases h₂ <;> exact Lex.nil
  | head hab =>
    intro h₂
    cases h₂ with
    | head hbc => exact Lex.head (by omega)
    | tail _ => exact Lex.head hab
  | tail _ ih =>
    intro h₂
    cases h₂ with
    | head hbc => exact Lex.head hbc
    | tail h₂' => exact Lex.tail (ih h₂')

/-- Big-endian encoding is strictly monotone with respect to
lexicographic order: numeric order and byte order agree. -/
theorem beBytes_lex_beBytes {w m n : Nat} (hn : n < 256 ^ w) (h : m < n) :
    Lex (beBytes w m) (beBytes w n) := by
  induction w generalizing m n with
  | zero =>
    simp only [Nat.pow_zero] at hn
    omega
  | succ w ih =>
    have hpos : 0 < 256 ^ w := Nat.pow_pos (by omega)
    have hdm := Nat.div_add_mod m (256 ^ w)
    have hdn := Nat.div_add_mod n (256 ^ w)
    have hle : m / 256 ^ w ≤ n / 256 ^ w := Nat.div_le_div_right (Nat.le_of_lt h)
    simp only [beBytes]
    by_cases hd : m / 256 ^ w = n / 256 ^ w
    · rw [hd]
      rw [hd] at hdm
      exact Lex.tail (ih (Nat.mod_lt n hpos) (by omega))
    · exact Lex.head (by omega)

end Bijou
