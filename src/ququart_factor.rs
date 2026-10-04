//! Recycled radix-four phase circuit and source-bound Gödel factor closure.
use crate::anyon_ququart::{QuquartDigit, QuquartPhaseReadout, QuquartSicOutcome, SicPhaseEvidence};
use crate::phase_unbraid::PhaseReadoutAccumulator;
use crate::reversible_modular::ModularMultiply;
use alloc::{collections::BTreeMap, string::String, vec::Vec};
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
    fn measure_sic_outcome(&mut self) -> Result<QuquartSicOutcome, String> {
        Err("phase device does not implement the sixteen-outcome SIC instrument".into())
    }
    fn reset_control(&mut self, measured: QuquartDigit) -> Result<(), String>;
    fn finish(&mut self) -> Result<(), String>;
    fn abort(&mut self);
}

/// Terminal closure binds the measured winding and both factor words to the
/// source through the Gödel multiplication check.
pub struct QuquartFactorClosure {
    source_word: String,
    p_word: String,
    q_word: String,
    order: BigUint,
    p: BigUint,
    q: BigUint,
}
impl QuquartFactorClosure {
    pub fn factors(&self) -> (&BigUint, &BigUint) { (&self.p, &self.q) }
    pub fn source_word(&self) -> &str { &self.source_word }
    pub fn factor_words(&self) -> (&str, &str) { (&self.p_word, &self.q_word) }
    pub fn order(&self) -> &BigUint { &self.order }
    pub fn word(&self) -> String { alloc::format!("{}|{}|{}", self.source_word, self.p_word, self.q_word) }
}
pub struct QuquartFactorShot {
    pub phase: QuquartPhaseReadout,
    pub closure: Option<QuquartFactorClosure>,
}

pub struct QuquartSicFactorShot {
    pub outcomes: Vec<QuquartSicOutcome>,
    pub outside_carrier: bool,
    pub closure: Option<QuquartFactorClosure>,
}

/// Close a measured winding with the native order-two factor relation and
/// verify its factor arms against the source word.
fn extract_certified_winding(
    source: &BigUint,
    base: &BigUint,
    order: &BigUint,
) -> Result<QuquartFactorClosure, String> {
    use crate::godel_calculus::{check, encode_cell_binary, Nat, Operator};
    let numeral = |value: &BigUint| Nat::from_bits_le(
        crate::native_numeral::to_bits_low_first(value));
    let source_numeral = numeral(source);
    let base_numeral = numeral(base);
    let order_numeral = numeral(order);
    let one = Nat::one();
    if source_numeral.is_zero() || order_numeral.is_zero()
        || order_numeral.bits_le()[0]
        || base_numeral.pow_mod(&order_numeral, &source_numeral) != Some(one.clone()) {
        return Err("measured phase did not close an even modular winding".into());
    }
    let half_order = Nat::from_bits_le(order_numeral.bits_le()[1..].to_vec());
    let half_power = base_numeral.pow_mod(&half_order, &source_numeral)
        .ok_or("measured winding has a zero source")?;
    if half_power == one || half_power.add(&one) == source_numeral {
        return Err("measured winding has a trivial half-winding".into());
    }
    let gcd = |mut left: Nat, mut right: Nat| {
        while !right.is_zero() {
            let remainder = left.div_rem(&right).expect("nonzero winding divisor").1;
            left = right;
            right = remainder;
        }
        left
    };
    if gcd(base_numeral, source_numeral.clone()) != one {
        return Err("measured phase base is not coprime to its source".into());
    }
    let mut p_numeral = gcd(half_power.sub(&one).ok_or("zero half-winding")?, source_numeral.clone());
    if p_numeral == one || p_numeral == source_numeral {
        p_numeral = gcd(half_power.add(&one), source_numeral.clone());
    }
    if p_numeral == one || p_numeral == source_numeral || p_numeral.is_zero() {
        return Err("measured winding did not yield a nontrivial factor".into());
    }
    let (q_numeral, remainder) = source_numeral.div_rem(&p_numeral)
        .ok_or("zero measured factor arm")?;
    let source_word = encode_cell_binary(&source_numeral);
    let p_word = encode_cell_binary(&p_numeral);
    let q_word = encode_cell_binary(&q_numeral);
    if q_numeral == one || !remainder.is_zero()
        || !check(&p_word, Operator::Mul, &q_word, &source_word)
            .map_err(|error| error.to_string())?.valid {
        return Err("measured factor arms did not close through the Gödel product".into());
    }
    let host_value = |value: &Nat| value.bits_le().iter().enumerate().fold(
        BigUint::zero(), |result, (bit, set)| {
            if *set { result | (BigUint::one() << bit) } else { result }
        });
    let p = host_value(&p_numeral);
    let q = host_value(&q_numeral);
    Ok(QuquartFactorClosure { source_word, p_word, q_word, order: order.clone(), p, q })
}

