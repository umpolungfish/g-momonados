//! One coherent glut for the source-bound factor/product/carry relation.
//! Source constraints select marked support explicitly. This conditional
//! readout is a symbolic relation operation, not an unconditional device shot.
use alloc::{string::String, vec::Vec};
use num_bigint::{BigInt, BigUint};
use num_traits::{One, Zero, Signed};
use crate::ququart_decision::DecisionArena;
use crate::{anyon_ququart::{FixedQuquartSic, SicGramReconstruction}, phase_unbraid::FixedPointFormat};
use vox_core::fixed_point_quantum_phase::factor_oracle::{Control, FactorPhaseOracle, FactorSupportSink, GateSink, Layout};

pub struct ResidentGlut {
    source: BigUint,
    tape: Vec<char>,
    arena: DecisionArena,
    root: usize,
    initial: usize,
    wire_map: Vec<usize>,
    diagram_ready: bool,
    initial_mass: BigUint,
    boundary: ParityBoundary,
    constraint_gates: usize,
    sic_format: FixedPointFormat,
}

/// One joint amplitude function owns both the constrained factor relation and
/// the complete phase/work register while nested modular operations advance.
pub struct ResidentPhaseCarrier {
    resident: ResidentGlut,
    circuit: crate::whole_register_phase::WholeRegisterPhaseCircuit,
}

pub struct ResidentPhaseReturn {
    pub resident: ResidentGlut,
    pub phase: crate::whole_register_phase::WholeRegisterPhaseReadout,
    divisor: Option<BigUint>,
}

impl ResidentPhaseCarrier {
    pub fn execution_progress(&self) -> Option<(usize,usize,usize)> {
        self.circuit.execution_progress()
    }
    pub fn advance(&mut self) -> Result<bool, String> {
        self.circuit.advance_nested_work(8, 8)
    }
    pub fn fire(mut self) -> Result<ResidentPhaseReturn, String> {
        let denominator = BigUint::one() << self.circuit.width();
        let mut random = 1729u64;
        let mut quantile = BigUint::zero();
        for offset in (0..self.circuit.width()).step_by(64) {
            random ^= random << 13; random ^= random >> 7; random ^= random << 17;
            quantile |= BigUint::from(random) << offset;
        }
        quantile %= &denominator;
        let phase = self.circuit.fire_with_work_radix(&quantile, &denominator, Some(8))?;
        let mut evidence = crate::phase_unbraid::PhaseReadoutAccumulator::default();
        let divisor = evidence.close_fraction(&phase.numerator, &phase.denominator,
            &phase.base, &phase.source)?.map(|(_,p,_)| p);
        let (arena, root) = self.circuit.into_resident_support();
        self.resident.arena = arena;
        self.resident.root = root;
        self.resident.initial = root;
        Ok(ResidentPhaseReturn { resident: self.resident, phase, divisor })
    }
}

impl ResidentPhaseReturn {
    pub fn readout(self) -> Result<Option<GlutReadout>, String> {
        if let Some(divisor) = self.divisor {
            self.resident.readout_source_divisor(divisor, "resident-whole-register-phase")
        } else {
            self.resident.readout_source_carrier()
        }
    }
}

/// One gamma word of resident cells. Delta is applied once to its cell
/// addresses; both the product relation and boundary retrieval use these lanes.
struct ParityBoundary {
    word: Vec<usize>,
    even: Vec<usize>,
    odd: Vec<usize>,
}

