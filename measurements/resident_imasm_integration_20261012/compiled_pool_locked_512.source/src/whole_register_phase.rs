//! One-shot phase estimation over a resident powerset decision register.
//!
//! The phase and work wires share one complex decision diagram. Controlled
//! modular powers and the complete inverse Fourier transform act on that
//! diagram before one full-width Born readout. The correlated powerset itself
//! is the re-entry state; no branch is detached for digitwise measurement or
//! feedback during execution.

use crate::phase_unbraid::{FixedComplex, FixedPointFormat};
use crate::phase_unbraid::PhaseReadoutAccumulator;
use crate::ququart_decision::DecisionArena;
use crate::ququart_folded_work::PreparedModularWork;
use crate::reversible_modular::{ElementaryGate, ModularMultiply, NestedOperation};
use crate::word_tape::{SignedWordTape, WordTape};
use alloc::string::String;
use num_bigint::BigUint;
use num_traits::{One, Zero};

fn fold_diagram(arena: &mut DecisionArena, root: &mut usize, banks: &mut [usize]) {
    let zero = arena.zero();
    let mut roots = [zero; 5];
    roots[0] = *root;
    assert!(banks.len()<roots.len(),"phase bank exceeded retained frame slots");
    roots[1..1+banks.len()].copy_from_slice(banks);
    arena.fold(&mut roots);
    *root = roots[0];
    banks.copy_from_slice(&roots[1..1+banks.len()]);
}

