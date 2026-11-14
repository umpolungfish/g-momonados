"""
Set Theory discipline register for CL8NK/CL9NK decomposition
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


class SetTheoryRegister(DisciplineRegister):
    """
    Set Theory discipline register.
    
    Maps CLINK primitives to set-theoretic expressions.
    Reference: ZFC set theory baseline.
    """
    
    @classmethod
    def get_name(cls) -> str:
        return "set_theory"
    
    @classmethod
    def get_description(cls) -> str:
        return "ZFC set theory - the axiomatic foundation of mathematics"
    
    def map_primitive(self, primitive_idx: int, context: Optional[Dict] = None):
        """Map a primitive to its set-theoretic expression"""
        PrimitiveMapping = _get_primitive_mapping()
        
        if primitive_idx < 0 or primitive_idx >= 12:
            raise ValueError(f"Primitive index must be 0-11, got {primitive_idx}")
        
        symbol = PRIMITIVE_SYMBOLS[primitive_idx]
        
        # Define the mappings
        mappings = {
            # ⊢: Dimensionality - point in the universe
            0: {
                "conventional_math": r"x \in V \land \dim(V) = \omega",
                "lean_expression": "x ∈ V ∧ dim V = ℵ₀",
                "description": "x is an element of the set-theoretic universe V with infinite dimension",
                "promoted_atom": None,
            },
            
            # ⊣: Topology - directed graph
            1: {
                "conventional_math": r"\text{graph}(x) \land \text{branch}(x)",
                "lean_expression": "IsDirectedGraph x",
                "description": "x has branching graph topology (self-referential)",
                "promoted_atom": "HOLOBOUND",
            },
            
            # ≻: Coupling - supervenience
            2: {
                "conventional_math": r"x \subseteq y \land y \not\subseteq x",
                "lean_expression": "x ⊂ y",
                "description": "x is a proper subset of y (bidirectional feedback)",
                "promoted_atom": "LR_DUAL",
            },
            
            # ≺: Parity - no symmetry
            3: {
                "conventional_math": r"\neg\exists \text{sym}(x)",
                "lean_expression": "¬ IsSymmetric x",
                "description": "x has no symmetry (asymmetry)",
                "promoted_atom": None,
            },
            
            # ⋈: Fidelity - classical/deterministic
            4: {
                "conventional_math": r"P(x) \in \{0,1\} \land \text{det}(x)",
                "lean_expression": "(x = 0 ∨ x = 1) ∧ IsDeterministic x",
                "description": "x is classical (0 or 1) and deterministic",
                "promoted_atom": "PM_Z2",
            },
            
            # ⊤: Kinetics - slow near-equilibrium
            5: {
                "conventional_math": r"\tau \gg T \land \text{eq}(x) \land \text{gate\_open}(x)",
                "lean_expression": "τ > T ∧ IsEquilibrium x ∧ GateIsOpen x",
                "description": "Slow kinetics near equilibrium with open gate",
                "promoted_atom": None,
            },
            
            # ∈: Cardinality - maximal
            6: {
                "conventional_math": r"\forall y \subseteq x. \|y\| < \|x\|",
                "lean_expression": "∀ y ⊂ x, Cardinal.mk y < Cardinal.mk x",
                "description": "Maximal cardinality - every proper subset has smaller cardinality",
                "promoted_atom": "HOLOGRAPHIC_STATE",
            },
            
            # ∋: Composition - covering
            7: {
                "conventional_math": r"\bigcup x = V",
                "lean_expression": "⋃₀ x = V",
                "description": "x covers the universe (broadcast composition)",
                "promoted_atom": "BROADCAST_TRANSCENDENCE",
            },
            
            # ⊙: Criticality - self-modeling
            8: {
                "conventional_math": r"V = L \land \mu \circ \delta = \text{id}",
                "lean_expression": "V = L ∧ μ ∘ δ = id",
                "description": "Constructible universe with Frobenius identity (self-modeling criticality)",
                "promoted_atom": "PHI_C",
            },
            
            # ⊥: Chirality - involution
            9: {
                "conventional_math": r"\exists f. f: x \to x \land f \circ f = \text{id}",
                "lean_expression": "∃ f : x → x, f ∘ f = id",
                "description": "x admits an involution (one-step chirality)",
                "promoted_atom": "ETERNAL_FIXEDPOINT",
            },
            
            # ⊞: Stoichiometry - homogeneous
            10: {
                "conventional_math": r"\forall a,b \in x. \text{type}(a) = \text{type}(b)",
                "lean_expression": "∀ a b ∈ x, type a = type b",
                "description": "All elements of x have the same type (homogeneous)",
                "promoted_atom": None,
            },
            
            # ⊡: Winding - well-founded
            11: {
                "conventional_math": r"\text{well\_founded}(x)",
                "lean_expression": "WellFounded x",
                "description": "x is well-founded (trivial winding)",
                "promoted_atom": "ZWIND",
            },
        }
        
        info = mappings[primitive_idx]
        return PrimitiveMapping(
            primitive_idx=primitive_idx,
            primitive_symbol=symbol,
            conventional_math=info["conventional_math"],
            lean_expression=info["lean_expression"],
            discipline="set_theory",
            metadata={
                "description": info["description"],
                "promoted_atom": info["promoted_atom"],
            }
        )
    
    def compose_expression(self, mappings: List):
        """Compose set-theoretic expressions using logical conjunction"""
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
                "description": "Set-theoretic decomposition",
                "theory": "ZFC",
                "discipline": "set_theory",
            }
        )
    
    def to_lean_imports(self) -> List[str]:
        return [
            "import Mathlib.SetTheory.Cardinal.Basic",
            "import Mathlib.Logic.Basic",
        ]
    
    def to_lean_definitions(self) -> str:
        return """
-- Set Theory definitions for CLINK decomposition

-- Type for set-theoretic universe
variable {V : Type*} [Universe V]

-- Dimensionality predicate
def dim (V : Type*) : ℵ := sorry

-- Well-founded predicate
def WellFounded (x : Set V) : Prop := sorry

-- Constructible universe
def L : Set V := sorry

-- Frobenius unit and counit
def μ : V → V := sorry
def δ : V → V := sorry

-- Gate open predicate
def GateIsOpen (x : V) : Prop := sorry

-- Deterministic predicate
def IsDeterministic (x : V) : Prop := sorry

-- Directed graph predicate
def IsDirectedGraph (x : V) : Prop := sorry

-- Symmetric predicate
def IsSymmetric (x : V) : Prop := sorry

-- Type equality predicate
def type (x : V) : Type := sorry
"""
