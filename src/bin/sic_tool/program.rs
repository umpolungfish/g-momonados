//! Native operations composed on one resident fusion carrier. No symbolic-word
//! allowlist: the caller binds each position to a composition of primitives.
use g_momonados::anyon_pair::{FibonacciPair, COMPUTATIONAL_CHANNELS};
use g_momonados::anyon_ququart::{FixedQuquartSic, QuquartCarrier, QuquartDigit};
use g_momonados::phase_unbraid::FixedComplex;
use num_bigint::{BigInt, BigUint};
use num_traits::{Signed, Zero};
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn field<'a>(v: &'a Value, key: &str) -> Result<&'a str, String> {
    v.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing string {key}"))
}
fn natural(v: &Value, key: &str) -> Result<BigUint, String> {
    BigUint::parse_bytes(field(v, key)?.as_bytes(), 10)
        .ok_or_else(|| format!("invalid natural {key}"))
}

/// Recompute a source-bound SIC certificate without executing its carrier.
pub fn reconstruct(input: &str) -> Result<Value, String> {
    let record: Value = serde_json::from_str(input).map_err(|e| e.to_string())?;
    let source = natural(&record, "source")?;
    if source.bits() < 128 { return Err("source must be at least 128 bits".into()); }
    let algebra = FibonacciPair::new(&source)?;
    let sic = FixedQuquartSic::new(algebra.format())?;
    let gram: [(BigInt,BigInt);16] = record["gram"].as_array().ok_or("missing control Gram")?
        .iter().map(|entry| {
            let pair = entry.as_array().ok_or("Gram entry must be a complex pair")?;
            if pair.len() != 2 { return Err("Gram entry must have two coordinates".into()); }
            let integer = |value: &Value| value.as_str().ok_or("Gram coordinate must be a decimal string")?
                .parse::<BigInt>().map_err(|_| "invalid Gram integer".to_string());
            Ok((integer(&pair[0])?, integer(&pair[1])?))
        }).collect::<Result<Vec<_>,String>>()?.try_into().map_err(|_| "Gram must contain sixteen complex entries")?;
    let masses: [BigUint;16] = record["masses"].as_array().ok_or("missing projector masses")?
        .iter().map(|value| value.as_str().ok_or("mass must be a decimal string")?
            .parse::<BigUint>().map_err(|_| "invalid projector mass".to_string()))
        .collect::<Result<Vec<_>,String>>()?.try_into().map_err(|_| "SIC must contain sixteen masses")?;
    if sic.gram_masses(&gram)? != masses { return Err("SIC masses differ from the supplied control Gram".into()); }
    let certificate = sic.certify_gram_frame(&gram, &masses)?;
    Ok(json!({"source":source.to_string(),"fixed_point_bits":algebra.format().w_bits,
        "gram_convention":"inner_product_work_row_work_col",
        "recovered_gram":certificate.recovered.iter().map(|(re,im)| [re.to_string(),im.to_string()]).collect::<Vec<_>>(),
        "maximum_residual":certificate.maximum_residual.to_string(),
        "tolerance":certificate.tolerance.to_string()}))
}
fn state(carrier: &QuquartCarrier) -> Value {
    json!(carrier
        .amplitudes()
        .iter()
        .map(|z| [z.re.to_string(), z.im.to_string()])
        .collect::<Vec<_>>())
}
fn residual(carrier: &QuquartCarrier, before: &[FixedComplex; 5], scale: &BigInt) -> Value {
    let maximum = carrier
        .amplitudes()
        .iter()
        .zip(before)
        .map(|(a, b)| (&a.re - &b.re).abs().max((&a.im - &b.im).abs()))
        .max()
        .unwrap();
    json!({"metric":"maximum_absolute_real_or_imaginary_component", "numerator":maximum.to_string(),
           "denominator":scale.to_string(), "source_equal":maximum.is_zero()})
}

