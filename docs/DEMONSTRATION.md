# CL8NK/CL9NK Decomposition System - Demonstration

This document demonstrates the CL8NK/CL9NK Decomposition System with concrete examples and usage patterns.

## Quick Start

The system is fully operational and ready to use. Here's how to get started:

### 1. Basic Usage

```bash
# First, navigate to the proper directory
cd /home/mrnob0dy666/imsgct/G-mOMonadOS/py

# Decompose a catalog entry with default settings (cl8nk, auto-select discipline)
python3 clink_decompose.py "Feynman's_Sprinkler"

# Specify navigator and discipline
python3 clink_decompose.py "Feynman's_Sprinkler" --navigator cl8nk --discipline set_theory

# Use cl9nk instead
python3 clink_decompose.py "Feynman's_Sprinkler" --navigator cl9nk --discipline category_theory
```

### 2. List Available Disciplines

```bash
cd /home/mrnob0dy666/imsgct/G-mOMonadOS/py
python3 clink_decompose.py --list-disciplines
```

Output:
```
Available disciplines:
  - set_theory: ZFC set theory - the axiomatic foundation of mathematics
  - category_theory: Category theory - abstract algebra of functions and relations
```

### 3. Generate Output in Different Formats

```bash
# Console output (default)
python3 clink_decompose.py "Feynman's_Sprinkler" --output console

# Markdown file
python3 clink_decompose.py "Feynman's_Sprinkler" --output md --output-dir results/

# LaTeX document
python3 clink_decompose.py "Feynman's_Sprinkler" --output tex --output-dir results/

# Lean code
python3 clink_decompose.py "Feynman's_Sprinkler" --output lean --output-dir lean_code/

# JSON data
python3 clink_decompose.py "Feynman's_Sprinkler" --output json --output-dir data/
```

### 4. Use Multiple Disciplines

```bash
# Generate output for all available disciplines
python3 clink_decompose.py "Feynman's_Sprinkler" --all-disciplines --output md
```

This creates separate files for each discipline:
- `Feynman's_Sprinkler_cl8nk_set_theory_*.md`
- `Feynman's_Sprinkler_cl8nk_category_theory_*.md`

### 5. Include Validation

```bash
python3 clink_decompose.py "Feynman's_Sprinkler" --validate
```

This adds validation results showing all checks passed.

---

## Example Outputs

### Example 1: Console Output (CL8NK, Set Theory)

```
================================================================================
CLINK Decomposition: Feynman's_Sprinkler
Navigator: cl8nk | Discipline: set_theory
CLINK Tuple: ⟨𐑦⋅𐑸⋅𐑾⋅𐑹⋅𐑐⋅𐑧⋅𐑲⋅𐑵⋅⊙⋅𐑫⋅𐑳⋅𐑟⟩
================================================================================

📊 STRUCTURAL INFORMATION
--------------------------------------------------------------------------------
Promoted Atoms: BROADCAST_TRANSCENDENCE, PHI_C
Tier: O₁
Distance: 1.8167

📐 CONVENTIONAL MATHEMATICAL EXPRESSION
--------------------------------------------------------------------------------
V = L ∧ μ ∘ δ = id ∧ x ∈ V ∧ dim(V) = ω ∧ graph(x) ∧ branch(x) ∧ x ⊆ y ∧ y ⊈ x ∧ 
¬∃sym(x) ∧ P(x) ∈ {0,1} ∧ det(x) ∧ ∀y ⊆ x. |y| < |x| ∧ τ ≫ T ∧ eq(x) ∧ gate_open(x) ∧ 
∪x = V ∧ ∃f. f:x→x ∧ f∘f = id ∧ ∀a,b ∈ x. type(a) = type(b) ∧ well_founded(x)

🔷 LEAN CODE
--------------------------------------------------------------------------------
import Mathlib.SetTheory.Cardinal.Basic
import Mathlib.Logic.Basic

open CategoryTheory

def dim (V : Type*) : ℵ := sorry
def WellFounded (x : Set V) : Prop := sorry
def L : Set V := sorry
def μ : V → V := sorry
def δ : V → V := sorry

def Feynmans_Sprinkler : Prop :=
  V = L ∧ μ ∘ δ = id ∧ x ∈ V ∧ dim V = ℵ₀ ∧ IsDirectedGraph x ∧ x ⊂ y ∧ 
  ¬ IsSymmetric x ∧ (x = 0 ∨ x = 1) ∧ IsDeterministic x ∧ 
  ∀ y ⊂ x, Cardinal.mk y < Cardinal.mk x ∧ τ > T ∧ IsEquilibrium x ∧ 
  GateIsOpen x ∧ ⋃₀ x = V ∧ ∃ f : x → x, f ∘ f = id ∧ 
  ∀ a b ∈ x, type a = type b ∧ WellFounded x

example : μ ∘ δ = id := by
  sorry

✅ VALIDATION RESULTS
--------------------------------------------------------------------------------
✅ structural_tuple: CLINK tuple is structurally valid
✅ primitive_coverage: All 12 primitives are mapped
✅ frobenius_identity: Frobenius identity check (μ∘δ=id) - structural validation only
✅ type_consistency: Type consistency check - placeheld
✅ dual_pair_covariance: All dual pairs are complete
✅ tier_assessment: Tier assessment valid: O₁
```

