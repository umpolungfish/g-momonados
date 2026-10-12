//! N-only resident synthesis and the retained historical scheduling probe.
pub use crate::resident_glut::{ResidentGlut as FactorSynthesis, GlutReadout};
use alloc::{string::String, vec::Vec};
use num_bigint::BigUint;
use num_traits::{One, Zero};
use crate::whole_register_phase::{WholeRegisterPhaseExecutor, WholeRegisterPhaseReadout};

pub struct RetainedArmProbe {
    source: BigUint,
    native: vox_core::factor_synthesis::FactorSynthesis,
    quantum: Option<WholeRegisterPhaseExecutor>,
}

pub struct FactorReturn {
    pub factors: Vec<BigUint>,
    pub producer: &'static str,
    pub phase: Option<WholeRegisterPhaseReadout>,
}

impl RetainedArmProbe {
    pub fn joint_progress(&self) -> Option<(usize, usize, usize)> {
        self.quantum.as_ref()?.streaming_progress()
    }
    /// Sole instance input. Base 2 and the whole-register measurement seed
    /// 1729 are fixed program constants; register width is derived from N.
    pub fn new(source: BigUint) -> Result<Self, String> {
        let tape = vox_core::morphism_factor::decimal_to_tape(&source.to_str_radix(10))
            .ok_or_else(|| String::from("source numeral conversion failed"))?;
        let native = vox_core::factor_synthesis::FactorSynthesis::new(&tape)?;
        Ok(Self { source, native, quantum: None })
    }

    fn verified(&self, p: BigUint, q: BigUint, producer: &'static str,
        phase: Option<WholeRegisterPhaseReadout>) -> Result<FactorReturn, String> {
        if p <= BigUint::one() || q <= BigUint::one() || &p * &q != self.source {
            return Err("synthesis return failed exact proper product verification".into());
        }
        Ok(FactorReturn { factors: alloc::vec![p, q], producer, phase })
    }

    pub fn advance_native(&mut self) -> Result<Option<FactorReturn>, String> {
        let Some(result) = self.native.advance()? else { return Ok(None) };
        let decode = |tape: &[char]| BigUint::parse_bytes(
            vox_core::morphism_factor::dec_of(tape).as_bytes(), 10)
            .ok_or_else(|| String::from("factor numeral conversion failed"));
        self.verified(decode(&result.p)?, decode(&result.q)?, result.producer, None).map(Some)
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
