"""
Model Theory Discipline Register
Maps CLINK primitives to concepts in Model Theory
"""

from clink_decomposition.registers.base import DisciplineRegister
from clink_decomposition.core.models import PrimitiveMapping


class ModelTheoryRegister(DisciplineRegister):
    """Model Theory - The study of mathematical structures and their relationship to formal languages"""
    
    def get_name(self):
        return "model_theory"
    
    def get_description(self):
        return "Model Theory - The study of mathematical structures (models) and their relationship to formal languages"
    
    def to_lean_imports(self):
        return [
            "Mathlib.ModelTheory.Basic",
            "Mathlib.Logic.Basic",
        ]
    
    def to_lean_definitions(self):
        return """
-- Model Theory definitions for CLINK decomposition

-- Language of a structure
structure Language where
  symbols : Set String
  arities : String → ℕ

-- Structure (model)
structure Structure (L : Language) where
  domain : Type*
  interpretation : String → (domain → ... → domain)

-- Satisfaction relation
variable {L : Language} {M : Structure L}
variable (φ : Formula L) (a : Assignment M)

-- Assignment
def Assignment (M : Structure L) : Type* := M.domain → Prop

-- Formula satisfaction
def Satisfies (M : Structure L) (φ : Formula L) : Prop := True

-- Elementarity (embedding)
def ElementaryEmbedding {M N : Structure L} (f : M.domain → N.domain) : Prop := True

-- Model completeness
def ModelComplete (T : Theory L) : Prop := True
"""
    
    def map_primitive(self, primitive_idx, context=None):
        """Map CLINK primitives to Model Theory concepts"""
        context = context or {}
        
        mappings = {
            0: PrimitiveMapping(
                primitive_idx=0,
                primitive_symbol="⊢",
                primitive_name="Dimensionality",
                conventional_math="|M| = κ",
                lean_expression="|M.domain| = κ",
                description="Model M has cardinality κ",
                promoted_atom=None
            ),
            1: PrimitiveMapping(
                primitive_idx=1,
                primitive_symbol="⊣",
                primitive_name="Topology",
                conventional_math="M ≺ N",
                lean_expression="M ≺ N",
                description="M is an elementary substructure of N",
                promoted_atom="HOLOBOUND"
            ),
            2: PrimitiveMapping(
                primitive_idx=2,
                primitive_symbol="≻",
                primitive_name="Coupling",
                conventional_math="M ≡ N",
                lean_expression="M ≡ N",
                description="M and N are elementarily equivalent",
                promoted_atom="LR_DUAL"
            ),
            3: PrimitiveMapping(
                primitive_idx=3,
                primitive_symbol="≺",
                primitive_name="Parity",
                conventional_math="¬(M ≅ N)",
                lean_expression="¬(M ≅ N)",
                description="M and N are not isomorphic",
                promoted_atom=None
            ),
            4: PrimitiveMapping(
                primitive_idx=4,
                primitive_symbol="⋈",
                primitive_name="Fidelity",
                conventional_math="M ⊨ φ ∨ M ⊨ ¬φ",
                lean_expression="M ⊨ φ ∨ M ⊨ ¬φ",
                description="Model M decides φ (classical completeness)",
                promoted_atom="PM_Z2"
            ),
            5: PrimitiveMapping(
                primitive_idx=5,
                primitive_symbol="⊤",
                primitive_name="Kinetics",
                conventional_math="Th(M) ≫ T",
                lean_expression="Th M ⊇ T",
                description="Theory of M extends T",
                promoted_atom=None
            ),
            6: PrimitiveMapping(
                primitive_idx=6,
                primitive_symbol="∈",
                primitive_name="Cardinality",
                conventional_math="∀a ∈ M. φ(a)",
                lean_expression="∀ a ∈ M.domain, φ a",
                description="φ holds for all elements of M",
                promoted_atom="HOLOGRAPHIC_STATE"
            ),
            7: PrimitiveMapping(
                primitive_idx=7,
                primitive_symbol="∋",
                primitive_name="Composition",
                conventional_math="∪ᵢ Mᵢ = M",
                lean_expression="⋃ i, Mᵢ = M",
                description="M is the union of elementary chain",
                promoted_atom="BROADCAST_TRANSCENDENCE"
            ),
            8: PrimitiveMapping(
                primitive_idx=8,
                primitive_symbol="⊙",
                primitive_name="Criticality",
                conventional_math="Th(M) = Th(M) ∧ μ∘δ = id",
                lean_expression="Th M = Th M ∧ μ ∘ δ = id",
                description="Theory of M is Frobenius-closed",
                promoted_atom="PHI_C"
            ),
            9: PrimitiveMapping(
                primitive_idx=9,
                primitive_symbol="⊥",
                primitive_name="Chirality",
                conventional_math="∃f: M→M. f is automorphism",
                lean_expression="∃ f : M.domain → M.domain, IsAutomorphism f",
                description="M has a nontrivial automorphism",
                promoted_atom="ETERNAL_FIXEDPOINT"
            ),
            10: PrimitiveMapping(
                primitive_idx=10,
                primitive_symbol="⊞",
                primitive_name="Stoichiometry",
                conventional_math="∀M ∈ K. L(M) = L",
                lean_expression="∀ M ∈ K, L M = L",
                description="All models in class K have the same language",
                promoted_atom=None
            ),
            11: PrimitiveMapping(
                primitive_idx=11,
                primitive_symbol="⊡",
                primitive_name="Winding",
                conventional_math="K is stable",
                lean_expression="IsStable K",
                description="Class K is stable (no order property)",
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
        """Compose mappings into a single model theory expression"""
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
            description="Composed model theory expression",
            promoted_atom=None
        )
