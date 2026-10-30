//! Source-bound nonunit observable. The producer deposits raw residuals; only
//! values recovered from measured SIC masses enter the gcd selector.
use crate::{anyon_ququart::FixedQuquartSic,phase_unbraid::FixedPointFormat,word_tape::WordTape};
use num_bigint::{BigInt,BigUint};
use num_traits::{One,Zero};
use serde_json::{json,Value};

pub struct NonunitObservable {
    source:WordTape,
    frame:FixedQuquartSic,
    tree:Vec<BigUint>,
    residual_bank:Vec<WordTape>,
    base:usize,
    pending:Vec<usize>,
    pub raw_count:usize,
    pub active_count:usize,
    pub readouts:Vec<Value>,
    pub selected:Option<(WordTape,WordTape)>,
}
impl NonunitObservable {
    pub fn prepare(source:WordTape,residuals:&[WordTape])->Result<Self,String>{
        if !source.is_odd()||source.bit_len()<4{return Err("nonunit observable requires an odd source greater than seven".into());}
        let n=source.to_biguint();
        if residuals.iter().any(|r|r.gt(&source)){return Err("residual escaped its source ring".into());}
        // Zero is the full kernel and one is the empty kernel. Neither can
        // distinguish a proper divisor. No gcd is used to admit other leaves.
        let leaves:Vec<_>=residuals.iter().filter(|r|!r.is_zero()&&!r.is_one()&&*r!=&source).map(WordTape::to_biguint).collect();
        let base=leaves.len().max(1).next_power_of_two();let mut tree=vec![BigUint::one();2*base];
        for(i,r)in leaves.iter().enumerate(){tree[base+i]=r.clone();}
        for i in (1..base).rev(){tree[i]=(&tree[2*i]*&tree[2*i+1])%&n;}
        let frame=FixedQuquartSic::new(&FixedPointFormat::for_modulus_bits(n.bits())?)?;
        let pending=if leaves.is_empty(){vec![]}else{vec![1]};
        Ok(Self{source,frame,tree,residual_bank:residuals.to_vec(),base,pending,raw_count:residuals.len(),active_count:leaves.len(),readouts:Vec::new(),selected:None})
    }
    /// One observable evaluation. Gcd consumes the recovered coherence only.
    pub fn advance(&mut self)->Result<bool,String>{
        let Some(node)=self.pending.pop()else{return Ok(true)};
        let residue=&self.tree[node];
        let native=vox_core::combined_factor::native_word(residue);
        let hex=vox_core::combined_factor::hex_word(residue);
        let bits=vox_core::combined_factor::decode_hex_bits(&hex)?;
        let word=WordTape::from_canonical_word(&native).ok_or("observable native encoding failed")?;
        if (0..word.bit_len()).map(|i|word.bit_at(i)).collect::<Vec<_>>()!=bits{return Err("observable dual encoding lost the residue".into());}
        // Native place values transported through hex T/F/I/fork labels.
        // Pack four real planes into two complex coordinates and keep the
        // reference arm 1. rho_01 and rho_02 then recover every plane.
        let mut planes:[BigInt;4]=core::array::from_fn(|_|BigInt::zero());
        for(bit,&set)in bits.iter().enumerate(){if set{planes[bit%4]+=BigInt::one()<<(bit-bit%4);}}
        let amplitudes=[(BigInt::one(),BigInt::zero()),(planes[0].clone(),planes[1].clone()),
            (planes[2].clone(),planes[3].clone()),(BigInt::zero(),BigInt::one())];
        let gram=core::array::from_fn(|k|{let(a,b)=&amplitudes[k/4];let(c,d)=&amplitudes[k%4];(a*c+b*d,a*d-b*c)});
        let masses=self.frame.gram_mass_numerators(&gram)?;
        let exact=self.frame.exact_gram_from_masses(&masses)?;
        let coordinates=[&exact.numerators[1].0,&exact.numerators[1].1,&exact.numerators[2].0,&exact.numerators[2].1];
        if coordinates.iter().any(|v|*v%&exact.denominator!=BigInt::zero()){return Err("observable bit plane is not integral".into());}
        let recovered_planes=coordinates.map(|v|v/&exact.denominator);
        let reconstructed=(0..4).fold(BigInt::zero(),|sum,j|sum+(&recovered_planes[j]<<j));
        let recovered=reconstructed.to_biguint().ok_or("negative modular residue")?;
        // This comparison verifies state transport, after the mass-only return.
        if &recovered!=residue{return Err("observable mass readout changed its deposited residue".into());}
        let recovered_word=WordTape::from_biguint(&recovered);
        let divisor=self.source.gcd(&recovered_word);
        let proper=!divisor.is_one()&&!divisor.is_zero()&&self.source.gt(&divisor);
        self.readouts.push(json!({"tree_node":node,"residue_native":native,"residue_hex":hex,
            "mass_numerators":masses.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "exact_denominator":exact.denominator.to_string(),"recovered_residue":recovered.to_string(),
            "observable_gcd":divisor.to_biguint().to_string(),"proper_kernel":proper,
            "plane_order":["T","F","I","fork"],"recovered_planes":recovered_planes.map(|p|p.to_string()),
            "selector_input":"mass-only recovered rho_01 and rho_02; weighted bit-plane return"}));
        if proper{
            let(q,rem)=self.source.divmod(&divisor).ok_or("observable selected zero")?;
            if !rem.is_zero()||divisor.mul(&q)!=self.source{return Err("observable product failed source closure".into());}
            self.selected=Some((divisor,q));return Ok(true);
        }
        if divisor==self.source&&node<self.base{
            self.pending.push(2*node+1);self.pending.push(2*node);
        }
        Ok(self.pending.is_empty())
    }
    pub fn record(&self)->Value{json!({"N":self.source.to_biguint().to_string(),"residual_count":self.raw_count,
        "residual_bank":self.residual_bank.iter().map(|r|r.to_biguint().to_string()).collect::<Vec<_>>(),"active_residuals":self.active_count,"observable_evaluations":self.readouts.len(),"readouts":self.readouts,
        "pending_tree_nodes":self.pending,"continuation_retained":!self.pending.is_empty(),
        "status":if self.selected.is_some(){"factor"}else if self.pending.is_empty(){"empty_proper_kernel"}else{"held"},
        "factor":self.selected.as_ref().map(|(d,_)|d.to_biguint().to_string()),
        "quotient":self.selected.as_ref().map(|(_,q)|q.to_biguint().to_string()),"product_closes":self.selected.is_some()})}
}