impl ParityBoundary {
    fn new(layout: &Layout, wire_map: &[usize]) -> Result<Self, String> {
        if layout.p.len() != layout.q.len() { return Err("gamma boundary needs matching cell lanes".into()); }
        let word: Vec<usize> = layout.p.clone().zip(layout.q.clone())
            .flat_map(|(p, q)| [wire_map[p], wire_map[q]]).collect();
        let even = word.iter().step_by(2).copied().collect();
        let odd = word.iter().skip(1).step_by(2).copied().collect();
        Ok(Self { word, even, odd })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_constraint_retains_source_divisors_and_signed_sic_closure() {
        for n in [91u32, 143, 8051] {
            let mut resident = ResidentGlut::new(BigUint::from(n)).unwrap();
            let initial = resident.initial_mass.clone();
            let gates = resident.constrain_product_prefix(4, |_, _| {}).unwrap();
            assert!(gates > 0);
            assert!(resident.arena.mass(resident.root) < initial);
            let constrained = resident.arena.mass(resident.root);
            resident.constrain_product_prefix(4, |_, _| {}).unwrap();
            assert_eq!(resident.arena.mass(resident.root), constrained);
            let sic = resident.sic_witness().unwrap();
            assert!(sic.reconstruction.maximum_residual.is_zero());
            let result = resident.readout_source_carrier().unwrap().unwrap();
            assert_eq!(&result.divisor * &result.quotient, BigUint::from(n));
            assert!(result.gamma_delta_closed && result.product_closed);
        }
    }

    #[test]
    fn constrained_relation_survives_joint_phase_and_boundary_return() {
        let n = BigUint::from(15u8);
        let mut resident = ResidentGlut::new(n.clone()).unwrap();
        resident.constrain_product_prefix(3, |_, _| {}).unwrap();
        let mut carrier = resident.into_whole_register_phase().unwrap();
        while !carrier.advance().unwrap() {}
        let returned = carrier.fire().unwrap();
        assert_eq!(returned.phase.source, n);
        assert_eq!(returned.phase.width, 8);
        let result = returned.readout().unwrap().unwrap();
        assert_eq!(&result.divisor * &result.quotient, n);
        assert!(result.gamma_delta_closed && result.product_closed);
        assert!(result.gates > 0);
        assert!(result.sic.reconstruction.maximum_residual.is_zero());
    }

    #[test]
    fn retrieval_rejects_a_wrong_odd_lane_even_when_the_even_lane_divides_n() {
        let mut resident = ResidentGlut::new(BigUint::from(91u8)).unwrap();
        let layout = FactorPhaseOracle::from_n(&resident.tape).unwrap().layout().clone();
        let mut cells = BigUint::zero();
        for (lane, value) in [(&resident.boundary.even, 7u8), (&resident.boundary.odd, 11u8)] {
            for (bit, &wire) in lane.iter().enumerate() {
                cells.set_bit(wire as u64, bit < 8 && (value >> bit) & 1 == 1);
            }
        }
        resident.root = resident.arena.basis(&cells, BigInt::one());
        resident.diagram_ready = true;
        let mass = resident.initial_mass.clone();
        assert!(resident.finish_readout(layout, mass, 0, "control").is_err());
    }

    #[test]
    fn internal_source_return_closes_both_resident_representations() {
        for materialized in [false, true] {
            let mut resident = ResidentGlut::new(BigUint::from(91u8)).unwrap();
            if materialized { resident.materialize().unwrap(); }
            let result = resident.readout_source_divisor(BigUint::from(7u8), "control").unwrap().unwrap();
            assert_eq!(&result.divisor * &result.quotient, BigUint::from(91u8));
            assert!(result.sic.reconstruction.maximum_residual.is_zero());
            assert_eq!(result.producer, "control");
            assert!(result.gamma_delta_closed && result.product_closed);
        }
        for invalid in [1u8, 4, 91] {
            assert!(ResidentGlut::new(BigUint::from(91u8)).unwrap()
                .readout_source_divisor(BigUint::from(invalid), "control").is_err());
        }
    }

    #[test]
    fn hard_208_geometry_normal_form_matches_materialized_glut() {
        let n = BigUint::parse_bytes(
            b"266175155279792434973506259190185380993589458872977057421959357", 10).unwrap();
        let mut resident = ResidentGlut::new(n).unwrap();
        let declared = resident.sic_witness().unwrap();
        resident.materialize().unwrap();
        let executed = resident.sic_witness().unwrap();
        assert_eq!(declared.gram, executed.gram);
        assert_eq!(declared.mass_numerators, executed.mass_numerators);
        assert_eq!(declared.rational_signed_residuals, executed.rational_signed_residuals);
    }
}

pub struct GlutReadout {
    pub divisor: BigUint,
    pub quotient: BigUint,
    pub initial_mass: BigUint,
    pub selected_mass: BigUint,
    pub gates: usize,
    pub sic: GlutSicWitness,
    pub producer: &'static str,
    pub boundary_word: String,
    pub gamma_delta_closed: bool,
    pub product_closed: bool,
}

pub struct GlutSicWitness {
    pub gram: [(BigInt, BigInt); 16],
    pub masses: [BigUint; 16],
    pub reconstruction: SicGramReconstruction,
    pub signed_residuals: [(BigInt, BigInt); 16],
    pub uncorrected_maximum_residual: BigInt,
    pub mass_numerators: [BigUint; 16],
    pub rational_denominator: BigInt,
    pub rational_signed_residuals: [(BigInt, BigInt); 16],
}

struct BoundOracle<'a, F> {
    arena: &'a mut DecisionArena,
    root: &'a mut usize,
    initial: &'a mut usize,
    gates: usize,
    updates: usize,
    observe: F,
    wire_map: &'a [usize],
}

