//! Physical three-object weaving from the Fibonacci fusion hexagon.
//! Fig. 2 and Eqs. (7)-(9), https://arxiv.org/abs/1511.00719.
//! F changes the fusion basis; only R steps emit physical crossings.
use alloc::string::String;
use alloc::vec::Vec;
use num_bigint::BigInt;
use num_traits::Zero;

#[derive(Clone, Debug)]
pub enum FusionStep {
    F,
    R(i32),
}

pub struct WeaveWord {
    pub word: Vec<i32>,
    pub final_vertex: u8,
    /// Physical word phase relative to the symbolic F/R product, in fifths
    /// of a winding, for the kernel's conjugate chirality. R' = e^(-4πi/5) R.
    pub phase_fifths: BigInt,
}

/// Refine the symbolic fusion sequence before translating its changing
/// bases into physical crossings. Unlike refining a fixed-coordinate braid,
/// this keeps the moving-object path needed for composite-strand lifting.
pub fn refine_sequence(seed: &[FusionStep], iterations: usize) -> Result<Vec<FusionStep>, String> {
    let mut sequence = seed.to_vec();
    for _ in 0..iterations {
        let inverse: Vec<_> = sequence
            .iter()
            .rev()
            .map(|step| match step {
                FusionStep::F => Ok(FusionStep::F),
                FusionStep::R(n) => n
                    .checked_neg()
                    .map(FusionStep::R)
                    .ok_or("weave inverse overflow"),
            })
            .collect::<Result<_, _>>()?;
        let capacity = sequence
            .len()
            .checked_mul(5)
            .and_then(|n| n.checked_add(4))
            .ok_or("fusion sequence length overflow")?;
        let mut next = Vec::new();
        next.try_reserve_exact(capacity)
            .map_err(|_| "fusion sequence allocation failed")?;
        next.extend_from_slice(&sequence);
        next.push(FusionStep::R(1));
        next.extend_from_slice(&inverse);
        next.push(FusionStep::R(3));
        next.extend_from_slice(&sequence);
        next.push(FusionStep::R(3));
        next.extend_from_slice(&inverse);
        next.push(FusionStep::R(1));
        next.extend_from_slice(&sequence);
        sequence = next;
    }
    Ok(sequence)
}

/// Complete an exchange seed with diagonal R steps as specified by the
/// fusion hexagon. These steps preserve the off-diagonal magnitude.
pub fn compile_exchange(steps: &[FusionStep]) -> Result<WeaveWord, String> {
    let mut completed = steps.to_vec();
    let mut weave = compile_weave(&completed)?;
    if weave.final_vertex == 2 || weave.final_vertex == 3 {
        completed.insert(0, FusionStep::R(1));
        weave = compile_weave(&completed)?;
    }
    if weave.final_vertex == 5 {
        completed.push(FusionStep::R(1));
        weave = compile_weave(&completed)?;
    }
    if weave.final_vertex != 4 {
        return Err("fusion sequence is not an exchange seed".into());
    }
    Ok(weave)
}

/// Start at hexagon vertex 1, with the moving object left of both warps.
/// Standard-basis endpoints are vertices 1 (returned) and 4 (exchanged).
/// Other endpoints require a basis completion before use as a logical gate.
pub fn compile_weave(steps: &[FusionStep]) -> Result<WeaveWord, String> {
    let capacity = steps
        .iter()
        .try_fold(0usize, |total, step| {
            let count = match step {
                FusionStep::F => 0,
                FusionStep::R(power) => {
                    usize::try_from(power.unsigned_abs()).ok()?.checked_mul(2)?
                }
            };
            total.checked_add(count)
        })
        .ok_or("weave word length overflow")?;
    let mut word = Vec::new();
    word.try_reserve_exact(capacity)
        .map_err(|_| "weave word allocation failed")?;
    let mut vertex = 1u8;
    let mut phase_fifths = BigInt::zero();
    for step in steps {
        match step {
            FusionStep::F => {
                vertex = match vertex {
                    1 => 2,
                    2 => 1,
                    3 => 4,
                    4 => 3,
                    5 => 6,
                    6 => 5,
                    _ => unreachable!(),
                }
            }
            FusionStep::R(power) => {
                let sense = if *power < 0 { -1 } else { 1 };
                if vertex == 1 || vertex == 6 {
                    phase_fifths -= BigInt::from(*power) * BigInt::from(2u8);
                }
                for _ in 0..power.unsigned_abs() {
                    vertex = match vertex {
                        2 => {
                            word.push(sense);
                            3
                        }
                        3 => {
                            word.push(sense);
                            2
                        }
                        4 => {
                            word.push(2 * sense);
                            5
                        }
                        5 => {
                            word.push(2 * sense);
                            4
                        }
                        1 => {
                            word.extend_from_slice(&[-sense, -2 * sense]);
                            6
                        }
                        6 => {
                            word.extend_from_slice(&[-2 * sense, -sense]);
                            1
                        }
                        _ => unreachable!(),
                    };
                }
            }
        }
    }
    Ok(WeaveWord {
        word,
        final_vertex: vertex,
        phase_fifths,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anyon_local::FibonacciLocal;
    use crate::phase_unbraid::FixedComplex;
    use num_bigint::BigUint;
    use num_traits::Signed;

    #[test]
    fn anyon_weave_hexagon_128_bit_semiprime() {
        let n = BigUint::parse_bytes(b"296650821743515430283258444261036507151", 10).unwrap();
        assert_eq!(n.bits(), 128);
        let algebra = FibonacciLocal::new(&n).unwrap();
        for steps in [
            alloc::vec![FusionStep::F, FusionStep::R(3), FusionStep::F],
            alloc::vec![
                FusionStep::F,
                FusionStep::R(-1),
                FusionStep::F,
                FusionStep::R(1),
                FusionStep::F,
                FusionStep::R(-1)
            ],
        ] {
            let weave = compile_weave(&steps).unwrap();
            assert!(weave.final_vertex == 1 || weave.final_vertex == 4);
            let mut symbolic = crate::anyon_local::LocalMatrix::identity(algebra.format());
            for step in &steps {
                let gate = match step {
                    FusionStep::F => algebra.associator().clone(),
                    FusionStep::R(power) => {
                        let generator = if *power < 0 { -1 } else { 1 };
                        algebra
                            .evaluate(&alloc::vec![generator; power.unsigned_abs() as usize])
                            .unwrap()
                    }
                };
                symbolic = gate.multiply(&symbolic, algebra.format());
            }
            let physical = algebra.evaluate(&weave.word).unwrap();
            // The kernel's local matrix uses the first-pair channel; the
            // paper's standard basis uses the last-pair channel.
            let physical = algebra
                .associator()
                .multiply(&physical, algebra.format())
                .multiply(algebra.associator(), algebra.format());
            let phase = FixedComplex::winding_twiddle(
                &weave.phase_fifths,
                &BigUint::from(5u8),
                algebra.format(),
            )
            .unwrap();
            for (actual, ideal) in physical.0.iter().zip(&symbolic.0) {
                let ideal = ideal.mul(&phase, algebra.format());
                assert!(
                    (&actual.re - &ideal.re).abs() < BigInt::from(4096u16),
                    "real residual {}",
                    (&actual.re - &ideal.re).abs()
                );
                assert!(
                    (&actual.im - &ideal.im).abs() < BigInt::from(4096u16),
                    "imaginary residual {}",
                    (&actual.im - &ideal.im).abs()
                );
            }
        }
    }
}