pub fn close_sic_phase_evidence(
    evidence: &SicPhaseEvidence,
    source: &BigUint,
    base: &BigUint,
) -> Result<Option<QuquartFactorClosure>, String> {
    evidence.validate_source(source)?;
    if evidence.observation_count() == 0 {
        return Err("SIC factor closure requires measured phase evidence".into());
    }
    if crate::factor_routes::big_gcd(base.clone(), source.clone()) != BigUint::one() {
        return Err("SIC factor closure requires a base coprime to its source".into());
    }
    let mut ranked_orders = BTreeMap::<BigUint, BigUint>::new();
    for (_, denominator, score) in evidence.scores() {
        if denominator <= &BigUint::one() || score.is_zero() {
            continue;
        }
        ranked_orders
            .entry(denominator.clone())
            .and_modify(|known| {
                if score > known {
                    *known = score.clone();
                }
            })
            .or_insert_with(|| score.clone());
    }
    let mut ranked_orders: Vec<_> = ranked_orders.into_iter().collect();
    ranked_orders.sort_by(|left, right| right.1.cmp(&left.1));
    for (order, _) in ranked_orders {
        if (&order & BigUint::one()).is_one() || base.modpow(&order, source) != BigUint::one() {
            continue;
        }
        if let Ok(closure) = extract_certified_winding(source, base, &order) {
            return Ok(Some(closure));
        }
    }
    Ok(None)
}

/// Source-bound radix-four controlled powers prepared before execution.
/// This contains no period, eigenphase or factor.
#[derive(Clone)]
pub struct QuquartPowerSchedule {
    source: BigUint,
    base: BigUint,
    powers: Vec<BigUint>,
}
impl QuquartPowerSchedule {
    pub fn prepare(source: &BigUint, base: &BigUint) -> Result<Self, String> {
        let count = ModularMultiply::new(source)?.width().checked_add(4)
            .ok_or("ququart phase width overflow")?;
        let mut powers = Vec::with_capacity(count);
        let mut power = base % source;
        for _ in 0..count {
            powers.push(power.clone());
            let square = &power * &power % source;
            power = &square * &square % source;
        }
        Self::from_prepared(source, base, powers)
    }
    pub fn from_prepared(source: &BigUint, base: &BigUint, powers: Vec<BigUint>) -> Result<Self, String> {
        let expected = ModularMultiply::new(source)?.width().checked_add(4)
            .ok_or("ququart phase width overflow")?;
        if powers.len() != expected { return Err("prepared ququart schedule has incomplete phase resolution".into()); }
        let mut a = base.clone();
        let mut b = source.clone();
        while !b.is_zero() { let r = &a % &b; a = b; b = r; }
        if !a.is_one() { return Err("ququart phase base must be coprime to source".into()); }
        let mut expected_power = base % source;
        for power in &powers {
            if power != &expected_power { return Err("prepared controlled power differs from source and base".into()); }
            let square = power * power % source;
            expected_power = &square * &square % source;
        }
        Ok(Self { source: source.clone(), base: base.clone(), powers })
    }
    pub fn powers(&self) -> &[BigUint] { &self.powers }
}

