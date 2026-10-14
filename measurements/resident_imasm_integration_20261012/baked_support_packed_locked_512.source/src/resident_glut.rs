//! One coherent glut for the source-bound factor/product/carry relation.
//! Source constraints select marked support explicitly. This conditional
//! readout is a symbolic relation operation, not an unconditional device shot.
use alloc::{string::String, vec::Vec};
use alloc::boxed::Box;
use num_bigint::{BigInt, BigUint};
use num_traits::{One, Zero};
use crate::ququart_decision::DecisionArena;
use crate::word_tape::{SignedWordTape, WordTape};
use crate::{anyon_ququart::{FixedQuquartSic, SicGramReconstruction}, phase_unbraid::FixedPointFormat};
use vox_core::fixed_point_quantum_phase::factor_oracle::{Control, FactorPhaseOracle, FactorSupportSink, GateSink, Layout};

pub struct ResidentGlut {
    source: WordTape,
    tape: Vec<char>,
    arena: DecisionArena,
    root: usize,
    initial: usize,
    banked_roots: Vec<usize>,
    preparation_sic_ancestry: Option<String>,
    wire_map: Vec<usize>,
    diagram_ready: bool,
    initial_mass: WordTape,
    boundary: ParityBoundary,
    constraint_gates: usize,
    sic_format: FixedPointFormat,
    sic_frame: Option<FixedQuquartSic>,
}

/// One joint amplitude function owns both the constrained factor relation and
/// the complete phase/work register while nested modular operations advance.
pub struct ResidentPhaseCarrier {
    resident: ResidentGlut,
    circuit: crate::whole_register_phase::WholeRegisterPhaseCircuit,
    next_product_columns: usize,
    source_producer: Option<&'static str>,
    phase_source_relation: bool,
    phase_evidence: crate::phase_unbraid::PhaseReadoutAccumulator,
    #[cfg(feature = "hosted")]
    reentry: Option<ResidentReentry>,
}

#[cfg(feature = "hosted")]
struct ResidentReentry {
    driver: Option<vox_core::dialectic_lean::ResidentY>,
    clock: crate::ququart_reentry_clock::RuntimeClock,
    record: serde_json::Value,
    state: u8,
    ticks: u64,
    forward_calls: u64,
    reverse_calls: u64,
}

#[cfg(feature = "hosted")]
impl ResidentReentry {
    fn from_record(record: &serde_json::Value) -> Result<Self, String> {
        let driver = vox_core::dialectic_lean::ResidentY::from_certified_program(
            record["Y_program"].as_str().ok_or("missing baked Y program")?)?;
        let clock = crate::ququart_reentry_clock::RuntimeClock::from_record(&record["clock"])?;
        let state = record["clock"]["seed"].as_u64().ok_or("missing clock seed")? as u8;
        Ok(Self { driver: Some(driver), clock, record: record.clone(), state,
            ticks: 0, forward_calls: 0, reverse_calls: 0 })
    }

    fn tick(&mut self) -> Result<bool, String> {
        if self.state != self.clock.read(self.ticks) { return Err("resident clock lost its cached observation".into()); }
        self.state = self.clock.step(self.state);
        self.ticks = self.ticks.checked_add(1).ok_or("resident clock overflow")?;
        if self.state != self.clock.read(self.ticks) { return Err("resident clock lost its reduction successor".into()); }
        let reverse = self.state & 1 != 0;
        if reverse { self.reverse_calls += 1; } else { self.forward_calls += 1; }
        Ok(reverse)
    }
}

pub struct ResidentPhaseReturn {
    program: crate::whole_register_phase::WholeRegisterPhaseProgram,
    pub resident: ResidentGlut,
    pub phase: crate::whole_register_phase::WholeRegisterPhaseReadout,
    source_producer: Option<&'static str>,
    phase_source_relation: bool,
    phase_evidence: crate::phase_unbraid::PhaseReadoutAccumulator,
    #[cfg(feature = "hosted")]
    reentry: Option<ResidentReentry>,
}

