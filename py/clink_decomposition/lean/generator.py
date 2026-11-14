"""
Lean Code Generator for CL8NK/CL9NK decomposition
"""

from typing import List, Dict, Any, Optional
from ..core.models import MathematicalExpression, NavigatorOutput


class LeanGenerator:
    """
    Generates Lean code from mathematical expressions.
    
    Creates complete Lean files with imports, definitions, and proofs.
    """
    
    def __init__(self):
        self.standard_imports = [
            "import Mathlib.Tactic",
            "import Mathlib.Data.Real.Basic",
            "import Mathlib.Logic.Basic",
        ]
    
    def generate(
        self,
        expression: MathematicalExpression,
        navigator_output: NavigatorOutput
    ) -> str:
        """Generate complete Lean code"""
        
        # Get discipline-specific imports and definitions
        register = None
        try:
            from ..registers import get_discipline
            register = get_discipline(expression.discipline)
        except:
            pass
        
        # Get imports
        imports = []
        if register:
            imports.extend(register.to_lean_imports())
        else:
            imports = self.standard_imports
        
        # Get definitions
        definitions = ""
        if register:
            definitions = register.to_lean_definitions()
        
        # Get composed expression
        composed = expression.lean
        
        # Build the Lean file
        lean_code = ""
        
        # Add imports
        for imp in imports:
            lean_code += f"{imp}\n"
        
        lean_code += "\n\n"
        
        # Add definitions
        if definitions:
            lean_code += f"-- Auto-generated definitions for {expression.discipline}\n"
            lean_code += definitions + "\n\n"
        
        # Add composed expression as a definition
        lean_code += f"-- CLINK Decomposition: {navigator_output.entry_name}\n"
        lean_code += f"-- Navigator: {navigator_output.navigator}\n"
        lean_code += f"-- Discipline: {expression.discipline}\n"
        lean_code += f"-- CLINK Tuple: {navigator_output.clink_tuple}\n"
        
        if navigator_output.promoted_atoms:
            lean_code += f"-- Promoted Atoms: {', '.join(navigator_output.promoted_atoms)}\n"
        
        if navigator_output.tier:
            lean_code += f"-- Tier: {navigator_output.tier}\n"
        
        if navigator_output.distance:
            lean_code += f"-- Distance: {navigator_output.distance}\n"
        
        lean_code += "\n"
        
        # Define the composed expression
        safe_name = navigator_output.entry_name.replace("'", "").replace(" ", "_")
        lean_code += f"def {safe_name} : Prop :=\n"
        lean_code += f"  {composed}\n"
        
        # Add validation theorem
        lean_code += f"\n-- Frobenius identity: μ ∘ δ = id\n"
        lean_code += f"example : μ ∘ δ = id := by\n"
        lean_code += f"  sorry  -- Proof that decomposition satisfies Frobenius identity\n"
        
        return lean_code
    
    def generate_imports(self, discipline: str) -> List[str]:
        """Get imports for a specific discipline"""
        try:
            from ..registers import get_discipline
            register = get_discipline(discipline)
            if register:
                return register.to_lean_imports()
        except:
            pass
        return self.standard_imports
    
    def generate_definitions(self, discipline: str) -> str:
        """Get definitions for a specific discipline"""
        try:
            from ..registers import get_discipline
            register = get_discipline(discipline)
            if register:
                return register.to_lean_definitions()
        except:
            pass
        return ""
