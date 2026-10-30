//! N-only resident synthesis and the retained historical scheduling probe.
pub use crate::resident_glut::{ResidentGlut as FactorSynthesis, GlutReadout};
use alloc::{string::String, vec::Vec};
use num_bigint::BigUint;
use num_traits::{One, Zero};
use crate::whole_register_phase::{WholeRegisterPhaseExecutor, WholeRegisterPhaseReadout};
use crate::word_tape::WordTape;

pub struct RetainedArmProbe {
    source: BigUint,
    native: Option<vox_core::factor_synthesis::FactorSynthesis>,
    quantum: Option<WholeRegisterPhaseExecutor>,
    last_phase: Option<WholeRegisterPhaseReadout>,
    phase_readouts: u64,
}

pub struct FactorReturn {
    pub factors: Vec<BigUint>,
    pub producer: &'static str,
    pub phase: Option<WholeRegisterPhaseReadout>,
    pub native_membrane:Option<vox_core::glut_system::PreparedGlutMembrane>,
}

impl RetainedArmProbe {
    pub fn from_word(source: &WordTape) -> Result<Self,String> {
        Self::from_imasm_tape(&source.parity_cells())
    }

    /// Enter both retained arms from the source's canonical IMASM tape. The
    /// native synthesis keeps that tape directly; the phase executor derives
    /// its numeric backend value from the same bound tape.
    pub fn from_imasm_tape(tape: &[char]) -> Result<Self, String> {
        Self::from_imasm_tape_with_native(tape, true)
    }

    /// Prepare only the phase arm when the host membrane already owns and
    /// advances the native factor/product resident.
    pub fn phase_only_from_imasm_tape(tape: &[char]) -> Result<Self, String> {
        Self::from_imasm_tape_with_native(tape, false)
    }

    fn from_imasm_tape_with_native(tape: &[char], include_native: bool) -> Result<Self, String> {
        let decimal = vox_core::morphism_factor::dec_of(tape);
        let source = BigUint::parse_bytes(decimal.as_bytes(), 10)
            .ok_or_else(|| String::from("source IMASM tape is not a numeral"))?;
        if source <= BigUint::one() {
            return Err(String::from("source IMASM tape must encode N > 1"));
        }
        let native = if include_native {
            Some(vox_core::factor_synthesis::FactorSynthesis::new(tape)?)
        } else { None };
        Ok(Self { source, native, quantum: None, last_phase: None, phase_readouts: 0 })
    }

    pub fn joint_progress(&self) -> Option<(usize, usize, usize)> {
        self.quantum.as_ref()?.streaming_progress()
    }
    pub fn last_phase(&self) -> Option<&WholeRegisterPhaseReadout> {
        self.last_phase.as_ref()
    }
    pub fn circuit_braid_state(&self) -> Option<&vox_core::gfe_circuit_braid::GfeCircuitBraidState> {
        self.native.as_ref().map(|native| native.circuit_braid_state())
    }
    pub fn phase_readouts(&self) -> u64 { self.phase_readouts }
    /// Sole instance input. Base 2 and the whole-register measurement seed
    /// 1729 are fixed program constants; register width is derived from N.
    pub fn new(source: BigUint) -> Result<Self, String> {
        let tape = vox_core::morphism_factor::decimal_to_tape(&source.to_str_radix(10))
            .ok_or_else(|| String::from("source numeral conversion failed"))?;
        Self::from_imasm_tape(&tape)
    }

    fn verified(&self, p: BigUint, q: BigUint, producer: &'static str,
        phase: Option<WholeRegisterPhaseReadout>) -> Result<FactorReturn, String> {
        if p <= BigUint::one() || q <= BigUint::one() || &p * &q != self.source {
            return Err("synthesis return failed exact proper product verification".into());
        }
        Ok(FactorReturn { factors: alloc::vec![p, q], producer, phase,native_membrane:None })
    }

    pub fn advance_native(&mut self) -> Result<Option<FactorReturn>, String> {
        let Some(native) = self.native.as_mut() else { return Ok(None) };
        let Some(result) = native.advance()? else { return Ok(None) };
        let decode = |tape: &[char]| BigUint::parse_bytes(
            vox_core::morphism_factor::dec_of(tape).as_bytes(), 10)
            .ok_or_else(|| String::from("factor numeral conversion failed"));
        let mut returned=self.verified(decode(&result.p)?, decode(&result.q)?, result.producer, None)?;
        returned.native_membrane=result.membrane;
        Ok(Some(returned))
    }

    /// One joint Born readout. No digitwise phase execution or chosen quantile.
    /// The complete phase evidence and random stream survive between returns.
    pub fn advance_joint(&mut self) -> Result<Option<FactorReturn>, String> {
        if (&self.source % BigUint::from(2u8)).is_zero() {
            let p = BigUint::from(2u8);
            return self.verified(p.clone(), &self.source / p, "source-valuation", None).map(Some);
        }
        if self.quantum.is_none() {
            let mut executor = WholeRegisterPhaseExecutor::prepare(
                self.source.clone(), BigUint::from(2u8), None)?;
            executor.use_streaming_nested_work(8)?;
            self.quantum = Some(executor);
        }
        let Some(shot) = self.quantum.as_mut().unwrap().advance_streaming_next()? else {
            return Ok(None);
        };
        self.last_phase = Some(shot.phase.clone());
        self.phase_readouts = self.phase_readouts.saturating_add(1);
        if let Some(closure) = shot.closure {
            let (p, q) = closure.factors();
            return self.verified(p.clone(), q.clone(), "whole-register-phase", Some(shot.phase)).map(Some);
        }
        Ok(None)
    }

    /// Inclusive native return followed by a complete-register quantum return.
    /// Each component retains its state; the two timings must be recorded separately.
    pub fn advance(&mut self) -> Result<Option<FactorReturn>, String> {
        if let Some(result) = self.advance_native()? { return Ok(Some(result)) }
        self.advance_joint()
    }
}