impl ResidentPhaseCarrier {
    #[cfg(feature = "hosted")]
    pub fn from_baked(data:&'static crate::baked_resident::BakedResident)->Result<Self,String> {
        let source=WordTape::from_canonical_word(data.source_word).ok_or("invalid compiled resident source")?;
        if data.phase.source_word!=source.as_word() || data.sic.modulus_bits!=data.sic_modulus_bits {
            return Err("compiled resident changed its source or SIC ancestry".into());
        }
        let initial_mass=WordTape::from_canonical_word(data.initial_mass_word).ok_or("invalid compiled resident mass")?;
        let tape=source.parity_cells();
        let layout=FactorPhaseOracle::from_n(&tape).map_err(String::from)?.layout().clone();
        let circuit=crate::whole_register_phase::WholeRegisterPhaseCircuit::from_baked(data.phase)?;
        let wire_map=data.wire_map.to_vec();
        if wire_map.len()!=layout.cells || wire_map.iter().any(|&wire|wire>=circuit.cells()) {
            return Err("compiled factor frame escaped the common register".into());
        }
        let mut distinct=wire_map.clone();distinct.sort_unstable();distinct.dedup();
        if distinct.len()!=wire_map.len() {return Err("compiled factor frame repeated a cell".into());}
        let boundary=ParityBoundary::new(&layout,&wire_map)?;
        let mut arena=DecisionArena::new(0);let root=arena.zero();
        let sic_format=FixedPointFormat::for_modulus_bits(data.sic_modulus_bits)?;
        let sic_frame=FixedQuquartSic::from_baked(data.sic)?;
        let next_product_columns=if data.product_closure_prepared {tape.len()+1} else {0};
        let reentry_record:serde_json::Value=serde_json::from_str(data.reentry_json).map_err(|e|e.to_string())?;
        let reentry=Some(ResidentReentry::from_record(&reentry_record)?);
        Ok(Self {resident:ResidentGlut {source,tape,arena,root,initial:root,banked_roots:Vec::new(),
            preparation_sic_ancestry:Some(String::from(data.preparation_sic_json)),wire_map,
            diagram_ready:true,initial_mass,boundary,constraint_gates:data.constraint_gates,sic_format,sic_frame:Some(sic_frame)},
            circuit,next_product_columns,source_producer:None,phase_source_relation:data.phase_source_relation,
            phase_evidence:Default::default(),reentry})
    }
    #[cfg(feature = "hosted")]
    pub fn bind_certified_reentry(&mut self, clock: &serde_json::Value) -> Result<(), String> {
        if self.execution_progress().is_some() || self.reentry.is_some() {
            return Err("re-entry binding crossed its preparation boundary".into());
        }
        self.reentry = Some(ResidentReentry::from_record(&serde_json::json!({
            "Y_program": vox_core::dialectic_lean::ResidentY::certified_program(), "clock": clock }))?);
        Ok(())
    }

    #[cfg(feature = "hosted")]
    pub fn reentry_operations(&self) -> Option<(u64, u64, u64, u64, u64)> {
        self.reentry.as_ref().and_then(|r| r.driver.as_ref().map(|y|
            (y.contractions, y.calls, r.ticks, r.forward_calls, r.reverse_calls)))
    }
    #[cfg(feature = "hosted")]
    pub fn prepared_record(&mut self) -> Result<serde_json::Value,String> {
        if self.circuit.execution_progress().is_some()
            || self.next_product_columns!=0 && self.next_product_columns<=self.resident.tape.len() {
            return Err("resident preparation crossed product execution".into());
        }
        let phase = self.circuit.prepared_record()?;
        let frame = self.resident.sic_frame.as_ref().ok_or("joint SIC frame must be constructed in preparation")?;
        Ok(serde_json::json!({"format":"resident-imasm-prepared-v1",
            "source_word":self.resident.source.as_word(),"initial_mass_word":self.resident.initial_mass.as_word(),
            "wire_map":self.resident.wire_map,"constraint_gates":self.resident.constraint_gates,
            "product_closure_prepared":self.next_product_columns>self.resident.tape.len(),
            "phase_source_relation":self.phase_source_relation,
            "sic_modulus_bits":self.resident.sic_format.modulus_bits,"sic_frame":frame.prepared_record(),"phase":phase,
            "reentry":self.reentry.as_ref().map(|r| r.record.clone())}))
    }

    #[cfg(feature = "hosted")]
    pub fn from_prepared_record(record: &serde_json::Value, work: crate::ququart_folded_work::PreparedModularWork)
        -> Result<Self,String> {
        if record["format"].as_str()!=Some("resident-imasm-prepared-v1") {
            return Err("unrecognized resident preparation".into());
        }
        let source = WordTape::from_canonical_word(record["source_word"].as_str().ok_or("missing prepared source")?)
            .ok_or("invalid prepared source word")?;
        if record["phase"]["source_word"].as_str()!=Some(source.as_word()) {
            return Err("prepared factor and phase frames changed their common source".into());
        }
        let initial_mass = WordTape::from_canonical_word(record["initial_mass_word"].as_str().ok_or("missing prepared mass")?)
            .ok_or("invalid prepared mass word")?;
        let tape = source.parity_cells();
        let layout = FactorPhaseOracle::from_n(&tape).map_err(String::from)?.layout().clone();
        let wire_map = record["wire_map"].as_array().ok_or("prepared factor frame lost its cells")?.iter()
            .map(|n| n.as_u64().and_then(|n| usize::try_from(n).ok()).ok_or_else(|| String::from("invalid prepared cell address")))
            .collect::<Result<Vec<_>,_>>()?;
        let circuit = crate::whole_register_phase::WholeRegisterPhaseCircuit::from_prepared_record(&record["phase"],work)?;
        if wire_map.len()!=layout.cells || wire_map.iter().any(|&wire| wire>=circuit.cells()) {
            return Err("prepared factor frame escaped the common register".into());
        }
        let mut distinct = wire_map.clone(); distinct.sort_unstable(); distinct.dedup();
        if distinct.len()!=wire_map.len() { return Err("prepared factor frame repeated a cell".into()); }
        let boundary = ParityBoundary::new(&layout,&wire_map)?;
        let mut arena = DecisionArena::new(0);
        let root = arena.zero();
        let sic_format = FixedPointFormat::for_modulus_bits(record["sic_modulus_bits"].as_u64().ok_or("missing prepared SIC precision")?)?;
        let sic_frame = FixedQuquartSic::from_prepared_record(&record["sic_frame"])?;
        let constraint_gates = record["constraint_gates"].as_u64().and_then(|n| usize::try_from(n).ok()).ok_or("missing prepared constraint count")?;
        let product_closure_prepared = record["product_closure_prepared"].as_bool().unwrap_or(false);
        let next_product_columns = if product_closure_prepared { tape.len()+1 } else { 0 };
        Ok(Self { resident:ResidentGlut { source,tape,arena,root,initial:root,banked_roots:Vec::new(),
            preparation_sic_ancestry:Some(serde_json::json!({"signed":record["SIC_signed_residuals"],
                "denominator":record["SIC_rational_denominator_word"],"rational":record["SIC_rational_residual_words"]}).to_string()),wire_map,diagram_ready:true,
            initial_mass,boundary,constraint_gates,sic_format,sic_frame:Some(sic_frame) },
            circuit,next_product_columns,source_producer:None,
            phase_source_relation:record["phase_source_relation"].as_bool().unwrap_or(false),
            phase_evidence: Default::default(),
            reentry:record.get("reentry").filter(|v| !v.is_null()).map(ResidentReentry::from_record).transpose()? })
    }

