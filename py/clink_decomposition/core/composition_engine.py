"""
Composition Engine: Combines primitive mappings into full mathematical expressions
"""

from typing import List, Optional, Dict


class CompositionEngine:
    """
    Composes individual primitive mappings into full mathematical expressions.
    
    Supports discipline-specific composition rules.
    """
    
    def __init__(self):
        self.composition_rules = {
            "set_theory": " ∧ ",
            "category_theory": " ∧ ",
            "group_theory": " × ",
            "topology": " ∩ ",
            "differential_geometry": " ⊕ ",
        }
    
    def compose(
        self, 
        mappings: List, 
        discipline: str = "set_theory",
        context: Optional[Dict] = None
    ):
        """
        Compose primitive mappings into a full expression.
        
        Args:
            mappings: List of PrimitiveMapping objects
            discipline: Name of the discipline
            context: Optional context parameters
            
        Returns:
            MathematicalExpression with composed expressions
        """
        from ..registers import get_discipline
        
        if not mappings:
            raise ValueError("No mappings provided for composition")
        
        # Get the register for this discipline
        register = get_discipline(discipline)
        if register is None:
            raise ValueError(f"Unknown discipline: {discipline}")
        
        # Use the register's composition method
        return register.compose_expression(mappings)
    
    def compose_from_primitives(
        self, 
        primitives: List[int], 
        discipline: str = "set_theory",
        context: Optional[Dict] = None
    ) -> object:
        """
        Compose from primitive indices directly.
        
        Args:
            primitives: List of primitive indices (0-11)
            discipline: Name of the discipline
            context: Optional context parameters
            
        Returns:
            MathematicalExpression
        """
        from ..registers import get_discipline
        
        register = get_discipline(discipline)
        if register is None:
            raise ValueError(f"Unknown discipline: {discipline}")
        
        # Map each primitive
        mappings = [register.map_primitive(p, context) for p in primitives]
        
        # Compose using the register
        return register.compose_expression(mappings)
    
    def get_composition_operator(self, discipline: str) -> str:
        """Get the composition operator for a discipline"""
        return self.composition_rules.get(discipline, " ∧ ")
