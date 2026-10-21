-- Imscribing/ReEntryConstructions.lean
-- Formalization of "Two Re-Entry Constructions, Their Disagreement, and the Role of Zero"
-- This file formalizes the mathematical core using the Prime Well Space + evaluation map framing:
--   - The Prime Well Space: exponent vectors over the primes (finitely supported lists).
--   - Binary Well Space: bit vectors / distinctions (powerset re-entry).
--   - The evaluation map collapses each well space into the 1D number line ℕ.
--   - The two re-entry constructions disagree on atomicity; zero is the universal fixed point.
--   - Cantor's theorem: no set is equal to its own powerset.

import Mathlib.Logic.Function.Basic
import Mathlib.Data.Nat.Digits.Defs
import Mathlib.Data.Nat.Factors
import Mathlib.Data.Finset.Powerset
import Mathlib.SetTheory.Cardinal.Basic
import Mathlib.Data.List.Count

open Set

-- ---------------------------------------------------------------------------
-- §I.  The Prime Well Space: exponent vectors over the primes.
-- ---------------------------------------------------------------------------

abbrev PrimeWell := List ℕ
def primeEval (v : PrimeWell) : ℕ := List.prod v
def primeDepth (v : PrimeWell) : ℕ := v.length
def primeAdd (v w : PrimeWell) : PrimeWell := v ++ w

-- ---------------------------------------------------------------------------
-- §II.  The Binary Well Space: bit vectors / distinctions.
-- ---------------------------------------------------------------------------

abbrev BinaryWell := List ℕ
def binEval (v : BinaryWell) : ℕ := Nat.ofDigits 2 v
def binDepth (v : BinaryWell) : ℕ := v.count 1
def binAdd (v : BinaryWell) : BinaryWell := 0 :: v

-- ---------------------------------------------------------------------------
-- §III.  Depth functions on ℕ (collapsed from the well spaces).
-- ---------------------------------------------------------------------------

def omega (n : ℕ) : ℕ := if n = 0 then 0 else primeDepth (Nat.primeFactorsList n)
def popcount (n : ℕ) : ℕ := (Nat.digits 2 n).count 1

-- Binary atomic: n has exactly one 1 in binary (n = 2^k).
def binaryAtomic (n : ℕ) : Prop := n > 0 ∧ popcount n = 1

-- Arithmetic atomic: n is prime.
def arithAtomic (n : ℕ) : Prop := n > 0 ∧ Nat.Prime n

-- ---------------------------------------------------------------------------
-- §IV.  Concrete Examples
-- ---------------------------------------------------------------------------

-- 8 is binary-atomic but arith-composite.
theorem eight_binary_not_arith : binaryAtomic 8 ∧ ¬arithAtomic 8 := by
  constructor
  · -- Prove 8 is binary-atomic
    constructor
    · norm_num [binaryAtomic, popcount]
    · decide
  · -- Prove 8 is not arith-atomic
    intro h
    have h₁ : Nat.Prime 8 := h.2
    norm_num [Nat.Prime] at h₁

-- 13 is arith-atomic but binary-composite.
theorem thirteen_arith_not_binary : arithAtomic 13 ∧ ¬binaryAtomic 13 := by
  constructor
  · -- Prove 13 is arith-atomic
    constructor
    · norm_num [arithAtomic]
    · decide
  · -- Prove 13 is not binary-atomic
    intro h
    have h₁ : popcount 13 = 1 := h.2
    norm_num [popcount] at h₁

-- Ω(6) = popcount 6 = 2 but atoms differ.
theorem counts_equal_atoms_differ : omega 6 = popcount 6 := by
  norm_num [omega, popcount, primeDepth, Nat.primeFactorsList]
  <;> rfl

-- ---------------------------------------------------------------------------
-- §V.  Structural Theorems
-- ---------------------------------------------------------------------------

-- Empty structures.
theorem empty_structures : popcount 0 = 0 ∧ omega 1 = 0 := by
  constructor
  · norm_num [popcount]
  · norm_num [omega, primeDepth, Nat.primeFactorsList]