pub struct QuquartFactorExecutor<D> {
    device: D,
    evidence: PhaseReadoutAccumulator,
    schedule: Option<QuquartPowerSchedule>,
    measured_phases: Vec<(BigUint, BigUint)>,
    measured_sic: Vec<Vec<QuquartSicOutcome>>,
}
impl<D: QuquartPhaseDevice> QuquartFactorExecutor<D> {
    pub fn new(device: D) -> Self {
        Self {
            device,
            evidence: PhaseReadoutAccumulator::default(),
            schedule: None,
            measured_phases: Vec::new(),
            measured_sic: Vec::new(),
        }
    }
    pub fn with_prepared_schedule(device: D, schedule: QuquartPowerSchedule) -> Self {
        Self { device, evidence: PhaseReadoutAccumulator::default(), schedule: Some(schedule), measured_phases: Vec::new(), measured_sic: Vec::new() }
    }
    /// Resident measurements, released only in the terminal report.
    pub fn measured_phases(&self) -> &[(BigUint, BigUint)] { &self.measured_phases }
    pub fn measured_sic_outcomes(&self) -> &[Vec<QuquartSicOutcome>] { &self.measured_sic }
    pub fn device(&self) -> &D {
        &self.device
    }
    pub fn shot(&mut self, source: &BigUint, base: &BigUint) -> Result<QuquartFactorShot, String> {
        if self.schedule.as_ref().map(|schedule| &schedule.source != source || &schedule.base != base).unwrap_or(true) {
            self.schedule = Some(QuquartPowerSchedule::prepare(source, base)?);
            self.evidence = PhaseReadoutAccumulator::default();
            self.measured_phases.clear();
            self.measured_sic.clear();
        }
        let powers = &self.schedule.as_ref().unwrap().powers;
        let count = powers.len();
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
            self.measured_phases.push((phase.numerator().clone(), phase.denominator()?));
            let closure = phase.close(&mut self.evidence, source, base)?;
            let closure = if let Some((order, measured_p, measured_q)) = closure {
                let result = extract_certified_winding(source, base, &order)?;
                let (p, q) = result.factors();
                if !((p == &measured_p && q == &measured_q) || (p == &measured_q && q == &measured_p)) {
                    return Err("resident closure changed the measured factor arms".into());
                }
                Some(result)
            } else { None };
            Ok(QuquartFactorShot {
                phase,
                closure,
            })
        })();
        if result.is_err() {
            self.device.abort();
        }
        result
    }

    pub fn sic_shot(
        &mut self,
        source: &BigUint,
        base: &BigUint,
        phase_evidence: &mut SicPhaseEvidence,
    ) -> Result<QuquartSicFactorShot, String> {
        phase_evidence.validate_source(source)?;
        if self.schedule.as_ref().map(|schedule| &schedule.source != source || &schedule.base != base).unwrap_or(true) {
            self.schedule = Some(QuquartPowerSchedule::prepare(source, base)?);
            self.evidence = PhaseReadoutAccumulator::default();
            self.measured_phases.clear();
            self.measured_sic.clear();
        }
        let powers = &self.schedule.as_ref().unwrap().powers;
        if let Err(error) = self.device.begin(source, base, powers.len()) {
            self.device.abort();
            return Err(error);
        }
        let result = (|| {
            let mut outcomes = Vec::with_capacity(powers.len());
            let mut staged_evidence = phase_evidence.clone();
            for power in powers.iter().rev() {
                self.device.fourier(false)?;
                self.device.controlled_multiply(power)?;
                self.device.fourier(true)?;
                let outcome = self.device.measure_sic_outcome()?;
                match outcome {
                    QuquartSicOutcome::Carrier(_) => {
                        staged_evidence.observe(
                            outcome,
                            power,
                            &BigUint::zero(),
                            &BigUint::one(),
                        )?;
                    }
                    QuquartSicOutcome::OutsideCarrier => {
                        outcomes.push(outcome);
                        self.measured_sic.push(outcomes.clone());
                        self.device.abort();
                        return Ok(QuquartSicFactorShot { outcomes, outside_carrier: true, closure: None });
                    }
                }
                outcomes.push(outcome);
            }
            self.device.finish()?;
            *phase_evidence = staged_evidence;
            let closure = close_sic_phase_evidence(phase_evidence, source, base)?;
            self.measured_sic.push(outcomes.clone());
            Ok(QuquartSicFactorShot { outcomes, outside_carrier: false, closure })
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
    fn prepared_schedule_rejects_changed_source_power_or_resolution() {
        let n = BigUint::parse_bytes(b"74190557381655886253556359637698917958655348096397072330011641798610025580603", 10).unwrap();
        let base = BigUint::from(2u8);
        let schedule = QuquartPowerSchedule::prepare(&n, &base).unwrap();
        for (j, power) in schedule.powers().iter().enumerate() {
            assert_eq!(*power, base.modpow(&(BigUint::one() << (2 * j)), &n));
        }
        let mut wrong = schedule.powers().to_vec();
        wrong[17] += 1u8;
        assert!(QuquartPowerSchedule::from_prepared(&n, &base, wrong).is_err());
        let mut short = schedule.powers().to_vec();
        short.pop();
        assert!(QuquartPowerSchedule::from_prepared(&n, &base, short).is_err());
        assert!(QuquartPowerSchedule::from_prepared(&n, &BigUint::from(3u8), schedule.powers().to_vec()).is_err());
    }
    #[test]
    fn resident_extraction_seam_preserves_rsa100_certified_winding() {
        // Integration of the existing extraction seam, not a measurement test.
        // This certified relation is test-only and is never baked into a membrane.
        let decimal = |text: &[u8]| BigUint::parse_bytes(text, 10).unwrap();
        let n = decimal(b"1522605027922533360535618378132637429718068114961380688657908494580122963258952897654000350692006139");
        let order = decimal(b"761302513961266680267809189066318714859034057480651309369510315012584735325452345278878285127821940");
        let base = BigUint::from(2u8);
        assert!(base.modpow(&order, &n).is_one());
        let closure = extract_certified_winding(&n, &base, &order).unwrap();
        let (p, q) = closure.factors();
        let closure_word = closure.word();
        let fields: Vec<_> = closure_word.split('|').collect();
        assert_eq!(fields, vec![closure.source_word.as_str(), closure.p_word.as_str(), closure.q_word.as_str()]);
        assert_eq!(closure.order(), &order);
        assert_eq!(p * q, n);
        assert_eq!(*p, decimal(b"37975227936943673922808872755445627854565536638199"));
        assert_eq!(*q, decimal(b"40094690950920881030683735292761468389214899724061"));

        let format = crate::phase_unbraid::FixedPointFormat::for_modulus(&n).unwrap();
        let mut sic_evidence = SicPhaseEvidence::new(&format).unwrap();
        sic_evidence
            .add_hypothesis(BigUint::one(), order.clone())
            .unwrap();
        assert!(close_sic_phase_evidence(&sic_evidence, &n, &base).is_err());
        let sic = crate::anyon_ququart::FixedQuquartSic::new(&format).unwrap();
        let likelihoods = sic
            .phase_outcome_masses(
                &BigUint::one(),
                &order,
                &BigUint::one(),
                &BigUint::zero(),
                &BigUint::one(),
            )
            .unwrap();
        let mask = likelihoods.iter().position(|mass| !mass.is_zero()).unwrap() as u8;
        sic_evidence
            .observe(
                QuquartSicOutcome::Carrier(crate::sic::SixteenOutcome::new(mask).unwrap()),
                &BigUint::one(),
                &BigUint::zero(),
                &BigUint::one(),
            )
            .unwrap();
        let sic_closure = close_sic_phase_evidence(&sic_evidence, &n, &base)
            .unwrap()
            .unwrap();
        assert_eq!(sic_closure.order(), &order);
        assert_eq!(sic_closure.factors(), (p, q));
    }
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
            if n.bits() <= 200 { continue; }
            let mut executor = QuquartFactorExecutor::new(Schedule::default());
            let shot = executor.shot(&n, &BigUint::from(2u8)).unwrap();
            assert!(shot.closure.is_none());
            assert_eq!(executor.measured_phases().len(), 1);
            assert!(executor.measured_phases()[0].0.is_zero());
            assert_eq!(executor.measured_phases()[0].1, shot.phase.denominator().unwrap());
            assert!(executor.device().finished);
            assert!(!executor.device().aborted);
            assert_eq!(
                shot.phase.denominator().unwrap().bits(),
                2 * (n.bits() + 4) + 1
            );
        }
    }

    #[test]
    fn sic_factor_shot_preserves_full_mask_sequence_and_updates_phase_evidence() {
        #[derive(Default)]
        struct SicSchedule {
            expected: usize,
            measured: usize,
            outside_at: Option<usize>,
        }
        impl QuquartPhaseDevice for SicSchedule {
            fn begin(&mut self, _: &BigUint, _: &BigUint, count: usize) -> Result<(), String> {
                self.expected = count;
                self.measured = 0;
                Ok(())
            }
            fn fourier(&mut self, _: bool) -> Result<(), String> { Ok(()) }
            fn controlled_multiply(&mut self, _: &BigUint) -> Result<(), String> { Ok(()) }
            fn feedback(&mut self, _: &BigUint, _: usize) -> Result<(), String> { Ok(()) }
            fn measure_phase_digit(&mut self) -> Result<QuquartDigit, String> {
                Err("SIC test device does not emit radix-four digits".into())
            }
            fn measure_sic_outcome(&mut self) -> Result<QuquartSicOutcome, String> {
                if self.outside_at == Some(self.measured) {
                    self.measured += 1;
                    return Ok(QuquartSicOutcome::OutsideCarrier);
                }
                let outcome = crate::sic::SixteenOutcome::new((self.measured % 16) as u8)
                    .map_err(|error| error.to_string())?;
                self.measured += 1;
                Ok(QuquartSicOutcome::Carrier(outcome))
            }
            fn reset_control(&mut self, _: QuquartDigit) -> Result<(), String> { Ok(()) }
            fn finish(&mut self) -> Result<(), String> {
                if self.measured == self.expected { Ok(()) }
                else { Err("SIC test schedule did not measure every stage".into()) }
            }
            fn abort(&mut self) {}
        }

        let source = BigUint::parse_bytes(b"296650821743515430283258444261036507151", 10).unwrap();
        let base = BigUint::from(2u8);
        let format = crate::phase_unbraid::FixedPointFormat::for_modulus(&source).unwrap();
        let mut phase_evidence = crate::anyon_ququart::SicPhaseEvidence::new(&format).unwrap();
        phase_evidence
            .add_hypothesis(BigUint::zero(), BigUint::one())
            .unwrap();
        let mut executor = QuquartFactorExecutor::new(SicSchedule::default());
        let result = executor.sic_shot(&source, &base, &mut phase_evidence).unwrap();
        assert!(!result.outside_carrier);
        assert_eq!(result.outcomes.len(), executor.schedule.as_ref().unwrap().powers.len());
        assert!(result.outcomes.iter().enumerate().all(|(index, outcome)| {
            matches!(outcome, QuquartSicOutcome::Carrier(mask) if mask.mask() == (index % 16) as u8)
        }));
        assert_eq!(executor.measured_sic_outcomes(), &[result.outcomes]);
        assert_eq!(phase_evidence.scores().count(), 1);

        let mut outside_evidence = crate::anyon_ququart::SicPhaseEvidence::new(&format).unwrap();
        outside_evidence
            .add_hypothesis(BigUint::zero(), BigUint::one())
            .unwrap();
        let prior_scores: Vec<_> = outside_evidence
            .scores()
            .map(|(numerator, denominator, score)| (numerator.clone(), denominator.clone(), score.clone()))
            .collect();
        let mut outside_executor = QuquartFactorExecutor::new(SicSchedule {
            outside_at: Some(2),
            ..SicSchedule::default()
        });
        let outside_result = outside_executor
            .sic_shot(&source, &base, &mut outside_evidence)
            .unwrap();
        assert!(outside_result.outside_carrier);
        assert_eq!(outside_result.outcomes.len(), 3);
        let resulting_scores: Vec<_> = outside_evidence
            .scores()
            .map(|(numerator, denominator, score)| (numerator.clone(), denominator.clone(), score.clone()))
            .collect();
        assert_eq!(resulting_scores, prior_scores);
    }
}
