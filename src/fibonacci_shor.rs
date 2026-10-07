#![allow(dead_code)]
//! Exact Shor gate planning and exploratory Fibonacci braid routing.
//!
//! The gate plan preserves controlled modular multiplication and signed IQFT
//! phase denominators. Physical assembly reports a missing calibrated lowering
//! until every gate's computational action and leakage have been measured.
//! The standalone seed helpers are exploratory words, not certified gates.

use alloc::vec::Vec;
use alloc::vec;

// ── Gate decomposition for Shor's circuit ─────────────────────────────

/// Controlled-U^{2^k} gate: |c⟩|t⟩ → |c⟩U^{2^k}|t⟩ if c=1
/// For modular exponentiation, U = multiplication by a mod N.
/// In the qubit model, this is decomposed into:
///   - Phase estimation uses controlled-U^{2^0}, controlled-U^{2^1}, ..., controlled-U^{2^{n-1}}
///   - Each controlled-U^{2^k} is a modular multiplication by a^{2^k} mod N

/// Fibonacci anyon strand count for k qubits.
/// 1 qubit needs 4 anyons = 3 strands (fusion dim 2).
/// 2 qubits need 7 anyons = 6 strands (fusion dim 8).
pub fn strands_for_qubits(k: usize) -> usize {
    if k == 0 { return 3; }
    // Each qubit adds 3 strands (4 anyons, but shared boundaries reduce by 1)
    3 * k + 1
}

/// Estimate braid word length for Shor's algorithm on N with n qubits.
/// H-layer: n × SK-depth (~50 braids each)
/// Controlled-U chain: n × O(n²) controlled-phase gates
/// Inverse QFT: O(n²) controlled-phase gates
/// Each gate: ~50-200 braid generators
pub fn estimate_braid_length(n_qubits: usize) -> usize {
    let sk_depth = 50;  // Solovay-Kitaev base depth
    let n = n_qubits;
    // H-layer
    let h_count = n * sk_depth;
    // Controlled-U chain: n controlled-U's, each ~ n² * 2 controlled-phase gates
    let cu_count = n * (n * n * 2) * sk_depth;
    // Inverse QFT: n*(n-1)/2 controlled-phase gates
    let iqft_count = (n * n.saturating_sub(1) / 2) * sk_depth;
    h_count + cu_count + iqft_count
}

// ── Shor circuit parameters ───────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct ShorCircuitParams {
    pub n_qubits: usize,        // Period register qubits
    pub n_work_qubits: usize,   // Work register qubits (for modular arithmetic)
    pub n_total_qubits: usize,  // Total qubits
    pub a: u64,                 // Base for exponentiation
    pub n_val: u64,             // Number to factor
    pub period: Option<u64>,    // Unset during construction; requires readout
    pub estimated_braid_len: usize,
    pub strands: usize,
    pub fusion_dim: usize,      // Fusion space dimension
}

impl ShorCircuitParams {
    pub fn new(n_qubits: usize, a: u64, n_val: u64) -> Self {
        let n_work = if n_val <= 1 { 1 } else {
            let mut bits = 0; let mut v = n_val - 1;
            while v > 0 { bits += 1; v >>= 1; }
            bits.max(1)
        };
        let n_total = n_qubits + n_work;
        let strands = strands_for_qubits(n_total);
        let fusion_dim = fibonacci_dim(strands);
        let braid_len = estimate_braid_length(n_qubits);

        ShorCircuitParams {
            n_qubits, n_work_qubits: n_work, n_total_qubits: n_total,
            a, n_val, period: None,
            estimated_braid_len: braid_len,
            strands,
            fusion_dim,
        }
    }
}

fn fibonacci_dim(strands: usize) -> usize {
    if strands <= 1 { return 1; }
    if strands == 2 { return 1; }
    let n = strands - 1; // Fusion space for n anyons = F_{n-1}
    let mut a = 1usize;
    let mut b = 1usize;
    for _ in 2..n {
        let t = a.saturating_add(b);
        a = b;
        b = t;
    }
    b
}