### Example 2: Markdown Output (CL8NK, Category Theory)

The markdown file contains:
- Title with metadata (entry name, navigator, discipline, CLINK tuple)
- Structural information table (promoted atoms, tier, distance)
- Conventional mathematical expression in LaTeX format
- Lean code in a code block
- Validation results

### Example 3: Lean Output (CL9NK, Category Theory)

```lean
import Mathlib.CategoryTheory.Basic
import Mathlib.CategoryTheory.Limits.Basic
import Mathlib.CategoryTheory.Adjunction.Basic
import Mathlib.CategoryTheory.Monoidal.Basic

open CategoryTheory

def dim {C : Type*} [Category C] (x : C) : ℕ := sorry
def IsOpen {C : Type*} [Category C] {J : Type*} [Small {J}] 
    (D : J ⥤ C) [HasLimitsOfShape J C] : Prop := sorry
def Representable {C : Type*} [Category C] (F : C ⥤ Type*) : Prop := sorry
def IsClosed (C : Type*) [Category C] : Prop := sorry
def CL (C D : Type*) [Category C] [Category D] : Type* := sorry

def Feynmans_Sprinkler : Prop :=
  IsClosed C ∧ CL(C, C) = C ∧ x : C ∧ dim x = n ∧ L ⊣ R ∧ Monad T ∧ Cᵒᵖ ∧ 
  Epi f ∧ Representable h_X ∧ IsLimit D ∧ IsOpen (lim D) ∧ Functor C D ∧ 
  ∃ F : C ⥤ C, F ∘ F ≅ 𝟭 C ∧ MonoidalCategory C ∧ IsColimit ℕ ≅ X

example : μ ∘ δ = id := by
  sorry  -- Proof that decomposition satisfies Frobenius identity
```

### Example 4: JSON Output

```json
{
  "entry_name": "Feynman's_Sprinkler",
  "navigator": "cl8nk",
  "discipline": "category_theory",
  "clink_tuple": "⟨𐑦⋅𐑸⋅𐑾⋅𐑹⋅𐑐⋅𐑧⋅𐑲⋅𐑵⋅⊙⋅𐑫⋅𐑳⋅𐑟⟩",
  "promoted_atoms": ["BROADCAST_TRANSCENDENCE", "PHI_C"],
  "tier": "O₁",
  "distance": 1.8167,
  "conventional_math": "...",
  "lean_code": "...",
  "validation_results": {},
  "metadata": {
    "description": "Category-theoretic decomposition",
    "theory": "Category Theory",
    "discipline": "category_theory",
    "context": {...}
  },
  "timestamp": "2026-11-14T07:18:19.019686"
}
```

---

## Python API Usage

### Basic API Example

```python
import sys
sys.path.insert(0, '/home/mrnob0dy666/imsgct')

from clink_decomposition import (
    InputProcessor,
    ContextSelector,
    CompositionEngine,
    OutputGenerator,
    Validator,
    get_discipline,
    list_disciplines
)

# Step 1: Process entry through navigator
processor = InputProcessor()
navigator_output = processor.process_entry("Feynman's_Sprinkler", "cl8nk")

# Step 2: Select discipline
selector = ContextSelector()
discipline = selector.select_discipline("Feynman's_Sprinkler", "cl8nk")

# Step 3: Get register and map primitives
register = get_discipline(discipline)
context = selector.get_context_parameters("Feynman's_Sprinkler", discipline)
mappings = [register.map_primitive(p, context) for p in navigator_output.primitives]

# Step 4: Compose expression
engine = CompositionEngine()
mathematical_expression = engine.compose(mappings, discipline, context)

# Step 5: Generate output
generator = OutputGenerator()
output = generator.generate(navigator_output, mathematical_expression)

# Optional: Validate
validator = Validator()
validation_results = validator.validate(mathematical_expression, navigator_output)

print(f"Conventional: {output.conventional_math}")
print(f"Lean: {output.lean_code}")
```

### Working with Multiple Disciplines

