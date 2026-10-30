//! N-only horn-torus carrier using the existing y_divisor factor path.
//! The resident produces the divisor, retains its SIC/native state, and sends
//! that internally produced divisor through the same Lean Y return.
use alloc::{format, string::String, vec::Vec};
use crate::{belnap::B4, word_tape::WordTape};
use num_bigint::BigUint;
use std::time::{Duration, Instant};
use serde_json::{json, Value};

pub struct HornTorusCarrier {
    source: WordTape,
    banks: [WordTape; 2],
    widths: Vec<(usize, usize)>,
    order: vox_core::ordering::Permutation,
    lane_order: vox_core::ordering::Permutation,
    factors: Option<[WordTape; 2]>,
    execution_evidence: B4,
    factor_evidence: [B4; 2],
    product_evidence: B4,
    steps: Vec<Value>,
    batches: Vec<Value>,
    next: usize,
    round: u64,
    resident: Option<crate::factor_synthesis::FactorSynthesis>,
    resident_return: Option<crate::factor_synthesis::GlutReadout>,
    producer: &'static str,
    lean_y_return: Option<String>,
    preparation_seconds: f64,
    production_seconds: f64,
    inverse_return_verified: bool,
    paired_boundary: bool,
    producing_slot: Option<usize>,
}

impl HornTorusCarrier {
    pub fn prepare(raw: &str) -> Result<Self, String> {
        let clock = Instant::now();
        let source = WordTape::from_canonical_word(raw).or_else(|| {
            BigUint::parse_bytes(raw.trim().as_bytes(), 10).map(|n| WordTape::from_biguint(&n))
        }).ok_or("horn carrier expects N alone as a decimal or canonical word")?;
        if source.bit_len() < 128 || !source.is_odd() {
            return Err("horn carrier requires an odd source of at least 128 bits".into());
        }
        let widths = vox_core::glut_system::source_factor_geometries(source.bit_len());
        let order = vox_core::ordering::selected("separating", widths.len())?;
        let lane_order = vox_core::ordering::selected("lanes", 2)?;
        let (left, right) = source.deinterlace();
        let banks = [left, right];
        if banks[0].interlace(&banks[1]) != source {
            return Err("source-bank inverse failed before production".into());
        }
        let resident = crate::factor_synthesis::FactorSynthesis::new(source.to_biguint())?;
        let source_word = source.as_word().to_string();
        Ok(Self {
            source, banks, widths, order, lane_order, factors: None,
            execution_evidence: B4::N, factor_evidence: [B4::N; 2], product_evidence: B4::N,
            steps: vec![json!({"u":0,"mark":"⊢","source_word":source_word}),
                json!({"u":1,"mark":"⊙","state":"source-bound"}),
                json!({"u":2,"mark":"⊣","state":"open"}),
                json!({"u":3,"mark":"∈","state":"source-banks-retained"})],
            batches: Vec::new(), next: 0, round: 0, resident: Some(resident),
            resident_return: None, producer: "resident-y-pending", lean_y_return: None,
            preparation_seconds: clock.elapsed().as_secs_f64(), production_seconds: 0.0,
            inverse_return_verified: true, paired_boundary: false, producing_slot: None,
        })
    }

