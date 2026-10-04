//! SIC analysis/synthesis coordinates. Logical evidence, frame labels, and
//! continuous probability states have distinct types.
pub mod certificate;
pub mod exact;
pub mod fixed;
pub mod frame;
pub mod gpu;
pub mod qubit;
pub mod residual;
pub mod wh;
pub use frame::{Complex, Operator, Sic, SicCoordinates, SicFrame};
pub use qubit::{
    sic_measure, truth_measure, BlochVector, QubitSicState, QubitState, SicDistribution, SicVertex,
    Simplex4, TetraSic,
};

#[derive(Debug, Clone, PartialEq)]
pub struct SicError(pub String);
impl std::fmt::Display for SicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for SicError {}
pub(crate) fn error(message: &str) -> SicError {
    SicError(message.into())
}
pub const TOLERANCE: f64 = 1e-12;
#[cfg(test)]
mod tests;

/// Dual coefficients may be negative; they are never clamped.
pub fn urgleichung<const M: usize>(p: &SicDistribution, conditional: &[[f64; 4]; M]) -> [f64; M] {
    std::array::from_fn(|j| {
        (0..4)
            .map(|i| (3.0 * p.probabilities()[i] - 0.5) * conditional[j][i])
            .sum()
    })
}
