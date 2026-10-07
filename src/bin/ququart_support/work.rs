use super::numeral;
use g_momonados::{
    godel_calculus::{encode_cell_binary, Nat},
    reversible_modular::{ControlledX, ModularMultiply, NestedOperation},
};
use num_bigint::BigUint;
use num_traits::{ToPrimitive, Zero};
use serde_json::{json, Value};
fn word(n: &BigUint) -> String {
    encode_cell_binary(&Nat::from_bits_le(
        (0..n.bits()).map(|b| n.bit(b)).collect(),
    ))
}
fn index(n: usize) -> String {
    word(&BigUint::from(n))
}
fn wires(w: &[usize]) -> Vec<String> {
    w.iter().map(|&v| index(v)).collect()
}
fn controls(c: &[(usize, bool)]) -> (Vec<String>, Vec<String>) {
    (
        c.iter().map(|&(w, _)| index(w)).collect(),
        c.iter().map(|&(_, v)| index(usize::from(v))).collect(),
    )
}
fn compile_legacy(p: &Value) -> Result<Value, String> {
    let n = numeral(p["source_word"].as_str().ok_or("missing source")?)?;
    let radix = g_momonados::ququart_factor::power_of_two_radix_word(
        p["radix_word"].as_str().ok_or("missing radix")?,
    )?;
    let bits = radix.bits_le().len() - 1;
    let arithmetic = ModularMultiply::new(&n)?;
    let powers = p["prepared_operator"]["controlled_power_words"]
        .as_array()
        .ok_or("missing powers")?;
    let mut stages = Vec::new();
    for power in powers {
        let multiplier = numeral(power.as_str().ok_or("nonword power")?)?;
        let mut register = Vec::new();
        let mut operations = Vec::new();
        for (lane, value) in [(0, multiplier.clone()), (1, &multiplier * &multiplier % &n)] {
            arithmetic.emit_ququart_nested_radix(&value,lane,bits,|operation| {
                operations.push(match operation {
                    NestedOperation::ModularAdd {register:r,digit,value,modulus,controls:c}=> {
                        if modulus!=n || (!register.is_empty() && register!=r) {return Err("inconsistent work template".into());}
                        register=r; let (control_words,polarity_words)=controls(&c);
                        json!({"modular_add":{"digit_words":wires(&digit),"value_word":word(&value),"control_words":control_words,"polarity_words":polarity_words}})
                    },
                    NestedOperation::Toggle(g)=> {let (control_words,polarity_words)=controls(&g.controls);
                        json!({"toggle":{"target_word":index(g.target),"control_words":control_words,"polarity_words":polarity_words}})},
                    _=>return Err("unexpected prepared operation".into()),
                }); Ok(())
            })?;
        }
        stages.push(json!({"multiplier_word":power,"register_words":wires(&register),"operations":operations}));
    }
    Ok(Value::Array(stages))
}
pub fn compile(p: &Value) -> Result<Value, String> {
    // Retained older artifacts are checked in their original wire format.
    if p.get("prepared_work").is_some_and(Value::is_array) {
        return compile_legacy(p);
    }

    // Pool operations as they are generated. Building the complete legacy
    // stage array first duplicates every wire word before the pool is made;
    // at larger source widths that temporary representation dwarfs the final
    // prepared work and can exhaust the preparation process.
    let n = numeral(p["source_word"].as_str().ok_or("missing source")?)?;
    let radix = g_momonados::ququart_factor::power_of_two_radix_word(
        p["radix_word"].as_str().ok_or("missing radix")?,
    )?;
    let bits = radix.bits_le().len() - 1;
    let arithmetic = ModularMultiply::new(&n)?;
    let powers = p["prepared_operator"]["controlled_power_words"]
        .as_array()
        .ok_or("missing powers")?;
    let mut pool = Vec::new();
    let mut known = std::collections::HashMap::new();
    let mut register = Value::Array(Vec::new());
    let mut stages = Vec::new();

    for power in powers {
        let multiplier = numeral(power.as_str().ok_or("nonword power")?)?;
        let mut stage_references = Vec::new();
        let mut stage_register = Vec::new();
        for (lane, value) in [
            (0, multiplier.clone()),
            (1, &multiplier * &multiplier % &n),
        ] {
            arithmetic.emit_ququart_nested_radix(&value, lane, bits, |operation| {
                let encoded = match operation {
                    NestedOperation::ModularAdd {
                        register: register_wires,
                        digit,
                        value,
                        modulus,
                        controls: literals,
                    } => {
                        if modulus != n
                            || (!stage_register.is_empty() && stage_register != register_wires)
                        {
                            return Err("inconsistent work template".into());
                        }
                        stage_register = register_wires;
                        let (control_words, polarity_words) = controls(&literals);
                        json!({"modular_add": {
                            "digit_words": wires(&digit),
                            "value_word": word(&value),
                            "control_words": control_words,
                            "polarity_words": polarity_words
                        }})
                    }
                    NestedOperation::Toggle(gate) => {
                        let (control_words, polarity_words) = controls(&gate.controls);
                        json!({"toggle": {
                            "target_word": index(gate.target),
                            "control_words": control_words,
                            "polarity_words": polarity_words
                        }})
                    }
                    _ => return Err("unexpected prepared operation".into()),
                };

                let key = encoded.to_string();
                let id = if let Some(&id) = known.get(&key) {
                    id
                } else {
                    let id = pool.len();
                    pool.push(encoded);
                    known.insert(key, id);
                    id
                };
                stage_references.push(index(id));
                Ok(())
            })?;
        }
        let stage_register_words = wires(&stage_register);
        if !stage_register_words.is_empty() {
            if register.as_array().unwrap().is_empty() {
                register = Value::Array(stage_register_words.into_iter().map(Value::String).collect());
            } else if register != json!(stage_register_words) {
                return Err("work pool has inconsistent workspace".into());
            }
        }
        stages.push(json!({
            "multiplier_word": power,
            "operation_words": stage_references
        }));
    }
    Ok(json!({"register_words":register,"operations":pool,"stages":stages}))
}
fn number(v: &Value) -> Result<BigUint, String> {
    numeral(v.as_str().ok_or("work value must be a word")?)
}
// Wire and polarity words repeat throughout the prepared circuit. Validate
// each distinct word once; cache only its decoded index, never a work result.
struct WireReader<'a> {
    indices: std::collections::HashMap<&'a str, usize>,
}
impl<'a> WireReader<'a> {
    fn wire(&mut self, v: &'a Value) -> Result<usize, String> {
        let text = v.as_str().ok_or("work wire must be a word")?;
        if let Some(&index) = self.indices.get(text) {
            return Ok(index);
        }
        let index = numeral(text)?
            .to_usize()
            .ok_or("work wire exceeds indexing")?;
        self.indices.insert(text, index);
        Ok(index)
    }
    fn wires(&mut self, v: &'a Value) -> Result<Vec<usize>, String> {
        v.as_array()
            .ok_or("missing wire words")?
            .iter()
            .map(|v| self.wire(v))
            .collect()
    }
    fn controls(&mut self, v: &'a Value) -> Result<Vec<(usize, bool)>, String> {
        let w = self.wires(&v["control_words"])?;
        let p = self.wires(&v["polarity_words"])?;
        if w.len() != p.len() || p.iter().any(|&x| x > 1) {
            return Err("malformed literal controls".into());
        }
        Ok(w.into_iter().zip(p.into_iter().map(|x| x == 1)).collect())
    }
}
fn decode_legacy(p: &Value) -> Result<Vec<(BigUint, Vec<NestedOperation>)>, String> {
    let n = number(&p["source_word"])?;
    let mut stages = Vec::new();
    let mut reader = WireReader {
        indices: std::collections::HashMap::new(),
    };
    for stage in p["prepared_work"]
        .as_array()
        .ok_or("missing baked work boundaries")?
    {
        let register = reader.wires(&stage["register_words"])?;
        let mut ops = Vec::new();
        for op in stage["operations"]
            .as_array()
            .ok_or("missing work operations")?
        {
            ops.push(if let Some(v) = op.get("modular_add") {
                let value = number(&v["value_word"])?;
                if value >= n && !value.is_zero() {
                    return Err("work translation outside source".into());
                }
                NestedOperation::ModularAdd {
                    register: register.clone(),
                    digit: reader.wires(&v["digit_words"])?,
                    value,
                    modulus: n.clone(),
                    controls: reader.controls(v)?,
                }
            } else if let Some(v) = op.get("toggle") {
                NestedOperation::Toggle(ControlledX {
                    target: reader.wire(&v["target_word"])?,
                    controls: reader.controls(v)?,
                })
            } else {
                return Err("unknown baked work operation".into());
            });
        }
        stages.push((number(&stage["multiplier_word"])?, ops));
    }
    Ok(stages)
}