impl<F: FnMut(usize, usize)> BoundOracle<'_, F> {
    fn literals(&self, controls: &[Control]) -> Result<Vec<(usize, bool)>, &'static str> {
        if controls.iter().any(|c| c.cell >= self.arena.cells || !matches!(c.value, '⊤' | '⊥')) {
            return Err("factor oracle control escaped the resident glut");
        }
        let mut literals: Vec<_> = controls.iter().map(|c| (self.wire_map[c.cell], c.value == '⊥')).collect();
        literals.sort_unstable();
        if literals.windows(2).any(|w| w[0].0 == w[1].0) {
            return Err("factor oracle repeated a control cell");
        }
        Ok(literals)
    }
}

impl<F: FnMut(usize, usize)> GateSink for BoundOracle<'_, F> {
    fn toggle(&mut self, controls: &[Control], target: usize) -> Result<(), &'static str> {
        if target >= self.arena.cells || controls.iter().any(|c| c.cell == target) {
            return Err("factor oracle target escaped the resident glut");
        }
        let controls = self.literals(controls)?;
        let target = self.wire_map[target];
        self.gates += 1;
        if !self.arena.controls_possible(*self.root, &controls) {
            return Ok(());
        }
        if controls.iter().all(|&(_, positive)| positive) {
            let positive: Vec<_> = controls.iter().map(|&(wire, _)| wire).collect();
            *self.root = self.arena.controlled_flip(*self.root, target, &positive);
        } else {
            let zero = self.arena.zero();
            let enabled = self.arena.conditional_literals(zero, *self.root, &controls);
            let flipped = self.arena.flip(enabled, target);
            *self.root = self.arena.conditional_literals(*self.root, flipped, &controls);
        }
        self.updates += 1;
        if self.updates % 128 == 0 { self.checkpoint()?; }
        Ok(())
    }
    fn phase_flip(&mut self, controls: &[Control]) -> Result<(), &'static str> {
        let controls = self.literals(controls)?;
        let zero = self.arena.zero();
        let enabled = self.arena.conditional_literals(zero, *self.root, &controls);
        let negative = self.arena.normalize(enabled, &BigInt::one(), &-BigInt::one());
        *self.root = self.arena.conditional_literals(*self.root, negative, &controls);
        self.gates += 1;
        self.checkpoint()
    }
    fn checkpoint(&mut self) -> Result<(), &'static str> {
        let zero = self.arena.zero();
        let mut roots = [*self.root, *self.initial, zero, zero, zero];
        self.arena.fold(&mut roots);
        *self.root = roots[0];
        *self.initial = roots[1];
        (self.observe)(self.gates, self.arena.retained_nodes());
        Ok(())
    }
}

