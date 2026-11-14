"""
Base class for discipline registers
"""

from abc import ABC, abstractmethod
from typing import List, Optional, Dict, Any


# Global discipline registry
_DISCIPLINE_REGISTRY: dict = {}


class DisciplineRegister(ABC):
    """
    Abstract base class for discipline-specific registers.
    
    Each discipline register maps the 12 CLINK primitives to mathematical
    expressions in that discipline, and provides methods for composition
    and Lean code generation.
    """
    
    @classmethod
    @abstractmethod
    def get_name(cls) -> str:
        """Get the name of this discipline"""
        pass
    
    @classmethod
    @abstractmethod
    def get_description(cls) -> str:
        """Get a description of this discipline"""
        pass
    
    @abstractmethod
    def map_primitive(self, primitive_idx: int, context: Optional[Dict] = None):
        """
        Map a single primitive to its expression in this discipline.
        
        Args:
            primitive_idx: Index of the primitive (0-11)
            context: Optional context parameters
            
        Returns:
            PrimitiveMapping with conventional math and Lean expressions
        """
        pass
    
    def map_all_primitives(self, context: Optional[Dict] = None) -> List:
        """Map all 12 primitives"""
        return [self.map_primitive(i, context) for i in range(12)]
    
    @abstractmethod
    def compose_expression(self, mappings: List):
        """
        Compose individual primitive mappings into a full expression.
        
        Args:
            mappings: List of PrimitiveMapping objects
            
        Returns:
            MathematicalExpression with composed conventional and Lean expressions
        """
        pass
    
    def validate(self, expression) -> bool:
        """
        Validate that the expression satisfies discipline-specific constraints.
        
        Args:
            expression: MathematicalExpression to validate
            
        Returns:
            True if valid, False otherwise
        """
        # Default: always valid
        return True
    
    def to_lean_imports(self) -> List[str]:
        """Get list of Lean imports required for this discipline"""
        return []
    
    def to_lean_definitions(self) -> str:
        """Get Lean definitions required for this discipline"""
        return ""


def register_discipline(discipline_name: str, discipline_class: type):
    """
    Register a discipline class with the global registry.
    
    Args:
        discipline_name: Name to register (e.g., "set_theory")
        discipline_class: Class implementing DisciplineRegister
    """
    global _DISCIPLINE_REGISTRY
    _DISCIPLINE_REGISTRY[discipline_name] = discipline_class()


def list_disciplines() -> list:
    """List all registered disciplines"""
    return list(_DISCIPLINE_REGISTRY.keys())


def get_discipline(discipline_name: str):
    """Get a registered discipline instance"""
    return _DISCIPLINE_REGISTRY.get(discipline_name)
