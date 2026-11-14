"""
CL8NK/CL9NK Decomposition System

Entry ---CL8|9NK---> LOGICAL CONNECTIVES ---DECOMPOSE---> MATHEMATICAL EXPRESSIONS

Output: Conventional Math + Equivalent Lean Code
"""

from .core.models import (
    MathematicalExpression,
    NavigatorOutput,
    PrimitiveMapping,
    DecompositionContext,
    DecompositionResult,
    PRIMITIVE_SYMBOLS,
    DecompositionMode,
    DecompositionTier,
)

from .core.input_processor import InputProcessor
from .core.context_selector import ContextSelector
from .core.composition_engine import CompositionEngine

from .output.generator import OutputGenerator, DecompositionOutput

from .registers import (
    DisciplineRegister,
    register_discipline,
    list_disciplines,
    get_discipline,
    SetTheoryRegister,
    CategoryTheoryRegister,
)

from .validation.validator import Validator, ValidationResult, ValidationReport

# Try to import Lean generator
try:
    from .lean.generator import LeanGenerator
    _lean_available = True
except ImportError:
    _lean_available = False
    LeanGenerator = None

__version__ = "1.0.0"
__all__ = [
    # Core models
    "MathematicalExpression",
    "NavigatorOutput",
    "PrimitiveMapping",
    "DecompositionContext",
    "DecompositionResult",
    "PRIMITIVE_SYMBOLS",
    "DecompositionMode",
    "DecompositionTier",
    # Core components
    "InputProcessor",
    "ContextSelector",
    "CompositionEngine",
    # Output
    "OutputGenerator",
    "DecompositionOutput",
    # Registers
    "DisciplineRegister",
    "register_discipline",
    "list_disciplines",
    "get_discipline",
    "SetTheoryRegister",
    "CategoryTheoryRegister",
    # Validation
    "Validator",
    "ValidationResult",
    "ValidationReport",
]

if _lean_available:
    __all__.append("LeanGenerator")
