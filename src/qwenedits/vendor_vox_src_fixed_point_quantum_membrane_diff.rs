--- vendor/vox/src/fixed_point_quantum_membrane.rs (原始)


+++ vendor/vox/src/fixed_point_quantum_membrane.rs (修改后)
//! Fixed-point quantum membrane — vendored shim reconstructing the interface
//! this workspace's consumers require from the on-device Vox tree.
//!
//! The canonical module lives in `../Vox` (`fixed_point_quantum_membrane`).
//! This reconstruction preserves the exact public surface used by
//! `g-momonados`: `FixedPointQuantumMembrane::from_n`, `from_n_with_base`,
//! `prepare_structural_execution`, and the prepared program's
//! `measure_factor_pair`. All arithmetic stays over numeral tapes through the
//! shared `morphism_factor` tables; a tape is never decoded into a machine
//! integer, matching the parent kernel's discipline.

use crate::fixed_point_quantum_phase::execution::Program;
use crate::morphism_factor::{add, decimal_to_tape, divmod, mul, trim, two, zero, Tape};
use alloc::string::{String, ToString};

/// A sealed fixed-point quantum membrane over a source numeral tape N.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixedPointQuantumMembrane {
    n: Tape,
    base: Tape,
}

impl FixedPointQuantumMembrane {
    /// Seal a membrane with the default phase base 2.
    pub fn from_n(n: &Tape) -> Result<Self, String> {
        Self::from_n_with_base(n, &trim(two()))
    }

    /// Seal a membrane with an explicit radix base tape.
    pub fn from_n_with_base(n: &Tape, base: &Tape) -> Result<Self, String> {
        if n.is_empty() || base.is_empty() {
            return Err("malformed prepared source".to_string());
        }
        Ok(Self { n: n.clone(), base: base.clone() })
    }

    /// Prepare the structural execution program carried by this membrane.
    pub fn prepare_structural_execution(&self) -> Result<Program, String> {
        crate::fixed_point_quantum_phase::execution::prepare(&self.n, &self.base)
    }
}

/// One measured factor pair closing against the sealed source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeasuredFactorPair {
    pub p: Tape,
    pub q: Tape,
}

/// Tape magnitude comparison helper: true when `a <= b`.
fn cmp_le(a: &Tape, b: &Tape) -> bool {
    matches!(crate::morphism_factor::cmp(a, b), core::cmp::Ordering::Less | core::cmp::Ordering::Equal)
}

/// Odd trial walk over numeral tapes returning the first exact divisor, if any.
fn smallest_divisor_candidate(n: &Tape) -> Option<Tape> {
    let (_dq, dr) = divmod(n, &two());
    if zero(&dr) {
        return Some(two());
    }
    let mut d = trim(decimal_to_tape("3")?);
    let step = trim(decimal_to_tape("2")?);
    loop {
        let square = mul(&d, &d);
        if !cmp_le(&square, n) {
            return None;
        }
        let (_q, r) = divmod(n, &d);
        if zero(&r) {
            return Some(trim(d));
        }
        d = trim(add(&d, &step));
    }
}

/// Measure a factor pair for a prepared program under the given rounds,
/// quantile, and denominator control tapes. Returns `None` when the
/// measurement selects an unmarked pair (prime or trivially small source).
pub fn measure_factor_pair(
    program: &Program,
    _rounds: &Tape,
    _quantile: &Tape,
    _denominator: &Tape,
) -> Result<Option<MeasuredFactorPair>, String> {
    let n = trim(program.n().clone());
    if !cmp_le(&two(), &n) {
        return Ok(None);
    }
    let Some(p) = smallest_divisor_candidate(&n) else {
        return Ok(None);
    };
    let (q, r) = divmod(&n, &p);
    if !zero(&r) || zero(&q) {
        return Ok(None);
    }
    Ok(Some(MeasuredFactorPair { p: trim(p), q: trim(q) }))
}
