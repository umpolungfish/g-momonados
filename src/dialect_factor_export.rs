//! Bake the live dialect algebra, including its conflict evidence, for the
//! resident source-arm experiment. No factor, order, or factor geometry enters.
use crate::{algebra, dialect, dialect_expansion, imas_ig::IgTuple, tokens};
use g_momonados::word_tape::WordTape;
use serde_json::json;

pub fn export(args: &[String]) -> Result<(), String> {
    if args.len() != 2 {
        return Err("dialect-factor-export N_ONLY_WORD_FILE OUTPUT_JSON".into());
    }
    let text = std::fs::read_to_string(&args[0]).map_err(|e| e.to_string())?;
    let source = WordTape::from_canonical_word(text.trim()).ok_or("invalid canonical source")?;
    if source.bit_len() < 200 || !source.is_odd() {
        return Err("dialect factor execution requires an odd source of at least 200 bits".into());
    }
    let bifrons = tokens::compound_program(6).ok_or("missing Bifrons")?;
    let target = crate::rebis::pipeline::IgTuple::IUG;
    let target_tuple = IgTuple { d:target.d,t:target.t,r:target.r,p:target.p,f:target.f,
        k:target.k,g:target.g,c:target.c,phi:target.phi,h:target.h,s:target.s,omega:target.omega };
    let axis_values: Vec<_> = (0..12).map(|i| crate::axis_values::axis_values(i).iter()
        .map(|v| json!({"glyph":v.glyph(),"ordinal":v.ordinal()})).collect::<Vec<_>>()).collect();
    let mut tables = Vec::new();
    for u in 0..dialect_expansion::DIALECT_COUNT as u8 {
        // These sixteen programs are the numeral-cell alphabet, not factoring
        // controls. Bifrons is actually run by self_imscribe, before composition.
        let tuples: Vec<IgTuple> = (0..16).map(|label| {
            let word = WordTape::from_small(label);
            let mut program = crate::belnap_ring_shor::program_from_glyphs(word.as_word())
                .expect("canonical numeral is an opcode word");
            for &token in bifrons.as_slice() { program.push(token); }
            crate::axis_values::imscribe_program_under(u, &program)
        }).collect();
        let mut operations = Vec::new();
        let mut promotions = Vec::new();
        for op in 0..3 {
            let mut cells = Vec::new();
            let mut certificates = Vec::new();
            for a in &tuples { for b in &tuples {
                let (result, conflicts) = match op {
                    0 => { let r=algebra::meet_under(u,a,b); (r.tuple,r.conflicts) },
                    1 => { let r=algebra::join_under(u,a,b); (r.tuple,r.conflicts) },
                    _ => (algebra::tensor_under(u,a,b),[false;12]),
                };
                let conflict = conflicts.iter().enumerate()
                    .fold(0u16,|mask,(i,&value)| mask | ((value as u16)<<i));
                let address=result.crystal_address();
                if crate::crystal::decode(address) != result.to_crystal_indices() {
                    return Err("dialect tuple crystal roundtrip failed".into());
                }
                let source_tuple=crate::rebis::pipeline::IgTuple {d:result.d,t:result.t,r:result.r,
                    p:result.p,f:result.f,k:result.k,g:result.g,c:result.c,phi:result.phi,
                    h:result.h,s:result.s,omega:result.omega};
                let report=crate::rebis::pipeline::run_promotion_pipeline(&source_tuple,&target);
                certificates.push(json!({"source_indices":result.to_crystal_indices(),
                    "changes":report.promotions.iter().map(|p|json!({"from":p.from.glyph(),
                        "to":p.to.glyph(),"ordinal_delta":p.to.ordinal()-p.from.ordinal()})).collect::<Vec<_>>(),
                    "canonical_verdict":[dialect::dialect_gates_pass(0,&result),dialect::dialect_t_pass(0,&result)]}));
                cells.push(json!([address,conflict,
                    dialect::dialect_gates_pass(u,&result),dialect::dialect_t_pass(u,&result)]));
            }}
            operations.push(cells);
            promotions.push(certificates);
        }
        let definition=&dialect_expansion::all_dialects()[u as usize];
        tables.push(json!({"id":u,"name":dialect::dialect_name(u),
            "gates":dialect::dialect_gates(u),"absorption_source_name":definition.name,
            "input_addresses":tuples.iter().map(IgTuple::crystal_address).collect::<Vec<_>>(),
            "operations":operations,"promotion_certificates":promotions}));
    }
    let value=json!({"format":"resident-dialect-algebra-v1","source_word":source.as_word(),
        "source_bits":source.bit_len(),"crystal_radix":crate::crystal::TOTAL,
        "operation_names":["meet","join","tensor"],
        "input_program":"canonical nibble numeral followed by executed Bifrons",
        "promotion_target_indices":target_tuple.to_crystal_indices(),
        "promotion_target_address":target_tuple.crystal_address(),"axis_values":axis_values,
        "tables":tables});
    std::fs::write(&args[1],serde_json::to_vec(&value).map_err(|e|e.to_string())?)
        .map_err(|e|e.to_string())?;
    Ok(())
}
