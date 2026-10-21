import Mathlib.Data.ZMod.Basic
import Mathlib.Order.SetNotation
import Mathlib.Tactic.Ring

/-! Source-bound point projection. No CRT legs or nontrivial idempotent
witness are parameters of the operator. Support is carried as one set. -/
namespace ConventionalPointProjection

/-- Combine the diagonal and linear bit terms before adding the whole
    correlated upper suffix. This is the row executed by modular_affine_add;
    it does not assert that the point projection has been computed. -/
theorem fused_bit_row (N : Nat) (place bit suffix : ZMod N)
    (hbit : bit * bit = bit) :
    (place * bit) * (place * bit) - place * bit +
        2 * (place * bit) * (2 * place * suffix) =
      bit * (place * place - place + 4 * place * place * suffix) := by
  calc
    _ = (place * place) * (bit * bit) - place * bit +
        4 * place * place * bit * suffix := by ring
    _ = _ := by rw [hbit]; ring

def pointF (N : Nat) (support : Set (ZMod N)) : Set (ZMod N) :=
  {e | e ∈ support ∧ e * e - e = 0 ∧ e ≠ 0 ∧ e ≠ 1}

theorem pointF_monotone (N : Nat) : Monotone (pointF N) := by
  intro a b hab e he
  exact ⟨hab he.1, he.2⟩

theorem pointF_contracts (N : Nat) (support : Set (ZMod N)) :
    pointF N support ⊆ support := fun _ h => h.1

theorem pointF_idempotent (N : Nat) (support : Set (ZMod N)) :
    pointF N (pointF N support) = pointF N support := by
  ext e
  constructor
  · exact fun h => h.1
  · exact fun h => ⟨h, h.2⟩

theorem pointF_preserves_every_solution (N : Nat) (support : Set (ZMod N))
    (e : ZMod N) (hs : e ∈ support) (hi : e * e = e)
    (h0 : e ≠ 0) (h1 : e ≠ 1) : e ∈ pointF N support :=
  ⟨hs, sub_eq_zero.mpr hi, h0, h1⟩

theorem pointF_univ (N : Nat) :
    pointF N Set.univ = {e : ZMod N | e * e = e ∧ e ≠ 0 ∧ e ≠ 1} := by
  ext e
  simp only [pointF, Set.mem_setOf_eq, Set.mem_univ, true_and, sub_eq_zero]

theorem pointF_greatest_fixed (N : Nat) (support : Set (ZMod N))
    (h : pointF N support = support) : support ⊆ pointF N Set.univ := by
  intro e he
  rw [← h] at he
  exact ⟨Set.mem_univ e, he.2⟩

#print axioms pointF_monotone
#print axioms pointF_idempotent
#print axioms pointF_preserves_every_solution
#print axioms pointF_univ
#print axioms fused_bit_row
end ConventionalPointProjection
