//! Executed source-address square winding on a retained amplitude register.
//! No residue or candidate loop: row operations act on the shared decision DAG.
use crate::{ququart_decision::DecisionArena, word_tape::WordTape,
    anyon_ququart::FixedQuquartSic, phase_unbraid::FixedPointFormat};
use num_bigint::{BigInt,BigUint};
use num_traits::{One,Zero};
use serde_json::{json,Value};

pub mod last_use;
pub mod arithmetic_image;
pub mod existential;
mod axis_tensor;

fn retain(arena:&DecisionArena,roots:&[usize])->Result<Value,String>{
    if let Some(path)=std::env::var_os("SOURCE_BRAID_CHECKPOINT_PATH"){
        let path=std::path::PathBuf::from(path);
        let started=std::time::Instant::now();
        let nodes=arena.write_prepared_many(roots,&path)?;
        Ok(json!({"checkpoint_path":path,"nodes":nodes,
            "serialization_seconds":started.elapsed().as_secs_f64(),
            "format":"prepared amplitude graph; fixed support recomputed on restore"}))
    }else{Ok(arena.prepared_record_many(roots))}
}

pub fn execute(source:WordTape)->Result<Value,String>{
    execute_with_node_budget(source,500_000)
}

/// The budget is checked between exact operations. It is not a hard bound on
/// temporary allocations inside an arithmetic row.
pub fn execute_with_node_budget(source:WordTape,node_budget:usize)->Result<Value,String>{
    let started=std::time::Instant::now();
    let n=source.to_biguint();
    if n<=BigUint::one(){return Err("source must exceed one".into());}
    let width=source.bit_len();
    let x:Vec<_>=(0..width).map(|i|2*i).collect();
    let r:Vec<_>=(0..2*width+1).map(|i|2*i+1).collect();
    let flag=4*width+2;
    let mut arena=DecisionArena::new(flag+1);
    let mut root=arena.basis(&BigUint::zero(),BigInt::one());
    for &wire in &x {let flipped=arena.flip(root,wire);root=arena.sum(root,flipped);}
    root=arena.compare_constant(root,&x,&n,&[],flag);
    let zero=arena.zero();
    root=arena.conditional_literals(zero,root,&[(flag,true)]);
    root=arena.flip(root,flag);
    if arena.mass(root)!=n{return Err("source family mass changed".into());}
    let prepared=root;
    let preparation_seconds=started.elapsed().as_secs_f64();
    let mut rows=Vec::new();
    let mut integer_bank=None;
    let mut joint=None;
    let mut observed=None;
    let mut reduction_seconds=0.0;
    let mut contraction_seconds=0.0;
    let mut tape=String::from("⊢≻∈");
    tape.extend(core::iter::repeat('⊤').take(width));
    tape.push_str("⊥⊞≺∋⊙⊡⊣");
    let mut executed_tape=String::new();
    let mut held=false;
    let tape_execution=vox_core::phase_word::execute_with(&tape,&source.as_word().to_string(),|mark,_|{
        if matches!(mark,'⊤'|'⊥'|'⊞') && arena.retained_nodes()>node_budget {
            held=true;
            return Err("retained square-winding node boundary".into());
        }
        executed_tape.push(mark);
        match mark {
            '⊤'=>{
                let i=rows.len();
                if i>=width{return Err("square row tape exceeded source width".into());}
                let row_started=std::time::Instant::now();
                eprintln!("SOURCE_SQUARE row={} stage=enter nodes={}",i,arena.retained_nodes());
                root=arena.integer_affine_add(root,&r,&x[i+1..],2*i+2,
                    &(BigUint::one()<<(2*i)),&[(x[i],true)])?;
                rows.push(json!({"row":i,"nodes":arena.retained_nodes(),
                    "seconds":row_started.elapsed().as_secs_f64()}));
                arena.fold_roots(&[prepared,root]);
                eprintln!("SOURCE_SQUARE row={} stage=return nodes={} seconds={:.9}",i,arena.retained_nodes(),row_started.elapsed().as_secs_f64());
            }
            '⊥'=>{
                if rows.len()!=width || integer_bank.is_some(){return Err("square return tape order changed".into());}
                integer_bank=Some(root);
                let clock=std::time::Instant::now();
                root=arena.reduce_register(root,&r,&n)?;
                reduction_seconds=clock.elapsed().as_secs_f64();
                if arena.mass(root)!=n{return Err("square winding changed joint mass".into());}
                joint=Some(root);
            }
            '⊞'=>{
                let joint_root=joint.ok_or("observation tape preceded square return")?;
                let clock=std::time::Instant::now();
                let mut branch=joint_root;
                for &wire in &x {
                    branch=arena.contract_internal_wire(branch,wire);
                    branch=arena.conditional_literals(zero,branch,&[(wire,false)]);
                    arena.fold_roots(&[prepared,integer_bank.unwrap(),joint_root,branch]);
                }
                observed=Some(branch);
                contraction_seconds=clock.elapsed().as_secs_f64();
            }
            _=>{}
        }
        Ok(())
    });
    if held {
        let mut roots=vec![prepared,root];
        if let Some(bank)=integer_bank{roots.push(bank);}
        if let Some(bank)=joint{roots.push(bank);}
        if let Some(bank)=observed{roots.push(bank);}
        return Ok(json!({"schema":"retained-square-winding-held/v1",
            "status":"held","N":n.to_string(),"source_word":source.as_word(),
            "input_boundary":"N-only","factors":null,"collision_count":null,
            "node_budget":node_budget,"retained_nodes":arena.retained_nodes(),
            "executed_tape_prefix":executed_tape,"planned_tape":tape,
            "next_square_row":rows.len(),"source_rows":rows,
            "integer_bank_present":integer_bank.is_some(),"joint_bank_present":joint.is_some(),
            "observation_bank_present":observed.is_some(),
            "source_wires":x,"return_wires":r,"comparison_flag":flag,
            "preparation_seconds":preparation_seconds,"seconds_before_serialization":started.elapsed().as_secs_f64(),
            "retained":retain(&arena,&roots)?,
            "continuation":"open interpreter prefix; next arithmetic mark has not executed"}));
    }
    let tape_read=tape_execution?;
    let integer_bank=integer_bank.ok_or("tape omitted integer bank")?;
    let joint=joint.ok_or("tape omitted square return")?;
    let observed=observed.ok_or("tape omitted contraction")?;
    if tape_read.exposed!=0 || tape_read.surviving.is_empty(){
        return Err("tape lost its banked source observation".into());
    }
    let readout_started=std::time::Instant::now();
    let gram=arena.control_gram(&[observed,zero,zero,zero]);
    let frame=FixedQuquartSic::new(&FixedPointFormat::for_modulus_bits(n.bits())?)?;
    let masses=frame.gram_mass_numerators(&gram)?;
    let recovered=frame.exact_gram_from_masses(&masses)?;
    if &recovered.numerators[0].0 % &recovered.denominator != BigInt::zero(){
        return Err("nonintegral collision readout".into());
    }
    let count=(&recovered.numerators[0].0 / &recovered.denominator)
        .to_biguint().ok_or("negative collision count")?;
    let top=&n*4u8+BigUint::one();
    let mut factors=None;
    if count<=top {
        let twice_sum=&top-&count;
        if (&twice_sum%2u8).is_zero(){
            let sum=twice_sum/2u8;
            let sq=&sum*&sum;
            if sq>=&n*4u8 {
                let delta=sq-&n*4u8;let gap=delta.sqrt();
                if &gap*&gap==delta && sum>=gap && ((&sum-&gap)%2u8).is_zero(){
                    let p=(&sum-&gap)/2u8;
                    if p>BigUint::one() && p<n && (&n%&p).is_zero(){
                        let p=WordTape::from_biguint(&p);
                        let(q,rem)=source.divmod(&p).ok_or("zero divisor")?;
                        if rem.is_zero() && p.mul(&q)==source {
                            factors=Some(json!({"p":p.to_biguint().to_string(),
                                "q":q.to_biguint().to_string(),"p_word":p.as_word(),
                                "q_word":q.as_word(),"exact_word_product":true}));
                        }
                    }
                }
            }
        }
    }
    Ok(json!({"schema":"retained-square-winding/v2","N":n.to_string(),
        "input_boundary":"N-only","collision_count":count.to_string(),
        "execution_tape":tape,"tape_deposits":tape_read.deposits,
        "tape_restored":tape_read.restored,"tape_exposed":tape_read.exposed,
        "preparation_seconds":preparation_seconds,"reduction_seconds":reduction_seconds,
        "contraction_seconds":contraction_seconds,
        "readout_and_factor_seconds":readout_started.elapsed().as_secs_f64(),
        "source_rows":rows,"source_mass":arena.mass(joint).to_string(),
        "sic_mass_numerators":masses.map(|v|v.to_string()),
        "sic_denominator":recovered.denominator.to_string(),"factors":factors,
        "seconds":started.elapsed().as_secs_f64(),
        "retained":retain(&arena,&[prepared,integer_bank,joint,observed])?,
        "observation":"norm of source-contracted return amplitudes; original joint bank retained"}))
}

#[cfg(test)] mod tests {
    #[test] fn requested_small_source(){
        let result=super::execute(crate::word_tape::WordTape::from_small(143)).unwrap();
        println!("{}",serde_json::to_string(&result).unwrap());
        assert_eq!(result["collision_count"],"525");
        assert_eq!(result["factors"]["p"],"11");
        assert_eq!(result["factors"]["q"],"13");
        let (arena,roots)=crate::ququart_decision::DecisionArena::from_prepared_record_many(&result["retained"]).unwrap();
        let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../measurements/source_braid_state_20261028/stream_roundtrip.state.json");
        arena.write_prepared_many(&roots,&path).unwrap();
        let record=serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let (restored,returned)=crate::ququart_decision::DecisionArena::from_prepared_record_many(&record).unwrap();
        let before:[usize;4]=roots.try_into().unwrap();
        let after:[usize;4]=returned.try_into().unwrap();
        assert_eq!(arena.control_gram(&before),restored.control_gram(&after));
    }
}
