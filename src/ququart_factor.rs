//! Recycled radix-four phase circuit and source-bound Gödel factor closure.
use crate::anyon_ququart::{QuquartDigit, QuquartPhaseReadout};
use crate::phase_unbraid::PhaseReadoutAccumulator;
use crate::reversible_modular::ModularMultiply;
use alloc::{string::String, vec::Vec};
use num_bigint::BigUint;
use num_traits::{One, Zero};

/// Full Z4 Fourier decomposition on little-endian control lanes zero and one.
/// The controlled-S uses exact T/CNOT decomposition; the final swap fixes
/// the four-channel output order. Each target can enter the existing braid
/// compiler with its computational and outside-carrier residual checks.
pub fn fourier_braid_targets(inverse: bool) -> Vec<crate::recycled_carrier::BraidTarget> {
    use crate::recycled_carrier::BraidTarget;
    let mut targets = alloc::vec![
        BraidTarget::H(1),
        BraidTarget::T {
            qubit: 0,
            inverse: false
        },
        BraidTarget::T {
            qubit: 1,
            inverse: false
        },
        BraidTarget::Cnot {
            control: 1,
            target: 0
        },
        BraidTarget::T {
            qubit: 0,
            inverse: true
        },
        BraidTarget::Cnot {
            control: 1,
            target: 0
        },
        BraidTarget::H(0),
        BraidTarget::Cnot {
            control: 0,
            target: 1
        },
        BraidTarget::Cnot {
            control: 1,
            target: 0
        },
        BraidTarget::Cnot {
            control: 0,
            target: 1
        },
    ];
    if inverse {
        targets.reverse();
        for target in &mut targets {
            if let BraidTarget::T { inverse, .. } = target {
                *inverse = !*inverse;
            }
        }
    }
    targets
}

/// exp(-2 pi i k n/4^m) factors into lane phases with weights one and two.
pub fn feedback_braid_targets(
    n: &BigUint,
    m: usize,
) -> Result<[crate::recycled_carrier::BraidTarget; 2], String> {
    use crate::recycled_carrier::BraidTarget;
    let bits = m
        .checked_mul(2)
        .ok_or("ququart feedback denominator overflow")?;
    Ok([
        BraidTarget::Feedback {
            qubit: 0,
            numerator: n.clone(),
            denominator_bits: bits,
        }
        .reduced_feedback(),
        BraidTarget::Feedback {
            qubit: 1,
            numerator: n * 2u8,
            denominator_bits: bits,
        }
        .reduced_feedback(),
    ])
}

/// For b=base^(4^j), the two control lanes apply b and b^2 to the same
/// modular register. A control k=k0+2*k1 therefore applies base^(k*4^j).
pub fn emit_controlled_ququart_power<F>(
    source: &BigUint,
    multiplier: &BigUint,
    mut emit: F,
) -> Result<(), String>
where
    F: FnMut(crate::reversible_modular::ElementaryGate) -> Result<(), String>,
{
    let arithmetic = ModularMultiply::new(source)?;
    arithmetic.emit_ququart_elementary(multiplier, 0, &mut emit)?;
    let square = multiplier * multiplier % source;
    arithmetic.emit_ququart_elementary(&square, 1, emit)
}

/// A shot carries one pure four-level control and a uniform mixed residue.
/// Implementations must execute the controlled powers on the shared work
/// register and supply fusion measurements. SIC masks cannot stand in for
/// phase digits without executing the requested measurement frame.
pub trait QuquartPhaseDevice {
    fn begin(
        &mut self,
        source: &BigUint,
        base: &BigUint,
        phase_digits: usize,
    ) -> Result<(), String>;
    /// F4[j,k]=exp(2 pi i j k/4)/2, or its adjoint.
    fn fourier(&mut self, inverse: bool) -> Result<(), String>;
    /// |j,x> -> |j, multiplier^j x mod N>, j in 0..4.
    fn controlled_multiply(&mut self, multiplier: &BigUint) -> Result<(), String>;
    /// diag(exp(-2 pi i j numerator/4^denominator_digits)).
    fn feedback(&mut self, numerator: &BigUint, denominator_digits: usize) -> Result<(), String>;
    fn measure_phase_digit(&mut self) -> Result<QuquartDigit, String>;
    fn reset_control(&mut self, measured: QuquartDigit) -> Result<(), String>;
    fn finish(&mut self) -> Result<(), String>;
    fn abort(&mut self);
}