// ── Fibonacci anyon braid words for Shor gates ────────────────────────

/// Generate the braid word for a Hadamard layer on n qubits.
/// Exploratory seed; its Hadamard action has not been calibrated.
/// For multiple qubits, H is applied in parallel on independent 3-strand blocks.
pub fn hadamard_layer_braid(n_qubits: usize) -> Vec<i32> {
    // Single-qubit H: approximate as σ₁⁻¹ σ₂ σ₁ (Fibonacci anyon H)
    // This is a braid word in the 3-strand representation
    // σ_i are generators, negative indices are inverses
    let mut word = Vec::new();
    for q in 0..n_qubits {
        let base = (q * 3) as i32;
        // H ≈ σ_{base+1}^{-1} σ_{base+2} σ_{base+1}
        word.push(-(base + 2));  // σ_i^{-1}
        word.push(base + 3);      // σ_{i+1}
        word.push(base + 2);      // σ_i
    }
    word
}

/// Generate the braid word for a T gate on qubit q.
/// Exploratory seed; its T-gate action has not been calibrated.
pub fn t_gate_braid(qubit: usize) -> Vec<i32> {
    // T gate SK approximation (depth-4 baseline)
    let base = (qubit * 3) as i32;
    vec![
        base + 1, base + 2, base + 1, base + 2,
        -(base + 1), base + 2, base + 1,
        base + 2, -(base + 1), base + 2,
    ]
}

/// Route an exploratory two-block seed between qubits c and t.
/// In the Fibonacci model, this requires braiding anyons from different qubit blocks.
/// The minimum non-trivial braiding between two 3-strand blocks needs 6 strands.
pub fn controlled_phase_braid(control: usize, target: usize) -> Vec<i32> {
    assert_ne!(control, target, "a two-qubit braid needs distinct blocks");
    let low = control.min(target);
    let high = control.max(target);
    let cross1 = i32::try_from(3 * low + 3).expect("strand index exceeds i32");
    // Transport the distant block to the boundary, apply the local seed,
    // then undo transport. Each exchange crosses all nine strand pairs.
    // This repairs support; the seed still requires gate-action calibration.
    let mut route = Vec::new();
    for block in ((low + 1)..high).rev() {
        let base = i32::try_from(3 * block).expect("strand index exceeds i32");
        for right in 0..3 {
            for left in (1..=3).rev() {
                route.push(base + right + left);
            }
        }
    }
    let mut word = route.clone();
    word.extend([
        cross1, -(cross1 + 1), cross1,
        cross1 + 1, -cross1, cross1 + 1,
        cross1, -(cross1 + 1), cross1,
    ]);
    word.extend(route.iter().rev().map(|g| -g));
    word
}

/// Inverse QFT braid word for n qubits.
/// IQFT = sequence of controlled-R_k gates followed by H on each qubit.
pub fn inverse_qft_braid(n_qubits: usize) -> Vec<i32> {
    let mut word = Vec::new();
    // For each qubit: apply controlled-R_k with subsequent qubits, then H
    for i in 0..n_qubits {
        // Apply controlled-R_{k} gates: controlled-Z/2^{k} = controlled-phase(-π/2^{k-1})
        for j in (i + 1)..n_qubits {
            let k = j - i + 1; // R_k = controlled-Z/2^{k}
            // Approximate R_k as repeated controlled-Z
            for _ in 0..k {
                word.extend(controlled_phase_braid(i, j));
            }
        }
        // Apply H to qubit i
        let base = (i * 3) as i32;
        word.push(-(base + 2));
        word.push(base + 3);
        word.push(base + 2);
    }
    word
}

pub use crate::fibonacci_shor_plan::ShorGate;