pub fn decode(p: &Value) -> Result<g_momonados::ququart_folded_work::PreparedModularWork, String> {
    use g_momonados::ququart_folded_work::PreparedModularWork;
    let source = number(&p["source_word"])?;
    let radix = g_momonados::ququart_factor::power_of_two_radix_word(
        p["radix_word"].as_str().ok_or("missing work radix")?)?;
    let digit_bits = radix.bits_le().len()-1;
    if p["prepared_work"].is_array() {
        let mut operations = Vec::new();
        let mut stages = Vec::new();
        for (power, ops) in decode_legacy(p)? {
            let start = operations.len();
            operations.extend(ops);
            stages.push((power, (start..operations.len()).collect()));
        }
        let prepared = PreparedModularWork { source, digit_bits, operations, stages };
        validate_recovery(p,&prepared)?;
        return Ok(prepared);
    }
    let work = &p["prepared_work"];
    // Decode the pool through the same operation reader, once. References are
    // numeral words and are checked before they can select any operation.
    let temporary = json!({"source_word":p["source_word"],"prepared_work":[{
        "multiplier_word":word(&BigUint::from(0u8)),
        "register_words":work["register_words"],"operations":work["operations"]}]});
    let mut decoded = decode_legacy(&temporary)?;
    let operations = decoded.pop().ok_or("missing work pool")?.1;
    let mut reader = WireReader {
        indices: std::collections::HashMap::new(),
    };
    let mut stages = Vec::new();
    for stage in work["stages"]
        .as_array()
        .ok_or("missing pooled work stages")?
    {
        let references = reader.wires(&stage["operation_words"])?;
        if references.iter().any(|&id| id >= operations.len()) {
            return Err("prepared operation reference outside pool".into());
        }
        stages.push((number(&stage["multiplier_word"])?, references));
    }
    let prepared = PreparedModularWork { source, digit_bits, operations, stages };
    validate_recovery(p,&prepared)?;
    Ok(prepared)
}