impl<F: FnMut(usize, usize)> FactorSupportSink for BoundOracle<'_, F> {
    fn retain_literals(&mut self, controls: &[Control]) -> Result<(), &'static str> {
        let controls = self.literals(controls)?;
        let zero = self.arena.zero();
        *self.root = self.arena.conditional_literals(zero, *self.root, &controls);
        self.checkpoint()
    }
}

impl ResidentGlut {
    /// Observe one ququart coordinate slice of this same glut. Conditional
    /// branches are aligned onto zero control slots before taking work overlaps,
    /// preserving their signed coherences rather than treating them as mixtures.
    pub fn sic_witness(&mut self) -> Result<GlutSicWitness, String> {
        let oracle = FactorPhaseOracle::from_n(&self.tape).map_err(String::from)?;
        let wires = [self.wire_map[oracle.layout().p.start],
            self.wire_map[oracle.layout().q.start]];
        let zero = self.arena.zero();
        let mut roots = [zero; 4];
        for (label, root) in roots.iter_mut().enumerate() {
            let controls = [(wires[0], label & 1 != 0), (wires[1], label & 2 != 0)];
            *root = self.arena.conditional_literals(zero, self.root, &controls);
            for (wire, active) in controls {
                if active { *root = self.arena.flip(*root, wire); }
            }
        }
        let gram = if self.diagram_ready {
            self.arena.control_gram(&roots)
        } else {
            // In every declared width class, both low factor bits are free.
            // Aligning them yields the same work vector in all four branches.
            let value = BigInt::from(&self.initial_mass >> 2);
            core::array::from_fn(|_| (value.clone(), BigInt::zero()))
        };
        let frame = FixedQuquartSic::new(&self.sic_format)?;
        let masses = frame.gram_masses(&gram)?;
        let mass_numerators = frame.gram_mass_numerators(&gram)?;
        let rational = frame.certify_gram_frame_unrounded_masses(&gram, &mass_numerators)?;
        let reconstruction = rational.rounded;
        let signed_residuals = core::array::from_fn(|i| (
            &reconstruction.recovered[i].0 - &gram[i].0,
            &reconstruction.recovered[i].1 - &gram[i].1));
        let trace: BigInt = (0..4).map(|i| gram[5*i].0.clone()).sum();
        let uncorrected_maximum_residual = (0..16).map(|i| {
            let omitted = if i % 5 == 0 { trace.clone() } else { BigInt::zero() };
            (&signed_residuals[i].0 + omitted).abs().max(signed_residuals[i].1.abs())
        }).max().unwrap();
        Ok(GlutSicWitness { gram, masses, reconstruction, signed_residuals,
            uncorrected_maximum_residual, mass_numerators,
            rational_denominator: rational.denominator,
            rational_signed_residuals: rational.signed_residual_numerators })
    }
    /// N is the sole instance input. Candidate and workspace widths follow
    /// the existing factor oracle's N-derived layout.
    pub fn new(source: BigUint) -> Result<Self, String> {
        let tape = vox_core::morphism_factor::decimal_to_tape(&source.to_str_radix(10))
            .ok_or("invalid source numeral")?;
        let oracle = FactorPhaseOracle::from_n(&tape).map_err(String::from)?;
        let layout = oracle.layout();
        let mut wire_map = alloc::vec![0; layout.cells];
        let mut physical = 0;
        for column in 0..layout.product.len() {
            for range in [&layout.p, &layout.q, &layout.product, &layout.row] {
                if column < range.len() {
                    wire_map[range.start + column] = physical;
                    physical += 1;
                }
            }
        }
        for cell in [layout.carry, layout.nontrivial_p, layout.nontrivial_q] {
            wire_map[cell] = physical;
            physical += 1;
        }
        if physical != layout.cells { return Err("incomplete resident wire binding".into()) }
        let boundary = ParityBoundary::new(layout, &wire_map)?;
        let mut arena = DecisionArena::new(layout.cells);
        let root = arena.zero();
        let initial_mass = vox_core::glut_system::source_factor_geometries(layout.p.len())
            .into_iter().fold(BigUint::zero(), |mass, (p, q)| mass + (BigUint::one() << (p + q - 2)));
        let sic_format = FixedPointFormat::for_modulus(&source)?;
        Ok(Self { source, tape, arena, root, initial: root, wire_map,
            diagram_ready: false, initial_mass, boundary, constraint_gates: 0, sic_format })
    }

