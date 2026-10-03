//! Belnap verdicts over measured Fibonacci braid residuals.

use crate::anyon_pair::{CnotResidual as RawResidual, FibonacciPair};
use crate::belnap_residual::{CnotResidual as BelnapResidual, Thresholds, Tier, V};
use core::cmp::Ordering;
use num_bigint::BigUint;
use num_traits::Zero;

// A finite, nonnegative f64 is an exact dyadic rational. Cross multiplication
// compares the residual ratio to that rational without rounding either limb.
fn compare_to_threshold(value: &BigUint, scale: &BigUint, threshold: f64) -> Ordering {
    let bits = threshold.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1u64 << 52) - 1);
    let (mantissa, shift) = if exponent == 0 {
        (fraction, -1074)
    } else {
        (fraction | (1u64 << 52), exponent - 1023 - 52)
    };
    let right = scale * BigUint::from(mantissa);
    if shift >= 0 {
        value.cmp(&(right << shift as usize))
    } else {
        (value << (-shift) as usize).cmp(&right)
    }
}

/// Classify already evaluated residuals without converting them to floating point.
/// Leakage is the maximum amplitude component returned by the pair algebra.
pub fn classify_residual(
    raw: &RawResidual,
    scale: &BigUint,
    thresholds: Thresholds,
) -> Result<(BelnapResidual, Tier), String> {
    if scale.is_zero() {
        return Err("Belnap residual scale must be positive".into());
    }
    if !thresholds.eps.is_finite()
        || !thresholds.reject.is_finite()
        || thresholds.eps < 0.0
        || thresholds.reject <= thresholds.eps
    {
        return Err("Belnap thresholds require finite 0 <= eps < reject".into());
    }
    let classify = |value: &BigUint| {
        if compare_to_threshold(value, scale, thresholds.eps) != Ordering::Greater {
            V::T
        } else if compare_to_threshold(value, scale, thresholds.reject) != Ordering::Less {
            V::F
        } else {
            V::B
        }
    };
    let residual = BelnapResidual {
        comp: classify((&raw.computational).max(&raw.unitarity)),
        leak: classify(&raw.leakage),
    };
    Ok((residual, residual.tier()))
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
        classify_residual(&raw, &scale, thresholds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_traits::One;

    #[test]
    fn retains_positive_residuals_and_exact_band_boundaries_at_all_requested_widths() {
        for bits in [128usize, 256, 512, 1024, 2048] {
            let scale = BigUint::one() << (2 * bits + 16);
            let mut raw = RawResidual {
                computational: BigUint::zero(),
                leakage: BigUint::zero(),
                unitarity: BigUint::one(),
            };
            let thresholds = Thresholds {
                eps: 0.0,
                reject: 0.5,
            };
            let (verdict, tier) = classify_residual(&raw, &scale, thresholds).unwrap();
            assert_eq!(verdict.comp, V::B, "{bits}-bit positive error was lost");
            assert_eq!(tier, Tier::Inconsistent);
            raw.unitarity = BigUint::zero();
            raw.leakage = &scale >> 1usize;
            assert_eq!(
                classify_residual(&raw, &scale, thresholds).unwrap().1,
                Tier::Crowley
            );
            raw.leakage -= BigUint::one();
            assert_eq!(
                classify_residual(&raw, &scale, thresholds).unwrap().0.leak,
                V::B
            );
            let thresholds = Thresholds {
                eps: 0.25,
                reject: 0.5,
            };
            raw.leakage = &scale >> 2usize;
            assert_eq!(
                classify_residual(&raw, &scale, thresholds).unwrap().1,
                Tier::Terminal
            );
            raw.leakage += BigUint::one();
            assert_eq!(
                classify_residual(&raw, &scale, thresholds).unwrap().1,
                Tier::Crowley
            );
        }
    }

    #[test]
    fn handles_subnormal_thresholds_and_rejects_invalid_configuration() {
        let scale = BigUint::one() << 4112usize;
        let mut raw = RawResidual {
            computational: &scale >> 1074usize,
            leakage: BigUint::zero(),
            unitarity: BigUint::zero(),
        };
        let thresholds = Thresholds {
            eps: f64::from_bits(1),
            reject: 0.5,
        };
        assert_eq!(
            classify_residual(&raw, &scale, thresholds).unwrap().1,
            Tier::Terminal
        );
        raw.computational += BigUint::one();
        assert_eq!(
            classify_residual(&raw, &scale, thresholds).unwrap().1,
            Tier::Inconsistent
        );
        for thresholds in [
            Thresholds {
                eps: f64::NAN,
                reject: 0.5,
            },
            Thresholds {
                eps: -1.0,
                reject: 0.5,
            },
            Thresholds {
                eps: 0.5,
                reject: 0.5,
            },
            Thresholds {
                eps: 0.0,
                reject: f64::INFINITY,
            },
        ] {
            assert!(classify_residual(&raw, &scale, thresholds).is_err());
        }
        assert!(classify_residual(
            &raw,
            &BigUint::zero(),
            Thresholds {
                eps: 0.0,
                reject: 0.5
            }
        )
        .is_err());
    }

    #[test]
    fn classifies_a_real_fibonacci_pair_evaluation() {
        let source = BigUint::parse_bytes(b"340282366920938461286658806734041124249", 10).unwrap();
        let pair = FibonacciPair::new(&source).unwrap();
        let thresholds = Thresholds {
            eps: 1e-6,
            reject: 1e-2,
        };
        let (residual, tier) = pair.belnap_verdict(&[1], thresholds).unwrap();
        assert_eq!(residual.comp, V::F);
        assert_eq!(tier, Tier::Failed);
    }
}