    /// One selected source-bound operation. Both candidate arms are deposited
    /// into this carrier before inverse recovery, evidence fusion and fixation.
    pub fn advance(&mut self) -> Result<bool, String> {
        if self.factors.is_some() { return Ok(true); }
        let physical_slot = self.next;
        let canonical_slot = self.order.indices[physical_slot];
        let inverse_slot = self.order.inverse().indices[canonical_slot];
        if inverse_slot != physical_slot { return Err("executed batch lost its inverse address".into()); }
        let input = self.banks[0].interlace(&self.banks[1]);
        if input != self.source { return Err("source changed inside the producing frame".into()); }
        let Some(resident) = self.resident.take() else { return Ok(false) };
        let clock = Instant::now();
        std::eprintln!("HORN_PRODUCTION route=y_divisor-factor source_bits={} resident=retained", input.bit_len());
        let mut candidate = match resident.readout(|_, _| {}) {
            Ok(Some(result)) => {
                if !result.gamma_delta_closed || !result.product_closed {
                    return Err("resident Y path failed its retained product boundary".into());
                }
                let divisor = WordTape::from_biguint(&result.divisor);
                self.producer = result.producer;
                self.resident_return = Some(result);
                Ok(Some(divisor))
            }
            Ok(None) => Ok(None),
            Err(error) => Err(error),
        };
        let producer_seconds = clock.elapsed().as_secs_f64();
        let mut y_seconds = 0.0;
        if let Ok(Some(divisor)) = &candidate {
            let y_clock = Instant::now();
            let args = vec!["divisor".into(), "from".into(), self.source.to_biguint().to_string(), divisor.to_biguint().to_string()];
            let mut bytes = Vec::new();
            let code = crate::lean_kernel::run_to(&args, |output| bytes.extend_from_slice(output));
            let report = String::from_utf8_lossy(&bytes).into_owned();
            let certified = code == 0 && report.contains("hasT := true")
                && report.contains(&format!("factor={} ", divisor.to_biguint()));
            self.lean_y_return = Some(report);
            y_seconds = y_clock.elapsed().as_secs_f64();
            if !certified { candidate = Err("internally produced divisor failed the Lean Y return".into()); }
        }
        let seconds = clock.elapsed().as_secs_f64();
        self.production_seconds += seconds;
        let mut batch = json!({"physical_slot":physical_slot,"canonical_slot":canonical_slot,
            "inverse_slot":inverse_slot,"seconds":seconds,"producer_seconds":producer_seconds,
            "lean_y_seconds":y_seconds,"producer":self.producer,"route":"y_divisor-factor"});
        match candidate {
            Ok(candidate) => {
                self.execution_evidence = self.execution_evidence.join(B4::T);
                batch["factor_emitted"] = json!(candidate.is_some());
                if let Some(divisor) = candidate {
                    // One emitted divisor suffices. Derive the other arm on
                    // this same source before exposing either at the return.
                    let (quotient, remainder) = self.source.divmod(&divisor).ok_or("zero producing divisor")?;
                    let proper = !divisor.is_one() && self.source.gt(&divisor)
                        && !quotient.is_one() && self.source.gt(&quotient) && remainder.is_zero();
                    self.factor_evidence[0] = self.factor_evidence[0].join(if proper { B4::T } else { B4::F });
                    self.factor_evidence[1] = self.factor_evidence[1].join(if proper { B4::T } else { B4::F });
                    let logical = [divisor, quotient];
                    let physical = self.lane_order.apply(&logical)?;
                    let returned = self.lane_order.inverse().apply(&physical)?;
                    self.inverse_return_verified &= returned == logical;
                    let product = returned[0].mul(&returned[1]);
                    let exact = product == self.source;
                    self.product_evidence = self.product_evidence.join(if exact { B4::T } else { B4::F });
                    let packed = returned[0].interlace(&returned[1]);
                    let (p_back, q_back) = packed.deinterlace();
                    self.paired_boundary = p_back == returned[0] && q_back == returned[1]
                        && p_back.interlace(&q_back) == packed && p_back.mul(&q_back) == self.source;
                    self.steps.extend([
                        json!({"u":8,"mark":"⊤","p_word":returned[0].as_word(),"q_word":returned[1].as_word()}),
                        json!({"u":10,"mark":"⋈","packed_word":packed.as_word()}),
                        json!({"u":11,"mark":"≺","inverse_return":self.inverse_return_verified}),
                        json!({"u":12,"mark":"⊥/⊤","product_word":product.as_word(),"remainder_word":remainder.as_word()}),
                    ]);
                    if !proper || !exact || !self.paired_boundary || !self.inverse_return_verified {
                        batch["return_error"] = json!("emitted candidate failed source-bound return");
                        self.batches.push(batch);
                        return Err("producing carrier retained a rejected candidate".into());
                    }
                    self.factors = Some([returned[0].clone(), returned[1].clone()]);
                    self.producing_slot = Some(physical_slot);
                    self.steps.extend([
                        json!({"u":13,"mark":"∋","evidence":self.fused().name()}),
                        json!({"u":14,"mark":"⊙","source_word":self.source.as_word()}),
                        json!({"u":15,"mark":"⊡⊣","state":"factor-produced"}),
                    ]);
                }
            }
            Err(error) => {
                self.execution_evidence = self.execution_evidence.join(B4::F);
                batch["execution_error"] = json!(error);
                self.batches.push(batch);
                return Err(format!("horn producing operation failed: {error}"));
            }
        }
        self.batches.push(batch);
        self.next += 1;
        if self.next == self.widths.len() { self.next = 0; self.round += 1; }
        Ok(self.factors.is_some())
    }

    fn fused(&self) -> B4 {
        self.factor_evidence[0].join(self.factor_evidence[1])
            .join(self.product_evidence).join(self.execution_evidence)
    }