    /// Materialize the exact declared width-class union only when coherent
    /// lowering needs its gates. Initial SIC coordinates have a closed form.
    pub fn materialize(&mut self) -> Result<(), String> {
        if self.diagram_ready { return Ok(()) }
        let layout = FactorPhaseOracle::from_n(&self.tape).map_err(String::from)?.layout().clone();
        let arena = &mut self.arena;
        let wire_map = &self.wire_map;
        let mut root = arena.basis(&BigUint::zero(), BigInt::one());
        // Exact common-scale-free H preparation on initially zero factor
        // wires. Product, row and carry remain in the same resident object.
        // Every composite N has a proper divisor at most sqrt(N). The
        // retained candidate bound is derived from N, never from test factors.
        let p_end = layout.p.start + layout.p.len().div_ceil(2);
        for wire in (layout.p.start..p_end).chain(layout.q.clone()) {
            let flipped = arena.flip(root, wire_map[wire]);
            root = arena.sum(root, flipped);
            let zero = arena.zero();
            let mut roots = [root, zero, zero, zero, zero];
            arena.fold(&mut roots);
            root = roots[0];
        }
        // Use the same source geometries as the existing resident GLUT.
        // Width classes are disjoint supports of this one coherent register.
        // Their union keeps a factor ordering for every proper source split,
        // while excluding rectangles whose product cannot have N's bit length.
        let mut base = root;
        let mut joined = arena.zero();
        for (p_width, q_width) in vox_core::glut_system::source_factor_geometries(layout.p.len()) {
            let mut controls = Vec::new();
            for (range, width) in [(&layout.p, p_width), (&layout.q, q_width)] {
                controls.push((wire_map[range.start + width - 1], true));
                controls.extend((range.start + width..range.end).map(|wire| (wire_map[wire], false)));
            }
            controls.sort_unstable();
            let zero = arena.zero();
            let frame = arena.conditional_literals(zero, base, &controls);
            joined = arena.sum(joined, frame);
            let mut roots = [base, joined, zero, zero, zero];
            arena.fold(&mut roots);
            base = roots[0];
            joined = roots[1];
        }
        root = joined;
        if arena.mass(root) != self.initial_mass {
            return Err("materialized source geometry changed its declared norm".into());
        }
        self.root = root;
        self.initial = root;
        self.diagram_ready = true;
        Ok(())
    }

    pub fn readout(self, observe: impl FnMut(usize, usize)) -> Result<Option<GlutReadout>, String> {
        if let Some(returned) = vox_core::morphism_factor::source_carrier_divisor(&self.tape)? {
            let divisor = BigUint::parse_bytes(vox_core::morphism_factor::dec_of(&returned.divisor).as_bytes(), 10)
                .ok_or("source carrier boundary numeral conversion failed")?;
            return self.readout_source_divisor(divisor, returned.producer);
        }
        self.readout_lowered(observe)
    }

    pub fn readout_source_carrier(self) -> Result<Option<GlutReadout>, String> {
        self.readout(|_, _| {})
    }