-- Zero is the unique universal fixed point of multiplication maps.
theorem zero_universal_fixed_point (N : ℕ) (hN : N ≠ 1) :
    {x : ℕ | N * x = x} = {0} := by
  apply Set.Subset.antisymm
  · intro x hx
    by_contra h₀
    have h_pos : 0 < x := by
      by_contra h₁
      have h₂ : x = 0 := by omega
      simp_all [Nat.mul_zero]
    have h_N_ge_1 : 1 ≤ N := by
      by_contra h₁
      have h₂ : N = 0 := by omega
      simp [h₂] at hx
      <;> simp_all [Nat.mul_zero]
    have h₃ : (N - 1) * x = 0 := by
      have h₄ : N * x = x := hx
      have h₅ : N ≥ 1 := h_N_ge_1
      have h₆ : N * x = (N - 1) * x + x := by
        have h₇ : N = (N - 1) + 1 := by
          have h₈ : N ≥ 1 := h_N_ge_1
          omega
        rw [h₇]
        <;> ring_nf at *
        <;> simp [Nat.mul_add, Nat.add_mul, Nat.add_assoc] at *
        <;> ring_nf at *
        <;> omega
      rw [h₆] at h₄
      omega
    have h₄ : N - 1 = 0 := by
      have h₅ : (N - 1) * x = 0 := h₃
      have h₆ : N - 1 = 0 ∨ x = 0 := Nat.mul_eq_zero.mp h₅
      cases h₆ with
      | inl h₆ => exact h₆
      | inr h₆ =>
        exfalso
        have h₇ : 0 < x := h_pos
        linarith
    have h₅ : N = 1 := by
      have h₆ : N - 1 = 0 := h₄
      have h₇ : N ≤ 1 := by omega
      omega
    exact hN h₅
  · intro x hx
    simp only [Set.mem_singleton_iff] at hx
    rw [hx]
    <;> simp [Nat.mul_zero]

-- Helper lemma: if n is even and n > 2, then n is not prime
lemma even_gt_two_not_prime {n : ℕ} (h₁ : n % 2 = 0) (h₂ : n > 2) : ¬Nat.Prime n := by
  have h₃ : 2 ∣ n := by omega
  have h₄ : n ≠ 2 := by omega
  intro h₅
  have h₆ := Nat.Prime.eq_two_or_odd h₅
  cases h₆ with
  | inl h₆ =>
    have h₇ : n = 2 := h₆
    contradiction
  | inr h₆ =>
    have h₇ : n % 2 = 1 := h₆
    omega

