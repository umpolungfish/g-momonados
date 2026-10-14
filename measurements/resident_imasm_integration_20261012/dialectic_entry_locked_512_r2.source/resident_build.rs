//! Cargo membrane construction, using the same modules as the executable.
use crate::resident_work as work;
use g_momonados::{resident_glut::ResidentGlut,word_tape::WordTape};
use num_traits::Zero;
use std::time::Instant;

pub struct PreparedCompilation {
    pub record:serde_json::Value,
    pub work:g_momonados::ququart_folded_work::PreparedModularWork,
}

pub fn compile(source_text: &str) -> Result<PreparedCompilation, String> {
    let source = WordTape::from_canonical_word(source_text.trim()).ok_or("invalid source word")?;
    let preparation = Instant::now();
    let mut resident = ResidentGlut::from_word(source.as_word())?;
    let receipt_path = std::env::var("RESIDENT_CIRCUIT_OPERATOR_FILE")
        .map_err(|_| "set RESIDENT_CIRCUIT_OPERATOR_FILE to the split/fuse operator receipt")?;
    let receipt_text = std::fs::read_to_string(&receipt_path).map_err(|e| e.to_string())?;
    let receipt: serde_json::Value = serde_json::from_str(&receipt_text).map_err(|e| e.to_string())?;
    let circuit_spec = receipt["circuit"].as_str().ok_or("operator receipt lacks circuit")?;
    let braid_word = receipt["braid_word"].as_array().ok_or("operator receipt lacks braid word")?;
    if braid_word.is_empty() || !receipt["unitary"].as_bool().unwrap_or(false)
        || receipt["word_matrix_residual"].as_f64().unwrap_or(f64::INFINITY) >= 1e-6 {
        return Err("split/fused operator receipt lacks a passing unitary word readback".into());
    }
    let matrix = receipt["synthesized_matrix"].as_array().ok_or("operator receipt lacks synthesized matrix")?;
    if matrix.len() != 4 { return Err("operator matrix must contain four row-major entries".into()); }
    let mut synthesized = [(0.0, 0.0); 4];
    for (index, entry) in matrix.iter().enumerate() {
        synthesized[index] = (entry["re"].as_f64().ok_or("operator real coordinate is invalid")?,
            entry["im"].as_f64().ok_or("operator imaginary coordinate is invalid")?);
    }
    let target_matrix = receipt["target_matrix"].as_array().ok_or("operator receipt lacks target matrix")?;
    let target_values = target_matrix.iter().map(|entry| serde_json::json!({
        "re":entry["re"], "im":entry["im"]
    })).collect::<Vec<_>>();
    let synthesized_values = synthesized.iter().map(|(re, im)| serde_json::json!({"re":re,"im":im}))
        .collect::<Vec<_>>();
    let target_residual = receipt["target_projective_residual"].as_f64().ok_or("operator receipt lacks target residual")?;
    let word_residual = receipt["word_matrix_residual"].as_f64().ok_or("operator receipt lacks word residual")?;
    const COEFFICIENT_BITS: usize = 52;
    let coefficient_scale = (1u64 << COEFFICIENT_BITS) as f64;
    let numerator = synthesized.map(|(re,im)| (
        num_bigint::BigInt::from((re * coefficient_scale).round() as i64),
        num_bigint::BigInt::from((im * coefficient_scale).round() as i64)));
    let matrix_dual = resident.audit_circuit_braid_dual(&numerator,COEFFICIENT_BITS)?;
    if !matrix_dual.exact_full_state_return || !matrix_dual.exact_prepared_state_restoration
        || matrix_dual.exact_complex_gram_coordinates != matrix_dual.total_complex_gram_coordinates
        || !matrix_dual.exact_mass_return || !matrix_dual.maximum_absolute_gram_delta.is_zero() {
        return Err("split/fuse matrix-dual audit failed on the prepared resident factor state".into());
    }
    let q52_dual = resident.apply_circuit_braid_numerator(&numerator,COEFFICIENT_BITS)?;
    let complex_product = |x: &(num_bigint::BigInt,num_bigint::BigInt),
        y: &(num_bigint::BigInt,num_bigint::BigInt)|
        (&x.0 * &y.0 - &x.1 * &y.1, &x.0 * &y.1 + &x.1 * &y.0);
    let ad = complex_product(&numerator[0],&numerator[3]);
    let bc = complex_product(&numerator[1],&numerator[2]);
    let determinant = (&ad.0 - &bc.0, &ad.1 - &bc.1);
    let adjugate = [(numerator[3].0.clone(),numerator[3].1.clone()),
        (-numerator[1].0.clone(),-numerator[1].1.clone()),
        (-numerator[2].0.clone(),-numerator[2].1.clone()),
        (numerator[0].0.clone(),numerator[0].1.clone())];
    let complex_words = |gram: &[(num_bigint::BigInt, num_bigint::BigInt); 16]| gram.iter().map(|(re, im)|
        serde_json::json!({"re_word":signed_word(re),"im_word":signed_word(im)})).collect::<Vec<_>>();
    let circuit_record = serde_json::json!({
        "circuit":circuit_spec,
        "target_matrix":target_values,
        "synthesized_matrix":synthesized_values,
        "matrix_numerator_q52":numerator.map(|(re,im)|serde_json::json!({"re":re.to_string(),"im":im.to_string()})),
        "matrix_dual":{
            "adjugate_numerator":adjugate.map(|(re,im)|serde_json::json!({"re":re.to_string(),"im":im.to_string()})),
            "denominator_bits":COEFFICIENT_BITS,
            "determinant":{"re":determinant.0.to_string(),"im":determinant.1.to_string()},
            "composition":"adjugate_numerator × matrix_numerator = determinant × identity",
            "same_resident_diagram":true,
            "exact_full_state_return":matrix_dual.exact_full_state_return,
            "exact_prepared_state_restoration":matrix_dual.exact_prepared_state_restoration,
            "exact_complex_gram_coordinates":matrix_dual.exact_complex_gram_coordinates,
            "total_complex_gram_coordinates":matrix_dual.total_complex_gram_coordinates,
            "maximum_absolute_gram_delta":matrix_dual.maximum_absolute_gram_delta.to_string(),
            "exact_mass_return":matrix_dual.exact_mass_return,
            "mass_delta":matrix_dual.mass_delta.to_string()
        },
        "forward_audit_frame":matrix_dual.forward_frame.record(),
        "reverse_audit_frame":matrix_dual.reverse_frame.record(),
        "prepared_operator_frame":q52_dual.banked_frame.as_ref()
            .ok_or("prepared circuit lost its banked frame")?.record(),
        "braid_word":braid_word,
        "target_projective_residual":target_residual,
        "word_matrix_residual":word_residual,
        "unitary":receipt["unitary"],
        "operator_receipt_path":receipt_path,
        "resident_wires":q52_dual.wires,
        "coefficient_fraction_bits":q52_dual.coefficient_fraction_bits,
        "input_amplitude_scale_bits":q52_dual.input_amplitude_scale_bits,
        "mass_before_word":work::word_for_record(&q52_dual.mass_before),
        "mass_after_word":work::word_for_record(&q52_dual.mass_after),
        "gram_before":complex_words(&q52_dual.gram_before),
        "gram_after":complex_words(&q52_dual.gram_after),
        "changed_gram_coordinates":q52_dual.changed_coordinates
    });
    eprintln!("prepared-circuit-braid spec={} generators={} target_residual={:.6e} word_residual={:.6e} matrix_dual_state={} matrix_dual_gram={}/{} matrix_dual_mass={} changed_gram_coordinates={} seconds={:.6}",
        circuit_spec, braid_word.len(), target_residual, word_residual,
        matrix_dual.exact_full_state_return, matrix_dual.exact_complex_gram_coordinates,
        matrix_dual.total_complex_gram_coordinates, matrix_dual.exact_mass_return,
        q52_dual.changed_coordinates, preparation.elapsed().as_secs_f64());
    let witness = resident.sic_witness()?;
    eprintln!("prepared-SIC residual={} tolerance={} seconds={:.6}",witness.reconstruction.maximum_residual,
        witness.reconstruction.tolerance,preparation.elapsed().as_secs_f64());
    let mut carrier = resident.into_source_whole_register_phase_observed(|stage|
        eprintln!("preparation-stage {stage} seconds={:.6}",preparation.elapsed().as_secs_f64()))?;
    let clock_export: serde_json::Value = serde_json::from_str(include_str!(
        "measurements/membrane_reentry_20261010/certified_clocks.json")).map_err(|e| e.to_string())?;
    // The period-two oscillator is a fixed program constant, with no instance
    // witness. Its executed ticks order complete prepared modular lanes.
    let clock = clock_export["clocks"].as_array().ok_or("missing certified clock export")?.iter()
        .find(|r| r["retained"].as_u64()==Some(0) && r["seed"].as_u64()==Some(1))
        .ok_or("missing certified alternating clock")?;
    carrier.bind_certified_reentry(clock)?;
    let mut record = carrier.prepared_record()?;
    drop(carrier);
    let mut power = WordTape::from_small(2);
    let mut powers = Vec::with_capacity(source.bit_len());
    for _ in 0..source.bit_len() {
        powers.push(power.as_word().to_owned());
        let square = power.mul(&power).divmod(&source).unwrap().1;
        power = square.mul(&square).divmod(&source).unwrap().1;
    }
    let mut program = serde_json::json!({"source_word":source.as_word(),"base_word":WordTape::from_small(2).as_word(),
        "radix_word":WordTape::one_at(8).as_word(),"prepared_operator":{
            "controlled_power_words":powers,"phase_digits_word":WordTape::from_small(source.bit_len() as u64).as_word()}});
    program["prepared_work"] = work::compile(&program)?;
    let recovered=work::decode(&program)?;
    eprintln!("prepared-modular-program seconds={:.6}",preparation.elapsed().as_secs_f64());
    record["program"] = program;
    record["preparation_seconds"] = serde_json::json!(preparation.elapsed().as_secs_f64());
    record["preparation_nanoseconds"] = serde_json::json!(
        u64::try_from(preparation.elapsed().as_nanos()).map_err(|_| "preparation clock overflow")?);
    record["SIC_signed_residuals"] = serde_json::json!(witness.signed_residuals.iter().map(|(re,im)|
        serde_json::json!({"re_word":signed_word(re),"im_word":signed_word(im)})).collect::<Vec<_>>());
    record["SIC_rational_denominator_word"] = serde_json::json!(signed_word(&witness.rational_denominator));
    record["SIC_rational_residual_words"] = serde_json::json!(witness.rational_signed_residuals.iter().map(|(re,im)|
        serde_json::json!({"re_word":signed_word(re),"im_word":signed_word(im)})).collect::<Vec<_>>());
    record["SIC_gram_words"]=serde_json::json!(witness.gram.iter().map(|(re,im)|
        serde_json::json!({"re_word":signed_word(re),"im_word":signed_word(im)})).collect::<Vec<_>>());
    record["SIC_mass_numerator_words"]=serde_json::json!(witness.mass_numerators.iter().map(|mass|
        work::word_for_record(mass)).collect::<Vec<_>>());
    record["circuit_braid"] = circuit_record;
    Ok(PreparedCompilation {record,work:recovered})
}

