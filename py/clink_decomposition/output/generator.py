"""
Output Generator: Creates final output files with conventional math and Lean code
"""

from dataclasses import dataclass, field
from typing import List, Dict, Any, Optional
from datetime import datetime


@dataclass
class DecompositionOutput:
    """Complete output for a decomposition"""
    entry_name: str
    navigator: str
    discipline: str
    clink_tuple: str
    promoted_atoms: List[str]
    tier: Optional[str] = None
    distance: Optional[float] = None
    conventional_math: str = ""
    lean_code: str = ""
    validation_results: Dict = field(default_factory=dict)
    metadata: Dict = field(default_factory=dict)
    timestamp: datetime = field(default_factory=datetime.now)
    
    def to_dict(self) -> Dict:
        """Convert to dictionary for JSON serialization"""
        # Convert ValidationResult objects to dicts if needed
        validation_dict = {}
        if isinstance(self.validation_results, dict):
            for check_name, result in self.validation_results.items():
                if hasattr(result, '__dict__'):
                    # It's a ValidationResult object, convert to dict
                    validation_dict[check_name] = result.__dict__
                else:
                    validation_dict[check_name] = result
        else:
            validation_dict = self.validation_results or {}
        
        return {
            "entry_name": self.entry_name,
            "navigator": self.navigator,
            "discipline": self.discipline,
            "clink_tuple": self.clink_tuple,
            "promoted_atoms": self.promoted_atoms,
            "tier": self.tier,
            "distance": self.distance,
            "conventional_math": self.conventional_math,
            "lean_code": self.lean_code,
            "validation_results": validation_dict,
            "metadata": self.metadata,
            "timestamp": self.timestamp.isoformat(),
        }