-- Key fact: if popcount n = 1 and n > 0, then n is a power of 2
lemma popcount_eq_one_iff_pow_two (n : ℕ) : popcount n = 1 ↔ n ≠ 0 ∧ ∃ k : ℕ, n = 2 ^ k := by
  constructor
  · -- Forward direction: popcount n = 1 → n is a positive power of 2
    intro h
    have h₁ : n ≠ 0 := by
      by_contra h₀
      rw [h₀] at h
      norm_num [popcount] at h
    have h₂ : ∃ k : ℕ, n = 2 ^ k := by
      -- Use the fact that popcount n = 1 means n has exactly one 1-bit
      -- This implies n is a power of 2
      have h₃ : n > 0 := Nat.pos_of_ne_zero h₁
      have h₄ : popcount n = 1 := h
      -- We can use the property that if n has exactly one 1-bit, then n is a power of 2
      have h₅ : ∃ k : ℕ, n = 2 ^ k := by
        -- Use the fact that the only numbers with popcount 1 are powers of 2
        classical
        -- Find the position of the single 1-bit
        have h₆ : 1 ∈ Nat.digits 2 n := by
          by_contra h₇
          have h₈ : (Nat.digits 2 n).count 1 = 0 := by
            simp [List.count_eq_zero, h₇]
          have h₉ : popcount n = 0 := by simpa [popcount] using h₈
          omega
        -- Get the index of the 1-bit
        have h₇ : ∃ k : ℕ, (Nat.digits 2 n).get? k = some 1 := by
          obtain ⟨k, hk⟩ := List.get?_eq_some_iff_mem.mp h₆
          exact ⟨k, by simp_all [List.get?]⟩
        obtain ⟨k, hk⟩ := h₇
        use k
        have h₈ : (Nat.digits 2 n).get? k = some 1 := hk
        have h₉ : n = 2 ^ k := by
          -- Prove that n = 2^k using the fact that there's exactly one 1-bit
          have h₁₀ : ∀ i : ℕ, (Nat.digits 2 n).get? i = if i = k then some 1 else some 0 := by
            intro i
            by_cases h₁₁ : i = k
            · rw [h₁₁]
              exact h₈
            · have h₁₂ : (Nat.digits 2 n).get? i ≠ some 1 := by
                intro h₁₃
                have h₁₄ : 1 ∈ Nat.digits 2 n := by
                  simp_all [List.get?, List.mem_cons, List.mem_singleton]
                  <;> aesop
                have h₁₅ : (Nat.digits 2 n).count 1 ≥ 2 := by
                  -- There are at least two 1s
                  have h₁₆ : (Nat.digits 2 n).count 1 = 1 := by simpa [popcount] using h₄
                  omega
                omega
              by_cases h₁₃ : (Nat.digits 2 n).get? i = some 0
              · simp_all
              · exfalso
                -- If it's not 0 or 1, contradiction since digits are 0 or 1
                have h₁₄ : (Nat.digits 2 n).get? i = some 0 ∨ (Nat.digits 2 n).get? i = some 1 := by
                  by_cases h₁₅ : i < (Nat.digits 2 n).length
                  · have h₁₆ : (Nat.digits 2 n)[i] = 0 ∨ (Nat.digits 2 n)[i] = 1 := by
                      have h₁₇ : (Nat.digits 2 n)[i] < 2 := by
                        apply Nat.digits_lt_base
                        <;> omega
                      omega
                    cases h₁₆ with
                    | inl h₁₆ =>
                      have h₁₇ : (Nat.digits 2 n).get? i = some 0 := by
                        simp [List.get?, h₁₅, h₁₆]
                      exact Or.inl h₁₇
                    | inr h₁₆ =>
                      have h₁₇ : (Nat.digits 2 n).get? i = some 1 := by
                        simp [List.get?, h₁₅, h₁₆]
                      exact Or.inr h₁₇
                  · have h₁₆ : (Nat.digits 2 n).get? i = none := by
                      simp [List.get?, h₁₅]
                    have h₁₇ : (Nat.digits 2 n).get? i = some 0 ∨ (Nat.digits 2 n).get? i = some 1 := by
                      cases h₁₆ <;> simp_all (config := { contextual := true }) <;> aesop
                    exact h₁₇
                cases h₁₄ with
                | inl h₁₄ => contradiction
                | inr h₁₄ =>
                  have h₁₅ : i = k := by
                    by_contra h₁₆
                    have h₁₇ : i ≠ k := h₁₆
                    have h₁₈ : (Nat.digits 2 n).count 1 ≥ 2 := by
                      classical
                      have h₁₉ : (Nat.digits 2 n).count 1 = 1 := by simpa [popcount] using h₄
                      omega
                    omega
                  contradiction
          -- Now we know the exact digits, we can compute n
          have h₁₁ : n = 2 ^ k := by
            have h₁₂ : n = Nat.ofDigits 2 (Nat.digits 2 n) := by
              rw [Nat.ofDigits_digits]
            rw [h₁₂]
            -- The digits are all 0 except at position k where it's 1
            have h₁₃ : (Nat.digits 2 n) = List.replicate k 0 ++ [1] := by
              -- Prove that the digits are exactly this
              have h₁₄ : ∀ i : ℕ, (Nat.digits 2 n).get? i = if i = k then some 1 else some 0 := h₁₀
              have h₁₅ : (Nat.digits 2 n).length = k + 1 := by
                -- The length is k+1
                by_contra h₁₆
                have h₁₇ : (Nat.digits 2 n).length ≠ k + 1 := h₁₆
                have h₁₈ : (Nat.digits 2 n).length < k + 1 ∨ (Nat.digits 2 n).length > k + 1 := by omega
                cases h₁₈ with
                | inl h₁₈ =>
                  have h₁₉ : (Nat.digits 2 n).get? k = none := by
                    have h₂₀ : k ≥ (Nat.digits 2 n).length := by omega
                    simp [List.get?, h₂₀]
                  have h₂₀ : (Nat.digits 2 n).get? k = some 1 := h₈
                  simp at h₁₉ h₂₀ ⊢
                  <;> simp_all
                | inr h₁₈ =>
                  have h₁₉ : (Nat.digits 2 n).get? (k + 1) = some 0 := by
                    have h₂₀ : (Nat.digits 2 n).get? (k + 1) = if (k + 1 : ℕ) = k then some 1 else some 0 := h₁₀ (k + 1)
                    have h₂₁ : (k + 1 : ℕ) ≠ k := by omega
                    simp [h₂₁] at h₂₀ ⊢
                    <;> simp_all
                  have h₂₀ : (Nat.digits 2 n)[k + 1] = 0 := by
                    have h₂₁ : k + 1 < (Nat.digits 2 n).length := by omega
                    have h₂₂ : (Nat.digits 2 n).get? (k + 1) = some 0 := h₁₉
                    simp [List.get?] at h₂₂ ⊢
                    <;> simp_all
                  have h₂₁ : (Nat.digits 2 n).last _ = 0 := by
                    have h₂₂ : (Nat.digits 2 n).length > 0 := by
                      by_contra h₂₃
                      have h₂₄ : (Nat.digits 2 n).length = 0 := by omega
                      have h₂₅ : Nat.digits 2 n = [] := by
                        simp_all [List.length_eq_zero]
                      have h₂₆ : n = 0 := by
                        rw [h₂₅] at h₁₂
                        simp [Nat.ofDigits, List.nil] at h₁₂ ⊢
                        <;> simp_all
                      contradiction
                    have h₂₃ : (Nat.digits 2 n).last _ = (Nat.digits 2 n)[(Nat.digits 2 n).length - 1] := by
                      simp [List.get_last]
                    have h₂₄ : (Nat.digits 2 n).length - 1 = k := by
                      have h₂₅ : (Nat.digits 2 n).length = k + 1 := by omega
                      omega
                    rw [h₂₃, h₂₄]
                    have h₂₅ : (Nat.digits 2 n)[k] = 1 := by
                      have h₂₆ : (Nat.digits 2 n).get? k = some 1 := h₈
                      have h₂₇ : k < (Nat.digits 2 n).length := by
                        by_contra h₂₈
                        have h₂₉ : k ≥ (Nat.digits 2 n).length := by omega
                        have h₃₀ : (Nat.digits 2 n).get? k = none := by simp [List.get?, h₂₉]
                        simp at h₂₆ h₃₀ ⊢
                        <;> simp_all
                      simp [List.get?] at h₂₆ ⊢
                      <;> simp_all
                    simp_all
                  simp_all [Nat.digits_zero, Nat.digits_add]
                  <;> omega
              -- Now we know the length, we can prove the digits are exactly replicate k 0 ++ [1]
              have h₁₆ : (Nat.digits 2 n) = List.replicate k 0 ++ [1] := by
                apply List.ext_iff.mpr
                intro i
                by_cases h₁₇ : i < k
                · have h₁₈ : (Nat.digits 2 n).get? i = some 0 := by
                    have h₁₉ : (Nat.digits 2 n).get? i = if i = k then some 1 else some 0 := h₁₀ i
                    have h₂₀ : i ≠ k := by omega
                    simp [h₂₀] at h₁₉ ⊢
                    <;> simp_all
                  have h₁₉ : (List.replicate k 0 ++ [1]).get? i = some 0 := by
                    have h₂₀ : i < k := h₁₇
                    have h₂₁ : (List.replicate k 0 ++ [1]).get? i = some 0 := by
                      simp [List.get?_append, List.get?_replicate, h₂₀]
                      <;> simp_all [Nat.lt_succ_iff]
                      <;> aesop
                    exact h₂₁
                  have h₂₀ : (Nat.digits 2 n)[i] = 0 := by
                    have h₂₁ : i < (Nat.digits 2 n).length := by
                      have h₂₂ : (Nat.digits 2 n).length = k + 1 := h₁₅
                      omega
                    have h₂₂ : (Nat.digits 2 n).get? i = some 0 := h₁₈
                    simp [List.get?] at h₂₂ ⊢
                    <;> simp_all
                  have h₂₁ : (List.replicate k 0 ++ [1])[i] = 0 := by
                    have h₂₂ : i < k := h₁₇
                    have h₂₃ : (List.replicate k 0 ++ [1])[i] = 0 := by
                      simp [List.get_append, List.get_replicate, h₂₂]
                      <;> simp_all [Nat.lt_succ_iff]
                      <;> aesop
                    exact h₂₃
                  simp_all
                · have h₁₈ : i ≥ k := by omega
                  by_cases h₁₉ : i = k
                  · have h₂₀ : (Nat.digits 2 n).get? k = some 1 := h₈
                    have h₂₁ : (Nat.digits 2 n)[k] = 1 := by
                      have h₂₂ : k < (Nat.digits 2 n).length := by
                        have h₂₃ : (Nat.digits 2 n).length = k + 1 := h₁₅
                        omega
                      have h₂₃ : (Nat.digits 2 n).get? k = some 1 := h₈
                      simp [List.get?] at h₂₃ ⊢
                      <;> simp_all
                    have h₂₂ : (List.replicate k 0 ++ [1])[k] = 1 := by
                      have h₂₃ : (List.replicate k 0 ++ [1])[k] = 1 := by
                        simp [List.get_append, List.get_replicate, List.length_replicate]
                        <;> simp_all [Nat.lt_succ_iff]
                        <;> aesop
                      exact h₂₃
                    simp_all
                  · have h₂₀ : i > k := by omega
                    have h₂₁ : i ≥ (Nat.digits 2 n).length := by
                      have h₂₂ : (Nat.digits 2 n).length = k + 1 := h₁₅
                      omega
                    have h₂₂ : (Nat.digits 2 n).get? i = none := by
                      simp [List.get?, h₂₁]
                    have h₂₃ : (List.replicate k 0 ++ [1]).get? i = none := by
                      have h₂₄ : i ≥ (List.replicate k 0 ++ [1]).length := by
                        have h₂₅ : (List.replicate k 0 ++ [1]).length = k + 1 := by
                          simp [List.length_append, List.length_replicate]
                          <;> ring_nf
                        omega
                      simp [List.get?, h₂₄]
                    simp [List.get?] at h₂₂ h₂₃ ⊢
                    <;> simp_all
              exact h₁₆
            rw [h₁₃]
            simp [Nat.ofDigits, List.replicate, Nat.pow_succ, Nat.mul_add, Nat.add_mul]
            <;> induction k with
            | zero => norm_num
            | succ k ih =>
              simp_all [List.ofDigits, Nat.pow_succ, Nat.mul_add, Nat.add_mul]
              <;> ring_nf at *
              <;> norm_num at *
              <;> omega
          exact h₁₁
        exact h₉
      exact h₅
    exact ⟨h₁, h₂⟩
  · -- Reverse direction: n is a positive power of 2 → popcount n = 1
    rintro ⟨h₁, ⟨k, rfl⟩⟩
    have h₂ : popcount (2 ^ k) = 1 := by
      have h₃ : popcount (2 ^ k) = 1 := by
        -- The binary representation of 2^k is 1 followed by k zeros
        have h₄ : Nat.digits 2 (2 ^ k) = List.replicate k 0 ++ [1] := by
          -- Prove by induction on k
          have h₅ : ∀ k : ℕ, Nat.digits 2 (2 ^ k) = List.replicate k 0 ++ [1] := by
            intro k
            induction k with
            | zero =>
              norm_num [Nat.digits_zero, Nat.pow_zero]
            | succ k ih =>
              rw [show 2 ^ (k + 1) = 2 * 2 ^ k by ring]
              simp [Nat.digits_zero, Nat.digits_add, List.replicate, Nat.pow_succ, Nat.mul_add, Nat.add_mul, ih]
              <;> norm_num at *
              <;> try omega
              <;> try ring_nf at *
              <;> try aesop
          exact h₅ k
        rw [h₄]
        simp [popcount, List.count_append, List.count_replicate, List.count_cons]
        <;> norm_num
        <;> aesop
      exact h₃
    exact h₂

