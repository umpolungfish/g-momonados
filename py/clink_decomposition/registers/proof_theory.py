"""
Proof Theory Discipline Register
Maps CLINK primitives to concepts in Proof Theory
"""

from clink_decomposition.registers.base import DisciplineRegister
from clink_decomposition.core.models import PrimitiveMapping


class ProofTheoryRegister(DisciplineRegister):
    """Proof Theory - The study of formal proofs and proof systems"""
    
    def get_name(self):
        return "proof_theory"
    
    def get_description(self):
        return "Proof Theory - The study of formal proofs, derivations, and proof systems"
    
    def to_lean_imports(self):
        return [
            "Mathlib.Logic.Basic",
            "Mathlib.Tactic.Basic",
        ]
    
    def to_lean_definitions(self):
        return """
-- Proof Theory definitions for CLINK decomposition

-- Proof system
structure ProofSystem where
  axioms : Set Prop
  rules : Set (Prop → Prop)

-- Derivation (proof tree)
inductive Derivation : Prop → Type
  | axiom : (h : p ∈ axioms) → Derivation p
  | rule : (r : p → q) → (d : Derivation p) → Derivation q

-- Proof length
def ProofLength {p : Prop} (d : Derivation p) : ℕ
  | Derivation.axiom _ => 1
  | Derivation.rule _ d' => 1 + ProofLength d'

-- Provability
def Provable (T : ProofSystem) (p : Prop) : Prop := ∃ d : Derivation p, True

-- Consistency
def Consistent (T : ProofSystem) : Prop := ∀ p, ¬(Provable T p ∧ Provable T ¬p)

-- Completeness
def Complete (T : ProofSystem) (Σ : Set Prop) : Prop := 
  ∀ p ∈ Σ, (⊨ p → Provable T p)
"""
    
    def map_primitive(self, primitive_idx, context=None):
        """Map CLINK primitives to Proof Theory concepts"""
        context = context or {}
        
        mappings = {
            0: PrimitiveMapping(
                primitive_idx=0,
                primitive_symbol="⊢",
                primitive_name="Dimensionality",
                conventional_math="|Γ| = ω",
                lean_expression="|Γ| = ℵ₀",
                description="Context Γ has countably infinite size",
                promoted_atom=None
            ),
            1: PrimitiveMapping(
                primitive_idx=1,
                primitive_symbol="⊣",
                primitive_name="Topology",
                conventional_math="Γ ⊢ Δ",
                lean_expression="Γ ⊢ Δ",
                description="Sequent: from Γ derive Δ",
                promoted_atom="HOLOBOUND"
            ),
            2: PrimitiveMapping(
                primitive_idx=2,
                primitive_symbol="≻",
                primitive_name="Coupling",
                conventional_math="Γ ⊢ A → Γ, A ⊢ B",
                lean_expression="Γ ⊢ A → (Γ ∪ {A}) ⊢ B",
                description="Implication introduction rule",
                promoted_atom="LR_DUAL"
            ),
            3: PrimitiveMapping(
                primitive_idx=3,
                primitive_symbol="≺",
                primitive_name="Parity",
                conventional_math="¬(Γ ⊢ ⊥)",
                lean_expression="¬(Γ ⊢ False)",
                description="Γ is consistent (doesn't derive contradiction)",
                promoted_atom=None
            ),
            4: PrimitiveMapping(
                primitive_idx=4,
                primitive_symbol="⋈",
                primitive_name="Fidelity",
                conventional_math="Γ ⊢ A ∨ ¬A",
                lean_expression="Γ ⊢ A ∨ ¬A",
                description="Γ is classical (decides all propositions)",
                promoted_atom="PM_Z2"
            ),
            5: PrimitiveMapping(
                primitive_idx=5,
                primitive_symbol="⊤",
                primitive_name="Kinetics",
                conventional_math="⊢ Γ",
                lean_expression="⊢ Γ",
                description="Γ is derivable (all formulas in Γ are provable)",
                promoted_atom=None
            ),
            6: PrimitiveMapping(
                primitive_idx=6,
                primitive_symbol="∈",
                primitive_name="Cardinality",
                conventional_math="∀A ∈ Γ. |A| < |Γ|",
                lean_expression="∀ A ∈ Γ, |A| < |Γ|",
                description="Each formula in Γ is smaller than Γ itself",
                promoted_atom="HOLOGRAPHIC_STATE"
            ),
            7: PrimitiveMapping(
                primitive_idx=7,
                primitive_symbol="∋",
                primitive_name="Composition",
                conventional_math="Γ ⊢ A ∧ Δ ⊢ B",
                lean_expression="Γ ⊢ A ∧ Δ ⊢ B",
                description="Multiple sequents hold simultaneously",
                promoted_atom="BROADCAST_TRANSCENDENCE"
            ),
            8: PrimitiveMapping(
                primitive_idx=8,
                primitive_symbol="⊙",
                primitive_name="Criticality",
                conventional_math="(Γ ⊢ A) ⊢ (Γ ⊢ B) ∧ μ∘δ = id",
                lean_expression="(Γ ⊢ A) → (Γ ⊢ B) ∧ μ ∘ δ = id",
                description="Proof composition is Frobenius-closed",
                promoted_atom="PHI_C"
            ),
            9: PrimitiveMapping(
                primitive_idx=9,
                primitive_symbol="⊥",
                primitive_name="Chirality",
                conventional_math="∃σ. σ: Γ→Γ is involution",
                lean_expression="∃ σ : Γ → Γ, σ ∘ σ = id",
                description="There exists a proof substitution that is its own inverse",
                promoted_atom="ETERNAL_FIXEDPOINT"
            ),
            10: PrimitiveMapping(
                primitive_idx=10,
                primitive_symbol="⊞",
                primitive_name="Stoichiometry",
                conventional_math="∀A ∈ Γ. type(A) = Prop",
                lean_expression="∀ A ∈ Γ, type A = Prop",
                description="All formulas in Γ are propositions",
                promoted_atom=None
            ),
            11: PrimitiveMapping(
                primitive_idx=11,
                primitive_symbol="⊡",
                primitive_name="Winding",
                conventional_math="Proof tree is well-founded",
                lean_expression="WellFounded (ProofTree Γ)",
                description="All proof derivations are finite and well-founded",
                promoted_atom="ZWIND"
            ),
        }
        
        return mappings.get(primitive_idx, PrimitiveMapping(
            primitive_idx=primitive_idx,
            primitive_symbol="?",
            primitive_name="Unknown",
            conventional_math="",
            lean_expression="",
            description="",
            promoted_atom=None
        ))
    
    def compose_expression(self, mappings):
        """Compose mappings into a single proof theory expression"""
        conventional_parts = [m.conventional_math for m in mappings]
        lean_parts = [m.lean_expression for m in mappings]
        
        composed_conventional = " \\land ".join(conventional_parts)
        composed_lean = " ∧ ".join(lean_parts)
        
        return PrimitiveMapping(
            primitive_idx=-1,
            primitive_symbol="COMPOSED",
            primitive_name="Composed Expression",
            conventional_math=composed_conventional,
            lean_expression=composed_lean,
            description="Composed proof theory expression",
            promoted_atom=None
        )