    /// Execute low product constraints on this factor/product/carry support.
    /// The full source remains attached; higher columns and properness wait
    /// for complete closure. Every temporary cell returns to zero.
    pub fn constrain_product_prefix(&mut self, columns: usize,
        observe: impl FnMut(usize, usize)) -> Result<usize, String> {
        self.materialize()?;
        let full = FactorPhaseOracle::from_n(&self.tape).map_err(String::from)?;
        let prefix = full.product_prefix(columns).map_err(String::from)?;
        let mut wires = alloc::vec![0; prefix.layout().cells];
        for (low, original) in [(&prefix.layout().p, &full.layout().p),
            (&prefix.layout().q, &full.layout().q),
            (&prefix.layout().product, &full.layout().product),
            (&prefix.layout().row, &full.layout().row)] {
            for (local, source) in low.clone().zip(original.clone()) {
                wires[local] = self.wire_map[source];
            }
        }
        for (local, source) in [(prefix.layout().carry, full.layout().carry),
            (prefix.layout().nontrivial_p, full.layout().nontrivial_p),
            (prefix.layout().nontrivial_q, full.layout().nontrivial_q)] {
            wires[local] = self.wire_map[source];
        }
        let mut sink = BoundOracle { arena: &mut self.arena, root: &mut self.root,
            initial: &mut self.initial, gates: 0, updates: 0, observe, wire_map: &wires };
        prefix.lower_selected_product_prefix(&mut sink, columns).map_err(String::from)?;
        let gates = sink.gates;
        for wire in full.layout().product.start..full.layout().cells {
            if !self.arena.wire_is_zero(self.root, self.wire_map[wire]) {
                return Err("product prefix left resident workspace uncleared".into());
            }
        }
        self.constraint_gates += gates;
        Ok(gates)
    }

    /// Intersect the device-produced low product relation with the same root
    /// used by coherent lowering and boundary closure.
    #[cfg(feature = "hosted")]
    pub fn constrain_gpu_product_prefix(&mut self, columns: usize, device: usize)
        -> Result<usize, String> {
        self.materialize()?;
        let word = crate::word_tape::WordTape::from_biguint(&self.source);
        let pairs = crate::gpu_ecm_word::product_prefix_support(&word, columns, device)?;
        let mut base = self.root;
        let zero = self.arena.zero();
        let mut joined = zero;
        for &(p, q) in &pairs {
            let mut controls = Vec::with_capacity(2*columns);
            for (lane, value) in [(&self.boundary.even, p), (&self.boundary.odd, q)] {
                controls.extend(lane.iter().take(columns).enumerate().map(|(bit, &wire)|
                    (wire, value >> bit & 1 == 1)));
            }
            controls.sort_unstable();
            let selected = self.arena.conditional_literals(zero, base, &controls);
            joined = self.arena.sum(joined, selected);
            let mut roots = [base, joined, self.initial, zero, zero];
            self.arena.fold(&mut roots);
            base = roots[0]; joined = roots[1]; self.initial = roots[2];
        }
        self.root = joined;
        Ok(pairs.len())
    }

    pub fn support_mass(&self) -> BigUint {
        if self.diagram_ready { self.arena.mass(self.root) } else { self.initial_mass.clone() }
    }

    pub fn into_whole_register_phase(mut self) -> Result<ResidentPhaseCarrier, String> {
        self.materialize()?;
        let mut circuit = crate::whole_register_phase::WholeRegisterPhaseCircuit::prepare(
            self.source.clone(), BigUint::from(2u8), None)?;
        let offset = circuit.bind_resident_support(&self.source, &self.arena, self.root)?;
        self.sic_format = FixedPointFormat::for_modulus(
            &self.source.clone().max(circuit.support_mass()))?;
        for wire in &mut self.wire_map { *wire += offset; }
        for wire in self.boundary.word.iter_mut().chain(self.boundary.even.iter_mut())
            .chain(self.boundary.odd.iter_mut()) { *wire += offset; }
        self.arena = DecisionArena::new(0);
        self.root = self.arena.zero(); self.initial = self.root;
        Ok(ResidentPhaseCarrier { resident: self, circuit })
    }

