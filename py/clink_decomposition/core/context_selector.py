"""
Context Selector: Determines appropriate decomposition context
"""

from typing import List, Dict, Optional


def list_disciplines() -> List[str]:
    """Import and call list_disciplines from registers"""
    from ..registers import list_disciplines as _list_disciplines
    return _list_disciplines()


class ContextSelector:
    """
    Selects the appropriate discipline and context for decomposition.
    
    Uses heuristics based on entry name and catalog metadata.
    """
    
    # Entry name to discipline mappings (heuristics)
    ENTRY_DISCIPLINE_MAP: Dict[str, str] = {
        "torque": "topology",
        "feynman's_sprinkler": "category_theory",
        "angular_momentum": "differential_geometry",
        "prime": "number_theory",
        "group": "group_theory",
        "manifold": "differential_geometry",
        "category": "category_theory",
        "set": "set_theory",
        "topos": "category_theory",
        "braid": "category_theory",
        "quantum": "quantum_mechanics",
        "information": "information_theory",
    }
    
    # Navigator to discipline preferences
    NAVIGATOR_DISCIPLINE_PREFERENCES: Dict[str, List[str]] = {
        "cl8nk": ["topology", "category_theory", "set_theory", "group_theory"],
        "cl9nk": ["category_theory", "topology", "set_theory", "differential_geometry"],
    }
    
    def select_discipline(
        self, 
        entry_name: str, 
        navigator: str = "cl8nk",
        available_disciplines: Optional[List[str]] = None
    ) -> str:
        """
        Select the most appropriate discipline for an entry.
        
        Args:
            entry_name: Name of the catalog entry
            navigator: Navigator used (cl8nk or cl9nk)
            available_disciplines: Optional list of available disciplines
            
        Returns:
            Name of the selected discipline
        """
        if available_disciplines is None:
            available_disciplines = list_disciplines()
        
        if not available_disciplines:
            raise ValueError("No disciplines available")
        
        # Try direct match
        entry_lower = entry_name.lower()
        for entry_pattern, discipline in self.ENTRY_DISCIPLINE_MAP.items():
            if entry_pattern.lower() in entry_lower:
                if discipline in available_disciplines:
                    return discipline
        
        # Try navigator preferences
        preferences = self.NAVIGATOR_DISCIPLINE_PREFERENCES.get(navigator, [])
        for pref in preferences:
            if pref in available_disciplines:
                return pref
        
        # Default to first available
        return available_disciplines[0]
    
    def suggest_alternative_disciplines(
        self, 
        entry_name: str, 
        current_discipline: str,
        available_disciplines: Optional[List[str]] = None
    ) -> List[str]:
        """
        Suggest alternative disciplines for an entry.
        
        Args:
            entry_name: Name of the catalog entry
            current_discipline: Currently selected discipline
            available_disciplines: Optional list of available disciplines
            
        Returns:
            List of alternative discipline names
        """
        if available_disciplines is None:
            available_disciplines = list_disciplines()
        
        entry_lower = entry_name.lower()
        alternatives = []
        
        # Find all disciplines that match entry patterns
        for entry_pattern, discipline in self.ENTRY_DISCIPLINE_MAP.items():
            if entry_pattern.lower() in entry_lower:
                if discipline in available_disciplines and discipline != current_discipline:
                    alternatives.append(discipline)
        
        # If no matches, return all available except current
        if not alternatives:
            alternatives = [d for d in available_disciplines if d != current_discipline]
        
        return alternatives[:3]  # Return up to 3 suggestions
    
    def get_context_parameters(
        self, 
        entry_name: str, 
        discipline: str
    ) -> Dict:
        """
        Get discipline-specific context parameters.
        
        Args:
            entry_name: Name of the catalog entry
            discipline: Selected discipline
            
        Returns:
            Dictionary of context parameters
        """
        # Default empty context
        context = {"entry": entry_name, "discipline": discipline}
        
        # Add discipline-specific parameters
        if discipline == "topology":
            context.update({
                "manifold_dimension": 3,
                "field": "real",
                "compactness": "non_compact",
            })
        elif discipline == "category_theory":
            context.update({
                "category_type": "concrete",
                "morphisms": "continuous",
            })
        elif discipline == "set_theory":
            context.update({
                "universe_size": "inaccessible",
                "axioms": "ZFC",
            })
        
        return context
