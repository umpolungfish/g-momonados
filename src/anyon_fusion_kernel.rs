//! Local running-charge transitions for an arbitrary Fibonacci fusion tree.
//! Each exchange changes one intermediate charge and has at most two outputs.
use crate::anyon_local::{FibonacciLocal, LocalMatrix};
use crate::phase_unbraid::{FixedComplex, FixedPointFormat};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use num_bigint::{BigInt, BigUint};
use num_traits::Zero;

/// One ensemble member of the uniform mixed work register. Sampling a fresh
/// residue per shot represents the mixture without allocating N amplitudes.
pub struct PreparedRegister {
    residue: BigUint,
    bits: Vec<bool>,
}
impl PreparedRegister {
    pub fn uniform<F>(source: &BigUint, entropy: F) -> Result<Self, String>
    where
        F: FnMut(&mut [u8]) -> Result<(), String>,
    {
        let layout = crate::reversible_modular::ModularMultiply::new(source)?;
        let residue = uniform_below(source, entropy)?;
        let mut bits = vec![false; layout.elementary_qubits()];
        for bit in 0..layout.width() {
            bits[bit + 1] = residue.bit(bit as u64);
        }
        Ok(Self { residue, bits })
    }
    pub fn residue(&self) -> &BigUint {
        &self.residue
    }
    pub fn logical_bits(&self) -> &[bool] {
        &self.bits
    }
}

fn uniform_below<F>(bound: &BigUint, mut entropy: F) -> Result<BigUint, String>
where
    F: FnMut(&mut [u8]) -> Result<(), String>,
{
    if bound.is_zero() {
        return Err("uniform draw requires a positive bound".into());
    }
    let bits = bound.bits();
    let bytes =
        usize::try_from(bits.div_ceil(8)).map_err(|_| "entropy width exceeds host indexing")?;
    let mut buffer = Vec::new();
    buffer
        .try_reserve_exact(bytes)
        .map_err(|_| "entropy allocation failed")?;
    buffer.resize(bytes, 0);
    loop {
        entropy(&mut buffer)?;
        let unused = (8 - bits % 8) % 8;
        if unused != 0 {
            buffer[bytes - 1] &= 0xff >> unused;
        }
        let draw = BigUint::from_bytes_le(&buffer);
        if draw < *bound {
            return Ok(draw);
        }
    }
}

/// Coherent running-charge carrier. Identical paths are combined before any
/// Born projection. The limit bounds stored paths, never truncates amplitudes.
pub struct FusionState {
    paths: BTreeMap<Vec<u8>, FixedComplex>,
    path_limit: usize,
    format: FixedPointFormat,
}

impl FusionState {
    pub fn basis(kernel: &FusionKernel, path: Vec<u8>, path_limit: usize) -> Result<Self, String> {
        kernel.stencil(&path, 1)?;
        if path_limit == 0 {
            return Err("fusion path budget must be positive".into());
        }
        let amplitude = FixedComplex {
            re: kernel.format().scale(),
            im: BigInt::zero(),
        };
        Ok(Self {
            paths: BTreeMap::from([(path, amplitude)]),
            path_limit,
            format: kernel.format().clone(),
        })
    }

    pub fn path_count(&self) -> usize {
        self.paths.len()
    }

    /// Transactional exchange: exhausted storage leaves the original state intact.
    pub fn exchange(&mut self, kernel: &FusionKernel, generator: i32) -> Result<(), String> {
        if kernel.format() != &self.format {
            return Err(
                "fusion state and exchange kernel use different fixed-point formats".into(),
            );
        }
        let mut next: BTreeMap<Vec<u8>, FixedComplex> = BTreeMap::new();
        for (path, amplitude) in &self.paths {
            let (site, weights) = kernel.stencil(path, generator)?;
            for (charge, weight) in weights.iter().enumerate() {
                let value = amplitude.mul(weight, kernel.format());
                if value.re.is_zero() && value.im.is_zero() {
                    continue;
                }
                let mut output = path.clone();
                output[site] = charge as u8;
                let combined = next.entry(output).or_insert_with(FusionKernel::zero);
                combined.re += value.re;
                combined.im += value.im;
                if next.len() > self.path_limit {
                    return Err("coherent fusion path budget exhausted".into());
                }
            }
        }
        next.retain(|_, value| !value.re.is_zero() || !value.im.is_zero());
        if next.is_empty() {
            return Err("exchange produced a zero fusion state".into());
        }
        self.paths = next;
        Ok(())
    }