```python
from clink_decomposition import list_disciplines, get_discipline

# Get all available disciplines
disciplines = list_disciplines()

# Process with each discipline
for discipline in disciplines:
    register = get_discipline(discipline)
    mappings = [register.map_primitive(p) for p in navigator_output.primitives]
    expression = register.compose_expression(mappings)
    print(f"{discipline}: {expression.conventional}")
```

### Custom Output Generation

```python
from clink_decomposition.output.generator import OutputGenerator

generator = OutputGenerator()
output = generator.generate(
    navigator_output=navigator_output,
    mathematical_expression=mathematical_expression,
    validation_results=validation_results
)

# Access output components
print(f"Entry: {output.entry_name}")
print(f"Discipline: {output.discipline}")
print(f"Conventional Math: {output.conventional_math}")
print(f"Lean Code: {output.lean_code}")

# Generate files
generator.generate_file(output, "results/", "md")  # Markdown
generator.generate_file(output, "results/", "lean")  # Lean code
generator.generate_file(output, "results/", "json")  # JSON
```

---

## The 12 CLINK Primitives

The system works with 12 fundamental primitives organized into dual pairs:

| Index | Symbol | Name | Conceptual Domain | Set Theory Mapping | Category Theory Mapping |
|-------|--------|------|-----------------|-------------------|------------------------|
| 0 | ⊢ | Dimensionality | Point in the universe | x ∈ V ∧ dim(V) = ω | x ∈ C ∧ dim(x) = n |
| 1 | ⊣ | Topology | Directed graph / branching | graph(x) ∧ branch(x) | L ⊣ R |
| 2 | ≻ | Coupling | Supervenience / proper subset | x ⊆ y ∧ y ⊈ x | T:C→C ∧ μ:T²→T ∧ η:I→T |
| 3 | ≺ | Parity | No symmetry / asymmetry | ¬∃sym(x) | C^op |
| 4 | ⋈ | Fidelity | Classical / deterministic | P(x) ∈ {0,1} ∧ det(x) | f:X→Y ∧ epi(f) |
| 5 | ⊤ | Kinetics | Slow near-equilibrium | τ ≫ T ∧ eq(x) ∧ gate_open(x) | lim← D ∧ open(lim) |
| 6 | ∈ | Cardinality | Maximal / set membership | ∀y ⊆ x. \|y\| < \|x\| | h_X:C→Set ∧ rep(h_X) |
| 7 | ∋ | Composition | Covering / broadcast | ∪x = V | [C,D] |
| 8 | ⊙ | Criticality | Self-modeling / Frobenius | V = L ∧ μ∘δ = id | C closed ∧ CL(C,C) = C |
| 9 | ⊥ | Chirality | Involution / one-step | ∃f. f:x→x ∧ f∘f = id | ∃F:C→C ∧ F∘F ≅ id |
| 10 | ⊞ | Stoichiometry | Homogeneous / same type | ∀a,b ∈ x. type(a) = type(b) | C monoidal ∧ ⊗:C×C→C |
| 11 | ⊡ | Winding | Well-founded / trivial | well_founded(x) | colim ℕ ≅ X |

### Dual Pairs

1. **(⊢, ⊣)** - Dimensionality ↔ Topology
2. **(≻, ≺)** - Coupling ↔ Parity
3. **(⋈, ⊤)** - Fidelity ↔ Kinetics
4. **(∈, ∋)** - Cardinality ↔ Composition
5. **(⊙, ⊥)** - Criticality ↔ Chirality
6. **(⊞, ⊡)** - Stoichiometry ↔ Winding

---

## Comparison: CL8NK vs CL9NK

### Feynman's_Sprinkler with CL8NK

```
CLINK Tuple: ⟨𐑦⋅𐑸⋅𐑾⋅𐑹⋅𐑐⋅𐑧⋅𐑲⋅𐑵⋅⊙⋅𐑫⋅𐑳⋅𐑟⟩
Promoted Atoms: BROADCAST_TRANSCENDENCE, PHI_C
Tier: O₁
Distance: 1.8167
```

### Feynman's_Sprinkler with CL9NK

```
CLINK Tuple: ⟨𐑛𐑥𐑑𐑬𐑐𐑪𐑔𐑝⊙𐑫𐑳𐑭⟩
Promoted Atoms: PRIME_POINT, BROADCAST_TRANSCENDENCE, PHI_C
Tier: O₁
Distance: 1.7454
```

**Key Difference:** CL9NK adds the `PRIME_POINT` promoted atom, which appears in the structural information but the primitive mappings remain the same (both navigators identify the same 12 primitives).

---

## Use Cases

### 1. Mathematical Research

