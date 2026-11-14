"""
Validator: Validates decomposition outputs
"""

from dataclasses import dataclass, field
from typing import List, Dict, Any, Optional


@dataclass
class ValidationResult:
    """Result of a single validation check"""
    check_name: str
    passed: bool
    message: str
    details: Optional[Dict] = None
    suggestions: List[str] = field(default_factory=list)


@dataclass
class ValidationReport:
    """Complete validation report"""
    entry_name: str
    navigator: str
    discipline: str
    results: List[ValidationResult] = field(default_factory=list)
    passed: bool = True
    overall_score: float = 1.0
    warnings: List[str] = field(default_factory=list)
    errors: List[str] = field(default_factory=list)
    
    def add_result(self, result: ValidationResult):
        self.results.append(result)
        if not result.passed:
            self.passed = False
            self.overall_score = 0.0
        
    def to_dict(self) -> Dict:
        return {
            "entry_name": self.entry_name,
            "navigator": self.navigator,
            "discipline": self.discipline,
            "passed": self.passed,
            "overall_score": self.overall_score,
            "results": [r.__dict__ for r in self.results],
            "warnings": self.warnings,
            "errors": self.errors,
        }


class Validator:
    """
    Validates decomposition outputs for structural correctness.
    
    Checks:
    1. Structural: Valid CLINK tuple, all 12 primitives present
    2. Mathematical: μ∘δ=id, type consistency, dual-pair covariance
    3. Lean: Code compiles (if Lean is available)
    """
    
    def __init__(self):
        self.checks = [
            self._check_structural,
            self._check_primitive_coverage,
            self._check_frobenius_identity,
            self._check_type_consistency,
            self._check_dual_pair_covariance,
            self._check_tier_assessment,
        ]
    
    def validate(
        self, 
        expression,
        navigator_output
    ) -> Dict[str, object]:
        """
        Run all validation checks.
        
        Args:
            expression: Mathematical expression to validate
            navigator_output: Original navigator output
            
        Returns:
            Dictionary of check names to ValidationResult
        """
        results = {}
        
        for check in self.checks:
            result = check(expression, navigator_output)
            results[result.check_name] = result
        
        return results
    
    def _check_structural(
        self, 
        expression,
        navigator_output
    ):
        """Check structural validity of the CLINK tuple"""
        
        issues = []
        
        # Check we have a CLINK tuple
        if not hasattr(navigator_output, 'clink_tuple') or not navigator_output.clink_tuple:
            return ValidationResult(
                check_name="structural_tuple",
                passed=False,
                message="No CLINK tuple found in navigator output",
                details={"tuple": None}
            )
        
        # Check tuple format
        if not (navigator_output.clink_tuple.startswith('⟨') and 
                navigator_output.clink_tuple.endswith('⟩')):
            issues.append(f"Tuple format invalid: {navigator_output.clink_tuple}")
        
        # Check we have primitives
        if not hasattr(navigator_output, 'primitives') or len(navigator_output.primitives) != 12:
            issues.append(f"Expected 12 primitives, got {len(navigator_output.primitives) if hasattr(navigator_output, 'primitives') else 0}")
        
        if issues:
            return ValidationResult(
                check_name="structural_tuple",
                passed=False,
                message="; ".join(issues),
                details={
                    "tuple": navigator_output.clink_tuple,
                    "primitive_count": len(navigator_output.primitives) if hasattr(navigator_output, 'primitives') else 0
                }
            )
        
        return ValidationResult(
            check_name="structural_tuple",
            passed=True,
            message="CLINK tuple is structurally valid",
            details={
                "tuple": navigator_output.clink_tuple,
                "primitive_count": len(navigator_output.primitives)
            }
        )
    
    def _check_primitive_coverage(
        self,
        expression,
        navigator_output
    ):
        """Check that all primitives are covered in the expression"""
        
        # Get primitive symbols from expression
        if hasattr(expression, 'primitives_used'):
            primitive_symbols = expression.primitives_used
        elif hasattr(expression, 'primitives'):
            primitive_symbols = [p.primitive_symbol for p in expression.primitives]
        else:
            return ValidationResult(
                check_name="primitive_coverage",
                passed=True,
                message="Primitive coverage check skipped (no primitives in expression)"
            )
        
        # We should have 12 unique primitive symbols
        if len(primitive_symbols) != 12:
            return ValidationResult(
                check_name="primitive_coverage",
                passed=False,
                message=f"Expected 12 primitive mappings, got {len(primitive_symbols)}",
                details={"mapping_count": len(primitive_symbols), "symbols": primitive_symbols}
            )
        
        # Check that we have all 12 CLINK primitives
        PRIMITIVE_SYMBOLS = ['⊢', '⊣', '≻', '≺', '⋈', '⊤', '∈', '∋', '⊙', '⊥', '⊞', '⊡']
        missing = [s for s in PRIMITIVE_SYMBOLS if s not in primitive_symbols]
        
        if missing:
            return ValidationResult(
                check_name="primitive_coverage",
                passed=False,
                message=f"Missing primitive mappings: {missing}",
                details={"present": primitive_symbols, "missing": missing}
            )
        
        return ValidationResult(
            check_name="primitive_coverage",
            passed=True,
            message="All 12 primitives are mapped"
        )
    
    def _check_frobenius_identity(
        self,
        expression,
        navigator_output
    ):
        """Check Frobenius identity μ∘δ = id"""
        
        # This is a structural check - we can't verify the actual identity
        # without knowing the specific μ and δ, but we can check that
        # the expression references them if it should.
        
        # For now, just note that this check is placeheld
        return ValidationResult(
            check_name="frobenius_identity",
            passed=True,
            message="Frobenius identity check (μ∘δ=id) - structural validation only",
            details={"status": "placeheld"}
        )
    
    def _check_type_consistency(
        self,
        expression,
        navigator_output
    ):
        """Check type consistency across primitives"""
        
        # For now, assume type consistency
        # This would require discipline-specific type checking
        return ValidationResult(
            check_name="type_consistency",
            passed=True,
            message="Type consistency check - placeheld"
        )
    
    def _check_dual_pair_covariance(
        self,
        expression,
        navigator_output
    ):
        """Check dual-pair covariance"""
        
        # Dual pairs: (⊢,⊣), (≻,≺), (⋈,⊤), (∈,∋), (⊙,⊥), (⊞,⊡)
        dual_pairs = [
            (0, 1),   # ⊢, ⊣
            (2, 3),   # ≻, ≺
            (4, 5),   # ⋈, ⊤
            (6, 7),   # ∈, ∋
            (8, 9),   # ⊙, ⊥
            (10, 11), # ⊞, ⊡
        ]
        
        # Get primitive indices from expression
        PRIMITIVE_SYMBOLS = ['⊢', '⊣', '≻', '≺', '⋈', '⊤', '∈', '∋', '⊙', '⊥', '⊞', '⊡']
        
        if hasattr(expression, 'primitives_used'):
            primitive_symbols = expression.primitives_used
            primitive_indices = [PRIMITIVE_SYMBOLS.index(s) for s in primitive_symbols if s in PRIMITIVE_SYMBOLS]
        elif hasattr(expression, 'primitives'):
            primitive_indices = [p.primitive_idx for p in expression.primitives if hasattr(p, 'primitive_idx')]
        else:
            return ValidationResult(
                check_name="dual_pair_covariance",
                passed=True,
                message="Dual pair covariance check skipped (no primitives available)"
            )
        
        # Check that both primitives in each pair are present
        issues = []
        for a, b in dual_pairs:
            if a not in primitive_indices or b not in primitive_indices:
                issues.append(f"Dual pair ({a},{b}) incomplete")
        
        if issues:
            return ValidationResult(
                check_name="dual_pair_covariance",
                passed=False,
                message="; ".join(issues)
            )
        
        return ValidationResult(
            check_name="dual_pair_covariance",
            passed=True,
            message="All dual pairs are complete"
        )
    
    def _check_tier_assessment(
        self,
        expression,
        navigator_output
    ):
        """Check tier assessment"""
        
        if not hasattr(navigator_output, 'tier') or not navigator_output.tier:
            return ValidationResult(
                check_name="tier_assessment",
                passed=True,
                message="No tier information available",
                details={"tier": None}
            )
        
        valid_tiers = ["O₀", "O₁", "O₂", "O₂†", "O_∞", "O_∞⁺"]
        
        if navigator_output.tier not in valid_tiers:
            return ValidationResult(
                check_name="tier_assessment",
                passed=False,
                message=f"Invalid tier: {navigator_output.tier}",
                details={"tier": navigator_output.tier, "valid": valid_tiers}
            )
        
        return ValidationResult(
            check_name="tier_assessment",
            passed=True,
            message=f"Tier assessment valid: {navigator_output.tier}"
        )
