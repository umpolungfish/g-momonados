"""
Category Theory discipline register for CL8NK/CL9NK decomposition
"""

from typing import List, Optional, Dict
from .base import DisciplineRegister


# Lazy import to avoid circular dependencies
PRIMITIVE_SYMBOLS = ['⊢', '⊣', '≻', '≺', '⋈', '⊤', '∈', '∋', '⊙', '⊥', '⊞', '⊡']


def _get_primitive_mapping():
    """Lazy load PrimitiveMapping"""
    from clink_decomposition.core.models import PrimitiveMapping
    return PrimitiveMapping


def _get_mathematical_expression():
    """Lazy load MathematicalExpression"""
    from clink_decomposition.core.models import MathematicalExpression
    return MathematicalExpression


class CategoryTheoryRegister(DisciplineRegister):
    """
    Category Theory discipline register.
    
    Maps CLINK primitives to categorical expressions.
    """
    
    @classmethod
    def get_name(cls) -> str:
        return "category_theory"
    
    @classmethod
    def get_description(cls) -> str:
        return "Category theory - abstract algebra of functions and relations"
    
    def map_primitive(self, primitive_idx: int, context: Optional[Dict] = None):
        """Map a primitive to its categorical expression"""
        PrimitiveMapping = _get_primitive_mapping()
        
        if primitive_idx < 0 or primitive_idx >= 12:
            raise ValueError(f"Primitive index must be 0-11, got {primitive_idx}")
        
        symbol = PRIMITIVE_SYMBOLS[primitive_idx]
        
        # Define the mappings
        mappings = {
            # ⊢: Dimensionality - object in category
            0: {
                "conventional_math": r"x \in \mathcal{C} \land \dim(x) = n",
                "lean_expression": "x : C ∧ dim x = n",
                "description": "x is an object in category C with dimension n",
                "promoted_atom": None,
            },
            
            # ⊣: Topology - adjunction
            1: {
                "conventional_math": r"L \dashv R",
                "lean_expression": "L ⊣ R",
                "description": "L is left adjoint to R (branching topology)",
                "promoted_atom": "HOLOBOUND",
            },
            
            # ≻: Coupling - monad
            2: {
                "conventional_math": r"T: \mathcal{C} \to \mathcal{C} \land \mu: T^2 \to T \land \eta: I \to T",
                "lean_expression": "Monad T",
                "description": "T is a monad with unit η and multiplication μ (supervenience)",
                "promoted_atom": "LR_DUAL",
            },
            
            # ≺: Parity - duality
            3: {
                "conventional_math": r"C^{op}",
                "lean_expression": "Cᵒᵖ",
                "description": "Opposite category (no symmetry -> duality)",
                "promoted_atom": None,
            },
            
            # ⋈: Fidelity - epimorphism
            4: {
                "conventional_math": r"f: X \to Y \land \text{epi}(f)",
                "lean_expression": "Epi f",
                "description": "f is an epimorphism (classical/deterministic)",
                "promoted_atom": "PM_Z2",
            },
            
            # ⊤: Kinetics - limit
            5: {
                "conventional_math": r"\lim_{\leftarrow} D \land \text{open}(\lim)",
                "lean_expression": "IsLimit D ∧ IsOpen (lim D)",
                "description": "Limit of diagram D is open (slow near-equilibrium)",
                "promoted_atom": None,
            },
            
            # ∈: Cardinality - representable
            6: {
                "conventional_math": r"h_X: \mathcal{C} \to \text{Set} \land \text{rep}(h_X)",
                "lean_expression": "Representable h_X",
                "description": "h_X is a representable functor (maximal cardinality)",
                "promoted_atom": "HOLOGRAPHIC_STATE",
            },
            
            # ∋: Composition - functor category
            7: {
                "conventional_math": r"[\mathcal{C}, \mathcal{D}]",
                "lean_expression": "Functor C D",
                "description": "Functor category from C to D (broadcast composition)",
                "promoted_atom": "BROADCAST_TRANSCENDENCE",
            },
            
            # ⊙: Criticality - closed category
            8: {
                "conventional_math": r"\mathcal{C} \text{ closed} \land \text{CL}(\mathcal{C}, \mathcal{C}) = \mathcal{C}",
                "lean_expression": "IsClosed C ∧ CL(C, C) = C",
                "description": "C is a closed category with Frobenius structure (self-modeling)",
                "promoted_atom": "PHI_C",
            },
            
            # ⊥: Chirality - involution
            9: {
                "conventional_math": r"\exists F: \mathcal{C} \to \mathcal{C} \land F \circ F \cong \text{id}",
                "lean_expression": "∃ F : C ⥤ C, F ∘ F ≅ 𝟭 C",
                "description": "Involution functor on C (one-step chirality)",
                "promoted_atom": "ETERNAL_FIXEDPOINT",
            },
            
            # ⊞: Stoichiometry - monoidal category
            10: {
                "conventional_math": r"\mathcal{C} \text{ monoidal} \land \otimes: \mathcal{C} \times \mathcal{C} \to \mathcal{C}",
                "lean_expression": "MonoidalCategory C",
                "description": "C is a monoidal category (homogeneous)",
                "promoted_atom": None,
            },
            
            # ⊡: Winding - colimit
            11: {
                "conventional_math": r"\text{colim} \mathbb{N} \cong X",
                "lean_expression": "IsColimit ℕ ≅ X",
                "description": "X is the colimit of ℕ (trivial winding)",
                "promoted_atom": "ZWIND",
            },
        }
        
        info = mappings[primitive_idx]
        return PrimitiveMapping(
            primitive_idx=primitive_idx,
            primitive_symbol=symbol,
            conventional_math=info["conventional_math"],
            lean_expression=info["lean_expression"],
            discipline="category_theory",
            metadata={
                "description": info["description"],
                "promoted_atom": info["promoted_atom"],
            }
        )
    
    def compose_expression(self, mappings: List):
        """Compose categorical expressions using logical conjunction"""
        MathematicalExpression = _get_mathematical_expression()
        
        if not mappings:
            raise ValueError("No mappings provided")
        
        # Compose conventional math
        conventional_parts = [m.conventional_math for m in mappings]
        composed_conventional = r" \\land ".join(conventional_parts)
        
        # Compose Lean
        lean_parts = [m.lean_expression for m in mappings]
        composed_lean = " ∧ ".join(lean_parts)
        
        # Get primitives used
        primitives_used = [m.primitive_symbol for m in mappings]
        
        return MathematicalExpression(
            conventional=composed_conventional,
            lean=composed_lean,
            primitives_used=primitives_used,
            discipline=self.get_name(),
            composition_steps=[f"Mapped {m.primitive_symbol} to {m.conventional_math}" for m in mappings],
            metadata={
                "description": "Category-theoretic decomposition",
                "theory": "Category Theory",
                "discipline": "category_theory",
            }
        )
    
    def to_lean_imports(self) -> List[str]:
        return [
            "import Mathlib.CategoryTheory.Basic",
            "import Mathlib.CategoryTheory.Limits.Basic",
            "import Mathlib.CategoryTheory.Adjunction.Basic",
            "import Mathlib.CategoryTheory.Monoidal.Basic",
        ]
    
    def to_lean_definitions(self) -> str:
        return """
-- Category Theory definitions for CLINK decomposition

open CategoryTheory

-- Dimension predicate for objects
def dim {C : Type*} [Category C] (x : C) : ℕ := sorry

-- Open limit predicate
def IsOpen {C : Type*} [Category C] {J : Type*} [Small {J}] 
    (D : J ⥤ C) [HasLimitsOfShape J C] : Prop := sorry

-- Representable functor predicate
def Representable {C : Type*} [Category C] (F : C ⥤ Type*) : Prop := sorry

-- Closed category structure
def IsClosed (C : Type*) [Category C] : Prop := sorry

-- Center of a category
def CL (C D : Type*) [Category C] [Category D] : Type* := sorry
"""