pub fn run(raw:&str,b1:u64,b2:u64,curves:u32,sigma:u64,device:usize,seconds:u64)->Result<Value,String>{
    let n=vox_core::godel_calculus::parse_natural_word(raw)?;
    let source=WordTape::from_canonical_word(&vox_core::godel_calculus::encode_cell_binary(&n)).ok_or("invalid observable source")?;
    let start=std::time::Instant::now();
    let residuals=crate::gpu_ecm_word::residuals(&source,b1,b2,device,sigma,curves)?;
    let producer_seconds=start.elapsed().as_secs_f64();
    let mut observable=NonunitObservable::prepare(source,&residuals)?;
    while start.elapsed().as_secs()<seconds{if observable.advance()?{break;}}
    let mut result=observable.record();
    result["producer"]=json!("GPU ECM raw-residual kernel; no gcd or factor publication");
    result["parameters"]=json!({"B1":b1,"B2":b2,"curves":curves,"sigma":sigma,"device":device});
    result["producer_seconds"]=json!(producer_seconds);result["total_seconds"]=json!(start.elapsed().as_secs_f64());
    if observable.selected.is_none(){
        result["producer_continuation"]=json!({"N":n.to_string(),"next_sigma":sigma.checked_add(u64::from(curves)),
            "B1":b1,"B2":b2,"curves":curves,"device":device,"completed_batch_retained":true});
    }
    Ok(result)
}
