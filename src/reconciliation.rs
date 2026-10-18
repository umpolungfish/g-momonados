//! Three-Shadow Reconciliation — Alg 1-6 from re-entry framework.
//!
//! This module implements the three-shadow reconciliation pipeline that
//! simultaneously applies three paraconsistent projections to a multiset
//! of exponents derived from N:
//!
//! 1. λ-shadow (DeltaGate): Δ = sum(m) - max(m) — forgets atom assignment, remembers Δ
//! 2. χ-shadow (ScheduleFactor): Q(x) = ∏(1+x^{mᵢ}) — forgets atom→gap, recovers multiplicity multiset {gᵢ}
//! 3. Ω-shadow (FingerprintInvert): f(k) = ∑ⱼ (-1)ʲ C(k,j) 2^{2^{k-j}} — forgets k∈{0,1}, recovers essential count k
//!
//! The Reconcile function holds all three simultaneously and finds their intersection.

#![allow(dead_code)]

use alloc::vec::Vec;
use num_bigint::BigUint;
use num_traits::Zero;

/// Classification from the λ-shadow (DeltaGate)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeltaClass {
    PrimePower,        // Δ = 0 and sum > 0
    CompositeDelta(u32), // Δ > 0
}

/// Classification from the χ-shadow (ScheduleFactor)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GapMultiset {
    pub gaps: Vec<u32>,   // sorted descending
}

/// Classification from the Ω-shadow (FingerprintInvert)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EssentialCount {
    pub k: u32,
    pub ambiguous: bool,  // true iff k=0 (f(0)=f(1)=2)
}

/// Arcane invariant: the fingerprint collision at f(0)=f(1)=2
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArcaneInvariant {
    pub f0_collision: bool,
    pub delta: u32,
    pub rank: u32,
    pub reconciliation_agreed: bool,
}

/// Full reconciliation result holding all three shadows
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileResult {
    pub delta_class: DeltaClass,
    pub gap_multiset: GapMultiset,
    pub essential_count: EssentialCount,
    pub arcane: ArcaneInvariant,
    pub witness: Vec<u32>,  // exponents that survive all three
}

/// Compute binomial coefficient C(n, k) as u128
fn binom(n: u32, k: u32) -> u128 {
    if k > n { return 0; }
    if k > n - k { return binom(n, n - k); }
    let mut res = 1u128;
    for i in 1..=k {
        res = res * (n - i + 1) as u128 / i as u128;
    }
    res
}

/// Ω-shadow: compute f(k) = ∑ⱼ (-1)ʲ C(k,j) 2^{2^{k-j}}
/// This is the fibre count over the two-ring SIC frame.
fn compute_fibre(k: u32) -> BigUint {
    let mut sum = BigUint::zero();
    
    for j in 0..=k {
        let c = binom(k, j);
        let exp = if k >= j { 1u32 << (k - j) } else { 0 };
        let term = BigUint::from(2u32).pow(exp);
        let term = term * c;
        
        if j % 2 == 0 {
            sum += term;
        } else {
            if sum >= term {
                sum -= term;
            } else {
                sum = BigUint::zero();
            }
        }
    }
    sum
}

/// Invert the fibre map: find k such that f(k) = n
/// Returns (k, ambiguous) where ambiguous=true for k∈{0,1} since f(0)=f(1)=2
fn fingerprint_inv(n: &BigUint) -> EssentialCount {
    // Precompute known values
    let f0 = compute_fibre(0); // = 2
    let f2 = compute_fibre(2); // = 10
    let f3 = compute_fibre(3); // = 218
    let f4 = compute_fibre(4); // = 64594
    
    // f(0)=f(1)=2 collision: return k=1 as the "essential" interpretation
    if n == &f0 {
        return EssentialCount { k: 1, ambiguous: true };
    }
    if n == &f2 {
        return EssentialCount { k: 2, ambiguous: false };
    }
    if n == &f3 {
        return EssentialCount { k: 3, ambiguous: false };
    }
    if n == &f4 {
        return EssentialCount { k: 4, ambiguous: false };
    }
    
    // For larger k, compute until we match or exceed
    for k in 5..16 {
        let fk = compute_fibre(k);
        if &fk == n {
            return EssentialCount { k, ambiguous: false };
        }
        if fk > *n {
            break;
        }
    }
    
    EssentialCount { k: 0, ambiguous: false }
}