- Automatically generate formal mathematical expressions from catalog entries
- Explore different mathematical disciplines for the same entry
- Generate Lean code for formal verification

### 2. Education

- Visualize how abstract catalog entries map to concrete mathematical concepts
- Compare different mathematical disciplines' representations
- Learn about category theory and set theory through concrete examples

### 3. Documentation

- Automatically generate documentation for catalog entries
- Export to Markdown for README files
- Export to LaTeX for academic papers

### 4. Software Development

- Integrate with existing Lean projects
- Use the Python API to build custom tools
- Extend with new mathematical disciplines

---

## Advanced Features

### 1. Validation

The system performs 6 validation checks:

1. **Structural Tuple** - Verifies the CLINK tuple format
2. **Primitive Coverage** - Ensures all 12 primitives are mapped
3. **Frobenius Identity** - Checks μ∘δ=id condition (structural)
4. **Type Consistency** - Placeholder for discipline-specific checks
5. **Dual Pair Covariance** - Validates all dual pairs are complete
6. **Tier Assessment** - Validates the tier value

Run with: `--validate`

### 2. Context Parameters

The system uses context parameters based on the entry and discipline:

```python
selector = ContextSelector()
context = selector.get_context_parameters("Feynman's_Sprinkler", "set_theory")
# Returns: {'entry': 'Feynman's_Sprinkler', 'discipline': 'set_theory', 'universe_type': 'ZFC', 'cardinality': 'infinite'}
```

### 3. Custom Disciplines

Add new mathematical disciplines by:

1. Creating a new register class (e.g., `registers/topology.py`)
2. Implementing the `DisciplineRegister` interface
3. Registering it in `registers/__init__.py`

---

## Project Structure

```
clink_decomposition/
├── __init__.py              # Package initialization, exports
├── clink_decompose.py       # CLI entry point
├── core/
│   ├── __init__.py
│   ├── models.py            # Data models (NavigatorOutput, MathematicalExpression, etc.)
│   ├── input_processor.py   # Navigator interface
│   ├── context_selector.py  # Discipline selection
│   └── composition_engine.py # Expression composition
├── registers/
│   ├── __init__.py          # Discipline registry
│   ├── base.py              # Abstract DisciplineRegister class
│   ├── set_theory.py        # Set theory primitive mappings
│   └── category_theory.py   # Category theory primitive mappings
├── output/
│   ├── __init__.py
│   └── generator.py         # Output formatting and file generation
├── lean/
│   ├── __init__.py
│   └── generator.py         # Lean code generation
└── validation/
    ├── __init__.py
    └── validator.py         # Validation checks
```

---

## Performance

- **Entry processing**: ~0.1-0.5 seconds (includes navigator execution)
- **Expression composition**: ~0.01-0.1 seconds
- **Output generation**: ~0.01-0.1 seconds
- **Validation**: ~0.01-0.1 seconds

Total time for a single decomposition: ~0.2-0.8 seconds

---

## Limitations and Future Work

### Current Limitations

1. The Frobenius identity check is structural only (verifies the condition appears in the output)
2. Type consistency checks are placeholders
3. Lean code uses `sorry` for unimplemented definitions
4. Only two disciplines are currently implemented (set_theory, category_theory)

### Future Enhancements

1. **Additional Disciplines**
   - Group Theory
   - Topology
   - Differential Geometry
   - Quantum Mechanics
   - Information Theory

2. **Enhanced Validation**
   - Mathematical type checking
   - Formal proof verification
   - Lean compilation checks

3. **Advanced Composition**
   - Custom composition operators per discipline
   - Weighted compositions
   - Context-aware composition

4. **Interactive Mode**
   - REPL interface
   - Step-by-step decomposition
   - Interactive discipline selection

5. **Web Interface**
   - Web-based decomposition tool
   - Visual representation of CLINK tuples
   - Interactive exploration

---

## References

- **Navigators**: `imscribing_grammar/navigators/cl8nk_navigator.py`, `cl9nk_navigator.py`
- **Catalog**: `~/.imscrbgrmr/catalog.json`
- **Documentation**: `CLINK_DECOMPOSITION_USER_GUIDE.md`
- **Technical Plan**: `G-mOMonadOS/docs/CL8NK_CL9NK_DECOMPOSITION_PLAN.md`

---

## Support

For issues or questions:

1. Check the user guide: `CLINK_DECOMPOSITION_USER_GUIDE.md`
2. Review the examples in this demonstration
3. Enable verbose output: `--verbose`
4. Verify your environment setup

---

*This demonstration is for the CL8NK/CL9NK Decomposition System*
*Last updated: 2026-11-14*