    pub fn charge_masses(&self, site: usize) -> Result<[BigUint; 2], String> {
        let mut masses = [BigUint::zero(), BigUint::zero()];
        for (path, amplitude) in &self.paths {
            let charge = *path
                .get(site)
                .ok_or("fusion readout site is outside the tree")?;
            let mass = &amplitude.re * &amplitude.re + &amplitude.im * &amplitude.im;
            masses[charge as usize] += mass.to_biguint().ok_or("negative Born mass")?;
        }
        Ok(masses)
    }

    /// The first encoded triple is the recycled control. Its pair charge
    /// supplies the bit only while the triple's total charge remains tau.
    /// Vacuum total charge belongs to the leakage sector.
    pub fn control_projection(&self) -> Result<PoleProjection, String> {
        let mut masses = [BigUint::zero(), BigUint::zero(), BigUint::zero()];
        for (path, amplitude) in &self.paths {
            if path.len() < 3 {
                return Err("control readout requires a complete three-anyon encoding".into());
            }
            let channel = if path[2] == 1 { path[1] as usize } else { 2 };
            let mass = &amplitude.re * &amplitude.re + &amplitude.im * &amplitude.im;
            masses[channel] += mass.to_biguint().ok_or("negative Born mass")?;
        }
        Ok(PoleProjection { masses })
    }

    pub fn measure_control<F>(&mut self, entropy: F) -> Result<PoleOutcome, String>
    where
        F: FnMut(&mut [u8]) -> Result<(), String>,
    {
        let outcome = self.control_projection()?.sample(entropy)?;
        self.paths.retain(|path, _| match outcome {
            PoleOutcome::Truth => path[2] == 1 && path[1] == 0,
            PoleOutcome::False => path[2] == 1 && path[1] == 1,
            PoleOutcome::Unread => path[2] == 0,
        });
        Ok(outcome)
    }

    /// Retains the conditional amplitudes. Their common normalization cancels
    /// in subsequent Born ratios, so projection needs no lossy division.
    pub fn measure_charge<F>(&mut self, site: usize, entropy: F) -> Result<bool, String>
    where
        F: FnMut(&mut [u8]) -> Result<(), String>,
    {
        let outcome = sample_fusion_channel(&self.charge_masses(site)?, entropy)?;
        self.paths.retain(|path, _| path[site] == u8::from(outcome));
        Ok(outcome)
    }
}

/// Draw the fusion channel from contracted integer Born masses. The caller
/// supplies fresh entropy; no floating-point conversion or known period enters
/// this selection. Global contraction must supply these masses before calling.
pub fn sample_fusion_channel<F>(masses: &[BigUint; 2], mut entropy: F) -> Result<bool, String>
where
    F: FnMut(&mut [u8]) -> Result<(), String>,
{
    Ok(sample_born_masses(masses, &mut entropy)? == 1)
}