pub struct QuquartFactorShot {
    pub phase: QuquartPhaseReadout,
    pub factors: Option<(BigUint, BigUint)>,
    pub order: Option<BigUint>,
}
pub struct QuquartFactorExecutor<D> {
    device: D,
    evidence: PhaseReadoutAccumulator,
}
impl<D: QuquartPhaseDevice> QuquartFactorExecutor<D> {
    pub fn new(device: D) -> Self {
        Self {
            device,
            evidence: PhaseReadoutAccumulator::default(),
        }
    }
    pub fn device(&self) -> &D {
        &self.device
    }
    pub fn shot(&mut self, source: &BigUint, base: &BigUint) -> Result<QuquartFactorShot, String> {
        let arithmetic = ModularMultiply::new(source)?;
        let mut a = base.clone();
        let mut b = source.clone();
        while !b.is_zero() {
            let r = &a % &b;
            a = b;
            b = r;
        }
        if !a.is_one() {
            return Err("ququart phase base must be coprime to source".into());
        }
        let count = arithmetic
            .width()
            .checked_add(4)
            .ok_or("ququart phase width overflow")?;
        let mut powers = Vec::with_capacity(count);
        let mut power = base % source;
        for _ in 0..count {
            powers.push(power.clone());
            let square = &power * &power % source;
            power = &square * &square % source;
        }
        if let Err(e) = self.device.begin(source, base, count) {
            self.device.abort();
            return Err(e);
        }
        let result = (|| {
            let mut phase = QuquartPhaseReadout::default();
            for (i, power) in powers.iter().rev().enumerate() {
                self.device.fourier(false)?;
                self.device.controlled_multiply(power)?;
                if !phase.numerator().is_zero() {
                    self.device.feedback(phase.numerator(), i + 1)?;
                }
                self.device.fourier(true)?;
                let digit = self.device.measure_phase_digit()?;
                phase.push(digit)?;
                self.device.reset_control(digit)?;
            }
            self.device.finish()?;
            let closure = phase.close(&mut self.evidence, source, base)?;
            let (order, factors) = if let Some((order, p, q)) = closure {
                use crate::godel_calculus::{check, encode_cell_binary, Nat, Operator};
                let word = |n: &BigUint| {
                    encode_cell_binary(&Nat::from_bits_le(
                        crate::native_numeral::to_bits_low_first(n),
                    ))
                };
                let valid = check(&word(&p), Operator::Mul, &word(&q), &word(source))
                    .map_err(|e| e.to_string())?
                    .valid;
                if !valid || p <= BigUint::one() || q <= BigUint::one() {
                    return Err("ququart measured factor arms did not close".into());
                }
                (Some(order), Some((p, q)))
            } else {
                (None, None)
            };
            Ok(QuquartFactorShot {
                phase,
                order,
                factors,
            })
        })();
        if result.is_err() {
            self.device.abort();
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fourier_and_feedback_braid_targets_match_full_z4_operators() {
        use crate::recycled_carrier::BraidTarget;
        use crate::sic::Complex;
        fn apply(ray: &mut [Complex; 4], gate: BraidTarget) {
            match gate {
                BraidTarget::H(q) => {
                    let old = *ray;
                    for i in 0..4 {
                        let clear = i & !(1 << q);
                        let set = clear | (1 << q);
                        ray[i] = if i & (1 << q) == 0 {
                            old[clear] + old[set]
                        } else {
                            old[clear] - old[set]
                        };
                        ray[i] = ray[i].scale(1.0 / 2.0f64.sqrt());
                    }
                }
                BraidTarget::T { qubit, inverse } => {
                    let z = Complex::phase(if inverse {
                        -core::f64::consts::FRAC_PI_4
                    } else {
                        core::f64::consts::FRAC_PI_4
                    });
                    for (i, v) in ray.iter_mut().enumerate() {
                        if i & (1 << qubit) != 0 {
                            *v = *v * z;
                        }
                    }
                }
                BraidTarget::Cnot { control, target } => {
                    let old = *ray;
                    for i in 0..4 {
                        ray[if i & (1 << control) != 0 {
                            i ^ (1 << target)
                        } else {
                            i
                        }] = old[i];
                    }
                }
                BraidTarget::Feedback {
                    qubit,
                    numerator,
                    denominator_bits,
                } => {
                    use num_traits::ToPrimitive;
                    let angle = -core::f64::consts::TAU * numerator.to_f64().unwrap()
                        / 2.0f64.powi(denominator_bits as i32);
                    for (i, v) in ray.iter_mut().enumerate() {
                        if i & (1 << qubit) != 0 {
                            *v = *v * Complex::phase(angle);
                        }
                    }
                }
                _ => panic!("unexpected Z4 target"),
            }
        }
        for inverse in [false, true] {
            for l in 0..4 {
                let mut ray =
                    core::array::from_fn(|i| Complex::new(if i == l { 1.0 } else { 0.0 }, 0.0));
                for gate in fourier_braid_targets(inverse) {
                    apply(&mut ray, gate);
                }
                for (k, z) in ray.iter().enumerate() {
                    let angle = core::f64::consts::FRAC_PI_2
                        * (k * l) as f64
                        * if inverse { -1.0 } else { 1.0 };
                    assert!((*z - Complex::phase(angle).scale(0.5)).abs2().sqrt() < 1e-12);
                }
            }
        }
        for k in 0..4 {
            let mut ray =
                core::array::from_fn(|i| Complex::new(if i == k { 1.0 } else { 0.0 }, 0.0));
            for gate in feedback_braid_targets(&BigUint::from(7u8), 3).unwrap() {
                apply(&mut ray, gate);
            }
            let phase = Complex::phase(-core::f64::consts::TAU * k as f64 * 7.0 / 64.0);
            assert!((ray[k] - phase).abs2().sqrt() < 1e-12);
        }
    }
    #[test]
    fn all_four_control_values_apply_their_modular_power() {
        use crate::reversible_modular::ElementaryGate;
        let n = BigUint::parse_bytes(b"296650821743515430283258444261036507151", 10).unwrap();
        let circuit = ModularMultiply::new(&n).unwrap();
        let x = &n - BigUint::from(2u8);
        let b = BigUint::from(2u8);
        for k in 0..4u8 {
            let mut bits = alloc::vec![false;circuit.elementary_qubits()+1];
            bits[0] = k & 1 != 0;
            bits[1] = k & 2 != 0;
            for i in 0..circuit.width() {
                bits[i + 2] = x.bit(i as u64);
            }
            emit_controlled_ququart_power(&n, &b, |gate| {
                let (target, enabled) = match gate {
                    ElementaryGate::X(q) => (q, true),
                    ElementaryGate::Cnot { control, target } => (target, bits[control]),
                    ElementaryGate::Toffoli {
                        first,
                        second,
                        target,
                    } => (target, bits[first] && bits[second]),
                };
                if enabled {
                    bits[target] = !bits[target];
                }
                Ok(())
            })
            .unwrap();
            let mut result = BigUint::zero();
            for i in 0..circuit.width() {
                if bits[i + 2] {
                    result |= BigUint::one() << i;
                }
            }
            assert_eq!(result, &x * b.modpow(&BigUint::from(k), &n) % &n);
            assert_eq!(bits[0], k & 1 != 0);
            assert_eq!(bits[1], k & 2 != 0);
            assert!(bits[circuit.width() + 2..].iter().all(|b| !b));
        }
    }
    #[derive(Default)]
    struct Schedule {
        source: BigUint,
        base: BigUint,
        count: usize,
        steps: usize,
        inverse: bool,
        finished: bool,
        aborted: bool,
    }
    impl QuquartPhaseDevice for Schedule {
        fn begin(&mut self, n: &BigUint, a: &BigUint, count: usize) -> Result<(), String> {
            self.source = n.clone();
            self.base = a.clone();
            self.count = count;
            Ok(())
        }
        fn fourier(&mut self, inverse: bool) -> Result<(), String> {
            self.inverse = inverse;
            Ok(())
        }
        fn controlled_multiply(&mut self, p: &BigUint) -> Result<(), String> {
            assert!(!self.inverse);
            if self.steps == 0 || self.steps == self.count / 2 || self.steps + 1 == self.count {
                let exponent = BigUint::one() << (2 * (self.count - self.steps - 1));
                assert_eq!(*p, self.base.modpow(&exponent, &self.source));
            }
            Ok(())
        }
        fn feedback(&mut self, _: &BigUint, _: usize) -> Result<(), String> {
            Err("zero schedule must not require feedback".into())
        }
        fn measure_phase_digit(&mut self) -> Result<QuquartDigit, String> {
            assert!(self.inverse);
            self.steps += 1;
            Ok(QuquartDigit::T)
        }
        fn reset_control(&mut self, _: QuquartDigit) -> Result<(), String> {
            Ok(())
        }
        fn finish(&mut self) -> Result<(), String> {
            assert_eq!(self.steps, self.count);
            self.finished = true;
            Ok(())
        }
        fn abort(&mut self) {
            self.aborted = true;
        }
    }
    #[test]
    fn radix_four_factor_schedule_binds_source_and_all_controlled_powers() {
        // Recording-device test of circuit scheduling, not measured factor evidence.
        for line in include_str!("../measurements/anyon-extractor-width-controls.tsv")
            .lines()
            .skip(1)
        {
            let n = BigUint::parse_bytes(line.split('\t').nth(2).unwrap().as_bytes(), 10).unwrap();
            let mut executor = QuquartFactorExecutor::new(Schedule::default());
            let shot = executor.shot(&n, &BigUint::from(2u8)).unwrap();
            assert!(shot.factors.is_none());
            assert!(executor.device().finished);
            assert!(!executor.device().aborted);
            assert_eq!(
                shot.phase.denominator().unwrap().bits(),
                2 * (n.bits() + 4) + 1
            );
        }
    }
}