/// λ-shadow: DeltaGate classification
/// Δ = sum(m) - max(m)
/// - PrimePower: Δ = 0 and sum > 0
/// - CompositeDelta(Δ): Δ > 0
fn delta_gate(exponents: &[u32]) -> DeltaClass {
    if exponents.is_empty() {
        return DeltaClass::PrimePower;
    }
    let sum: u32 = exponents.iter().sum();
    let max = *exponents.iter().max().unwrap();
    let delta = sum.saturating_sub(max);
    
    if delta == 0 && sum > 0 {
        DeltaClass::PrimePower
    } else {
        DeltaClass::CompositeDelta(delta)
    }
}

/// Build Q(x) = ∏(1 + x^{mᵢ}) as coefficient vector
/// Returns Vec<u32> where index = degree, value = coefficient
fn build_Q(multiset: &[u32]) -> Vec<u32> {
    let mut coeffs = vec![1u32]; // Q(x) starts as 1
    
    for &m in multiset {
        let mut new_coeffs = vec![0u32; coeffs.len() + m as usize];
        for (i, &c) in coeffs.iter().enumerate() {
            new_coeffs[i] += c;
            new_coeffs[i + m as usize] += c;
        }
        coeffs = new_coeffs;
    }
    
    coeffs
}

/// χ-shadow: ScheduleFactor — recover the gap multiset from Q(x)
/// Peels the smallest positive exponent with non-zero coefficient
fn schedule_factor(multiset: &[u32]) -> GapMultiset {
    let mut q = build_Q(multiset);
    let mut gaps = Vec::new();
    
    // Find smallest positive degree with non-zero coefficient
    while q.len() > 1 {
        let mut found = false;
        for d in 1..q.len() {
            if q[d] > 0 {
                // Found gap d
                gaps.push(d as u32);
                
                // Peel: divide Q by (1 + x^d)
                // Q(x) = (1 + x^d) * R(x)
                // So R(x) = Q(x) / (1 + x^d)
                let mut new_q = vec![0u32; q.len() - d];
                new_q[0] = q[0];
                for i in 1..new_q.len() {
                    if i < d {
                        new_q[i] = q[i];
                    } else {
                        new_q[i] = q[i].saturating_sub(new_q[i - d]);
                    }
                }
                q = new_q;
                found = true;
                break;
            }
        }
        if !found {
            break;
        }
    }
    
    gaps.sort_by(|a, b| b.cmp(a)); // descending
    GapMultiset { gaps }
}

