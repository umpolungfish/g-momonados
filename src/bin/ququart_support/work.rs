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
pub fn compile(p: &Value) -> Result<Value, String> {
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
pub fn decode(p: &Value) -> Result<Vec<(BigUint, Vec<NestedOperation>)>, String> {
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