pub fn shor_gate_plan(n_qubits: usize, a: u64, modulus: u64) -> Result<Vec<ShorGate>, &'static str> {
    crate::fibonacci_shor_plan::shor_gate_plan_for_source(n_qubits,
        &num_bigint::BigUint::from(a), &num_bigint::BigUint::from(modulus))
}

// ── Full Shor braid word assembly ─────────────────────────────────────

#[derive(Clone, Debug)]
pub struct FibonacciShorBraid {
    pub params: ShorCircuitParams,
    pub gates: Vec<ShorGate>,
    pub lowering_error: Option<&'static str>,
    pub hadamard_word: Vec<i32>,
    pub mod_exp_word: Vec<i32>,    // Controlled-U chain
    pub iqft_word: Vec<i32>,       // Inverse QFT
    pub total_word: Vec<i32>,
    pub total_length: usize,
}

/// Plan Shor gates and report whether their physical lowering is available.
/// Circuit: |0⟩^⊗n → H^⊗n → Controlled-U^{2^i} → IQFT → measure
pub fn assemble_shor_braid(n_qubits: usize, a: u64, n_val: u64) -> FibonacciShorBraid {
    let params = ShorCircuitParams::new(n_qubits, a, n_val);

    // Preserve exact requests. The former bit-dependent diagonal seeds did
    // not implement modular multiplication. No physical word is released until
    // every requested gate has a calibrated lowering and leakage measurement.
    let (gates, error) = match shor_gate_plan(n_qubits, a, n_val) {
        Ok(gates) => (gates, "Shor gate plan is available; calibrated Fibonacci braid lowering is required"),
        Err(error) => (Vec::new(), error),
    };
    FibonacciShorBraid {
        params, gates, lowering_error: Some(error),
        hadamard_word: Vec::new(), mod_exp_word: Vec::new(),
        iqft_word: Vec::new(), total_word: Vec::new(), total_length: 0,
    }
}

fn mod_pow(mut base: u64, mut exp: u64, modulus: u64) -> u64 {
    if modulus == 0 { return 0; }
    if modulus == 1 { return 0; }
    let mut result: u64 = 1;
    base %= modulus;
    while exp > 0 {
        if exp & 1 != 0 { result = ((result as u128 * base as u128) % modulus as u128) as u64; }
        exp >>= 1;
        base = ((base as u128 * base as u128) % modulus as u128) as u64;
    }
    result
}

// ── Quantum advantage certification ───────────────────────────────────

/// The braid circuit's real, graded quantities — no boolean verdict and no
/// "simulation" baseline. Two reasons, both empirical: the universe's couplings
/// are a signed continuum (ig-pulse coupling.json: of 7018 measured couplings
/// 81% are strictly graded, range −0.912..1.0, only 17% at the ±1/0 extremes),
/// and this kernel's native logic is Belnap FOUR, not two-valued. So this reports
/// the numbers and leaves any collapse to a caller that actually needs one.
/// Advantage here is TOPOLOGICAL (QM_as_Shadow_of_FDE §11): it is carried by the
/// logical-qubit capacity of the anyonic encoding, not by how hard the circuit is
/// to simulate — the whole method replaces simulation, so simulability cannot be
/// its yardstick.
#[derive(Clone, Debug)]
pub struct AdvantageCert {
    pub t_gate_error: f64,
    pub n_two_qubit_gates: usize,
    pub eps_2q: f64,
    pub accumulated_error: f64,   // t_gate × n_2q × ε_2q — continuous, not a threshold
    pub logical_qubits: usize,    // topologically protected capacity = ⌊log2(fusion_dim)⌋
}