pub fn emit_baked(prepared:&PreparedCompilation,out:&mut impl std::io::Write,out_dir:&std::path::Path)->std::io::Result<()> {
    use num_bigint::{BigUint,BigInt};
    use g_momonados::reversible_modular::NestedOperation;
    use std::collections::HashMap;
    let record=&prepared.record;
    let phase=&record["phase"];
    let sic=&record["sic_frame"];
    let support_check=Instant::now();
    let (checked,root)=crate::ququart_decision::DecisionArena::from_prepared_record(&phase["register"])
        .expect("Cargo checks every prepared support mask against the retained amplitude diagram");
    assert_eq!(checked.prepared_record(root),phase["register"]);
    drop(checked);
    eprintln!("prepared-support-cache verified_nodes={} masks={} seconds={:.6}",
        phase["register"]["nodes"].as_array().unwrap().len(),
        phase["register"]["support_masks"].as_array().unwrap().len(),support_check.elapsed().as_secs_f64());
    let index=|value:&serde_json::Value| usize::try_from(value.as_u64().expect("prepared address")).unwrap();
    let signed=|row:&serde_json::Value| crate::word_tape::SignedWordTape::from_prepared_record(row).expect("host-verified signed word");
    let integer=|value:BigInt| {let (sign,digits)=value.to_u32_digits();
        format!("BakedInteger {{negative:{},digits:&{:?}}}",sign==num_bigint::Sign::Minus,digits)};
    let complex=|row:&serde_json::Value| format!("BakedComplex {{re:{},im:{}}}",
        integer(signed(&row["re"]).to_bigint()),integer(signed(&row["im"]).to_bigint()));
    let word_complex=|row:&serde_json::Value| {
        let word=|r:&serde_json::Value| {let value=signed(r);
            format!("BakedSignedWord {{negative:{},word:{:?}}}",value.is_negative(),value.magnitude().as_word())};
        format!("BakedWordComplex {{re:{},im:{}}}",word(&row["re"]),word(&row["im"]))
    };
    writeln!(out,"use g_momonados::baked_resident::*;")?;
    writeln!(out,"const BAKED_SOURCE_WORD:&str={:?};",record["source_word"].as_str().unwrap())?;
    writeln!(out,"const BAKED_PREPARATION_NS:u64={};",record["preparation_nanoseconds"].as_u64().unwrap())?;
    writeln!(out,"static BAKED_NODES:&[BakedNode]=&[")?;
    for row in phase["register"]["nodes"].as_array().unwrap() {
        if row.get("re").is_some() {writeln!(out,"BakedNode::Leaf({}),",word_complex(row))?;}
        else {writeln!(out,"BakedNode::Branch {{wire:{},low:{},high:{}}},",index(&row["wire"]),index(&row["low"]),index(&row["high"]))?;}
    }
    writeln!(out,"];")?;
    let mut support_bytes=Vec::new();
    let mut support_spans=Vec::new();
    for mask in phase["register"]["support_masks"].as_array().unwrap() {
        let digits: Vec<u32> = mask.as_array().unwrap().iter().map(|digit|
            u32::try_from(digit.as_u64().unwrap()).unwrap()).collect();
        let start=support_bytes.len()/4;
        for digit in &digits {support_bytes.extend(digit.to_le_bytes());}
        assert!(support_bytes[start*4..].chunks_exact(4).map(|bytes|
            u32::from_le_bytes(bytes.try_into().unwrap())).eq(digits));
        support_spans.push((start,support_bytes.len()/4));
    }
    std::fs::write(out_dir.join("resident_support.bin"),&support_bytes)?;
    writeln!(out,"#[cfg(not(target_endian=\"little\"))] compile_error!(\"compiled support requires its little-endian limb image\");")?;
    // The equal-sized transmute is checked by Rust at compilation. Declaring
    // a u32 array supplies its alignment; no byte-pointer cast runs at import.
    writeln!(out,"static BAKED_SUPPORT_LIMBS:[u32;{}]=unsafe {{core::mem::transmute(*include_bytes!(concat!(env!(\"OUT_DIR\"),\"/resident_support.bin\")))}};",support_bytes.len()/4)?;
    writeln!(out,"static BAKED_SUPPORT_SPANS:&[(usize,usize)]=&{support_spans:?};")?;
    writeln!(out,"static BAKED_FIXED:&[(bool,usize,usize)]=&[")?;
    for fixed in phase["register"]["fixed"].as_array().unwrap() {
        writeln!(out,"({},{},{}),",fixed[0].as_bool().unwrap(),index(&fixed[1]),index(&fixed[2]))?;
    }
    writeln!(out,"];\nstatic BAKED_SUPPORT:BakedSupport=BakedSupport {{limbs:&BAKED_SUPPORT_LIMBS,spans:BAKED_SUPPORT_SPANS,fixed:BAKED_FIXED}};")?;
    let mut values:Vec<BigUint>=Vec::new();let mut value_ids=HashMap::new();
    let mut wires:Vec<Vec<usize>>=Vec::new();let mut wire_ids=HashMap::new();
    let mut controls:Vec<Vec<(usize,bool)>>=Vec::new();let mut control_ids=HashMap::new();
    fn intern<T:Clone+Eq+std::hash::Hash>(value:&T,pool:&mut Vec<T>,ids:&mut HashMap<T,usize>)->usize {
        if let Some(&id)=ids.get(value) {return id;}
        let id=pool.len();pool.push(value.clone());ids.insert(value.clone(),id);id
    }
    writeln!(out,"static BAKED_OPERATIONS:&[BakedOperation]=&[")?;
    for op in &prepared.work.operations {
        match op {
            NestedOperation::Toggle(gate)=>{
                let c=intern(&gate.controls,&mut controls,&mut control_ids);
                writeln!(out,"BakedOperation::Toggle {{target:{},controls:{c}}},",gate.target)?;
            }
            NestedOperation::ModularAdd {register,digit,value,modulus,controls:c}=>{
                assert_eq!(modulus,&prepared.work.source);
                let r=intern(register,&mut wires,&mut wire_ids);let d=intern(digit,&mut wires,&mut wire_ids);
                let v=intern(value,&mut values,&mut value_ids);let c=intern(c,&mut controls,&mut control_ids);
                writeln!(out,"BakedOperation::ModularAdd {{register:{r},digit:{d},value:{v},controls:{c}}},")?;
            }
            _=>panic!("N-only compiler produced an unexpected operation"),
        }
    }
    writeln!(out,"];")?;
    writeln!(out,"static BAKED_STAGES:&[(usize,&[usize])]=&[")?;
    for (power,refs) in &prepared.work.stages {
        let id=intern(power,&mut values,&mut value_ids);
        writeln!(out,"({id},&{refs:?}),")?;
    }
    writeln!(out,"];")?;
    writeln!(out,"static BAKED_VALUES:&[&[u32]]=&[")?;
    for value in values {let digits=value.to_u32_digits();assert_eq!(BigUint::new(digits.clone()),value);
        writeln!(out,"&{digits:?},")?;}
    writeln!(out,"];\nstatic BAKED_WIRES:&[&[usize]]=&[")?;
    for row in wires {writeln!(out,"&{row:?},")?;}
    writeln!(out,"];\nstatic BAKED_CONTROLS:&[&[(usize,bool)]]=&[")?;
    for row in controls {writeln!(out,"&{row:?},")?;}
    writeln!(out,"];")?;
    let lane_offsets:Vec<_>=prepared.work.stages.iter().map(|(_,refs)| {
        let lane=|id:usize| match &prepared.work.operations[id] {
            NestedOperation::Toggle(gate)=>gate.controls.iter().any(|&(wire,_)|wire==0),
            NestedOperation::ModularAdd {controls,..}=>controls.iter().any(|&(wire,_)|wire==0),
            _=>panic!("unexpected compiled lane operation"),
        };
        let offset=refs.iter().take_while(|&&id|lane(id)).count();
        assert!(refs[offset..].iter().all(|&id|!lane(id)));
        offset
    }).collect();
    writeln!(out,"static BAKED_WORK:BakedWork=BakedWork {{source:&{:?},digit_bits:{},values:BAKED_VALUES,wires:BAKED_WIRES,controls:BAKED_CONTROLS,operations:BAKED_OPERATIONS,stages:BAKED_STAGES,lane_offsets:&{:?}}};",
        prepared.work.source.to_u32_digits(),prepared.work.digit_bits,lane_offsets)?;
    writeln!(out,"static BAKED_RAYS:[[BakedComplex;4];16]=[")?;
    for ray in sic["rays"].as_array().unwrap() {write!(out,"[")?;for row in ray.as_array().unwrap() {write!(out,"{},",complex(row))?;}writeln!(out,"],")?;}
    writeln!(out,"];\nstatic BAKED_PROJECTORS:[[BakedWordComplex;16];16]=[")?;
    for projector in sic["projectors"].as_array().unwrap() {write!(out,"[")?;for row in projector.as_array().unwrap() {write!(out,"{},",word_complex(row))?;}writeln!(out,"],")?;}
    writeln!(out,"];")?;
    writeln!(out,"static BAKED_SIC:BakedSic=BakedSic {{modulus_bits:{},rays:&BAKED_RAYS,projectors:&BAKED_PROJECTORS}};",sic["modulus_bits"])?;
    writeln!(out,"static BAKED_FOURIER:&[BakedComplex]=&[")?;
    for row in phase["fourier"].as_array().unwrap() {writeln!(out,"{},",complex(row))?;}
    writeln!(out,"];")?;
    let work_wires:Vec<_>=phase["work_wires"].as_array().unwrap().iter().map(index).collect();
    writeln!(out,"static BAKED_PHASE:BakedPhase=BakedPhase {{source_word:BAKED_SOURCE_WORD,base_word:{:?},width:{},modulus_bits:{},work_wires:&{:?},hadamard:{},fourier:BAKED_FOURIER,cells:{},root:{},nodes:BAKED_NODES,support:&BAKED_SUPPORT,work:&BAKED_WORK}};",
        phase["base_word"].as_str().unwrap(),phase["width"],phase["modulus_bits"],work_wires,complex(&phase["hadamard"]),phase["register"]["cells"],phase["register"]["root"])?;
    let wire_map:Vec<_>=record["wire_map"].as_array().unwrap().iter().map(index).collect();
    let ancestry=serde_json::json!({"signed":record["SIC_signed_residuals"],"denominator":record["SIC_rational_denominator_word"],
        "rational":record["SIC_rational_residual_words"],"gram":record["SIC_gram_words"],"masses":record["SIC_mass_numerator_words"],
        "circuit_braid":record["circuit_braid"]});
    writeln!(out,"static BAKED_RESIDENT:BakedResident=BakedResident {{source_word:BAKED_SOURCE_WORD,initial_mass_word:{:?},wire_map:&{:?},constraint_gates:{},product_closure_prepared:{},phase_source_relation:{},sic_modulus_bits:{},sic:&BAKED_SIC,phase:&BAKED_PHASE,reentry_json:{:?},preparation_sic_json:{:?},preparation_nanoseconds:BAKED_PREPARATION_NS}};",
        record["initial_mass_word"].as_str().unwrap(),wire_map,record["constraint_gates"],record["product_closure_prepared"],record["phase_source_relation"],record["sic_modulus_bits"],record["reentry"].to_string(),ancestry.to_string())?;
    writeln!(out,"fn baked_membrane()->Result<g_momonados::resident_glut::ResidentPhaseCarrier,String> {{g_momonados::resident_glut::ResidentPhaseCarrier::from_baked(&BAKED_RESIDENT)}}")?;
    Ok(())
}

fn signed_word(value: &num_bigint::BigInt) -> String {
    use num_traits::Signed;
    format!("{}{}",if value.is_negative() { "≺" } else { "≻" },work::word_for_record(value.magnitude()))
}