/// Compare every reconstructed operation, in order, with the emitter bound to
/// the source, base, radix and controlled-power schedule. Pool syntax alone
/// cannot establish recovery of the transformed work carried by each stage.
fn validate_recovery(p: &Value, prepared: &g_momonados::ququart_folded_work::PreparedModularWork)
    -> Result<(),String>
{
    let source = number(&p["source_word"])?;
    let base = number(&p["base_word"])?;
    let radix = g_momonados::ququart_factor::power_of_two_radix_word(
        p["radix_word"].as_str().ok_or("missing work radix")?)?;
    let digit_bits = radix.bits_le().len()-1;
    let powers = p["prepared_operator"]["controlled_power_words"].as_array()
        .ok_or("missing source-bound controlled powers")?.iter().map(number)
        .collect::<Result<Vec<_>,_>>()?;
    let schedule = g_momonados::ququart_factor::QuquartPowerSchedule::from_prepared(&source,&base,powers)?;
    if number(&p["prepared_operator"]["phase_digits_word"])? != BigUint::from(schedule.powers().len()) {
        return Err("prepared work height differs from its phase register".into());
    }
    prepared.verify_recovery(&source,&base,digit_bits,schedule.powers().len())

}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rsa_200_and_256_bit_prepared_work_recovers_every_stack_stage() {
        for decimal in [
            "1156514714917773145849996001252587703581994899993461612691909",
            "101560191607051872909385412079844080615251494997013823952605603214371184345809",
        ] {
            let source = BigUint::parse_bytes(decimal.as_bytes(),10).unwrap();
            assert!(source.bits() >= 200);
            let base = BigUint::from(2u8);
            let schedule = g_momonados::ququart_factor::QuquartPowerSchedule::prepare(&source,&base).unwrap();
            let mut p = json!({"source_word":word(&source),"base_word":word(&base),
                "radix_word":word(&BigUint::from(16u8)),
                "prepared_operator":{"controlled_power_words":schedule.powers().iter().map(word).collect::<Vec<_>>(),
                    "phase_digits_word":word(&BigUint::from(schedule.powers().len()))}});
            p["prepared_work"] = compile(&p).unwrap();
            let recovered = decode(&p).unwrap();
            assert_eq!(recovered.stages.len(),source.bits() as usize+4);
            let references: usize = recovered.stages.iter().map(|(_,ops)|ops.len()).sum();
            println!("source_bits={} audited_stages={} operation_references={} unique_operations={}",
                source.bits(),recovered.stages.len(),references,recovered.operations.len());
            drop(recovered);
            let last = p["prepared_work"]["stages"].as_array().unwrap().len()-1;
            let references = p["prepared_work"]["stages"][last]["operation_words"].as_array_mut().unwrap();
            assert_ne!(references[0],references[1]);
            references.swap(0,1);
            assert!(decode(&p).is_err(),"wrong operation order must not recover at the deepest stage");
            p["prepared_work"]["stages"][last]["operation_words"].as_array_mut().unwrap().swap(0,1);
            p["prepared_work"]["stages"].as_array_mut().unwrap().pop();
            assert!(decode(&p).is_err(),"the deepest stage must not disappear from the stack");
        }
    }
}