pub fn run(input: &str) -> Result<Value, String> {
    let plan: Value = serde_json::from_str(input).map_err(|e| e.to_string())?;
    let source = natural(&plan, "source")?;
    if source.bits() < 128 {
        return Err("source must be at least 128 bits".into());
    }
    let algebra = FibonacciPair::new(&source)?;
    let sic = FixedQuquartSic::new(algebra.format())?;
    let scale = algebra.format().scale();
    let digit = plan
        .get("digit")
        .and_then(Value::as_u64)
        .ok_or("missing source digit")?;
    if digit > 3 {
        return Err("source digit must be 0 through 3".into());
    }
    let mut carrier = QuquartCarrier::basis(&algebra, QuquartDigit::try_from(digit as u8)?);
    let initial = carrier.amplitudes().clone();
    let mut returns: Vec<(Vec<i32>, [FixedComplex; 5])> = Vec::new();
    let mut splits: Vec<(String, [FixedComplex; 5])> = Vec::new();
    let mut evidence: BTreeMap<String, Value> = BTreeMap::new();
    let mut snapshots: BTreeMap<String, Value> = BTreeMap::new();
    let mut events = Vec::new();
    let steps = plan
        .get("steps")
        .and_then(Value::as_array)
        .ok_or("missing steps")?;
    for (i, step) in steps.iter().enumerate() {
        if step.get("i").and_then(Value::as_u64) != Some(i as u64) {
            return Err("unordered step index".into());
        }
        let symbol = field(step, "symbol")?;
        if symbol.chars().count() != 1 || !"⊢⊣≻≺⋈⊙∈∋⊤⊥⊞⊡".contains(symbol) {
            return Err("foreign symbol".into());
        }
        let actions = step
            .get("actions")
            .and_then(Value::as_array)
            .ok_or("missing actions")?;
        if actions.is_empty() {
            return Err("every position requires a bound operation".into());
        }
        for action in actions {
            let kind = field(action, "kind")?;
            let mut event = json!({"i":i,"symbol":symbol,"kind":kind});
            match kind {
                "retain" => {
                    event["state"] = state(&carrier);
                }
                "fourier" => {
                    let inverse = action["inverse"].as_bool().ok_or("Fourier action requires an inverse flag")?;
                    carrier.fourier_target(inverse);
                    event["inverse"] = json!(inverse);
                    event["state"] = state(&carrier);
                }
                "exchange" => {
                    let gs = action
                        .get("generators")
                        .and_then(Value::as_array)
                        .ok_or("missing generators")?;
                    if gs.is_empty() {
                        return Err("empty exchange composition".into());
                    }
                    let mut word = Vec::new();
                    for g in gs {
                        let n = g.as_i64().ok_or("invalid signed generator")?;
                        if n == 0 || !(-5..=5).contains(&n) {
                            return Err("generator outside six-strand carrier".into());
                        }
                        word.push(n as i32);
                    }
                    let before = carrier.amplitudes().clone();
                    for &g in &word {
                        carrier.exchange(&algebra, g)?;
                    }
                    event["generators"] = json!(word);
                    returns.push((word, before));
                }
                "inverse" => {
                    let (word, before) = returns.pop().ok_or("no retained operation to invert")?;
                    let inverse: Vec<i32> = word.iter().rev().map(|g| -g).collect();
                    for &g in &inverse {
                        carrier.exchange(&algebra, g)?;
                    }
                    event["generators"] = json!(inverse);
                    event["return"] = residual(&carrier, &before, &scale);
                }
                "split" => {
                    let id = field(action, "id")?.to_string();
                    if splits.iter().any(|(key, _)| key == &id) {
                        return Err("duplicate open split identifier".into());
                    }
                    splits.push((id.clone(), carrier.amplitudes().clone()));
                    event["id"] = json!(id);
                    event["sic_masses"] = json!(carrier.sic_masses(&sic)?.map(|m| m.to_string()));
                    event["retained_phase_and_leakage"] = state(&carrier);
                }
                "rejoin" => {
                    let id = field(action, "id")?;
                    let (key, before) = splits.pop().ok_or("no retained split")?;
                    if key != id {
                        return Err("rejoin must name the retained inner frame".into());
                    }
                    let a = carrier.amplitudes();
                    let gram = core::array::from_fn(|k| {
                        let x = &a[COMPUTATIONAL_CHANNELS[k / 4]];
                        let y = &a[COMPUTATIONAL_CHANNELS[k % 4]];
                        (&x.re * &y.re + &x.im * &y.im, &x.re * &y.im - &x.im * &y.re)
                    });
                    let masses = sic.gram_masses(&gram)?;
                    let reconstruction = sic.certify_gram_frame(&gram, &masses)?;
                    event["id"] = json!(id);
                    event["gram_dual_verified"] = json!(true);
                    event["gram_convention"] = json!("inner_product_work_row_work_col");
                    event["gram"] = json!(gram.iter().map(|(re,im)| [re.to_string(), im.to_string()]).collect::<Vec<_>>());
                    event["masses"] = json!(masses.map(|mass| mass.to_string()));
                    event["recovered_gram"] = json!(reconstruction.recovered.iter()
                        .map(|(re,im)| [re.to_string(), im.to_string()]).collect::<Vec<_>>());
                    event["reconstruction_residual"] = json!(reconstruction.maximum_residual.to_string());
                    event["reconstruction_tolerance"] = json!(reconstruction.tolerance.to_string());
                    event["retained_phase_and_leakage"] = state(&carrier);
                    event["source_return"] = residual(&carrier, &before, &scale);
                }
                "sic" => {
                    event["sic_masses"] = json!(carrier.sic_masses(&sic)?.map(|m| m.to_string()));
                }
                "evidence" => {
                    let p = field(action, "proposition")?;
                    let axis = field(action, "axis")?;
                    if axis != "support" && axis != "refutation" {
                        return Err("unknown evidence coordinate".into());
                    }
                    let witness = field(action, "witness")?;
                    let outcome = action
                        .get("outcome")
                        .and_then(Value::as_u64)
                        .ok_or("missing SIC outcome")?;
                    if outcome > 16 {
                        return Err("SIC outcome outside retained carrier/frame".into());
                    }
                    let numerator = natural(action, "numerator")?;
                    let denominator = natural(action, "denominator")?;
                    if denominator.is_zero() || numerator > denominator {
                        return Err("invalid evidence threshold".into());
                    }
                    let masses = carrier.sic_masses(&sic)?;
                    let total: BigUint = masses.iter().cloned().sum();
                    if total.is_zero() {
                        return Err("zero carrier mass".into());
                    }
                    let accepted = &masses[outcome as usize] * &denominator >= &total * &numerator;
                    let record = json!({"accepted":accepted,"witness":witness,"outcome":outcome,
                                      "mass":masses[outcome as usize].to_string(),"total":total.to_string(),
                                      "threshold":[numerator.to_string(),denominator.to_string()]});
                    let coordinates = evidence.entry(p.to_string()).or_insert_with(|| json!({}));
                    // Preserve every attributed observation; do not overwrite history.
                    if coordinates.get(axis).is_none() {
                        coordinates[axis] = json!([]);
                    }
                    coordinates[axis]
                        .as_array_mut()
                        .unwrap()
                        .push(record.clone());
                    event["proposition"] = json!(p);
                    event["axis"] = json!(axis);
                    event["observation"] = record;
                }
                "engage" => {
                    let p = field(action, "proposition")?;
                    let coordinates = evidence
                        .get(p)
                        .ok_or("engagement requires measured evidence")?;
                    if coordinates.get("support").is_none()
                        || coordinates.get("refutation").is_none()
                    {
                        return Err(
                            "engagement requires both attributed evidence coordinates".into()
                        );
                    }
                    event["proposition"] = json!(p);
                    event["evidence"] = coordinates.clone();
                }
                "latch" => {
                    let id = field(action, "id")?;
                    if snapshots.contains_key(id) {
                        return Err("latch identifier already bound".into());
                    }
                    let snapshot = json!({"state":state(&carrier),"evidence":evidence});
                    snapshots.insert(id.to_string(), snapshot.clone());
                    event["id"] = json!(id);
                    event["snapshot"] = snapshot;
                }
                _ => return Err(format!("unknown native primitive {kind}")),
            }
            events.push(event);
        }
    }
    if !splits.is_empty() {
        return Err("unrejoined native frame".into());
    }
    Ok(
        json!({"backend":"anyon-composition","source":source.to_string(),"fixed_point_bits":algebra.format().w_bits,
              "outcome_kernel_masks":(0..16).map(|mask| g_momonados::sic::SixteenOutcome::new(mask).unwrap().kernel_mask()).collect::<Vec<_>>(),
              "events":events,"final_state":state(&carrier),"source_return":residual(&carrier,&initial,&scale),
              "sic_masses":carrier.sic_masses(&sic)?.map(|m|m.to_string()),"evidence":evidence,"latches":snapshots,
              "readout":"non-destructive analysis; retained coherent phase and leakage"}),
    )
}
