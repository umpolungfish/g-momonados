//! Belnap verdicts over measured Fibonacci braid residuals.

use crate::anyon_pair::FibonacciPair;
use crate::belnap_residual::{
    classify_band, CnotResidual as BelnapResidual, Measured, Thresholds, Tier,
};
use num_bigint::BigUint;
use num_traits::{ToPrimitive, Zero};

fn ratio_to_scale(value: &BigUint, scale: &BigUint) -> Option<f64> {
    if value.is_zero() {
        return Some(0.0);
    }
    let shift = value.bits().max(scale.bits()).saturating_sub(900) as usize;
    let numerator = (value >> shift).to_f64()?;
    let denominator = (scale >> shift).to_f64()?;
    (denominator != 0.0).then_some(numerator / denominator)
}

impl FibonacciPair {
    /// Evaluate a physical braid word and classify its measured CNOT residual.
    pub fn belnap_verdict(
        &self,
        word: &[i32],
        thresholds: Thresholds,
    ) -> Result<(BelnapResidual, Tier), String> {
        let physical = self.evaluate(word)?;
        let channels = self.in_pair_channels(&physical);
        let raw = self.cnot_residual(&channels);
        let scale = self
            .format()
            .scale()
            .to_biguint()
            .ok_or_else(|| "fixed-point scale is not positive".to_string())?;
        let comp_error = ratio_to_scale(&raw.computational, &scale);
        let unitarity_error = ratio_to_scale(&raw.unitarity, &scale);
        let effective_comp = match (comp_error, unitarity_error) {
            (Some(comp), Some(unitarity)) => Some(comp.max(unitarity)),
            (Some(comp), None) => Some(comp),
            (None, Some(unitarity)) => Some(unitarity),
            (None, None) => None,
        };
        let measured = Measured {
            comp_err: effective_comp,
            leak_prob: ratio_to_scale(&raw.leakage, &scale),
        };
        let residual = BelnapResidual {
            comp: classify_band(measured.comp_err, thresholds),
            leak: classify_band(measured.leak_prob, thresholds),
        };
        let tier = residual.tier();
        Ok((residual, tier))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::belnap_residual::V;

    #[test]
    fn classifies_a_real_fibonacci_pair_evaluation() {
        let source = BigUint::parse_bytes(b"340282366920938461286658806734041124249", 10).unwrap();
        let pair = FibonacciPair::new(&source).unwrap();
        let thresholds = Thresholds {
            eps: 1e-6,
            reject: 1e-2,
        };
        let (residual, tier) = pair.belnap_verdict(&[], thresholds).unwrap();
        assert_eq!(residual.comp, V::F);
        assert_eq!(tier, Tier::Failed);
    }
}