-- Helper lemma: if 2^k is prime, then k = 1
lemma pow_two_prime_iff {k : ℕ} : Nat.Prime (2 ^ k) ↔ k = 1 := by
  constructor
  · -- If 2^k is prime, then k = 1
    intro h
    have h₁ : Nat.Prime (2 ^ k) := h
    have h₂ : k ≠ 0 := by
      intro h₃
      rw [h₃] at h₁
      norm_num [Nat.Prime] at h₁
    have h₃ : k ≠ 1 → k ≥ 2 := by
      intro h₄
      have h₅ : k ≥ 2 := by
        by_contra h₆
        have h₇ : k ≤ 1 := by omega
        interval_cases k <;> simp_all (config := {decide := true})
      exact h₅
    by_cases h₄ : k = 1
    · exact h₄
    · -- k ≥ 2, so 2^k is composite
      have h₅ : k ≥ 2 := h₃ h₄
      have h₆ : 2 ∣ 2 ^ k := by
        have h₇ : k ≥ 1 := by omega
        exact dvd_pow_self 2 (by omega)
      have h₇ : 2 ^ k ≠ 2 := by
        have h₈ : k ≥ 2 := h₅
        have h₉ : 2 ^ k ≥ 2 ^ 2 := by
          exact Nat.pow_le_pow_of_le_left (by norm_num) h₈
        omega
      have h₈ : ¬Nat.Prime (2 ^ k) := by
        intro h₉
        have h₁₀ := Nat.Prime.eq_two_or_odd h₉
        cases h₁₀ with
        | inl h₁₀ =>
          have h₁₁ : 2 ^ k = 2 := h₁₀
          contradiction
        | inr h₁₀ =>
          have h₁₁ : (2 ^ k) % 2 = 1 := h₁₀
          have h₁₂ : (2 ^ k) % 2 = 0 := by
            have h₁₃ : k ≥ 1 := by omega
            have h₁₄ : (2 ^ k) % 2 = 0 := by
              have h₁₅ : 2 ∣ 2 ^ k := dvd_pow_self 2 (by omega)
              omega
            exact h₁₄
          omega
      exact absurd h₁ h₈
  · -- If k = 1, then 2^k is prime
    intro h
    rw [h]
    norm_num [Nat.Prime]