    pub fn execution_progress(&self) -> Option<(usize,usize,usize)> {
        self.circuit.execution_progress()
    }
    pub fn lane_order(&self) -> &[(usize,bool)] {self.circuit.lane_order()}
    pub fn monadic_binds(&self) -> u64 {self.circuit.monadic_binds()}
    pub fn product_progress(&self) -> (usize,usize,usize) {
        (self.next_product_columns.min(self.resident.tape.len()),self.resident.tape.len(),self.resident.constraint_gates)
    }

    pub fn bind_source_divisor(&mut self, divisor: &BigUint, producer: &'static str)
        -> Result<WordTape,String> {
        let divisor=WordTape::from_biguint(divisor);
        let source = &self.resident.source;
        if !divisor.gt(&WordTape::one()) || divisor.ge(source) {
            return Err("source synthesis returned no proper divisor".into());
        }
        let (quotient,remainder) = source.divmod(&divisor).ok_or("source divisor exceeds its source")?;
        if !remainder.is_zero() || divisor.mul(&quotient)!=*source {
            return Err("source synthesis return failed exact IMASM product closure".into());
        }
        let (p,q) = if divisor.le(&quotient) { (divisor.clone(),quotient.clone()) } else { (quotient.clone(),divisor.clone()) };
        let mut controls = Vec::new();
        for (lane,value) in [(&self.resident.boundary.even,&p),(&self.resident.boundary.odd,&q)] {
            controls.extend(lane.iter().enumerate().map(|(bit,&wire)|(wire,value.bit_at(bit))));
        }
        controls.sort_unstable();
        self.circuit.refine_resident_support(|arena,root| {
            let zero=arena.zero();
            *root=arena.conditional_literals(zero,*root,&controls);
            if arena.mass_word(*root).is_zero() {
                return Err(String::from("source synthesis return is absent from the prepared resident register"));
            }
            Ok(())
        })?;
        self.next_product_columns=self.resident.tape.len()+1;
        self.source_producer=Some(producer);
        Ok(quotient)
    }

    pub fn bind_source_divisor_cells(&mut self, cells: &[char], producer: &'static str)
        -> Result<WordTape,String> {
        let divisor=WordTape::from_parity_cells(cells).ok_or("source divisor has invalid IMASM cells")?;
        self.bind_source_divisor(&divisor.to_biguint(),producer)
    }
    pub fn advance(&mut self) -> Result<bool, String> {
        #[cfg(feature = "hosted")]
        if self.reentry.is_some() {
            let mut reentry=self.reentry.take().unwrap();
            let mut driver = reentry.driver.take()
                .ok_or("resident Y is already executing its payload")?;
            let result = driver.advance(|| {
                self.advance_with_selector(||reentry.tick())
            });
            reentry.driver=Some(driver);
            self.reentry=Some(reentry);
            return result;
        }
        self.advance_clocked(false)
    }

    fn advance_clocked(&mut self, reverse: bool) -> Result<bool, String> {
        self.advance_with_selector(||Ok(reverse))
    }

    fn advance_with_selector(&mut self,select:impl FnMut()->Result<bool,String>) -> Result<bool,String> {
        if !self.phase_source_relation && self.next_product_columns <= self.resident.tape.len() {
            let row = self.next_product_columns;
            let full = FactorPhaseOracle::from_n(&self.resident.tape).map_err(String::from)?;
            let gates = self.circuit.refine_resident_support(|arena,root| {
                let mut initial = *root;
                let mut sink = BoundOracle { arena, root, initial: &mut initial,
                    gates: 0, updates: 0, observe: |_,_| {}, wire_map: &self.resident.wire_map };
                if row < full.layout().p.len() {
                    full.lower_selected_row(&mut sink, row).map_err(String::from)?;
                } else {
                    full.finish_selected_product(&mut sink).map_err(String::from)?;
                }
                Ok(sink.gates)
            })?;
            self.resident.constraint_gates += gates;
            self.next_product_columns += 1;
        }
        let phase_complete = self.circuit.advance_nested_work_with_selector(8,8,select)?;
        Ok(phase_complete && (self.phase_source_relation || self.next_product_columns > self.resident.tape.len()))
    }
    pub fn fire(mut self) -> Result<ResidentPhaseReturn, String> {
        while !self.advance()? {}
        let denominator = WordTape::one_at(self.circuit.width());
        let mut random = 1729u64;
        let mut cells = alloc::vec!['⊤';self.circuit.width()];
        for offset in (0..self.circuit.width()).step_by(64) {
            random ^= random << 13; random ^= random >> 7; random ^= random << 17;
            for bit in 0..64.min(self.circuit.width()-offset) {
                if random & (1u64 << bit) != 0 { cells[offset+bit] = '⊥'; }
            }
        }
        let quantile = WordTape::from_parity_cells(&cells).unwrap();
        let phase = self.circuit.fire_with_work_radix(&quantile.to_biguint(), &denominator.to_biguint(), Some(8))?;
        let (arena, root, banks, program) = self.circuit.into_banked_resident_support();
        self.resident.arena = arena;
        self.resident.root = root;
        self.resident.initial = banks.first().copied().unwrap_or(root);
        self.resident.banked_roots = banks;
        Ok(ResidentPhaseReturn { program,resident: self.resident, phase, source_producer:self.source_producer,
            phase_source_relation:self.phase_source_relation, phase_evidence:self.phase_evidence,
            #[cfg(feature = "hosted")]
            reentry:self.reentry })
    }
}

impl ResidentPhaseReturn {
    /// An unresolved closure returns the held membrane, including its evidence
    /// and continuation, to its caller. It is not an empty replacement carrier.
    pub fn close(mut self) -> Result<ResidentClosure, String> {
        if WordTape::from_biguint(&self.phase.source)!=self.resident.source
            || self.program.source_word()!=self.resident.source.as_word() {
            return Err("phase return changed the held membrane's source binding".into());
        }
        if self.phase_source_relation {
            eprintln!("resident-return-stage complete-phase-winding");
            let Some((order, divisor, quotient)) = self.phase_evidence.close_word_fraction(
                &WordTape::from_biguint(&self.phase.numerator),
                &WordTape::from_biguint(&self.phase.denominator),
                &WordTape::from_biguint(&self.phase.base), &self.resident.source)? else { return Ok(ResidentClosure::Open(self)); };
            eprintln!("resident-return-stage certified-winding");
            if divisor.mul(&quotient)!=self.resident.source {
                return Err("joint word winding changed its source product".into());
            }
            eprintln!("resident-measured-order-word {}",order.as_word());
            eprintln!("resident-return-stage condition-factor-boundary");
            let mut result = self.resident.readout_source_word_divisor(divisor, "resident-Y-clock-phase")?;
            if let Some(readout) = result.as_mut() {
                readout.phase_evidence = Some(self.phase_evidence);
                readout.phase = Some(self.phase);
                readout.program = Some(self.program);
                #[cfg(feature = "hosted")]
                { readout.reentry = self.reentry; }
            }
            return result.map(ResidentClosure::Closed).ok_or_else(||
                String::from("verified winding disappeared from its resident boundary"));
        }
        if self.resident.arena.mass_word(self.resident.root).is_zero() {
            return Ok(ResidentClosure::Open(self));
        }
        let layout = FactorPhaseOracle::from_n(&self.resident.tape)
            .map_err(String::from)?.layout().clone();
        let initial_mass = self.resident.initial_mass.clone();
        let gates = self.resident.constraint_gates;
        let mut result = self.resident.finish_readout(layout, initial_mass, gates,
            self.source_producer.unwrap_or("resident-product-phase"))?
            .ok_or_else(|| String::from("resident product closure lost its nonempty support"))?;
        result.phase_evidence=Some(self.phase_evidence);
        result.phase=Some(self.phase);
        result.program=Some(self.program);
        result.retained_membrane=Some(Box::new(self.resident));
        #[cfg(feature = "hosted")]
        { result.reentry=self.reentry; }
        Ok(ResidentClosure::Closed(result))
    }

    pub fn readout(self) -> Result<Option<GlutReadout>, String> {
        Ok(match self.close()? {
            ResidentClosure::Closed(readout) => Some(readout),
            ResidentClosure::Open(_) => None,
        })
    }
}

pub enum ResidentClosure {
    Closed(GlutReadout),
    Open(ResidentPhaseReturn),
}

impl GlutReadout {
    pub fn program_source_word(&self) -> Option<&str> {
        self.program.as_ref().map(|program|program.source_word())
    }
    pub fn retained_operation_count(&self) -> usize {
        self.program.as_ref().map_or(0,|program|program.operation_count())
    }
    pub fn banked_supports(&self) -> usize {
        self.retained_membrane.as_ref().map_or(0,|resident| resident.banked_roots.len())
    }
    pub fn source_word(&self) -> Option<&str> {
        self.retained_membrane.as_ref().map(|resident| resident.source.as_word())
    }
}

#[cfg(feature = "hosted")]
impl GlutReadout {
    pub fn reentry_operations(&self) -> Option<(u64, u64, u64, u64, u64)> {
        self.reentry.as_ref().and_then(|r| r.driver.as_ref().map(|y|
            (y.contractions, y.calls, r.ticks, r.forward_calls, r.reverse_calls)))
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

    #[cfg(feature = "hosted")]
    #[test]
    #[ignore = "bounded locked-source 512-bit integration; run explicitly within the hard-semiprime battery"]
    fn source_phase_closure_retains_continuation_and_open_membrane() {
        let clocks: serde_json::Value = serde_json::from_str(include_str!(
            "../measurements/membrane_reentry_20261010/certified_clocks.json")).unwrap();
        let clock = clocks["clocks"].as_array().unwrap().iter().find(|r|
            r["retained"].as_u64()==Some(0) && r["seed"].as_u64()==Some(1)).unwrap();
        for open in [false,true] {
            let source = WordTape::from_canonical_word(include_str!(
                "../measurements/resident_imasm_integration_20261012/inputs/fresh_balanced_512.imasm").trim()).unwrap();
            assert!(source.bit_len()>=200);
            let mut carrier = ResidentGlut::from_word(source.as_word()).unwrap()
                .into_source_whole_register_phase_observed(|_| {}).unwrap();
            carrier.bind_certified_reentry(clock).unwrap();
            while !carrier.advance().unwrap() {}
            let operations = carrier.reentry_operations();
            let mut returned = carrier.fire().unwrap();
            if open { returned.phase.numerator=BigUint::zero(); }
            match returned.close().unwrap() {
                ResidentClosure::Closed(result) => {
                    assert!(!open);
                    assert_eq!(result.reentry_operations(),operations);
                    assert!(result.phase_evidence.is_some());
                    let phase=result.phase.as_ref().unwrap();
                    assert_eq!(phase.source,source.to_biguint());
                    assert!(phase.monadic_binds>2);
                    assert_eq!(result.source_word(),Some(source.as_word()));
                    let retained=result.retained_membrane.as_ref().unwrap();
                    assert!(!retained.arena.mass_word(retained.root).is_zero());
                    assert!(!retained.boundary.word.is_empty());
                    assert!(retained.sic_frame.is_some());
                    assert_eq!(retained.banked_roots.len(),3);
                    assert!(retained.banked_roots.iter().all(|&root| !retained.arena.mass_word(root).is_zero()));
                    assert_eq!(&result.divisor * &result.quotient,source.to_biguint());
                    assert!(result.gamma_delta_closed && result.product_closed);
                }
                ResidentClosure::Open(held) => {
                    assert_eq!(held.resident.source,source);
                    let r=held.reentry.as_ref().unwrap();
                    let y=r.driver.as_ref().unwrap();
                    assert_eq!(Some((y.contractions,y.calls,r.ticks,r.forward_calls,r.reverse_calls)),operations);
                    assert_eq!(r.state,r.clock.read(r.ticks));
                    assert!(held.resident.sic_frame.is_some());
                    assert!(!held.resident.boundary.word.is_empty());
                    assert_eq!(held.resident.banked_roots.len(),3);
                }
            }
        }
    }

    #[test]
    fn prefix_constraint_retains_source_divisors_and_signed_sic_closure() {
        for n in [91u32, 143, 8051] {
            let mut resident = ResidentGlut::new(BigUint::from(n)).unwrap();
            let initial = resident.initial_mass.clone();
            let gates = resident.constrain_product_prefix(4, |_, _| {}).unwrap();
            assert!(gates > 0);
            assert!(resident.arena.mass_word(resident.root) < initial);
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
        assert_eq!(result.producer, "resident-product-phase");
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
    program: Option<crate::whole_register_phase::WholeRegisterPhaseProgram>,
    pub phase: Option<crate::whole_register_phase::WholeRegisterPhaseReadout>,
    /// The returned numbers are observations of this retained membrane.
    retained_membrane: Option<Box<ResidentGlut>>,
    phase_evidence: Option<crate::phase_unbraid::PhaseReadoutAccumulator>,
    #[cfg(feature = "hosted")]
    reentry: Option<ResidentReentry>,
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
            let value = BigInt::from(self.initial_mass.shr(2).to_biguint());
            core::array::from_fn(|_| (value.clone(), BigInt::zero()))
        };
        self.reconstruct_sic(gram)
    }

    fn reconstruct_sic(&mut self, gram: [(BigInt,BigInt);16])
        -> Result<GlutSicWitness,String> {
        if self.sic_frame.is_none() { self.sic_frame = Some(FixedQuquartSic::new(&self.sic_format)?); }
        let frame = self.sic_frame.as_ref().unwrap();
        let mass_numerators = frame.gram_mass_numerators(&gram)?;
        let masses = mass_numerators.clone().map(|ref numerator|
            WordTape::from_biguint(numerator).shr(2 * self.sic_format.w_bits as usize).to_biguint());
        let rational = frame.certify_gram_frame_unrounded_masses(&gram, &mass_numerators)?;
        let reconstruction = rational.rounded;
        let signed_words: [(SignedWordTape, SignedWordTape);16] = core::array::from_fn(|i| (
            SignedWordTape::from_bigint(&reconstruction.recovered[i].0).sub(&SignedWordTape::from_bigint(&gram[i].0)),
            SignedWordTape::from_bigint(&reconstruction.recovered[i].1).sub(&SignedWordTape::from_bigint(&gram[i].1))));
        let trace = (0..4).fold(SignedWordTape::zero(), |sum, i|
            sum.add(&SignedWordTape::from_bigint(&gram[5*i].0)));
        let uncorrected_maximum_residual = (0..16).map(|i| {
            let omitted = if i % 5 == 0 { trace.clone() } else { SignedWordTape::zero() };
            signed_words[i].0.add(&omitted).abs().max(signed_words[i].1.abs())
        }).max().unwrap().to_bigint();
        let signed_residuals = signed_words.map(|(re,im)| (re.to_bigint(),im.to_bigint()));
        Ok(GlutSicWitness { gram, masses, reconstruction, signed_residuals,
            uncorrected_maximum_residual, mass_numerators,
            rational_denominator: rational.denominator,
            rational_signed_residuals: rational.signed_residual_numerators })
    }
    /// N is the sole instance input. Candidate and workspace widths follow
    /// the existing factor oracle's N-derived layout.
    pub fn new(source: BigUint) -> Result<Self, String> {
        Self::from_word(WordTape::from_biguint(&source).as_word())
    }
    pub fn from_word(source_word: &str) -> Result<Self, String> {
        let source = WordTape::from_canonical_word(source_word).ok_or("invalid source numeral word")?;
        let tape = source.parity_cells();
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
            .into_iter().fold(WordTape::zero(), |mass, (p, q)| mass.add(&WordTape::one_at(p + q - 2)));
        let sic_format = FixedPointFormat::for_modulus_bits(source.bit_len().saturating_sub(1) as u64)?;
        Ok(Self { source, tape, arena, root, initial: root, wire_map,
            banked_roots:Vec::new(),
            preparation_sic_ancestry:None,
            diagram_ready: false, initial_mass, boundary, constraint_gates: 0, sic_format, sic_frame: None })
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
        if arena.mass_word(root) != self.initial_mass {
            return Err("materialized source geometry changed its declared norm".into());
        }
        self.root = root;
        self.initial = root;
        self.diagram_ready = true;
        Ok(())
    }

    pub fn readout(self, observe: impl FnMut(usize, usize)) -> Result<Option<GlutReadout>, String> {
        let trace = std::env::var_os("GM_FACTOR_TRACE").is_some();
        let trace_started = std::time::Instant::now();
        let mut retained = crate::factor_synthesis::RetainedArmProbe::new(self.source.to_biguint())?;
        let expected_source = self.source.clone();
        let expected_tape = self.tape.clone();
        let mut port_calls = 0u64;
        if let Some(returned) = vox_core::morphism_factor::source_carrier_divisor_with_port_and_observer(
            &self.tape, |source_tape| {
                if source_tape != expected_tape {
                    return Err(String::from("resident source port changed its N-only binding"));
                }
                port_calls = port_calls.saturating_add(1);
                if trace {
                    eprintln!("factor-trace elapsed={:.3} stage=retained-arm-port count={port_calls}",
                        trace_started.elapsed().as_secs_f64());
                }
                let native_started = std::time::Instant::now();
                let mut result = retained.advance_native()?;
                if trace {
                    eprintln!("factor-trace elapsed={:.3} stage=retained-native-complete count={} result={} step_seconds={:.3}",
                        trace_started.elapsed().as_secs_f64(), port_calls, result.is_some(),
                        native_started.elapsed().as_secs_f64());
                }
                if result.is_none() {
                    let phase_started = std::time::Instant::now();
                    result = retained.advance_joint()?;
                    if trace {
                        let progress = retained.joint_progress();
                        eprintln!("factor-trace elapsed={:.3} stage=whole-register-step-complete count={} result={} step_seconds={:.3} progress={progress:?}",
                            trace_started.elapsed().as_secs_f64(), port_calls,
                            result.is_some(), phase_started.elapsed().as_secs_f64());
                    }
                }
                let Some(result) = result else { return Ok(None) };
                let divisor = result.factors.into_iter().map(|factor| WordTape::from_biguint(&factor)).find(|factor|
                    factor.gt(&WordTape::one()) && !factor.ge(&expected_source)
                        && expected_source.divmod(factor).is_some_and(|(_, remainder)| remainder.is_zero()))
                    .ok_or("retained arm returned no proper divisor of N")?;
                let numeral = divisor.parity_cells();
                Ok(Some(vox_core::morphism_factor::SourceDivisor {
                    divisor: numeral, producer: result.producer,
                }))
            }, |stage, count| {
                if trace && (count <= 8 || count % 128 == 0
                    || stage.ends_with("return") || stage.ends_with("exhausted")) {
                    eprintln!("factor-trace elapsed={:.3} stage={} count={count}",
                        trace_started.elapsed().as_secs_f64(), stage);
                }
            })? {
            let divisor = WordTape::from_parity_cells(&returned.divisor)
                .ok_or("source carrier boundary numeral conversion failed")?;
            return self.readout_source_word_divisor(divisor, returned.producer);
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
        if columns == full.layout().p.len() {
            let mut sink = BoundOracle { arena: &mut self.arena, root: &mut self.root,
                initial: &mut self.initial, gates: 0, updates: 0, observe, wire_map: &self.wire_map };
            full.lower_selected(&mut sink).map_err(String::from)?;
            let gates = sink.gates;
            drop(sink);
            for wire in full.layout().product.start..full.layout().cells {
                if !self.arena.wire_is_zero(self.root, self.wire_map[wire]) {
                    return Err("product closure left resident workspace uncleared".into());
                }
            }
            self.constraint_gates += gates;
            return Ok(gates);
        }
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
        let pairs = crate::gpu_ecm_word::product_prefix_support(&self.source, columns, device)?;
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
        self.support_mass_word().to_biguint()
    }
    pub fn support_mass_word(&self) -> WordTape {
        if self.diagram_ready { self.arena.mass_word(self.root) } else { self.initial_mass.clone() }
    }

    pub fn into_whole_register_phase(self) -> Result<ResidentPhaseCarrier, String> {
        self.into_whole_register_phase_observed(|_| {})
    }

    pub fn into_prepared_whole_register_phase_observed(self,
        observe: impl FnMut(&'static str)) -> Result<ResidentPhaseCarrier,String> {
        self.into_whole_register_phase_inner(observe, true, false)
    }

    /// Hold the factor boundary in this register while the source-bound modular
    /// relation produces one complete phase constraint. Boundary closure uses
    /// that measured constraint, without materializing product gates or copying
    /// an unknown candidate into the modular unit register.
    pub fn into_source_whole_register_phase_observed(self,
        observe: impl FnMut(&'static str)) -> Result<ResidentPhaseCarrier,String> {
        self.into_whole_register_phase_inner(observe, false, true)
    }

    pub fn into_whole_register_phase_observed(self,
        observe: impl FnMut(&'static str)) -> Result<ResidentPhaseCarrier,String> {
        self.into_whole_register_phase_inner(observe, false, false)
    }

    fn into_whole_register_phase_inner(mut self,
        mut observe: impl FnMut(&'static str), product_closure_prepared: bool, phase_source_relation: bool)
        -> Result<ResidentPhaseCarrier,String> {
        observe("relation-materialize");
        self.materialize()?;
        observe("SIC-coordinate-operation");
        self.apply_sic_coordinate_filter()?;
        if product_closure_prepared {
            observe("source-product-closure");
            self.constrain_product_prefix(self.tape.len(), |_, _| {})?;
        }
        observe("whole-phase-prepare");
        let mut circuit = crate::whole_register_phase::WholeRegisterPhaseCircuit::prepare(
            self.source.to_biguint(), BigUint::from(2u8), None)?;
        observe("whole-phase-bind");
        let offset = circuit.bind_resident_support(&self.source.to_biguint(), &self.arena, self.root)?;
        observe("whole-phase-bound");
        let support = circuit.support_mass_word();
        observe("joint-support-measured");
        let precision_value = core::cmp::max(&self.source, &support);
        self.sic_format = FixedPointFormat::for_modulus_bits(precision_value.bit_len().saturating_sub(1) as u64)?;
        self.sic_frame = Some(FixedQuquartSic::new(&self.sic_format)?);
        observe("joint-SIC-prepared");
        for wire in &mut self.wire_map { *wire += offset; }
        for wire in self.boundary.word.iter_mut().chain(self.boundary.even.iter_mut())
            .chain(self.boundary.odd.iter_mut()) { *wire += offset; }
        if !phase_source_relation {
            let mapping = circuit.bind_candidate_work(&self.boundary.even)?;
            for wire in &mut self.wire_map { *wire = mapping[*wire]; }
            for wire in self.boundary.word.iter_mut().chain(self.boundary.even.iter_mut())
                .chain(self.boundary.odd.iter_mut()) { *wire = mapping[*wire]; }
            observe("candidate-work-bound");
        } else {
            observe("source-phase-relation-bound");
        }
        self.arena = DecisionArena::new(0);
        self.root = self.arena.zero(); self.initial = self.root;
        let next_product_columns = if product_closure_prepared { self.tape.len()+1 } else { 0 };
        Ok(ResidentPhaseCarrier { resident: self, circuit, next_product_columns, source_producer:None,
            phase_source_relation,
            phase_evidence: Default::default(),
            #[cfg(feature = "hosted")]
            reentry:None })
    }

    /// Apply the mass-reconstructed reduced density operator to the factor
    /// coordinates before copying their marks into the phase work register.
    /// The transpose converts <work_i|work_j> into the ket/bra convention.
    /// Removing a common power of two changes only the overall amplitude.
    pub fn apply_sic_coordinate_filter(&mut self) -> Result<GlutSicWitness, String> {
        self.materialize()?;
        let layout = FactorPhaseOracle::from_n(&self.tape).map_err(String::from)?.layout().clone();
        let bit = usize::from(layout.p.len() > 1);
        let wires = [self.wire_map[layout.p.start+bit], self.wire_map[layout.q.start+bit]];
        let zero = self.arena.zero();
        let mut aligned = [zero; 4];
        for (label, arm) in aligned.iter_mut().enumerate() {
            let controls = [(wires[0],label&1 != 0),(wires[1],label&2 != 0)];
            *arm = self.arena.conditional_literals(zero,self.root,&controls);
            for (wire, active) in controls {
                if active { *arm = self.arena.flip(*arm,wire); }
            }
        }
        let gram = self.arena.control_gram(&aligned);
        let witness = self.reconstruct_sic(gram)?;
        let mut matrix = core::array::from_fn(|entry| {
            let (re,im) = &witness.reconstruction.recovered[4*(entry%4)+entry/4];
            (SignedWordTape::from_bigint(re),SignedWordTape::from_bigint(im))
        });
        let common_shift = matrix.iter().flat_map(|(re,im)| [re,im])
            .filter(|value| !value.is_zero()).map(|value|
                (0..value.magnitude().bit_len()).take_while(|&bit|
                    !value.magnitude().bit_at(bit)).count()).min().unwrap_or(0);
        for (re,im) in &mut matrix { *re = re.shr(common_shift); *im = im.shr(common_shift); }
        self.root = crate::reentry_powerset::bind_one(self.root,|root|
            self.arena.apply_control_matrix(root,wires,&matrix))?;
        if self.arena.mass_word(self.root).is_zero() {
            return Err("SIC coordinate operator closed to empty resident support".into());
        }
        let mut roots = [self.root,zero,zero,zero,zero];
        self.arena.fold(&mut roots);
        self.root = roots[0]; self.initial = self.root;
        Ok(witness)
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
        let candidate = crate::gpu_ecm_word::factor(&self.source, 1000, 20_000, 0, 6, 256)?;
        match candidate {
            Some(candidate) => self.readout_source_word_divisor(candidate, "gpu-word-ecm-source"),
            None => Ok(None),
        }
    }

    fn readout_source_divisor(self, divisor: BigUint, producer: &'static str)
        -> Result<Option<GlutReadout>, String> {
        self.readout_source_word_divisor(WordTape::from_biguint(&divisor), producer)
    }
    fn readout_source_word_divisor(mut self, divisor: WordTape, producer: &'static str)
        -> Result<Option<GlutReadout>, String> {
        let source = self.source.clone();
        if !divisor.gt(&WordTape::one()) || divisor.ge(&source) {
            return Err("source relation arm returned no proper source divisor".into());
        }
        let (quotient, remainder) = source.divmod(&divisor).unwrap();
        if !remainder.is_zero() { return Err("source relation arm returned no proper source divisor".into()); }
        let (p, q) = if !divisor.gt(&quotient) { (divisor, quotient) } else { (quotient, divisor) };
        let layout = FactorPhaseOracle::from_n(&self.tape).map_err(String::from)?.layout().clone();
        let initial_mass = self.initial_mass.clone();
        let mut controls = Vec::new();
        for (lane, value) in [(&self.boundary.even, &p), (&self.boundary.odd, &q)] {
            controls.extend(lane.iter().enumerate().map(|(bit, &wire)|
                (wire, value.bit_at(bit))));
        }
        controls.sort_unstable();
        if self.diagram_ready {
            let zero = self.arena.zero();
            self.root = crate::reentry_powerset::bind_one(self.root,|root|
                Ok::<_,String>(self.arena.conditional_literals(zero, root, &controls)))?;
        } else {
            let p_bits = p.bit_len();
            let q_bits = q.bit_len();
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
        let mut result=self.finish_readout(layout, initial_mass, self.constraint_gates, producer)?;
        if let Some(readout)=result.as_mut() { readout.retained_membrane=Some(Box::new(self)); }
        Ok(result)
    }

    /// The retained limb ECM implementation derives its width from N and
    /// partitions a fixed curve schedule across the available devices.
    #[cfg(feature = "hosted")]
    pub fn readout_gpu_limbs(self) -> Result<Option<GlutReadout>, String> {
        let _device = cudarc::driver::CudaContext::new(0)
            .map_err(|error| alloc::format!("GPU 0 unavailable: {error}"))?;
        let candidate = crate::gpu_ecm::factor_once_b2(&self.source.to_biguint(), 1000, 20_000, 0);
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
                if WordTape::from_biguint(&divisor) == self.source { return Ok(None); }
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

    fn finish_readout(&mut self, layout: Layout, initial_mass: WordTape,
        gates: usize, producer: &'static str) -> Result<Option<GlutReadout>, String> {
        let trace = producer == "resident-Y-clock-phase" || std::env::var_os("GM_FACTOR_TRACE").is_some();
        let selected_mass = self.arena.mass_word(self.root);
        if selected_mass.is_zero() { return Ok(None) }
        if trace { eprintln!("resident-return-stage terminal-SIC"); }
        let sic = self.sic_witness()?;
        if trace { eprintln!("resident-return-stage banked-workspace"); }
        for wire in layout.product.start..layout.cells {
            if !self.arena.wire_is_zero(self.root, self.wire_map[wire]) {
                return Err("factor oracle failed to restore its shared workspace".into());
            }
        }
        let mut random = 1729u64;
        random ^= random << 13; random ^= random >> 7; random ^= random << 17;
        let measured = self.arena.sample_word(self.root, self.arena.cells,
            &WordTape::from_small(random), &WordTape::one_at(64)).map_err(String::from)?;
        if trace { eprintln!("resident-return-stage factor-marks"); }
        let decode = |lane: &[usize]| {
            let cells: Vec<_> = lane.iter().map(|&wire|
                if measured.bit_at(wire) { '⊥' } else { '⊤' }).collect();
            WordTape::from_parity_cells(&cells).expect("boundary retrieval reads numeral marks")
        };
        let p = decode(&self.boundary.even);
        let q = decode(&self.boundary.odd);
        let word_value = decode(&self.boundary.word);
        let boundary_word = word_value;
        let gamma_delta_closed = p.interlace(&q) == boundary_word;
        if !gamma_delta_closed { return Err("resident gamma/delta boundary failed same-word closure".into()); }
        let source = self.source.clone();
        let product_closed = p.mul(&q) == source;
        if !product_closed { return Err("resident parity lanes failed the source product closure".into()); }
        if !p.gt(&WordTape::one()) || p.ge(&source) {
            return Err("resident glut readout did not produce a proper source divisor".into());
        }
        let (quotient, remainder) = source.divmod(&p).unwrap();
        if !remainder.is_zero() || quotient != q || p.mul(&quotient) != source {
            return Err("derived quotient failed exact source product verification".into());
        }
        Ok(Some(GlutReadout { divisor: p.to_biguint(), quotient: q.to_biguint(), initial_mass: initial_mass.to_biguint(),
            phase:None,
            program:None,
            phase_evidence:None,
            #[cfg(feature = "hosted")]
            reentry:None,
            selected_mass: selected_mass.to_biguint(), gates, sic, producer,
            boundary_word: String::from(boundary_word.as_word()), gamma_delta_closed, product_closed,
            retained_membrane:None }))
    }
}