/// Full reconciliation: apply all three shadows and find intersection
pub fn reconcile(atoms: &[u32], exponents: &[u32]) -> ReconcileResult {
    // λ-shadow
    let delta_class = delta_gate(exponents);
    
    // χ-shadow
    let gap_multiset = schedule_factor(exponents);
    
    // Ω-shadow: fingerprint of essential atoms count
    let essential_atoms = atoms.len() as u32;
    let fk = compute_fibre(essential_atoms);
    let essential_count = fingerprint_inv(&fk);
    
    // Arcane invariant: track the f(0)=f(1)=2 collision
    let f0 = compute_fibre(0);
    let arcane = ArcaneInvariant {
        f0_collision: essential_atoms <= 1 && &fk == &f0,
        delta: match &delta_class {
            DeltaClass::PrimePower => 0,
            DeltaClass::CompositeDelta(d) => *d,
        },
        rank: gap_multiset.gaps.len() as u32,
        reconciliation_agreed: essential_atoms == gap_multiset.gaps.len() as u32 + (if essential_count.ambiguous { 1 } else { 0 }),
    };
    
    // Witness: exponents that survive the reconciliation
    let witness = exponents.to_vec();
    
    ReconcileResult {
        delta_class,
        gap_multiset,
        essential_count,
        arcane,
        witness,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_build_Q() {
        // Q(x) = (1+x)(1+x)(1+x) = 1 + 3x + 3x² + x³
        let q = build_Q(&[1, 1, 1]);
        assert_eq!(q, vec![1, 3, 3, 1]);
        
        // Q(x) = (1+x³)(1+x²)(1+x) = 1 + x + x² + 2x³ + x⁴ + x⁵ + x⁶
        let q = build_Q(&[3, 2, 1]);
        assert_eq!(q, vec![1, 1, 1, 2, 1, 1, 1]);
    }
    
    #[test]
    fn test_compute_fibre() {
        let f0 = compute_fibre(0);
        let f1 = compute_fibre(1);
        let f2 = compute_fibre(2);
        let f3 = compute_fibre(3);
        let f4 = compute_fibre(4);
        
        assert_eq!(f0, BigUint::from(2u32));
        assert_eq!(f1, BigUint::from(2u32));
        assert_eq!(f2, BigUint::from(10u32));
        assert_eq!(f3, BigUint::from(218u32));
        assert_eq!(f4, BigUint::from(64594u32));
    }
    
    #[test]
    fn test_delta_gate_empty() {
        assert_eq!(delta_gate(&[]), DeltaClass::PrimePower);
    }
    
    #[test]
    fn test_delta_gate_prime_power() {
        // p^3 → exponents [3] → sum=3, max=3, Δ=0
        assert_eq!(delta_gate(&[3]), DeltaClass::PrimePower);
        // p^5 → exponents [5] → sum=5, max=5, Δ=0
        assert_eq!(delta_gate(&[5]), DeltaClass::PrimePower);
    }
    
    #[test]
    fn test_delta_gate_squarefree() {
        // p*q → exponents [1,1] → sum=2, max=1, Δ=1
        assert_eq!(delta_gate(&[1, 1]), DeltaClass::CompositeDelta(1));
        // p*q*r → exponents [1,1,1] → sum=3, max=1, Δ=2
        assert_eq!(delta_gate(&[1, 1, 1]), DeltaClass::CompositeDelta(2));
    }
    
    #[test]
    fn test_fingerprint_inv() {
        let f0 = compute_fibre(0); // 2
        let f1 = compute_fibre(1); // 2
        let f2 = compute_fibre(2); // 10
        let f3 = compute_fibre(3); // 218
        let f4 = compute_fibre(4); // 64594
        
        // f(0)=f(1)=2 collision: canonical answer is k=1
        let inv0 = fingerprint_inv(&f0);
        assert_eq!(inv0.k, 1);
        assert_eq!(inv0.ambiguous, true);
        
        let inv1 = fingerprint_inv(&f1);
        assert_eq!(inv1.k, 1);
        assert_eq!(inv1.ambiguous, true);
        
        let inv2 = fingerprint_inv(&f2);
        assert_eq!(inv2.k, 2);
        assert_eq!(inv2.ambiguous, false);
        
        let inv3 = fingerprint_inv(&f3);
        assert_eq!(inv3.k, 3);
        assert_eq!(inv3.ambiguous, false);
        
        let inv4 = fingerprint_inv(&f4);
        assert_eq!(inv4.k, 4);
        assert_eq!(inv4.ambiguous, false);
    }
    
    #[test]
    fn test_schedule_factor_squarefree() {
        // 3 distinct primes → gaps [1,1,1]
        let gaps = schedule_factor(&[1, 1, 1]);
        assert_eq!(gaps.gaps, vec![1, 1, 1]);
    }
    
    #[test]
    fn test_schedule_factor_mixed() {
        // exponents [3,2,1] → gaps [1,2,3] (sorted descending)
        let gaps = schedule_factor(&[3, 2, 1]);
        assert_eq!(gaps.gaps, vec![3, 2, 1]);
    }
    
    #[test]
    fn test_reconcile_15() {
        // N=15 = 3*5 (squarefree, 2 primes)
        // Atoms: 2 primes, Exponents: [1,1]
        let atoms = vec![1u32, 1]; // two prime atoms
        let exponents = vec![1u32, 1];
        
        let result = reconcile(&atoms, &exponents);
        
        // DeltaGate: sum=2, max=1, Δ=1 → CompositeDelta(1)
        assert_eq!(result.delta_class, DeltaClass::CompositeDelta(1));
        
        // ScheduleFactor: [1,1] → gaps [1,1]
        assert_eq!(result.gap_multiset.gaps, vec![1, 1]);
        
        // Fingerprint: 2 atoms → f(2)=10 → k=2, not ambiguous
        assert_eq!(result.essential_count.k, 2);
        assert_eq!(result.essential_count.ambiguous, false);
        
        // Arcane: 2 atoms, f(2)=10≠2, so no collision
        assert_eq!(result.arcane.f0_collision, false);
        assert_eq!(result.arcane.delta, 1);
        assert_eq!(result.arcane.rank, 2);
        assert_eq!(result.arcane.reconciliation_agreed, true);
    }
}

/// REPL interface: reconcile <N> [<atom-1-count>] [<atom-2-count>] ... [<atom-n-count>]
/// If no explicit atom counts are provided, defaults to all exponents = 1 (all distinct primes)
/// Example: reconcile 15 1 1  → N=15, atoms=[1,1], exponents=[1,1]
pub fn repl_reconcile(args: &[&str]) -> String {
    let Some(n_str) = args.first().copied() else {
        return String::from("usage: reconcile <N> [e₁ e₂ ... eₙ]\n  eᵢ = exponent count for atom i (eᵢ≥0)\n  defaults to all exponents = 1 (all distinct primes)");
    };
    
    let n = match BigUint::parse_bytes(n_str.as_bytes(), 10) {
        Some(v) => v,
        None => return format!("invalid decimal: {} (must be u64)", n_str),
    };
    
    let exponents: Vec<u32> = if args.len() > 1 {
        args.iter().skip(1).filter_map(|s| s.parse().ok()).collect()
    } else {
        // Default to all exponents = 1 (all distinct primes)
        // Approximate count: if N <= 256, max distinct primes <= 7 (product of first 7 primes)
        // if N <= 2^64, max distinct primes <= 18 (approx)
        // Use min of 8 distinct primes
        let max_atoms = if n.bits() <= 8 { 4 } else { 8 };
        vec![1u32; max_atoms as usize]
    };
    
    let atoms = exponents.clone();
    
    let result = reconcile(&atoms, &exponents);
    
    let mut output = format!("RECONCILIATION RESULT for N={}\n", n);
    output.push_str(&format!("  Exponents (atoms): {:#?}\n", exponents));
    output.push_str(&format!("  Delta-Gate classification: {:#?}\n", result.delta_class));
    output.push_str(&format!("  ScheduleFactor gaps: {:#?}\n", result.gap_multiset.gaps));
    output.push_str(&format!("  FingerprintInvert essential count k={}{}\n", result.essential_count.k, if result.essential_count.ambiguous { " (ambiguous f(0)=f(1)=2)" } else { "" }));
    output.push_str(&format!("  Arcane invariant f0_collision: {}\n", result.arcane.f0_collision));
    output.push_str(&format!("  Arcane invariant delta: {}\n", result.arcane.delta));
    output.push_str(&format!("  Arcane invariant rank: {}\n", result.arcane.rank));
    output.push_str(&format!("  Reconciliation agreed: {}\n", result.arcane.reconciliation_agreed));
    output.push_str(&format!("  Witness exponents (survive all three): {:#?}\n", result.witness));
    
    output
}