//! Source-bound Shor gate requests with arbitrary-width modular arithmetic.
use alloc::{vec, vec::Vec};
use num_bigint::BigUint;
use num_traits::{One, Zero};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShorGate {
    Hadamard { qubit: usize },
    PrepareOne { work_start: usize },
    ControlledMultiply { control: usize, work_start: usize, work_width: usize,
        multiplier: BigUint, modulus: BigUint },
    /// Joint |11> phase exp(-2πi / 2^denominator_bits).
    InverseControlledPhase { control: usize, target: usize, denominator_bits: usize },
    Swap { left: usize, right: usize },
}
impl ShorGate {
    pub fn work_action(&self, enabled: bool, work: &BigUint) -> Option<BigUint> {
        match self {
            Self::ControlledMultiply { multiplier, modulus, .. } => Some(
                if enabled && work < modulus { work * multiplier % modulus }
                else { work.clone() }),
            _ => None,
        }
    }
}

pub fn shor_gate_plan_for_source(period_qubits: usize, base: &BigUint, modulus: &BigUint)
    -> Result<Vec<ShorGate>, &'static str>
{
    if period_qubits == 0 || modulus < &BigUint::from(2u8) {
        return Err("period register and modulus must be nonzero");
    }
    let mut x = base % modulus;
    let mut y = modulus.clone();
    while !x.is_zero() { let r = &y % &x; y = x; x = r; }
    if !y.is_one() { return Err("modular multiplication requires a base coprime to N"); }
    let width = usize::try_from((modulus - BigUint::one()).bits())
        .map_err(|_| "work width exceeds host indexing")?;
    period_qubits.checked_add(width).ok_or("register layout overflow")?;
    let mut gates = vec![ShorGate::PrepareOne { work_start: period_qubits }];
    for qubit in 0..period_qubits { gates.push(ShorGate::Hadamard { qubit }); }
    let mut multiplier = base % modulus;
    for control in 0..period_qubits {
        if !multiplier.is_one() {
            gates.push(ShorGate::ControlledMultiply { control, work_start: period_qubits,
                work_width: width, multiplier: multiplier.clone(), modulus: modulus.clone() });
        }
        multiplier = &multiplier * &multiplier % modulus;
    }
    for left in 0..period_qubits / 2 {
        gates.push(ShorGate::Swap { left, right: period_qubits - 1 - left });
    }
    for target in 0..period_qubits {
        for control in 0..target {
            gates.push(ShorGate::InverseControlledPhase { control, target,
                denominator_bits: target - control + 1 });
        }
        gates.push(ShorGate::Hadamard { qubit: target });
    }
    Ok(gates)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rsa_200_and_256_bit_shor_plan_preserves_modular_operations() {
        for decimal in [
            "1156514714917773145849996001252587703581994899993461612691909",
            "101560191607051872909385412079844080615251494997013823952605603214371184345809",
        ] {
            let n = BigUint::parse_bytes(decimal.as_bytes(), 10).unwrap();
            assert!(n.bits() >= 200);
            let period = 2 * n.bits() as usize;
            let mut plans = Vec::new();
            for base in [2u8, 8] {
                let plan = shor_gate_plan_for_source(period, &BigUint::from(base), &n).unwrap();
                let mut power = BigUint::from(base);
                for control in 0..period {
                    let gate = plan.iter().find(|gate| matches!(gate,
                        ShorGate::ControlledMultiply { control: wire, .. } if *wire == control));
                    if power.is_one() { assert!(gate.is_none()); }
                    else {
                        let gate = gate.expect("every nontrivial controlled power retained");
                        for work in [BigUint::zero(), BigUint::one(), &n - BigUint::one(), n.clone()] {
                            assert_eq!(gate.work_action(false, &work), Some(work.clone()));
                            let expected = if work < n { &work * &power % &n } else { work.clone() };
                            assert_eq!(gate.work_action(true, &work), Some(expected));
                        }
                        if let ShorGate::ControlledMultiply { work_start, work_width, modulus, .. } = gate {
                            assert_eq!(*work_start, period);
                            assert_eq!(*work_width, n.bits() as usize);
                            assert_eq!(modulus, &n);
                        }
                    }
                    power = &power * &power % &n;
                }
                assert!(plan.contains(&ShorGate::InverseControlledPhase {
                    control: 0, target: period - 1, denominator_bits: period }));
                plans.push(plan);
            }
            assert_ne!(plans[0], plans[1], "different bases retain different work permutations");
        }
    }
}