class OutputGenerator:
    """
    Generates output files containing both conventional mathematics and Lean code.
    """
    
    def __init__(self):
        self.formatter = ConventionalFormatter()
        self.lean_generator_class = None
        try:
            from clink_decomposition.lean.generator import LeanGenerator
            self.lean_generator_class = LeanGenerator
        except ImportError:
            pass
        
        # Fallback if LeanGenerator not available
        if self.lean_generator_class is None:
            self.lean_generator_class = FallbackLeanGenerator
    
    def generate(
        self, 
        navigator_output,
        mathematical_expression,
        validation_results: Optional[Dict] = None
    ) -> DecompositionOutput:
        """
        Generate complete output for a decomposition.
        
        Args:
            navigator_output: Output from CL8NK/CL9NK
            mathematical_expression: Mathematical expression from decomposition
            validation_results: Optional validation results
            
        Returns:
            DecompositionOutput with both representations
        """
        # Format conventional math
        conventional_math = self.formatter.format(
            mathematical_expression,
            navigator_output
        )
        
        # Generate Lean code
        lean_generator = self.lean_generator_class()
        lean_code = lean_generator.generate(
            mathematical_expression,
            navigator_output
        )
        
        return DecompositionOutput(
            entry_name=navigator_output.entry_name,
            navigator=navigator_output.navigator,
            discipline=mathematical_expression.discipline,
            clink_tuple=navigator_output.clink_tuple,
            promoted_atoms=navigator_output.promoted_atoms,
            tier=navigator_output.tier,
            distance=navigator_output.distance,
            conventional_math=conventional_math,
            lean_code=lean_code,
            validation_results=validation_results or {},
            metadata=mathematical_expression.metadata,
        )
    
    def generate_file(
        self, 
        output: DecompositionOutput,
        output_dir: str = "outputs",
        format: str = "md"
    ) -> str:
        """
        Generate output file.
        
        Args:
            output: DecompositionOutput to write
            output_dir: Directory for output files
            format: Output format ("md", "tex", "lean", "json")
            
        Returns:
            Path to the generated file
        """
        import os
        from datetime import datetime
        
        # Create output directory
        os.makedirs(output_dir, exist_ok=True)
        
        # Generate filename
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        filename = f"{output.entry_name}_{output.navigator}_{output.discipline}_{timestamp}"
        
        if format == "md":
            ext = ".md"
            content = self._format_markdown(output)
        elif format == "tex":
            ext = ".tex"
            content = self._format_tex(output)
        elif format == "lean":
            ext = ".lean"
            content = output.lean_code
        elif format == "json":
            ext = ".json"
            import json
            content = json.dumps(output.to_dict(), indent=2)
        else:
            raise ValueError(f"Unknown format: {format}")
        
        filepath = os.path.join(output_dir, f"{filename}{ext}")
        
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        
        return filepath
    
    def _format_markdown(self, output: DecompositionOutput) -> str:
        """Format output as Markdown"""
        md = f"""# CLINK Decomposition: {output.entry_name}

**Generated**: {output.timestamp.strftime("%Y-%m-%d %H:%M:%S")}  
**Navigator**: {output.navigator}  
**Discipline**: {output.discipline}  
**CLINK Tuple**: {output.clink_tuple}

---

## 📊 Structural Information

| Property | Value |
|----------|-------|
| Promoted Atoms | {', '.join(output.promoted_atoms) if output.promoted_atoms else 'None'} |
| Tier | {output.tier or 'Unknown'} |
| Distance | {output.distance or 'Unknown'} |

---

## 📐 Conventional Mathematical Expression

{output.conventional_math}

---

## 🔷 Lean Code

```lean
{output.lean_code}
```

---

## ✅ Validation Results

"""
        
        if output.validation_results:
            md += "\n"
            for check, result in output.validation_results.items():
                status = "✅" if result.get("passed", False) else "❌"
                md += f"- {status} **{check}**: {result.get('message', 'No message')}\n"
        
        return md
    
    def _format_tex(self, output: DecompositionOutput) -> str:
        """Format output as LaTeX"""
        promoted = ", ".join(output.promoted_atoms) if output.promoted_atoms else "None"
        tier = output.tier or "Unknown"
        distance = str(output.distance) if output.distance else "Unknown"
        
        tex = f"""\\documentclass{{article}}
\\usepackage{{amsmath, amssymb, amsthm}}
\\usepackage{{geometry}}
\\geometry{{margin=1in}}

\\title{{CLINK Decomposition: {output.entry_name}}}
\\author{{CL8NK/CL9NK Decomposition System}}
\\date{{{output.timestamp.strftime("%Y-%m-%d %H:%M:%S")}}}

\\begin{{document}}

\\maketitle

\\section{{Structural Information}}
\\begin{{center}}
\\begin{{tabular}}{{l|l}}
\\hline
Property & Value \\\\
\\hline
Entry & {output.entry_name} \\\
Navigator & {output.navigator} \\\
Discipline & {output.discipline} \\\
CLINK Tuple & {output.clink_tuple} \\\
Promoted Atoms & {promoted} \\\
Tier & {tier} \\\
Distance & {distance} \\\
\\hline
\\end{{tabular}}
\\end{{center}}

\\section{{Conventional Mathematical Expression}}

{output.conventional_math}

\\section{{Lean Code}}
\\begin{{verbatim}}
{output.lean_code}
\\end{{verbatim}}

\\end{{document}}
"""
        return tex


class ConventionalFormatter:
    """Formats conventional mathematical expressions"""
    
    def format(
        self, 
        expression,
        navigator_output
    ) -> str:
        """Format the conventional math expression"""
        
        # Add header
        header = f"-- {navigator_output.entry_name} ({navigator_output.navigator}, {expression.discipline})\n\n"
        
        # Format the composed expression
        formatted = expression.conventional
        
        # Add primitive breakdown if available
        if hasattr(expression, 'composition_steps') and expression.composition_steps:
            formatted += "\n\n-- Composition Steps:\n"
            for step in expression.composition_steps:
                formatted += f"--   {step}\n"
        
        return header + formatted


class FallbackLeanGenerator:
    """Fallback Lean generator if the real one is not available"""
    
    def generate(self, expression, navigator_output) -> str:
        """Generate Lean code"""
        # Get imports
        imports = [
            "import Mathlib.Tactic",
            "import Mathlib.Logic.Basic",
        ]
        
        # Build the Lean file
        lean_code = ""
        
        # Add imports
        for imp in imports:
            lean_code += f"{imp}\n"
        
        lean_code += "\n\n"
        
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
        lean_code += f"  {expression.lean}\n"
        
        return lean_code