pub fn certify_advantage(params: &ShorCircuitParams) -> AdvantageCert {
    let t_gate_err = 4e-3;     // T-gate error (magic state distillation)
    let eps_2q = 1e-2;         // Two-qubit gate error
    let n_2q = params.estimated_braid_len;

    // Accumulated gate error across the braid: a continuous quantity, reported raw.
    let accumulated_error = t_gate_err * (n_2q as f64) * eps_2q;

    // Topological capacity: the anyonic fusion space holds ⌊log2(dim)⌋ logical
    // qubits with intrinsic protection. This is where the advantage actually lives.
    let logical_qubits = if params.fusion_dim > 0 { params.fusion_dim.ilog2() as usize } else { 0 };

    AdvantageCert {
        t_gate_error: t_gate_err,
        n_two_qubit_gates: n_2q,
        eps_2q,
        accumulated_error,
        logical_qubits,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fibonacci_dim() {
        assert_eq!(fibonacci_dim(4), 2);  // F_3 = 2
        assert_eq!(fibonacci_dim(7), 8);  // F_6 = 8
        assert_eq!(fibonacci_dim(11), 55); // F_10 = 55
    }

    #[test]
    fn test_strands_for_qubits() {
        assert_eq!(strands_for_qubits(1), 4);
        assert_eq!(strands_for_qubits(2), 7);
        assert_eq!(strands_for_qubits(4), 13);
    }

    #[test]
    fn test_shor_n15_params() {
        let p = ShorCircuitParams::new(4, 7, 15);
        assert_eq!(p.n_qubits, 4);
        assert_eq!(p.n_work_qubits, 4);
        assert_eq!(p.period, None);
        assert_eq!(p.strands, 25); // 3*8+1
    }

    #[test]
    fn test_shor_n15_braid() {
        let b = assemble_shor_braid(4, 7, 15);
        assert!(!b.gates.is_empty());
        assert!(b.lowering_error.is_some());
        assert!(b.total_word.is_empty());
        assert_eq!(b.params.period, None);
    }

    #[test]
    fn test_advantage_n15() {
        let p = ShorCircuitParams::new(4, 7, 15);
        let cert = certify_advantage(&p);
        // No boolean verdict: the certificate reports graded quantities. N=15's
        // braid is 6900 generators, so the accumulated gate error is a real
        // continuous number (~0.276), and the topological capacity is whatever the
        // fusion space holds. Advantage, where it exists, is topological — carried
        // by logical_qubits — not a threshold on how simulable the circuit is.
        assert_eq!(cert.n_two_qubit_gates, estimate_braid_length(4));
        assert!((cert.accumulated_error - 0.276).abs() < 1e-9,
            "accumulated error should be a continuous value, got {}", cert.accumulated_error);
        assert_eq!(cert.logical_qubits, p.fusion_dim.ilog2() as usize);
    }

    #[test]
    fn test_hadamard_layer() {
        let word = hadamard_layer_braid(2);
        // Two qubits: 6 strand generators
        assert_eq!(word.len(), 6); // 3 per qubit
    }

    #[test]
    fn test_t_gate() {
        let word = t_gate_braid(0);
        assert_eq!(word.len(), 10);
    }

    #[test]
    fn distant_target_is_transported_and_untransported() {
        let word = controlled_phase_braid(0, 4);
        assert_eq!(word.len(), 63);
        assert_ne!(word, controlled_phase_braid(0, 1));
        let route_len = (word.len() - 9) / 2;
        let mut labels: Vec<usize> = (0..15).collect();
        for g in &word[..route_len] {
            let i = g.unsigned_abs() as usize - 1;
            labels.swap(i, i + 1);
        }
        assert_eq!(&labels[3..6], &[12, 13, 14]);
        for g in &word[route_len + 9..] {
            let i = g.unsigned_abs() as usize - 1;
            labels.swap(i, i + 1);
        }
        assert_eq!(labels, (0..15).collect::<Vec<_>>());
        assert_eq!(word, controlled_phase_braid(4, 0));
    }

    #[test]
    fn modular_power_does_not_overflow() {
        assert_eq!(mod_pow(u64::MAX - 1, 2, u64::MAX), 1);
        assert_eq!(estimate_braid_length(0), 0);
    }

    #[test]
    fn test_controlled_phase() {
        let word = controlled_phase_braid(0, 1);
        assert_eq!(word.len(), 9);
    }
}