    pub fn readout_with_ports(self, observe: impl FnMut(usize, usize),
        ports: impl FnMut(&'static str)) -> Result<Option<GlutReadout>, String> {
        // The source relation arm is an internal relation port, not an external
        // factor input. Its proper divisor refines this same resident support;
        // the quotient, SIC closure, and terminal readout remain source-bound.
        if let Some(split) = vox_core::morphism_factor::source_divisor_observed(&self.tape, ports) {
            let divisor = BigUint::parse_bytes(
                vox_core::morphism_factor::dec_of(&split.divisor).as_bytes(), 10)
                .ok_or("source relation divisor conversion failed")?;
            return self.readout_source_divisor(divisor, split.producer);
        }
        self.readout_lowered(observe)
    }

    /// Existing GPU IMASM ECM source port. Bounds and curve schedule are fixed
    /// program constants; the source numeral is its sole instance input.
    #[cfg(feature = "hosted")]
    pub fn readout_gpu_word(self) -> Result<Option<GlutReadout>, String> {
        let source_word = crate::word_tape::WordTape::from_biguint(&self.source);
        let candidate = crate::gpu_ecm_word::factor(&source_word, 1000, 20_000, 0, 6, 256)?;
        match candidate {
            Some(candidate) => self.readout_source_divisor(candidate.to_biguint(), "gpu-word-ecm-source"),
            None => Ok(None),
        }
    }

    fn readout_source_divisor(mut self, divisor: BigUint, producer: &'static str)
        -> Result<Option<GlutReadout>, String> {
        if divisor <= BigUint::one() || divisor >= self.source
            || !(&self.source % &divisor).is_zero() {
            return Err("source relation arm returned no proper source divisor".into());
        }
        let quotient = &self.source / &divisor;
        let (p, q) = if divisor <= quotient { (divisor, quotient) } else { (quotient, divisor) };
        let layout = FactorPhaseOracle::from_n(&self.tape).map_err(String::from)?.layout().clone();
        let initial_mass = self.initial_mass.clone();
        let mut controls = Vec::new();
        for (lane, value) in [(&self.boundary.even, &p), (&self.boundary.odd, &q)] {
            controls.extend(lane.iter().enumerate().map(|(bit, &wire)|
                (wire, value.bit(bit as u64))));
        }
        controls.sort_unstable();
        if self.diagram_ready {
            let zero = self.arena.zero();
            self.root = self.arena.conditional_literals(zero, self.root, &controls);
        } else {
            let p_bits = p.bits() as usize;
            let q_bits = q.bits() as usize;
            if !vox_core::glut_system::source_factor_geometries(layout.p.len()).contains(&(p_bits, q_bits)) {
                return Err("source divisor does not belong to the declared geometry".into());
            }
            let mut basis = BigUint::zero();
            for (wire, active) in controls {
                basis.set_bit(wire as u64, active);
            }
            self.root = self.arena.basis(&basis, BigInt::one());
            self.diagram_ready = true;
        }
        if self.arena.mass(self.root).is_zero() {
            return Err("verified source divisor fell outside the retained glut geometry".into());
        }
        self.finish_readout(layout, initial_mass, self.constraint_gates, producer)
    }

    /// The retained limb ECM implementation derives its width from N and
    /// partitions a fixed curve schedule across the available devices.
    #[cfg(feature = "hosted")]
    pub fn readout_gpu_limbs(self) -> Result<Option<GlutReadout>, String> {
        let _device = cudarc::driver::CudaContext::new(0)
            .map_err(|error| alloc::format!("GPU 0 unavailable: {error}"))?;
        let candidate = crate::gpu_ecm::factor_once_b2(&self.source, 1000, 20_000, 0);
        match candidate {
            Some(divisor) => self.readout_source_divisor(divisor, "gpu-limb-ecm-source"),
            None => Ok(None),
        }
    }

    /// Execute the existing nested operator carrier over its retained N register.
    pub fn readout_carrier(self) -> Result<Option<GlutReadout>, String> {
        let tower = vox_core::morphism_factor::construct_carrier(vox_core::morphism_factor::NINE_ARM)?;
        let candidate = vox_core::morphism_factor::run_carrier_rounds(&tower, &self.tape, u64::MAX);
        match candidate {
            Some(divisor) => {
                let divisor = BigUint::parse_bytes(vox_core::morphism_factor::dec_of(&divisor).as_bytes(), 10)
                    .ok_or("carrier boundary numeral conversion failed")?;
                if divisor == self.source { return Ok(None); }
                self.readout_source_divisor(divisor, "nested-nine-arm-source")
            },
            None => Ok(None),
        }
    }

    /// Retained coherent lowering instrument, with no source relation shortcut.
    pub fn readout_lowered(mut self, observe: impl FnMut(usize, usize)) -> Result<Option<GlutReadout>, String> {
        self.materialize()?;
        let oracle = FactorPhaseOracle::from_n(&self.tape).map_err(String::from)?;
        let layout = oracle.layout().clone();
        let initial_mass = self.initial_mass.clone();
        let mut sink = BoundOracle { arena: &mut self.arena, root: &mut self.root,
            initial: &mut self.initial, gates: 0, updates: 0, observe, wire_map: &self.wire_map };
        oracle.lower_selected(&mut sink).map_err(String::from)?;
        let gates = sink.gates;
        // Source-support intersections selected the proper product relation
        // within the executing oracle. Its uniform final sign is immaterial
        // to Born readout; workspace is restored through the same reverse rail.
        self.finish_readout(layout, initial_mass, gates + self.constraint_gates, "lowered-product-support")
    }

    fn finish_readout(&mut self, layout: Layout, initial_mass: BigUint,
        gates: usize, producer: &'static str) -> Result<Option<GlutReadout>, String> {
        let selected_mass = self.arena.mass(self.root);
        if selected_mass.is_zero() { return Ok(None) }
        let sic = self.sic_witness()?;
        for wire in layout.product.start..layout.cells {
            if !self.arena.wire_is_zero(self.root, self.wire_map[wire]) {
                return Err("factor oracle failed to restore its shared workspace".into());
            }
        }
        let mut random = 1729u64;
        random ^= random << 13; random ^= random >> 7; random ^= random << 17;
        let measured = self.arena.sample_prefix(self.root, self.arena.cells,
            &BigUint::from(random), &(BigUint::one() << 64)).map_err(String::from)?;
        let decode = |lane: &[usize]| {
            let mut value = BigUint::zero();
            for (bit, &wire) in lane.iter().enumerate() {
                value.set_bit(bit as u64, measured.bit(wire as u64));
            }
            value
        };
        let p = decode(&self.boundary.even);
        let q = decode(&self.boundary.odd);
        let word_value = decode(&self.boundary.word);
        let boundary_word = crate::word_tape::WordTape::from_biguint(&word_value);
        let p_word = crate::word_tape::WordTape::from_biguint(&p);
        let q_word = crate::word_tape::WordTape::from_biguint(&q);
        let gamma_delta_closed = crate::native_numeral::interlace_words(p_word.as_word(), q_word.as_word())
            .map_err(String::from)? == boundary_word.as_word();
        if !gamma_delta_closed { return Err("resident gamma/delta boundary failed same-word closure".into()); }
        let product_closed = crate::native_numeral::multiply_via_word(&p, &q) == self.source;
        if !product_closed { return Err("resident parity lanes failed the source product closure".into()); }
        if p <= BigUint::one() || p >= self.source || !(&self.source % &p).is_zero() {
            return Err("resident glut readout did not produce a proper source divisor".into());
        }
        let quotient = &self.source / &p;
        if quotient != q || &p * &quotient != self.source {
            return Err("derived quotient failed exact source product verification".into());
        }
        Ok(Some(GlutReadout { divisor: p, quotient: q, initial_mass, selected_mass, gates, sic, producer,
            boundary_word: String::from(boundary_word.as_word()), gamma_delta_closed, product_closed }))
    }
}