pub(crate) fn sample_born_masses<F>(masses: &[BigUint], mut entropy: F) -> Result<usize, String>
where
    F: FnMut(&mut [u8]) -> Result<(), String>,
{
    let total: BigUint = masses.iter().cloned().sum();
    if total.is_zero() {
        return Err("fusion measurement has zero total Born mass".into());
    }
    if let Some((index, _)) = masses.iter().enumerate().find(|(_, mass)| **mass == total) {
        return Ok(index);
    }
    let draw = uniform_below(&total, &mut entropy)?;
    let mut boundary = BigUint::zero();
    for (index, mass) in masses.iter().enumerate() {
        boundary += mass;
        if draw < boundary {
            return Ok(index);
        }
    }
    return Err("fusion mass partition failed to cover the draw".into());
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PoleOutcome {
    Truth,
    False,
    Unread,
}

impl PoleOutcome {
    /// Unread carrier weight cannot be converted to a phase bit.
    pub fn phase_bit(self) -> Option<bool> {
        match self {
            Self::Truth => Some(false),
            Self::False => Some(true),
            Self::Unread => None,
        }
    }
}

pub struct PoleProjection {
    /// Integer masses in common squared-amplitude units: T, F, unread.
    pub masses: [BigUint; 3],
}

/// Exact qubit SIC weights, retaining the quadratic irrational symbolically.
/// Probability i = (visible_trace + radical[i]/sqrt(3))/(4*carrier_trace).
/// Rows are the tetrahedron directions (+++), (+--), (-+-), (--+).
pub struct SicProjection {
    pub visible_trace: BigUint,
    pub carrier_trace: BigUint,
    pub radical: [BigInt; 4],
}

impl SicProjection {
    pub fn from_ray(
        truth: &FixedComplex,
        falsity: &FixedComplex,
        unread: BigUint,
    ) -> Result<Self, String> {
        let poles = PoleProjection::from_ray(truth, falsity, unread);
        let carrier_trace = poles.total_mass();
        if carrier_trace.is_zero() {
            return Err("SIC frame has zero carrier mass".into());
        }
        let x = (&truth.re * &falsity.re + &truth.im * &falsity.im) * 2u8;
        let y = (&truth.re * &falsity.im - &truth.im * &falsity.re) * 2u8;
        let z = BigInt::from(poles.masses[0].clone()) - BigInt::from(poles.masses[1].clone());
        Ok(Self {
            visible_trace: &poles.masses[0] + &poles.masses[1],
            carrier_trace,
            radical: [&x + &y + &z, &x - &y - &z, -&x + &y - &z, -&x - &y + &z],
        })
    }

    /// Reconstruct the unnormalized Bloch components from the SIC coefficients.
    pub fn bloch_components(&self) -> [BigInt; 3] {
        let b = &self.radical;
        [
            (&b[0] + &b[1] - &b[2] - &b[3]) / 4u8,
            (&b[0] - &b[1] + &b[2] - &b[3]) / 4u8,
            (&b[0] - &b[1] - &b[2] + &b[3]) / 4u8,
        ]
    }
}

impl PoleProjection {
    /// Quadratic pole overlaps for a pure ray, retaining the mass outside the
    /// two-pole sector. An entangled state requires globally contracted masses.
    pub fn from_ray(truth: &FixedComplex, falsity: &FixedComplex, unread: BigUint) -> Self {
        let norm = |value: &FixedComplex| {
            (&value.re * &value.re + &value.im * &value.im)
                .to_biguint()
                .expect("sum of integer squares is nonnegative")
        };
        Self {
            masses: [norm(truth), norm(falsity), unread],
        }
    }

    /// Trace the other member of an entangled pair before reading one pole.
    /// Basis order is TT, FT, TF, FF (first coordinate is low-order).
    pub fn from_joint_pair(amplitudes: &[FixedComplex; 4], first: bool, unread: BigUint) -> Self {
        let mut masses = [BigUint::zero(), BigUint::zero(), unread];
        for (index, value) in amplitudes.iter().enumerate() {
            let pole = if first { index & 1 } else { (index >> 1) & 1 };
            masses[pole] += (&value.re * &value.re + &value.im * &value.im)
                .to_biguint()
                .expect("sum of integer squares is nonnegative");
        }
        Self { masses }
    }

    /// Keep the joint amplitudes selected by the pole projector. Subsequent
    /// readings use this conditional ray, retaining the other member's phase.
    pub fn project_joint_pair(
        amplitudes: &mut [FixedComplex; 4],
        first: bool,
        outcome: PoleOutcome,
    ) -> Result<(), String> {
        let selected = outcome
            .phase_bit()
            .ok_or("unread outcome is outside the represented pair sector")?
            as usize;
        let retained = amplitudes.iter().enumerate().any(|(index, value)| {
            let pole = if first { index & 1 } else { (index >> 1) & 1 };
            pole == selected && (!value.re.is_zero() || !value.im.is_zero())
        });
        if !retained {
            return Err("pole projector has zero conditional mass".into());
        }
        for (index, value) in amplitudes.iter_mut().enumerate() {
            let pole = if first { index & 1 } else { (index >> 1) & 1 };
            if pole != selected {
                value.re = BigInt::zero();
                value.im = BigInt::zero();
            }
        }
        Ok(())
    }

    pub fn total_mass(&self) -> BigUint {
        self.masses.iter().cloned().sum()
    }

    /// Each probability is masses[i]/total_mass; no unread mass is discarded.
    pub fn sample<F>(&self, entropy: F) -> Result<PoleOutcome, String>
    where
        F: FnMut(&mut [u8]) -> Result<(), String>,
    {
        match sample_born_masses(&self.masses, entropy)? {
            0 => Ok(PoleOutcome::Truth),
            1 => Ok(PoleOutcome::False),
            2 => Ok(PoleOutcome::Unread),
            _ => Err("invalid pole projection channel".into()),
        }
    }
}

pub struct FusionKernel {
    format: FixedPointFormat,
    associator: LocalMatrix,
    phases: [FixedComplex; 2],
}

impl FusionKernel {
    pub fn new(source: &BigUint) -> Result<Self, String> {
        let local = FibonacciLocal::new(source)?;
        let exchange = local.evaluate(&[1])?;
        Ok(Self {
            format: local.format().clone(),
            associator: local.associator().clone(),
            phases: [exchange.0[0].clone(), exchange.0[3].clone()],
        })
    }

    pub fn format(&self) -> &FixedPointFormat {
        &self.format
    }

    fn zero() -> FixedComplex {
        FixedComplex {
            re: BigInt::zero(),
            im: BigInt::zero(),
        }
    }

    fn coefficient(&self, left: u8, channel: u8, middle: u8, right: u8) -> FixedComplex {
        if left == 0 || right == 0 {
            let allowed = if left == 0 {
                middle == 1 && channel == right
            } else {
                middle == 1 && channel == 1
            };
            FixedComplex {
                re: if allowed {
                    self.format.scale()
                } else {
                    BigInt::zero()
                },
                im: BigInt::zero(),
            }
        } else {
            self.associator.0[2 * channel as usize + middle as usize].clone()
        }
    }

    /// F R F on one running-charge site, including boundary sectors.
    /// Vacuum is 0 and tau is 1. Forbidden fusion paths have zero weight.
    pub fn exchange(
        &self,
        left: u8,
        right: u8,
        input: u8,
        output: u8,
        inverse: bool,
    ) -> Result<FixedComplex, String> {
        if [left, right, input, output]
            .iter()
            .any(|charge| *charge > 1)
        {
            return Err("Fibonacci charge must be vacuum or tau".into());
        }
        if (left == 0 || right == 0) && (input != 1 || output != 1) {
            return Ok(Self::zero());
        }
        let mut result = Self::zero();
        for channel in 0..2u8 {
            let mut phase = self.phases[channel as usize].clone();
            if inverse {
                phase.im = -phase.im;
            }
            let product = self
                .coefficient(left, channel, output, right)
                .mul(&phase, &self.format)
                .mul(&self.coefficient(left, channel, input, right), &self.format);
            result.re += product.re;
            result.im += product.im;
        }
        Ok(result)
    }

    /// A stencil evaluates a braid without materializing the global fusion basis.
    pub fn stencil(
        &self,
        path: &[u8],
        generator: i32,
    ) -> Result<(usize, [FixedComplex; 2]), String> {
        if path.len() < 2
            || path[0] != 1
            || path.iter().any(|charge| *charge > 1)
            || path.windows(2).any(|pair| pair == [0, 0])
        {
            return Err("invalid running-charge Fibonacci fusion path".into());
        }
        let crossing = generator.unsigned_abs() as usize;
        if crossing == 0 || crossing >= path.len() {
            return Err("exchange is outside the fusion tree".into());
        }
        let site = crossing - 1;
        let left = if site == 0 { 0 } else { path[site - 1] };
        let right = path[site + 1];
        Ok((
            site,
            [
                self.exchange(left, right, path[site], 0, generator < 0)?,
                self.exchange(left, right, path[site], 1, generator < 0)?,
            ],
        ))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn coherent_state_projects_and_preserves_budget_failure_at_required_widths() {
        use super::*;
        for line in include_str!("../measurements/anyon-extractor-width-controls.tsv")
            .lines()
            .skip(1)
        {
            let fields: Vec<_> = line.split('\t').collect();
            let source = BigUint::parse_bytes(fields[2].as_bytes(), 10).unwrap();
            let wanted = &source - BigUint::from(1u8);
            let mut draws = 0;
            let prepared = PreparedRegister::uniform(&source, |bytes| {
                draws += 1;
                let candidate = if draws == 1 { &source } else { &wanted };
                let encoded = candidate.to_bytes_le();
                bytes.fill(0);
                bytes[..encoded.len()].copy_from_slice(&encoded);
                Ok(())
            })
            .unwrap();
            assert_eq!(draws, 2);
            assert_eq!(prepared.residue(), &wanted);
            let width = source.bits() as usize;
            assert_eq!(prepared.logical_bits().len(), 3 * width + 4);
            assert!(!prepared.logical_bits()[0]);
            for bit in 0..width {
                assert_eq!(prepared.logical_bits()[1 + bit], wanted.bit(bit as u64));
            }
            assert!(prepared.logical_bits()[width + 1..].iter().all(|bit| !bit));
            assert!(
                PreparedRegister::uniform(&source, |_| Err("entropy unavailable".into())).is_err()
            );
            let kernel = FusionKernel::new(&source).unwrap();
            for (path, expected) in [
                (vec![1, 0, 1, 0, 1, 0], PoleOutcome::Truth),
                (vec![1, 1, 1, 0, 1, 0], PoleOutcome::False),
                (vec![1, 1, 0, 1, 1, 0], PoleOutcome::Unread),
            ] {
                let mut control = FusionState::basis(&kernel, path, 32).unwrap();
                let outcome = control
                    .measure_control(|bytes| {
                        bytes.fill(0);
                        Ok(())
                    })
                    .unwrap();
                assert_eq!(outcome, expected);
                assert_eq!(control.path_count(), 1);
                if expected == PoleOutcome::Unread {
                    assert_eq!(outcome.phase_bit(), None);
                }
            }
            let path = vec![1, 1, 1, 1, 1, 0];
            let mut bounded = FusionState::basis(&kernel, path.clone(), 1).unwrap();
            let before = bounded.charge_masses(1).unwrap();
            assert!(bounded.exchange(&kernel, 2).is_err());
            assert_eq!(bounded.charge_masses(1).unwrap(), before);
            let mut state = FusionState::basis(&kernel, path, 32).unwrap();
            state.exchange(&kernel, 2).unwrap();
            assert_eq!(state.path_count(), 2);
            let masses = state.charge_masses(1).unwrap();
            assert!(masses.iter().all(|mass| !mass.is_zero()));
            let outcome = state
                .measure_charge(1, |bytes| {
                    bytes.fill(0);
                    Ok(())
                })
                .unwrap();
            assert!(!outcome);
            assert_eq!(state.path_count(), 1);
            assert!(state.charge_masses(1).unwrap()[1].is_zero());
            assert!(state
                .measure_charge(99, |bytes| {
                    bytes.fill(0);
                    Ok(())
                })
                .is_err());
            println!(
                "{}-bit source: coherent exchange, conditional projection, transactional budget",
                source.bits()
            );
        }
    }
    use super::*;
    use crate::anyon_pair::FibonacciPair;
    use num_traits::Signed;

    #[test]
    fn quadratic_poles_preserve_unread_weight_and_conditional_singlet_readings() {
        for line in include_str!("../measurements/anyon-extractor-width-controls.tsv")
            .lines()
            .skip(1)
        {
            let fields: Vec<_> = line.split('\t').collect();
            let n = BigUint::parse_bytes(fields[2].as_bytes(), 10).unwrap();
            let mass = &n * &n;
            let zero = FixedComplex {
                re: BigInt::zero(),
                im: BigInt::zero(),
            };
            let amplitude = FixedComplex {
                re: BigInt::from(n.clone()),
                im: BigInt::zero(),
            };
            let true_pole = PoleProjection::from_ray(&amplitude, &zero, BigUint::zero());
            assert_eq!(
                true_pole.masses,
                [mass.clone(), BigUint::zero(), BigUint::zero()]
            );
            let balanced = PoleProjection::from_ray(&amplitude, &amplitude, BigUint::zero());
            assert_eq!(&balanced.masses[0] * 2u8, balanced.total_mass());
            let full = PoleProjection::from_ray(&amplitude, &zero, mass.clone());
            assert_eq!(full.total_mass(), &mass * 2u8);
            let encoded = mass.to_bytes_le();
            let outcome = full
                .sample(|bytes| {
                    bytes.fill(0);
                    bytes[..encoded.len()].copy_from_slice(&encoded);
                    Ok(())
                })
                .unwrap();
            assert_eq!(outcome, PoleOutcome::Unread);
            assert_eq!(outcome.phase_bit(), None);
            let mut singlet = [
                zero.clone(),
                amplitude.clone(),
                FixedComplex {
                    re: -&amplitude.re,
                    im: BigInt::zero(),
                },
                zero,
            ];
            let marginal = PoleProjection::from_joint_pair(&singlet, true, BigUint::zero());
            assert_eq!(marginal.masses[0], marginal.masses[1]);
            PoleProjection::project_joint_pair(&mut singlet, true, PoleOutcome::Truth).unwrap();
            let partner = PoleProjection::from_joint_pair(&singlet, false, BigUint::zero());
            assert_eq!(
                partner
                    .sample(|_| Err("deterministic partner must not read entropy".into()))
                    .unwrap(),
                PoleOutcome::False
            );
            assert!(
                PoleProjection::project_joint_pair(&mut singlet, false, PoleOutcome::Truth)
                    .is_err()
            );
            assert_eq!(
                PoleProjection::from_joint_pair(&singlet, false, BigUint::zero()).masses,
                partner.masses
            );
            let imaginary = FixedComplex {
                re: BigInt::zero(),
                im: amplitude.re.clone(),
            };
            let sic = SicProjection::from_ray(&amplitude, &imaginary, mass.clone()).unwrap();
            assert_eq!(sic.visible_trace, &mass * 2u8);
            assert_eq!(sic.carrier_trace, &mass * 3u8);
            assert_eq!(sic.radical.iter().cloned().sum::<BigInt>(), BigInt::zero());
            assert_eq!(
                sic.bloch_components(),
                [BigInt::zero(), BigInt::from(&mass * 2u8), BigInt::zero()]
            );
        }
    }

    #[test]
    fn integer_born_selection_preserves_required_widths_and_rejects_out_of_range_entropy() {
        for line in include_str!("../measurements/anyon-extractor-width-controls.tsv")
            .lines()
            .skip(1)
        {
            let fields: alloc::vec::Vec<_> = line.split('\t').collect();
            let n = BigUint::parse_bytes(fields[2].as_bytes(), 10).unwrap();
            let masses = [n.clone(), n.clone()];
            assert!(!sample_fusion_channel(&masses, |bytes| {
                bytes.fill(0);
                Ok(())
            })
            .unwrap());
            let mut attempts = 0;
            assert!(sample_fusion_channel(&masses, |bytes| {
                attempts += 1;
                if attempts == 1 {
                    bytes.fill(255);
                } else {
                    bytes.fill(0);
                    let encoded = n.to_bytes_le();
                    bytes[..encoded.len()].copy_from_slice(&encoded);
                }
                Ok(())
            })
            .unwrap());
            assert_eq!(attempts, 2);
            assert!(
                sample_fusion_channel(&[BigUint::zero(), n.clone()], |_| Err(
                    "entropy should not be read".into()
                ))
                .unwrap()
            );
            assert!(
                sample_fusion_channel(&masses, |_| Err("entropy source failed".into())).is_err()
            );
        }
        assert!(sample_fusion_channel(&[BigUint::zero(), BigUint::zero()], |_| Ok(())).is_err());
    }

    #[test]
    fn fusion_stencils_match_existing_pair_operators_at_all_source_widths() {
        let paths = [
            [1, 0, 1, 0, 1, 0],
            [1, 0, 1, 1, 1, 0],
            [1, 1, 0, 1, 1, 0],
            [1, 1, 1, 0, 1, 0],
            [1, 1, 1, 1, 1, 0],
        ];
        for line in include_str!("../measurements/anyon-extractor-width-controls.tsv")
            .lines()
            .skip(1)
        {
            let fields: alloc::vec::Vec<_> = line.split('\t').collect();
            let source = BigUint::parse_bytes(fields[2].as_bytes(), 10).unwrap();
            let kernel = FusionKernel::new(&source).unwrap();
            let pair = FibonacciPair::new(&source).unwrap();
            for generator in [-5, -4, -3, -2, -1, 1, 2, 3, 4, 5] {
                let matrix = pair.evaluate(&[generator]).unwrap();
                for (column, input) in paths.iter().enumerate() {
                    let (site, weights) = kernel.stencil(input, generator).unwrap();
                    for (row, output) in paths.iter().enumerate() {
                        let expected = if (0..6).any(|i| i != site && input[i] != output[i]) {
                            FusionKernel::zero()
                        } else {
                            weights[output[site] as usize].clone()
                        };
                        let actual = &matrix.0[5 * row + column];
                        assert!((&expected.re - &actual.re).abs() <= BigInt::from(8u8));
                        assert!((&expected.im - &actual.im).abs() <= BigInt::from(8u8));
                    }
                }
            }
            println!(
                "{}-bit source: signed fusion exchange stencils match all pair channels",
                source.bits()
            );
        }
    }
}