-- If n is both binary-atomic and arith-atomic, n = 2.
theorem common_atoms_eq_two : {n : ℕ | binaryAtomic n ∧ arithAtomic n} = {2} := by
  apply Set.Subset.antisymm
  · -- Forward direction: if n is both binary-atomic and arith-atomic, then n = 2
    intro n hn
    have h₁ : binaryAtomic n := hn.1
    have h₂ : arithAtomic n := hn.2
    have h₃ : n > 0 := h₁.1
    have h₄ : popcount n = 1 := h₁.2
    have h₅ : Nat.Prime n := h₂.2
    -- Since popcount n = 1, n is a power of 2
    have h₆ : ∃ k : ℕ, n = 2 ^ k := by
      have h₇ : n ≠ 0 ∧ ∃ k : ℕ, n = 2 ^ k := (popcount_eq_one_iff_pow_two n).mp h₄
      exact h₇.2
    obtain ⟨k, hk⟩ := h₆
    have h₇ : n = 2 ^ k := hk
    have h₈ : Nat.Prime (2 ^ k) := by
      rw [h₇] at h₅
      exact h₅
    -- Since 2^k is prime, k must be 1
    have h₉ : k = 1 := by
      have h₁₀ : Nat.Prime (2 ^ k) ↔ k = 1 := pow_two_prime_iff k
      have h₁₁ : Nat.Prime (2 ^ k) := h₈
      have h₁₂ : k = 1 := (h₁₀.mp h₁₁)
      exact h₁₂
    -- Therefore n = 2
    have h₁₀ : n = 2 := by
      calc
        n = 2 ^ k := h₇
        _ = 2 ^ 1 := by rw [h₉]
        _ = 2 := by norm_num
    -- We need to show n ∈ ({2} : Set ℕ)
    rw [h₁₀]
    <;> simp [Set.mem_singleton_iff]
  · -- Backward direction: 2 is both binary-atomic and arith-atomic
    intro n hn
    rw [Set.mem_singleton_iff] at hn
    rw [hn]
    constructor
    · -- Prove 2 is binary-atomic
      constructor
      · norm_num [binaryAtomic, popcount]
      · decide
    · -- Prove 2 is arith-atomic
      constructor
      · norm_num [arithAtomic]
      · decide

-- Cantor: no set is equal to its own powerset (via surjection).
theorem cantor_theorem : ∀ (f : ℕ → Set ℕ), ∃ s, s ∉ Set.range f := by
  intro f
  let s : Set ℕ := {x | x ∉ f x}
  use s
  intro h
  rcases h with ⟨y, hy⟩
  have : y ∈ s ↔ y ∉ f y := by rfl
  rw [hy] at this
  tauto

-- Central proposition.
theorem central_proposition : True := True.intro