"""
Mathematical Logic Discipline Register
Maps CLINK primitives to concepts in Mathematical Logic
"""

from clink_decomposition.registers.base import DisciplineRegister
from clink_decomposition.core.models import PrimitiveMapping


class LogicRegister(DisciplineRegister):
    """Mathematical Logic - The study of formal systems and their properties"""
    
    def get_name(self):
        return "mathematical_logic"
    
    def get_description(self):
        return "Mathematical Logic - The study of formal systems, logical connectives, and their properties"
    
    def to_lean_imports(self):
        return [
            "Mathlib.Logic.Basic",
            "Mathlib.Logic.Propositional",
        ]
    
    def to_lean_definitions(self):
        return """
-- Mathematical Logic definitions for CLINK decomposition

-- Proposition
variable (p q r : Prop)

-- Logical connectives
def And (p q : Prop) : Prop := p ∧ q
def Or (p q : Prop) : Prop := p ∨ q
def Not (p : Prop) : Prop := ¬ p
def Implies (p q : Prop) : Prop := p → q
def Iff (p q : Prop) : Prop := p ↔ q

-- Quantifiers
def Forall {α : Sort*} (P : α → Prop) : Prop := ∀ x, P x
def Exists {α : Sort*} (P : α → Prop) : Prop := ∃ x, P x

-- Satisfaction and validity
def Satisfies {L : Type*} (M : Type*) (φ : L → Prop) (a : L) : Prop := φ a
def Valid {L : Type*} (φ : L → Prop) : Prop := ∀ a, φ a
"""
    
    def map_primitive(self, primitive_idx, context=None):
        """Map CLINK primitives to Mathematical Logic concepts"""
        context = context or {}
        
        mappings = {
            0: PrimitiveMapping(
                primitive_idx=0,
                primitive_symbol="⊢",
                primitive_name="Dimensionality",
                conventional_math="φ ∈ L",
                lean_expression="φ ∈ L",
                description="Formula φ belongs to language L",
                promoted_atom=None
            ),
            1: PrimitiveMapping(
                primitive_idx=1,
                primitive_symbol="⊣",
                primitive_name="Topology",
                conventional_math="⊨ φ",
                lean_expression="⊨ φ",
                description="φ is satisfiable (there exists a model where φ holds)",
                promoted_atom="HOLOBOUND"
            ),
            2: PrimitiveMapping(
                primitive_idx=2,
                primitive_symbol="≻",
                primitive_name="Coupling",
                conventional_math="φ → ψ",
                lean_expression="φ → ψ",
                description="Implication: φ implies ψ",
                promoted_atom="LR_DUAL"
            ),
            3: PrimitiveMapping(
                primitive_idx=3,
                primitive_symbol="≺",
                primitive_name="Parity",
                conventional_math="¬(φ ↔ ψ)",
                lean_expression="¬(φ ↔ ψ)",
                description="φ and ψ are not equivalent",
                promoted_atom=None
            ),
            4: PrimitiveMapping(
                primitive_idx=4,
                primitive_symbol="⋈",
                primitive_name="Fidelity",
                conventional_math="φ ∨ ¬φ",
                lean_expression="φ ∨ ¬φ",
                description="Law of excluded middle (classical logic)",
                promoted_atom="PM_Z2"
            ),
            5: PrimitiveMapping(
                primitive_idx=5,
                primitive_symbol="⊤",
                primitive_name="Kinetics",
                conventional_math="⊢ φ",
                lean_expression="⊢ φ",
                description="φ is provable (derivable from axioms)",
                promoted_atom=None
            ),
            6: PrimitiveMapping(
                primitive_idx=6,
                primitive_symbol="∈",
                primitive_name="Cardinality",
                conventional_math="∀x ∈ M. ⊨ φ[x]",
                lean_expression="∀ x ∈ M, ⊨ φ[x]",
                description="For all x in model M, φ holds",
                promoted_atom="HOLOGRAPHIC_STATE"
            ),
            7: PrimitiveMapping(
                primitive_idx=7,
                primitive_symbol="∋",
                primitive_name="Composition",
                conventional_math="M ⊨ φ ∧ M ⊨ ψ",
                lean_expression="M ⊨ φ ∧ M ⊨ ψ",
                description="Model M satisfies both φ and ψ",
                promoted_atom="BROADCAST_TRANSCENDENCE"
            ),
            8: PrimitiveMapping(
                primitive_idx=8,
                primitive_symbol="⊙",
                primitive_name="Criticality",
                conventional_math="(φ → ψ) → ((ψ → φ) → (φ ↔ ψ))",
                lean_expression="(φ → ψ) → ((ψ → φ) → (φ ↔ ψ))",
                description="Implication introduces equivalence (Frobenius-like property)",
                promoted_atom="PHI_C"
            ),
            9: PrimitiveMapping(
                primitive_idx=9,
                primitive_symbol="⊥",
                primitive_name="Chirality",
                conventional_math="∃φ. ¬⊨ φ ∧ ¬⊨ ¬φ",
                lean_expression="∃ φ, ¬⊨ φ ∧ ¬⊨ ¬φ",
                description="There exists an independent formula (neither provable nor disprovable)",
                promoted_atom="ETERNAL_FIXEDPOINT"
            ),
            10: PrimitiveMapping(
                primitive_idx=10,
                primitive_symbol="⊞",
                primitive_name="Stoichiometry",
                conventional_math="∀φ ∀ψ. type(φ) = type(ψ)",
                lean_expression="∀ φ ψ, type φ = type ψ",
                description="All formulas have the same type (Proposition)",
                promoted_atom=None
            ),
            11: PrimitiveMapping(
                primitive_idx=11,
                primitive_symbol="⊡",
                primitive_name="Winding",
                conventional_math="WellFounded(⊨)",
                lean_expression="WellFounded (· ⊨ ·)",
                description="Satisfaction relation is well-founded",
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
        """Compose mappings into a single mathematical logic expression"""
        conventional_parts = [m.conventional_math for m in mappings]
        lean_parts = [m.lean_expression for m in mappings]
        
        # Use logical AND to compose
        composed_conventional = " \\land ".join(conventional_parts)
        composed_lean = " ∧ ".join(lean_parts)
        
        return PrimitiveMapping(
            primitive_idx=-1,
            primitive_symbol="COMPOSED",
            primitive_name="Composed Expression",
            conventional_math=composed_conventional,
            lean_expression=composed_lean,
            description="Composed mathematical logic expression",
            promoted_atom=None
        )
