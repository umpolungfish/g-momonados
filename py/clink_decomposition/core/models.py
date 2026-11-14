"""
Core data models for CL8NK/CL9NK decomposition
"""

from dataclasses import dataclass, field
from typing import List, Dict, Optional, Any, Union
from enum import Enum


# The 12 CLINK primitives
PRIMITIVE_SYMBOLS = ['⊢', '⊣', '≻', '≺', '⋈', '⊤', '∈', '∋', '⊙', '⊥', '⊞', '⊡']


class DecompositionMode(Enum):
    """Mode for decomposition"""
    CONVENTIONAL = "conventional"
    LEAN = "lean"
    BOTH = "both"


class DecompositionTier(Enum):
    """Tier of decomposition"""
    PRIMITIVE = "primitive"
    BASIC = "basic"
    ADVANCED = "advanced"
    COMPREHENSIVE = "comprehensive"


@dataclass
class NavigatorOutput:
    """
    Output from CL8NK/CL9NK navigator.
    
    Contains the raw tuple output and parsed components.
    """
    entry_name: str
    navigator: str  # "cl8nk" or "cl9nk"
    clink_tuple: str
    primitives: List[int] = field(default_factory=list)
    primitive_symbols: List[str] = field(default_factory=list)
    promoted_atoms: List[str] = field(default_factory=list)
    tier: Optional[str] = None
    distance: Optional[float] = None
    full_output: str = ""
    reference_tuple: Optional[str] = None
    match_count: Optional[int] = None
    close_count: Optional[int] = None
    distant_count: Optional[int] = None
    is_valid: bool = True
    error: Optional[str] = None


@dataclass
class PrimitiveMapping:
    """
    Mapping of a CLINK primitive to mathematical expressions.
    
    Each primitive can be mapped to:
    - Conventional mathematical notation
    - Lean code
    - Optional metadata about the mapping
    """
    primitive_idx: int
    primitive_symbol: str
    conventional_math: str
    lean_expression: str
    discipline: str
    metadata: Dict[str, Any] = field(default_factory=dict)
    
    def __post_init__(self):
        if self.primitive_idx < 0 or self.primitive_idx >= len(PRIMITIVE_SYMBOLS):
            raise ValueError(f"Primitive index {self.primitive_idx} out of range (0-11)")
        if self.primitive_symbol != PRIMITIVE_SYMBOLS[self.primitive_idx]:
            raise ValueError(f"Symbol mismatch: expected {PRIMITIVE_SYMBOLS[self.primitive_idx]}, got {self.primitive_symbol}")


@dataclass
class MathematicalExpression:
    """
    Complete mathematical expression from decomposition.
    
    Contains both conventional and Lean representations,
    along with structural information about the decomposition.
    """
    conventional: str
    lean: str
    primitives_used: List[str]
    discipline: str
    tier: DecompositionTier = DecompositionTier.BASIC
    composition_steps: List[str] = field(default_factory=list)
    imports: List[str] = field(default_factory=list)
    definitions: List[str] = field(default_factory=list)
    metadata: Dict[str, Any] = field(default_factory=dict)


@dataclass
class DecompositionContext:
    """
    Context for the decomposition process.
    
    Carries configuration and state through the pipeline.
    """
    entry: str
    mode: DecompositionMode = DecompositionMode.BOTH
    target_discipline: Optional[str] = None
    tier: DecompositionTier = DecompositionTier.COMPREHENSIVE
    use_cache: bool = True
    validate_structural: bool = True
    validate_frobenius: bool = True
    validate_covariance: bool = True
    timing: Dict[str, float] = field(default_factory=dict)
    cache: Dict[str, Any] = field(default_factory=dict)


@dataclass
class DecompositionResult:
    """
    Complete result of CL8NK/CL9NK decomposition.
    
    Contains all outputs and validation results.
    """
    success: bool
    input: str
    clink_type: str
    navigator_output: Optional[NavigatorOutput] = None
    mathematical_expression: Optional[MathematicalExpression] = None
    discipline: Optional[str] = None
    validation_results: Dict[str, Any] = field(default_factory=dict)
    errors: List[str] = field(default_factory=list)
    warnings: List[str] = field(default_factory=list)
    timing: Dict[str, float] = field(default_factory=dict)
    
    def add_error(self, error: str):
        """Add an error message"""
        self.success = False
        self.errors.append(error)
    
    def add_warning(self, warning: str):
        """Add a warning message"""
        self.warnings.append(warning)