    pub fn readout(&self) -> Value {
        let number = |w: &WordTape| w.to_biguint().to_string();
        let p = self.factors.as_ref().map(|arms| number(&arms[0]));
        let q = self.factors.as_ref().map(|arms| number(&arms[1]));
        let product = self.factors.as_ref().map(|arms| number(&arms[0].mul(&arms[1])));
        let resident_return = self.resident_return.as_ref().map(|r|json!({
                "source_word":r.source_word(),"producer":r.producer,
                "boundary_word":r.boundary_word,"gamma_delta_closed":r.gamma_delta_closed,
                "product_closed":r.product_closed,"banked_supports":r.banked_supports(),
                "sic_residual":r.sic.reconstruction.maximum_residual.to_string(),
                "sic_tolerance":r.sic.reconstruction.tolerance.to_string(),
                "sic_signed_residuals":r.sic.signed_residuals.iter().map(|(a,b)|[a.to_string(),b.to_string()]).collect::<Vec<_>>(),
                "sic_rational_denominator":r.sic.rational_denominator.to_string(),
                "sic_rational_signed_residuals":r.sic.rational_signed_residuals.iter().map(|(a,b)|[a.to_string(),b.to_string()]).collect::<Vec<_>>(),
                "belnap_relation_conflicts":r.retained_resident().and_then(|v|v.retained_native_membrane()).map(|m|
                    m.belnap_relation_conflicts().iter().map(|c|json!({"evidence":c.evidence.name(),"depth":c.depth,"decision":c.decision,
                        "support":c.support.iter().map(|(cell,value)|json!([cell,value.name()])).collect::<Vec<_>>(),
                        "implication":c.implication.iter().map(|(cell,value)|json!([cell,value.name()])).collect::<Vec<_>>()})).collect::<Vec<_>>()),
                "belnap_conflicts": r.retained_resident().and_then(|v|v.retained_native_membrane()).map(|m|
                    m.belnap_conflicts().iter().map(|c|json!({"cell":c.cell,"evidence":c.evidence.name(),
                        "previous":c.previous.name(),"incoming":c.incoming.name(),"depth":c.depth,
                        "previous_reason":c.previous_reason.as_ref().map(|r|r.iter().map(|(cell,value)|json!([cell,value.name()])).collect::<Vec<_>>()),
                        "incoming_reason":c.incoming_reason.as_ref().map(|r|r.iter().map(|(cell,value)|json!([cell,value.name()])).collect::<Vec<_>>())})).collect::<Vec<_>>()),
                "retained_native_membrane":r.retained_resident().is_some_and(|v|v.retained_native_membrane().is_some())}));
        json!({"schema":"horn-torus-carrier-v1","cell_carrier":"Belnap FOUR","source":number(&self.source),
            "source_word":self.source.as_word(),"p":p,"q":q,"derived_quotient":q,
            "p_word":self.factors.as_ref().map(|arms|arms[0].as_word()),
            "q_word":self.factors.as_ref().map(|arms|arms[1].as_word()),
            "source_banks":[self.banks[0].as_word(),self.banks[1].as_word()],
            "p_evidence":self.factor_evidence[0].name(),"q_evidence":self.factor_evidence[1].name(),
            "arm_join":self.factor_evidence[0].join(self.factor_evidence[1]).name(),
            "product_evidence":self.product_evidence.name(),"core":self.product_evidence.name(),
            "execution_evidence":self.execution_evidence.name(),"fused_evidence":self.fused().name(),
            "proper_divisor":self.factors.is_some(),"remainder":self.factors.as_ref().map(|_|"0"),
            "product":product,"paired_boundary":self.paired_boundary,
            "selected_order":self.order.indices,"inverse_order":self.order.inverse().indices,
            "factor_lane_order":self.lane_order.indices,"factor_lane_inverse":self.lane_order.inverse().indices,
            "inverse_return_verified":self.inverse_return_verified,"producing_slot":self.producing_slot,
            "boundary":if self.factors.is_some(){"closed"}else{"held"},
            "producer":self.producer,"route":"y_divisor-factor","preparation_seconds":self.preparation_seconds,
            "production_seconds":self.production_seconds,"operations":self.steps,"batches":self.batches,
            "lean_y_return":self.lean_y_return,
            "resident_return":resident_return,
            "continuation":{"next_physical_slot":self.next,"round":self.round,"resident_pending":self.resident.is_some()}})
    }
}

pub fn run(args: &[&str]) -> Result<(String, bool), String> {
    let source = args.first().ok_or("horn_factor <N-only decimal-or-word> [seconds=300]")?;
    let seconds = args.get(1).map_or(Ok(300u64), |raw| raw.parse().map_err(|_| "invalid duration"))?;
    let mut carrier = HornTorusCarrier::prepare(source)?;
    let started = Instant::now();
    let boundary = Duration::from_secs(seconds);
    let mut error = None;
    while started.elapsed() < boundary {
        match carrier.advance() {
            Ok(true) => break,
            Ok(false) => { if carrier.resident.is_none() { break; } },
            Err(message) => { error = Some(message); break; },
        }
    }
    let mut readout = carrier.readout();
    readout["trigger_to_readout_seconds"] = json!(started.elapsed().as_secs_f64());
    readout["execution_limit_seconds"] = json!(seconds);
    if let Some(error) = error { readout["error"] = json!(error); }
    let produced = carrier.factors.is_some();
    let mut output = String::new();
    if let Some(arms) = &carrier.factors {
        output.push_str(&format!("p = {}\nq = {}\np × q = N: true\n", arms[0].to_biguint(), arms[1].to_biguint()));
    }
    output.push_str(&format!("HORN_TORUS_RETURN {readout}\n"));
    Ok((output, produced))
}