fn apply_elementary(
    arena: &mut DecisionArena,
    root: &mut usize,
    work_wires: &[usize],
    control: usize,
    gate: ElementaryGate,
) -> Result<(), String> {
    let map_wire = |wire: usize| -> Result<usize, String> {
        if wire == 0 {
            Ok(control)
        } else {
            work_wires.get(wire - 1).copied()
                .ok_or_else(|| "modular gate wire overflow".into())
        }
    };
    let (target, mut controls) = match gate {
        ElementaryGate::X(target) => (map_wire(target)?, Vec::new()),
        ElementaryGate::Cnot { control, target } => {
            (map_wire(target)?, alloc::vec![map_wire(control)?])
        }
        ElementaryGate::Toffoli {
            first,
            second,
            target,
        } => (
            map_wire(target)?,
            alloc::vec![map_wire(first)?, map_wire(second)?],
        ),
    };
    controls.sort_unstable();
    if controls.iter().any(|&wire| wire == target)
        || controls.windows(2).any(|pair| pair[0] == pair[1])
        || target >= arena.cells
        || controls.iter().any(|&wire| wire >= arena.cells)
    {
        return Err("controlled modular gate escaped the prepared register".into());
    }
    *root = arena.controlled_flip(*root, target, &controls);
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WholeRegisterPhaseReadout {
    pub monadic_binds: u64,
    pub lane_order: Vec<(usize,bool)>,
    pub numerator: BigUint,
    pub denominator: BigUint,
    pub width: usize,
    pub source: BigUint,
    pub base: BigUint,
}

pub struct WholeRegisterFactorShot {
    pub phase: WholeRegisterPhaseReadout,
    pub closure: Option<crate::ququart_factor::QuquartFactorClosure>,
}

/// Source-bound factor route whose phase evidence enters as one complete
/// register sample and closes through the existing winding membrane.
pub struct WholeRegisterPhaseExecutor {
    source: BigUint,
    base: BigUint,
    width: Option<usize>,
    orientation: Vec<(bool, bool)>,
    random: u64,
    evidence: PhaseReadoutAccumulator,
    prepared_work: Option<PreparedModularWork>,
    streaming_digit_bits: Option<usize>,
    pending_circuit: Option<WholeRegisterPhaseCircuit>,
}

impl WholeRegisterPhaseExecutor {
    pub fn prepare(source: BigUint, base: BigUint, width: Option<usize>) -> Result<Self, String> {
        Self::prepare_with_reentry(source, base, width, 1729, &[])
    }

    /// Bind the Y/re-entry orientation word to the phase control coordinates.
    /// Each pair labels the two bits of one radix-four phase position.
    pub fn prepare_with_reentry(
        source: BigUint,
        base: BigUint,
        width: Option<usize>,
        seed: u64,
        orientation: &[(bool, bool)],
    ) -> Result<Self, String> {
        Self::prepare_with_reentry_and_work(source, base, width, seed, orientation, None)
    }

    /// Bind the prepared rewirer pool to the complete phase register. Every
    /// operation reference is recovered against the source-bound power stack
    /// before the executor can run.
    pub fn prepare_with_reentry_and_work(
        source: BigUint,
        base: BigUint,
        width: Option<usize>,
        seed: u64,
        orientation: &[(bool, bool)],
        prepared_work: Option<PreparedModularWork>,
    ) -> Result<Self, String> {
        let resolved_width = match width {
            Some(width) => width,
            None => usize::try_from(
                source
                    .bits()
                    .checked_mul(2)
                    .ok_or("phase register width overflow")?,
            )
            .map_err(|_| "phase register width exceeds host indexing")?,
        };
        if !orientation.is_empty() && orientation.len().checked_mul(2) != Some(resolved_width) {
            return Err("Y/re-entry orientation must cover the complete phase register".into());
        }
        // Validate the entire circuit before admitting this source/base pair.
        WholeRegisterPhaseCircuit::prepare_with_reentry_and_work(
            source.clone(),
            base.clone(),
            resolved_width,
            orientation,
            prepared_work.as_ref(),
        )?;
        Ok(Self {
            source,
            base,
            width: Some(resolved_width),
            orientation: orientation.to_vec(),
            random: seed.max(1),
            evidence: PhaseReadoutAccumulator::default(),
            prepared_work,
            streaming_digit_bits: None,
            pending_circuit: None,
        })
    }

    /// Use the existing nested modular emitter directly, without storing an
    /// externally prepared operation pool or changing the joint readout.
    pub fn use_streaming_nested_work(&mut self, digit_bits: usize) -> Result<(), String> {
        if digit_bits == 0 || self.prepared_work.is_some() {
            return Err("streaming nested work requires a positive radix and no external pool".into());
        }
        self.streaming_digit_bits = Some(digit_bits);
        Ok(())
    }

    /// Retain the complete powerset between operation returns. This performs
    /// no intermediate phase measurements; readout occurs after the full QFT.
    pub fn advance_streaming_next(&mut self) -> Result<Option<WholeRegisterFactorShot>, String> {
        let bits = self.streaming_digit_bits.ok_or("streaming work has not been selected")?;
        if self.pending_circuit.is_none() {
            self.pending_circuit = Some(WholeRegisterPhaseCircuit::prepare_with_reentry_and_work(
                self.source.clone(), self.base.clone(), self.width.unwrap(),
                &self.orientation, None)?);
        }
        if !self.pending_circuit.as_mut().unwrap().advance_nested_work(bits, 8)? {
            return Ok(None);
        }
        let mut circuit = self.pending_circuit.take().unwrap();
        let denominator = BigUint::one() << self.width.unwrap();
        let mut quantile = BigUint::zero();
        for offset in (0..self.width.unwrap()).step_by(64) {
            self.random ^= self.random << 13;
            self.random ^= self.random >> 7;
            self.random ^= self.random << 17;
            quantile |= BigUint::from(self.random) << offset;
        }
        quantile %= &denominator;
        let phase = circuit.fire_with_work_radix(&quantile, &denominator, Some(bits))?;
        self.close_readout(phase).map(Some)
    }

    /// Execution coordinates only; observing these does not measure a phase.
    pub fn streaming_progress(&self) -> Option<(usize, usize, usize)> {
        let circuit = self.pending_circuit.as_ref()?;
        let work = circuit.streaming_work.as_ref()?;
        Some((work.phase_wire, work.cursor, circuit.peak_nodes()))
    }

    /// Draw one complete-register sample from the executor's retained
    /// measurement stream. The seed initializes the stream once, not each shot.
    pub fn measure_next(&mut self) -> Result<WholeRegisterFactorShot, String> {
        self.measure_next_with_readout(|_| Ok(()))
    }

    /// Expose the complete measured register through a terminal port before
    /// feeding its value into the factor-closure operation.
    pub fn measure_next_with_readout(
        &mut self,
        readout: impl FnOnce(&WholeRegisterPhaseReadout) -> Result<(), String>,
    ) -> Result<WholeRegisterFactorShot, String> {
        let width = self.width.expect("prepared phase width");
        let denominator = BigUint::one() << width;
        let mut numerator = BigUint::zero();
        let mut offset = 0usize;
        while offset < width {
            self.random ^= self.random << 13;
            self.random ^= self.random >> 7;
            self.random ^= self.random << 17;
            numerator |= BigUint::from(self.random) << offset;
            offset = offset.saturating_add(64);
        }
        numerator %= &denominator;
        self.measure_with_readout(&numerator, &denominator, readout)
    }

    pub fn measure(
        &mut self,
        quantile: &BigUint,
        quantile_denominator: &BigUint,
    ) -> Result<WholeRegisterFactorShot, String> {
        self.measure_with_readout(quantile, quantile_denominator, |_| Ok(()))
    }

    pub fn measure_with_readout(
        &mut self,
        quantile: &BigUint,
        quantile_denominator: &BigUint,
        readout: impl FnOnce(&WholeRegisterPhaseReadout) -> Result<(), String>,
    ) -> Result<WholeRegisterFactorShot, String> {
        let mut circuit = WholeRegisterPhaseCircuit::prepare_with_reentry_and_work(
            self.source.clone(),
            self.base.clone(),
            self.width.unwrap(),
            &self.orientation,
            self.prepared_work.as_ref(),
        )?;
        let phase = circuit.fire_with_work_radix(quantile, quantile_denominator,
            self.streaming_digit_bits)?;
        readout(&phase)?;
        self.close_readout(phase)
    }

    fn close_readout(&mut self, phase: WholeRegisterPhaseReadout)
        -> Result<WholeRegisterFactorShot, String> {
        let closure = if let Some((order, measured_p, measured_q)) = self.evidence.close_fraction(
            &phase.numerator,
            &phase.denominator,
            &self.base,
            &self.source,
        )? {
            let result = crate::ququart_factor::extract_certified_winding_for_base(
                &self.source,
                &self.base,
                &order,
            )?;
            let (p, q) = result.factors();
            if !((p == &measured_p && q == &measured_q) || (p == &measured_q && q == &measured_p)) {
                return Err("whole-register winding changed its measured factor arms".into());
            }
            Some(result)
        } else {
            None
        };
        Ok(WholeRegisterFactorShot { phase, closure })
    }
}

/// Prepared source-bound phase circuit. The powerset IS re-entry: one
/// correlated decision object holds every phase branch as the controlled
/// modular operator acts with successive squared bases.
pub struct WholeRegisterPhaseCircuit {
    monadic_binds: u64,
    lane_order: Vec<(usize,bool)>,
    banked_roots: Vec<usize>,
    source: BigUint,
    base: BigUint,
    source_word: WordTape,
    base_word: WordTape,
    width: usize,
    format: FixedPointFormat,
    arithmetic: ModularMultiply,
    arena: DecisionArena,
    root: usize,
    gates_since_fold: usize,
    peak_nodes: usize,
    fired: bool,
    prepared_stages: Option<Vec<(BigUint, Vec<NestedOperation>, Vec<NestedOperation>)>>,
    streaming_work: Option<StreamingWork>,
    baked_work: Option<PreparedWork>,
    baked_lane_offsets: Vec<usize>,
    work_wires: Vec<usize>,
    hadamard_coefficient: FixedComplex,
    fourier_coefficients: Option<Vec<FixedComplex>>,
}

enum PreparedWork {
    Owned(PreparedModularWork),
    Compiled(&'static crate::baked_resident::BakedWork),
}
impl PreparedWork {
    fn digit_bits(&self)->usize {match self {Self::Owned(w)=>w.digit_bits,Self::Compiled(w)=>w.digit_bits}}
    fn operation_count(&self)->usize {match self {Self::Owned(w)=>w.operations.len(),Self::Compiled(w)=>w.operations.len()}}
    fn references(&self,stage:usize)->&[usize] {match self {Self::Owned(w)=>&w.stages[stage].1,Self::Compiled(w)=>w.stages[stage].1}}
    fn operation(&self,id:usize)->Result<NestedOperation,String> {
        match self {Self::Owned(w)=>w.operations.get(id).cloned().ok_or_else(||String::from("prepared instruction escaped its pool")),Self::Compiled(w)=>w.operation(id)}
    }
}

struct StreamingWork {
    digit_bits: usize,
    phase_wire: usize,
    multiplier: WordTape,
    operations: Vec<NestedOperation>,
    cursor: usize,
    pending_wires: Vec<usize>,
}

/// The word and its operation cursor remain owned after its register returns.
pub struct WholeRegisterPhaseProgram {
    source_word: WordTape,
    base_word: WordTape,
    width: usize,
    format: FixedPointFormat,
    arithmetic: ModularMultiply,
    work_wires: Vec<usize>,
    prepared_stages: Option<Vec<(BigUint,Vec<NestedOperation>,Vec<NestedOperation>)>>,
    baked_work: Option<PreparedWork>,
    baked_lane_offsets: Vec<usize>,
    continuation: Option<StreamingWork>,
    hadamard_coefficient: FixedComplex,
    fourier_coefficients: Option<Vec<FixedComplex>>,
    lane_order: Vec<(usize,bool)>,
}

impl WholeRegisterPhaseProgram {
    pub fn source_word(&self) -> &str {self.source_word.as_word()}
    pub fn operation_count(&self) -> usize {self.baked_work.as_ref().map_or(0,|work|work.operation_count())}
}

impl WholeRegisterPhaseCircuit {
    pub(crate) fn from_baked(data:&'static crate::baked_resident::BakedPhase)->Result<Self,String> {
        let source_word=WordTape::from_canonical_word(data.source_word).ok_or("invalid compiled phase source")?;
        let base_word=WordTape::from_canonical_word(data.base_word).ok_or("invalid compiled phase base")?;
        let source=source_word.to_biguint();
        let base=base_word.to_biguint();
        let work=data.work;
        #[cfg(feature="hosted")]
        eprintln!("resident-import stage=compiled-work operations={} stages={}",work.operations.len(),work.stages.len());
        if data.width==0 || data.width%2!=0 || BigUint::new(work.source.to_vec())!=source || work.stages.len()!=data.width/2 || work.digit_bits==0 {
            return Err("compiled phase program lost its source or stack height".into());
        }
        let format=FixedPointFormat::for_modulus_bits(data.modulus_bits)?;
        let arithmetic=ModularMultiply::new_phase_register(&source)?;
        if format!=FixedPointFormat::for_modulus(&source)? || data.cells<data.width+arithmetic.elementary_qubits()-1 {
            return Err("compiled phase register changed its source layout".into());
        }
        let work_wires=data.work_wires.to_vec();
        let mut distinct=work_wires.clone(); distinct.sort_unstable(); distinct.dedup();
        if work_wires.len()!=arithmetic.elementary_qubits()-1 || distinct.len()!=work_wires.len()
            || work_wires.iter().any(|&w|w<data.width || w>=data.cells) {
            return Err("compiled work map escaped its register".into());
        }
        let fourier_coefficients:Vec<_>=data.fourier.iter().map(|z|z.value()).collect();
        if fourier_coefficients.len()!=data.width-1 {return Err("compiled Fourier operator changed width".into());}
        #[cfg(feature="hosted")]
        eprintln!("resident-import stage=register nodes={}",data.nodes.len());
        let (arena,root)=DecisionArena::from_baked_nodes(data.cells,data.root,data.nodes)?;
        let peak_nodes=arena.retained_nodes();
        let baked_lane_offsets=work.lane_offsets()?;
        #[cfg(feature="hosted")]
        eprintln!("resident-import stage=phase-ready");
        Ok(Self {monadic_binds:0,lane_order:Vec::new(),banked_roots:Vec::new(),source,base,source_word,base_word,
            width:data.width,format,arithmetic,arena,root,work_wires,gates_since_fold:0,peak_nodes,fired:false,
            prepared_stages:None,streaming_work:None,baked_work:Some(PreparedWork::Compiled(work)),baked_lane_offsets,
            hadamard_coefficient:data.hadamard.value(),fourier_coefficients:Some(fourier_coefficients)})
    }
    #[cfg(feature = "hosted")]
    pub(crate) fn prepared_record(&mut self) -> Result<serde_json::Value,String> {
        if self.fired || self.streaming_work.is_some() || self.prepared_stages.is_some() {
            return Err("preparation crossed the execution trigger".into());
        }
        let denominator = WordTape::one_at(self.width).to_biguint();
        let numerators: Vec<_> = (1..self.width).map(|distance|
            SignedWordTape::from_magnitude(WordTape::one_at(self.width-distance-1),true).to_bigint()).collect();
        self.fourier_coefficients = Some(FixedComplex::winding_twiddle_table(&numerators,&denominator,&self.format)?);
        let complex = |z: &FixedComplex| serde_json::json!({
            "re":SignedWordTape::from_bigint(&z.re).prepared_record(),
            "im":SignedWordTape::from_bigint(&z.im).prepared_record()});
        Ok(serde_json::json!({"source_word":self.source_word.as_word(),"base_word":self.base_word.as_word(),
            "width":self.width,"modulus_bits":self.format.modulus_bits,"work_wires":self.work_wires,
            "hadamard":complex(&self.hadamard_coefficient),
            "fourier":self.fourier_coefficients.as_ref().unwrap().iter().map(complex).collect::<Vec<_>>(),
            "register":self.arena.prepared_record(self.root)}))
    }

    #[cfg(feature = "hosted")]
    pub(crate) fn from_prepared_record(record: &serde_json::Value, work: PreparedModularWork) -> Result<Self,String> {
        let numeral = |key: &str| WordTape::from_canonical_word(record[key].as_str()
            .ok_or_else(|| alloc::format!("missing prepared {key}"))?)
            .ok_or_else(|| alloc::format!("invalid prepared {key}"));
        let source_word = numeral("source_word")?;
        let base_word = numeral("base_word")?;
        let source = source_word.to_biguint();
        let base = base_word.to_biguint();
        let width = record["width"].as_u64().and_then(|n| usize::try_from(n).ok()).ok_or("invalid prepared phase width")?;
        if width==0 || width%2!=0 || work.source!=source || work.stages.len()!=width/2 || work.digit_bits==0 {
            return Err("prepared phase program lost its source or stack height".into());
        }
        let complex = |row: &serde_json::Value| -> Result<FixedComplex,String> {
            Ok(FixedComplex { re:SignedWordTape::from_prepared_record(&row["re"])?.to_bigint(),
                im:SignedWordTape::from_prepared_record(&row["im"])?.to_bigint() })
        };
        let hadamard_coefficient = complex(&record["hadamard"])?;
        let fourier_coefficients = record["fourier"].as_array().ok_or("missing prepared Fourier operator")?
            .iter().map(complex).collect::<Result<Vec<_>,_>>()?;
        if fourier_coefficients.len()!=width-1 { return Err("prepared Fourier operator changed width".into()); }
        let (arena,root) = DecisionArena::from_prepared_record(&record["register"])?;
        let peak_nodes = arena.retained_nodes();
        let baked_lane_offsets = work.stages.iter().map(|(_,references)| references.iter().take_while(|&&id| {
            let controls = match &work.operations[id] {
                NestedOperation::Toggle(gate) => &gate.controls,
                NestedOperation::ModularAdd { controls,.. } |
                NestedOperation::Add { controls,.. } |
                NestedOperation::Compare { controls,.. } => controls,
            };
            controls.iter().any(|&(wire,_)| wire==0)
        }).count()).collect();
        let arithmetic = ModularMultiply::new_phase_register(&source)?;
        let format = FixedPointFormat::for_modulus_bits(record["modulus_bits"].as_u64().ok_or("missing prepared phase precision")?)?;
        if format != FixedPointFormat::for_modulus(&source)? || arena.cells < width+arithmetic.elementary_qubits()-1 {
            return Err("prepared phase register changed its source layout".into());
        }
        let work_wires = match record.get("work_wires") {
            Some(rows) => rows.as_array().ok_or("invalid prepared work map")?.iter()
                .map(|v| v.as_u64().and_then(|v| usize::try_from(v).ok()).ok_or("invalid work wire"))
                .collect::<Result<Vec<_>,_>>()?,
            None => (width..width+arithmetic.elementary_qubits()-1).collect(),
        };
        let mut distinct = work_wires.clone(); distinct.sort_unstable(); distinct.dedup();
        if work_wires.len()!=arithmetic.elementary_qubits()-1 || distinct.len()!=work_wires.len()
            || work_wires.iter().any(|&w| w<width || w>=arena.cells) {
            return Err("prepared work map escaped its register".into());
        }
        Ok(Self { monadic_binds:0,lane_order:Vec::new(),banked_roots:Vec::new(),source,base,source_word,base_word,width,format,arithmetic,arena,root,work_wires,
            gates_since_fold:0,peak_nodes,fired:false,prepared_stages:None,streaming_work:None,
            baked_work:Some(PreparedWork::Owned(work)),baked_lane_offsets,hadamard_coefficient,fourier_coefficients:Some(fourier_coefficients) })
    }

    /// Prepare the complete control and work register from the resident
    /// `(base, source)` relation. No phase outcome or period enters preparation.
    pub fn prepare(source: BigUint, base: BigUint, width: Option<usize>) -> Result<Self, String> {
        let resolved_width = match width {
            Some(width) => width,
            None => WordTape::from_biguint(&source).bit_len().checked_mul(2)
                .ok_or("phase register width overflow")?,
        };
        Self::prepare_with_reentry(source, base, resolved_width, &[])
    }

    pub fn prepare_with_reentry(
        source: BigUint,
        base: BigUint,
        width: usize,
        orientation: &[(bool, bool)],
    ) -> Result<Self, String> {
        Self::prepare_with_reentry_and_work(source, base, width, orientation, None)
    }

    pub fn prepare_with_reentry_and_work(
        source: BigUint,
        base: BigUint,
        width: usize,
        orientation: &[(bool, bool)],
        prepared_work: Option<&PreparedModularWork>,
    ) -> Result<Self, String> {
        let source_word = WordTape::from_biguint(&source);
        let base_word = WordTape::from_biguint(&base);
        if !source_word.ge(&WordTape::from_small(4)) {
            return Err("phase register requires a modulus of at least four".into());
        }
        let mut gcd = source_word.clone();
        let mut remainder = base_word.clone();
        while !remainder.is_zero() {
            let next = gcd.divmod(&remainder).unwrap().1;
            gcd = remainder; remainder = next;
        }
        if !base_word.gt(&WordTape::one()) || base_word.ge(&source_word)
            || !gcd.is_one()
        {
            return Err("phase register base must be a nontrivial source-bound unit".into());
        }
        if width == 0 {
            return Err("phase register width must be positive".into());
        }
        if !orientation.is_empty() && orientation.len().checked_mul(2) != Some(width) {
            return Err("Y/re-entry orientation must cover the complete phase register".into());
        }
        let prepared_stages = prepared_work
            .map(|work| work.validated_lane_stages(&source, &base, work.digit_bits, width / 2))
            .transpose()?;
        if prepared_stages.is_some() && width % 2 != 0 {
            return Err("prepared rewirer requires complete radix-four phase positions".into());
        }
        let arithmetic = ModularMultiply::new_phase_register(&source)?;
        let cells = width
            .checked_add(arithmetic.elementary_qubits().saturating_sub(1))
            .ok_or("whole-register wire layout overflow")?;
        let format = FixedPointFormat::for_modulus(&source)?;
        let scale_word = WordTape::one_at(format.w_bits as usize);
        let hadamard_coefficient = FixedComplex {
            re: num_bigint::BigInt::from(scale_word.mul(&scale_word).shr1().isqrt().to_biguint()),
            im: num_bigint::BigInt::zero(),
        };
        let mut arena = DecisionArena::new(cells);
        let address = BigUint::one() << width;
        let root = arena.basis(&address, format.scale());
        let mut circuit = Self {
            monadic_binds:0,
            lane_order:Vec::new(),
            banked_roots:Vec::new(),
            source,
            base,
            source_word,
            base_word,
            width,
            format,
            arithmetic,
            arena,
            root,
            gates_since_fold: 0,
            peak_nodes: 0,
            fired: false,
            prepared_stages,
            streaming_work: None,
            baked_work: None,
            baked_lane_offsets: Vec::new(),
            work_wires: (width..cells).collect(),
            hadamard_coefficient,
            fourier_coefficients: None,
        };
        for wire in 0..width {
            circuit.root = circuit
                .arena
                .hadamard_with_scalar(circuit.root, wire, &circuit.format,&circuit.hadamard_coefficient)?;
            circuit.fold();
        }
        for (position, &(forward, inverse)) in orientation.iter().enumerate() {
            for (lane, active) in [forward, inverse].into_iter().enumerate() {
                if active {
                    let wire = 2 * position + lane;
                    circuit.root = circuit.arena.controlled_flip(circuit.root, wire, &[]);
                }
            }
        }
        circuit.fold();
        Ok(circuit)
    }

    pub fn width(&self) -> usize {
        self.width
    }
    pub(crate) fn cells(&self) -> usize { self.arena.cells }
    pub fn source(&self) -> &BigUint {
        &self.source
    }
    pub fn base(&self) -> &BigUint {
        &self.base
    }
    pub fn peak_nodes(&self) -> usize {
        self.peak_nodes
    }

    /// Import a source-matched factor/product/carry support into this complete
    /// phase carrier. Both supports then reside in one amplitude function.
    pub fn bind_resident_support(&mut self, source: &BigUint, arena: &DecisionArena,
        root: usize) -> Result<usize, String> {
        if source != &self.source || self.fired || self.streaming_work.is_some() {
            return Err("resident phase binding changed its source or firing boundary".into());
        }
        let offset = self.arena.cells;
        let expected_mass = self.arena.mass_word(self.root).mul(&arena.mass_word(root));
        let cells = offset.checked_add(arena.cells).ok_or("resident phase cell overflow")?;
        let mut joint = DecisionArena::new(cells);
        let phase = joint.import_shifted(&self.arena, self.root, 0)?;
        let factors = joint.import_shifted(arena, root, offset)?;
        let root = joint.amplitude_product(phase, factors);
        if joint.mass_word(root) != expected_mass {
            return Err("resident phase binding changed the tensor support norm".into());
        }
        self.arena = joint;
        self.root = root;
        self.fold();
        Ok(offset)
    }

    pub fn into_resident_support(self) -> (DecisionArena, usize) {
        (self.arena, self.root)
    }

    pub fn into_banked_resident_support(self) -> (DecisionArena,usize,Vec<usize>,WholeRegisterPhaseProgram) {
        let program=WholeRegisterPhaseProgram {
            source_word:self.source_word,base_word:self.base_word,width:self.width,format:self.format,
            arithmetic:self.arithmetic,work_wires:self.work_wires,prepared_stages:self.prepared_stages,
            baked_work:self.baked_work,baked_lane_offsets:self.baked_lane_offsets,continuation:self.streaming_work,
            hadamard_coefficient:self.hadamard_coefficient,fourier_coefficients:self.fourier_coefficients,
            lane_order:self.lane_order,
        };
        (self.arena,self.root,self.banked_roots,program)
    }

    pub fn support_mass(&self) -> BigUint { self.arena.mass(self.root) }
    pub fn lane_order(&self) -> &[(usize,bool)] { &self.lane_order }
    pub fn monadic_binds(&self) -> u64 { self.monadic_binds }
    pub fn support_mass_word(&self) -> crate::word_tape::WordTape { self.arena.mass_word(self.root) }

    pub fn execution_progress(&self) -> Option<(usize,usize,usize)> {
        self.streaming_work.as_ref().map(|work|
            (work.phase_wire,work.cursor,self.arena.retained_nodes()))
    }

    /// Run a source-relation operator on the very diagram advanced by phase
    /// work. The next modular return sees the refined support immediately.
    pub fn refine_resident_support<T>(&mut self,
        operation: impl FnOnce(&mut DecisionArena, &mut usize) -> Result<T,String>)
        -> Result<T,String> {
        if self.fired { return Err("resident refinement crossed phase readout".into()); }
        let mut result=None;
        self.root=crate::reentry_powerset::bind_one(self.root,|mut root| {
            result=Some(operation(&mut self.arena,&mut root)?);
            Ok::<_,String>(root)
        })?;
        self.fold();
        Ok(result.expect("resident refinement executed its single arrow"))
    }

    /// Copy the resident candidate lane into modular work coherently. Phase
    /// work now follows each retained factor branch instead of a detached unit.
    pub fn bind_candidate_work(&mut self, candidate: &[usize]) -> Result<Vec<usize>,String> {
        if self.fired || self.streaming_work.is_some() || candidate.is_empty()
            || candidate.len() > self.arithmetic.width()
            || candidate.iter().any(|&wire| wire < self.width+self.arithmetic.elementary_qubits()-1
                || wire >= self.arena.cells)
            || candidate.windows(2).any(|w| w[0] >= w[1]) {
            return Err("candidate lane escaped the resident phase work boundary".into());
        }
        let zero = self.arena.zero();
        let invalid = self.arena.conditional_literals(zero,self.root,&[(self.work_wires[0],false)]);
        if !self.arena.mass(invalid).is_zero() {
            return Err("candidate work requires the prepared unit boundary".into());
        }
        for bit in 1..self.arithmetic.width() {
            if !self.arena.wire_is_zero(self.root,self.work_wires[bit]) {
                return Err("candidate work crossed an already occupied modular cell".into());
            }
        }
        // Keep each copied bit beside its source. Grouping all copies before
        // all source bits makes the equality relation enumerate prefixes.
        let copied = &self.work_wires[..candidate.len()];
        let mut order = Vec::with_capacity(self.arena.cells);
        for old in 0..self.arena.cells {
            if copied.contains(&old) { continue; }
            if let Ok(bit) = candidate.binary_search(&old) { order.push(copied[bit]); }
            order.push(old);
        }
        let mut mapping = alloc::vec![0;self.arena.cells];
        for (new,old) in order.into_iter().enumerate() { mapping[old]=new; }
        #[cfg(feature = "hosted")]
        eprintln!("candidate-binding permutation-start nodes={}",self.arena.retained_nodes());
        let (arena,root) = self.arena.permuted(self.root,&mapping)?;
        self.arena=arena; self.root=root;
        for wire in &mut self.work_wires { *wire=mapping[*wire]; }
        self.fold();
        #[cfg(feature = "hosted")]
        eprintln!("candidate-binding permutation-end nodes={}",self.arena.retained_nodes());
        self.root = self.arena.flip(self.root,self.work_wires[0]);
        for (bit,&control) in candidate.iter().enumerate() {
            self.root = self.arena.controlled_flip(self.root,self.work_wires[bit],&[mapping[control]]);
            self.fold();
            #[cfg(feature = "hosted")]
            if (bit+1)%32==0 || bit+1==candidate.len() {
                eprintln!("candidate-binding copied={} nodes={}",bit+1,self.arena.retained_nodes());
            }
        }
        Ok(mapping)
    }

    /// Advance operations while the complete phase/work register stays resident.
    /// Only one controlled multiplier's operation pool is retained at a time.
    pub fn advance_nested_work(&mut self, digit_bits: usize, quantum: usize)
        -> Result<bool, String> {
        self.advance_nested_work_clocked(digit_bits, quantum, false)
    }

    /// A certified clock selects the next complete prepared lane. An unfinished
    /// lane retains its cursor; every lane executes exactly once on this root.
    pub fn advance_nested_work_clocked(&mut self, digit_bits: usize, quantum: usize,
        reverse: bool) -> Result<bool, String> {
        self.advance_nested_work_with_selector(digit_bits,quantum,||Ok(reverse))
    }

    pub fn advance_nested_work_with_selector(&mut self,digit_bits:usize,quantum:usize,
        mut select:impl FnMut()->Result<bool,String>) -> Result<bool,String> {
        if self.fired || self.prepared_stages.is_some() || digit_bits == 0 || quantum == 0 {
            return Err("invalid incremental nested-work state".into());
        }
        if self.streaming_work.is_none() {
            if self.banked_roots.is_empty() {self.banked_roots.push(self.root);}
            self.streaming_work = Some(StreamingWork { digit_bits, phase_wire: 0,
                multiplier: self.base_word.clone(), operations: Vec::new(), cursor: 0,
                pending_wires: (0..self.width).collect() });
        }
        for _ in 0..quantum {
            let mut work = self.streaming_work.take().unwrap();
            if work.digit_bits != digit_bits {
                self.streaming_work = Some(work);
                return Err("nested-work radix changed during execution".into());
            }
            if work.phase_wire == self.width {
                self.streaming_work = Some(work);
                return Ok(true);
            }
            if let Some(baked) = &self.baked_work {
                if digit_bits!=baked.digit_bits() {
                    return Err("execution changed the baked work radix".into());
                }
                if work.cursor == 0 {
                    let reverse=select()?;
                    work.phase_wire = if reverse { *work.pending_wires.last().unwrap() }
                        else { work.pending_wires[0] };
                    self.lane_order.push((work.phase_wire,reverse));
                }
                let references = baked.references(work.phase_wire/2);
                let lane = work.phase_wire%2;
                let first_lane = self.baked_lane_offsets[work.phase_wire/2];
                let operations = if lane==0 { &references[..first_lane] } else { &references[first_lane..] };
                let count = operations.len();
                let operation = operations.get(work.cursor).map(|&id| baked.operation(id)).transpose()?;
                if let Some(operation) = operation {
                    self.apply_prepared_operation(work.phase_wire,lane,&operation)?;
                    work.cursor += 1;
                }
                if work.cursor==count {
                    work.pending_wires.retain(|&wire| wire!=work.phase_wire);
                    work.cursor=0;
                    work.phase_wire=work.pending_wires.first().copied().unwrap_or(self.width);
                }
                self.streaming_work=Some(work);
                continue;
            }
            if work.operations.is_empty() {
                if select()? {return Err("reverse lane selection requires the prepared modular pool".into());}
                self.lane_order.push((work.phase_wire,false));
                self.arithmetic.emit_ququart_nested_radix(&work.multiplier.to_biguint(),
                    work.phase_wire % 2, digit_bits, |op| { work.operations.push(op); Ok(()) })?;
            }
            if !work.operations.is_empty() {
                self.apply_prepared_operation(work.phase_wire, work.phase_wire % 2,
                    &work.operations[work.cursor])?;
                work.cursor += 1;
            }
            if work.cursor == work.operations.len() {
                work.operations.clear();
                work.cursor = 0;
                work.phase_wire += 1;
                work.multiplier = work.multiplier.mul(&work.multiplier)
                    .divmod(&self.source_word).unwrap().1;
            }
            self.streaming_work = Some(work);
        }
        Ok(self.streaming_work.as_ref().unwrap().phase_wire == self.width)
    }

    fn fold(&mut self) {
        fold_diagram(&mut self.arena, &mut self.root,&mut self.banked_roots);
        self.gates_since_fold = 0;
        self.peak_nodes = self.peak_nodes.max(self.arena.retained_nodes());
    }

    fn controlled_multiply(
        &mut self,
        phase_wire: usize,
        multiplier: &BigUint,
    ) -> Result<(), String> {
        let arithmetic = &self.arithmetic;
        let work_wires = &self.work_wires;
        let arena = &mut self.arena;
        let root = &mut self.root;
        let banks = &mut self.banked_roots;
        let gates_since_fold = &mut self.gates_since_fold;
        let peak_nodes = &mut self.peak_nodes;
        arithmetic.emit_elementary(multiplier, |gate| {
            apply_elementary(arena, root, work_wires, phase_wire, gate)?;
            *gates_since_fold += 1;
            if *gates_since_fold == 8 {
                fold_diagram(arena, root,banks);
                *gates_since_fold = 0;
                *peak_nodes = (*peak_nodes).max(arena.retained_nodes());
            }
            Ok(())
        })
    }

    fn apply_prepared_operation(
        &mut self,
        phase_wire: usize,
        lane: usize,
        operation: &NestedOperation,
    ) -> Result<(), String> {
        let map_wire = |wire: usize| -> Result<usize, String> {
            match wire {
                0 | 1 if wire == lane => Ok(phase_wire),
                0 | 1 => Err("prepared operation addresses the other phase lane".into()),
                _ => self.work_wires.get(wire - 2).copied()
                    .ok_or_else(|| "prepared work wire overflow".into()),
            }
        };
        let controls = |items: &[(usize, bool)]| -> Result<Vec<(usize, bool)>, String> {
            let mut mapped = items
                .iter()
                .map(|&(wire, positive)| Ok((map_wire(wire)?, positive)))
                .collect::<Result<Vec<_>, String>>()?;
            mapped.sort_unstable();
            Ok(mapped)
        };
        // Bind the complete correlated diagram through the canonical powerset
        // arrow. This deterministic modular arrow has one complete successor.
        let input=crate::reentry_powerset::unit(self.root);
        let output=crate::reentry_powerset::bind(input, |root| {
        let successor = match operation {
            NestedOperation::Toggle(gate) => {
                let target = map_wire(gate.target)?;
                let controls = controls(&gate.controls)?;
                if target >= self.arena.cells
                    || controls.iter().any(|&(wire, _)| wire >= self.arena.cells)
                    || controls.iter().any(|&(wire, _)| wire == target)
                {
                    return Err("prepared toggle escaped its rewirer register".into());
                }
                let zero = self.arena.zero();
                let enabled = self.arena.conditional_literals(zero, root, &controls);
                let changed = self.arena.flip(enabled, target);
                self.arena
                    .conditional_literals(root, changed, &controls)
            }
            NestedOperation::ModularAdd {
                register,
                digit,
                value,
                modulus,
                controls: literals,
            } => {
                let register = register
                    .iter()
                    .map(|&wire| map_wire(wire))
                    .collect::<Result<Vec<_>, _>>()?;
                let digit = digit
                    .iter()
                    .map(|&wire| map_wire(wire))
                    .collect::<Result<Vec<_>, _>>()?;
                let controls = controls(literals)?;
                if register
                    .iter()
                    .chain(digit.iter())
                    .chain(controls.iter().map(|x| &x.0))
                    .any(|&wire| wire >= self.arena.cells)
                {
                    return Err("prepared modular addition escaped its rewirer register".into());
                }
                self.arena
                    .modular_digit_add(root, &register, &digit, value, modulus, &controls)
            }
            NestedOperation::Add {
                register,
                value,
                controls: literals,
            } => {
                let register = register
                    .iter()
                    .map(|&wire| map_wire(wire))
                    .collect::<Result<Vec<_>, _>>()?;
                let controls = controls(literals)?;
                self.arena
                    .add_constant(root, &register, value, &controls)
            }
            NestedOperation::Compare {
                register,
                value,
                controls: literals,
                flag,
            } => {
                let register = register
                    .iter()
                    .map(|&wire| map_wire(wire))
                    .collect::<Result<Vec<_>, _>>()?;
                let flag = map_wire(*flag)?;
                let controls = controls(literals)?;
                self.arena
                    .compare_constant(root, &register, value, &controls, flag)
            }
        };
        Ok::<_,String>(crate::reentry_powerset::unit(successor))
        })?;
        if output.len()!=1 {return Err("deterministic modular bind changed its successor multiplicity".into());}
        self.root=output.into_iter().next().unwrap();
        self.monadic_binds+=1;
        self.gates_since_fold += 1;
        if self.gates_since_fold == 8 {
            self.fold();
        }
        Ok(())
    }

    /// Fire the prepared circuit once and return the entire phase register in
    /// one sample. `quantile` is the Born-rank supplied by the measurement
    /// instrument; the phase word itself is not reconstructed from digits.
    pub fn fire(
        &mut self,
        quantile: &BigUint,
        quantile_denominator: &BigUint,
    ) -> Result<WholeRegisterPhaseReadout, String> {
        self.fire_with_work_radix(quantile, quantile_denominator, None)
    }

    pub fn fire_with_work_radix(
        &mut self,
        quantile: &BigUint,
        quantile_denominator: &BigUint,
        streaming_digit_bits: Option<usize>,
    ) -> Result<WholeRegisterPhaseReadout, String> {
        if self.fired {
            return Err("whole-register phase circuit has already fired".into());
        }
        let quantile = WordTape::from_biguint(quantile);
        let quantile_denominator = WordTape::from_biguint(quantile_denominator);
        if self.banked_roots.is_empty() {self.banked_roots.push(self.root);}
        if quantile_denominator.is_zero() || quantile.ge(&quantile_denominator) {
            return Err("whole-register phase measurement has an invalid Born quantile".into());
        }
        if let Some(baked) = &self.baked_work {
            let bits = baked.digit_bits();
            if streaming_digit_bits.is_some_and(|given| given!=bits) {
                return Err("readout changed the baked work radix".into());
            }
            while !self.advance_nested_work(bits,8)? {}
        } else if let Some(work) = &self.streaming_work {
            let bits = work.digit_bits;
            if streaming_digit_bits != Some(bits) {
                return Err("readout work radix differs from retained work".into());
            }
            while !self.advance_nested_work(bits, 8)? {}
        } else if let Some(stages) = self.prepared_stages.clone() {
            for (position, (_, forward, inverse)) in stages.iter().enumerate() {
                let phase_wires = [2 * position, 2 * position + 1];
                for (lane, (phase_wire, operations)) in
                    phase_wires.into_iter().zip([forward, inverse]).enumerate()
                {
                    for operation in operations {
                        self.apply_prepared_operation(phase_wire, lane, operation)?;
                    }
                }
            }
        } else if let Some(digit_bits) = streaming_digit_bits {
            if digit_bits == 0 {
                return Err("streaming nested work radix must be positive".into());
            }
            let arithmetic = ModularMultiply::new_phase_register(&self.source)?;
            let mut multiplier = self.base_word.clone();
            for phase_wire in 0..self.width {
                let lane = phase_wire % 2;
                arithmetic.emit_ququart_nested_radix(&multiplier.to_biguint(), lane, digit_bits,
                    |operation| self.apply_prepared_operation(phase_wire, lane, &operation))?;
                multiplier = multiplier.mul(&multiplier).divmod(&self.source_word).unwrap().1;
            }
        } else {
            let mut multiplier = self.base_word.clone();
            for phase_wire in 0..self.width {
                self.controlled_multiply(phase_wire, &multiplier.to_biguint())?;
                multiplier = multiplier.mul(&multiplier).divmod(&self.source_word).unwrap().1;
            }
        }
        self.fold();
        let phase_wires: Vec<_> = (0..self.width).collect();
        self.banked_roots.push(self.root);
        self.root = crate::reentry_powerset::bind_one(self.root,|root| if let Some(phases) = &self.fourier_coefficients {
            self.arena.fourier_register_prepared(root,&phase_wires,&self.format,
                phases,&self.hadamard_coefficient)
        } else { self.arena.fourier_register(root,&phase_wires,true,&self.format) })?;
        self.monadic_binds+=1;
        self.fold();
        let numerator_word = self.arena.sample_word(self.root, self.width,
            &quantile, &quantile_denominator)?;
        let measured: Vec<_> = phase_wires.iter().enumerate()
            .map(|(bit, &wire)| (wire, numerator_word.bit_at(bit))).collect();
        let zero = self.arena.zero();
        self.banked_roots.push(self.root);
        self.root = crate::reentry_powerset::bind_one(self.root,|root|
            Ok::<_,String>(self.arena.conditional_literals(zero, root, &measured)))?;
        self.monadic_binds+=1;
        self.fold();
        if self.arena.mass_word(self.root).is_zero() {
            return Err("joint phase measurement selected an empty branch".into());
        }
        self.fired = true;
        Ok(WholeRegisterPhaseReadout {
            monadic_binds:self.monadic_binds,
            lane_order:self.lane_order.clone(),
            numerator: numerator_word.to_biguint(),
            denominator: WordTape::one_at(self.width).to_biguint(),
            width: self.width,
            source: self.source.clone(),
            base: self.base.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resident_binding_preserves_full_phase_bins_and_factor_support() {
        let source = BigUint::from(15u8);
        for quantile in [1u8, 3, 5, 7] {
            let mut standalone = WholeRegisterPhaseCircuit::prepare(
                source.clone(), BigUint::from(2u8), Some(4)).unwrap();
            let expected = standalone.fire(&BigUint::from(quantile), &BigUint::from(8u8)).unwrap();
            let mut support = DecisionArena::new(2);
            let left = support.basis(&BigUint::from(1u8), num_bigint::BigInt::from(2));
            let right = support.basis(&BigUint::from(2u8), num_bigint::BigInt::from(-2));
            let root = support.sum(left, right);
            let mut joint = WholeRegisterPhaseCircuit::prepare(
                source.clone(), BigUint::from(2u8), Some(4)).unwrap();
            assert!(joint.bind_resident_support(&BigUint::from(21u8), &support, root).is_err());
            let offset = joint.bind_resident_support(&source, &support, root).unwrap();
            let measured = joint.fire(&BigUint::from(quantile), &BigUint::from(8u8)).unwrap();
            assert_eq!(measured, expected);
            for pair in 0u8..4 {
                let controls = [(offset, pair&1 != 0), (offset+1, pair&2 != 0)];
                let zero = joint.arena.zero();
                let selected = joint.arena.conditional_literals(zero, joint.root, &controls);
                assert_eq!(!joint.arena.mass(selected).is_zero(), matches!(pair, 1 | 2));
            }
            let zero = joint.arena.zero();
            let first = joint.arena.conditional_literals(zero, joint.root,
                &[(offset,true),(offset+1,false)]);
            let second = joint.arena.conditional_literals(zero, joint.root,
                &[(offset,false),(offset+1,true)]);
            assert_eq!(joint.arena.mass(first), joint.arena.mass(second));
        }
    }

    fn prepared_work(source: &BigUint, base: &BigUint, digit_bits: usize) -> PreparedModularWork {
        let arithmetic = ModularMultiply::new_phase_register(source).unwrap();
        let mut operations = Vec::new();
        let mut stages = Vec::new();
        let mut multiplier = base % source;
        for _ in 0..2 {
            let start = operations.len();
            for (lane, power) in [
                (0, multiplier.clone()),
                (1, &multiplier * &multiplier % source),
            ] {
                arithmetic
                    .emit_ququart_nested_radix(&power, lane, digit_bits, |operation| {
                        operations.push(operation);
                        Ok(())
                    })
                    .unwrap();
            }
            stages.push((multiplier.clone(), (start..operations.len()).collect()));
            let square = &multiplier * &multiplier % source;
            multiplier = &square * &square % source;
        }
        PreparedModularWork {
            source: source.clone(),
            digit_bits,
            operations,
            stages,
        }
    }

    #[test]
    fn complete_register_readout_matches_all_four_dense_period_bins() {
        for (quantile, expected) in [(1u8, 0u8), (3, 8), (5, 4), (7, 12)] {
            let mut circuit = WholeRegisterPhaseCircuit::prepare(
                BigUint::from(15u8),
                BigUint::from(2u8),
                Some(4),
            )
            .unwrap();
            let readout = circuit
                .fire(&BigUint::from(quantile), &BigUint::from(8u8))
                .unwrap();
            assert_eq!(readout.width, 4);
            assert_eq!(readout.denominator, BigUint::from(16u8));
            assert_eq!(readout.numerator, BigUint::from(expected));
            assert_eq!(readout.source, BigUint::from(15u8));
            assert_eq!(readout.base, BigUint::from(2u8));
            for wire in 0..readout.width {
                let zero = circuit.arena.zero();
                let wrong = circuit.arena.conditional_literals(zero,circuit.root,
                    &[(wire,!readout.numerator.bit(wire as u64))]);
                assert!(circuit.arena.mass_word(wrong).is_zero());
            }
            assert!(circuit.fire(&BigUint::zero(), &BigUint::one()).is_err());
        }
    }

    #[test]
    fn one_whole_register_sample_closes_the_source_bound_factor_pair() {
        let mut executor =
            WholeRegisterPhaseExecutor::prepare(BigUint::from(15u8), BigUint::from(2u8), Some(4))
                .unwrap();
        let shot = executor
            .measure(&BigUint::from(5u8), &BigUint::from(8u8))
            .unwrap();
        assert_eq!(shot.phase.numerator, BigUint::from(4u8));
        let closure = shot.closure.expect("phase winding closes N=15");
        let (p, q) = closure.factors();
        assert_eq!(p * q, BigUint::from(15u8));
    }

    #[test]
    fn terminal_readout_can_stop_before_factor_closure() {
        let mut executor =
            WholeRegisterPhaseExecutor::prepare(BigUint::from(15u8), BigUint::from(2u8), Some(4))
                .unwrap();
        let mut observed = None;
        let result =
            executor.measure_with_readout(&BigUint::from(5u8), &BigUint::from(8u8), |phase| {
                observed = Some(phase.clone());
                Err("terminal port consumed the phase".into())
            });
        assert!(matches!(result, Err(error) if error == "terminal port consumed the phase"));
        let phase = observed.unwrap();
        assert_eq!(phase.numerator, BigUint::from(4u8));
        assert_eq!(phase.denominator, BigUint::from(16u8));
    }

    #[test]
    fn prepared_rewirer_drives_the_single_full_register_sample() {
        let source = BigUint::from(15u8);
        let base = BigUint::from(2u8);
        let work = prepared_work(&source, &base, 1);
        let mut circuit = WholeRegisterPhaseCircuit::prepare_with_reentry_and_work(
            source,
            base,
            4,
            &[],
            Some(&work),
        )
        .unwrap();
        let readout = circuit
            .fire(&BigUint::from(5u8), &BigUint::from(8u8))
            .unwrap();
        assert_eq!(readout.numerator, BigUint::from(4u8));
        assert_eq!(readout.denominator, BigUint::from(16u8));
    }

    #[test]
    fn certified_clock_lane_order_preserves_every_joint_phase_bin() {
        for mode in 0..3 {
            for (quantile, expected) in [(1u8,0u8),(3,8),(5,4),(7,12)] {
                let source = BigUint::from(15u8);
                let base = BigUint::from(2u8);
                let mut prepared = WholeRegisterPhaseCircuit::prepare(source.clone(),base.clone(),Some(4)).unwrap();
                let record = prepared.prepared_record().unwrap();
                let mut circuit = WholeRegisterPhaseCircuit::from_prepared_record(
                    &record, prepared_work(&source,&base,1)).unwrap();
                let mut calls = 0;
                loop {
                    let reverse = mode==1 || mode==2 && calls%2==1;
                    calls += 1;
                    if circuit.advance_nested_work_clocked(1,1,reverse).unwrap() { break; }
                }
                assert!(circuit.streaming_work.as_ref().unwrap().pending_wires.is_empty());
                let readout = circuit.fire(&BigUint::from(quantile),&BigUint::from(8u8)).unwrap();
                assert_eq!(readout.numerator,BigUint::from(expected),"lane order {mode}");
                assert_eq!(readout.denominator,BigUint::from(16u8));
            }
        }
    }

    #[test]
    fn y_reentry_orientation_is_applied_before_the_joint_readout() {
        let mut executor = WholeRegisterPhaseExecutor::prepare_with_reentry(
            BigUint::from(15u8),
            BigUint::from(2u8),
            Some(4),
            1729,
            &[(true, false), (false, true)],
        )
        .unwrap();
        let shot = executor
            .measure(&BigUint::from(5u8), &BigUint::from(8u8))
            .unwrap();
        assert_eq!(shot.phase.numerator, BigUint::from(4u8));
        assert!(shot.closure.is_some());
        assert!(WholeRegisterPhaseExecutor::prepare_with_reentry(
            BigUint::from(15u8),
            BigUint::from(2u8),
            Some(4),
            1729,
            &[(true, false)],
        )
        .is_err());
    }
}
