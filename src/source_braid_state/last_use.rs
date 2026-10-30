//! Last-use contraction of the observation branch. The original source image
//! is retained as an exact functional bank, separately from its marginal.
use super::*;

/// Applied square rows in a source-addressed functional representation.
/// This is not a materialized joint amplitude DAG. Its read operation executes
/// native numeral arithmetic, including the inverse return on the same address.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FunctionalBank {
    pub source: WordTape,
    pub applied_rows: usize,
}

impl FunctionalBank {
    pub fn integer_return(&self, address: &WordTape) -> Result<WordTape, String> {
        if address >= &self.source { return Err("address outside source bank".into()); }
        let high = address.shr(self.applied_rows);
        address.mul(address).sub(&high.mul(&high).shl(2*self.applied_rows))
            .ok_or_else(|| "negative partial square return".into())
    }

    pub fn modular_return(&self, address: &WordTape) -> Result<WordTape, String> {
        self.integer_return(address)?.divmod(&self.source)
            .map(|(_, r)| r).ok_or_else(|| "zero source bank".into())
    }

    pub fn recover_zero_bank(&self, address: &WordTape, returned: &WordTape)
        -> Result<WordTape, String> {
        let offset = self.modular_return(address)?;
        returned.add(&self.source).sub(&offset).ok_or("negative inverse bank")?
            .divmod(&self.source).map(|(_, r)|r).ok_or_else(|| "zero source bank".into())
    }

    fn record(&self) -> Result<Value, String> {
        let addresses = [WordTape::zero(), self.source.shr(1),
            self.source.sub(&WordTape::one()).ok_or("empty source bank")?];
        let mut witnesses = Vec::new();
        for address in addresses {
            let integer = self.integer_return(&address)?;
            let modular = self.modular_return(&address)?;
            let recovered = self.recover_zero_bank(&address, &modular)?;
            if !recovered.is_zero() { return Err("functional source bank inverse failed".into()); }
            witnesses.push(json!({"address_word":address.as_word(),
                "integer_return_word":integer.as_word(),"modular_return_word":modular.as_word(),
                "inverse_return_word":recovered.as_word()}));
        }
        Ok(json!({"representation":"source-addressed functional bank, not a joint amplitude DAG",
            "source_word":self.source.as_word(),"source_extent":self.source.to_biguint().to_string(),
            "applied_rows":self.applied_rows,
            "integer_image":"x*x - ((x >> applied_rows)^2 << (2*applied_rows))",
            "modular_image":"integer_image mod N", "witnesses":witnesses}))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Continuation { bank: FunctionalBank, observation: usize }

pub fn execute(source: WordTape, node_budget: usize) -> Result<Value, String> {
    let started = std::time::Instant::now();
    let n = source.to_biguint();
    if n <= BigUint::one() { return Err("source must exceed one".into()); }
    let width = source.bit_len();
    let x: Vec<_> = (0..width).map(|i| 2*i).collect();
    let r: Vec<_> = (0..2*width+1).map(|i| 2*i+1).collect();
    let flag = 4*width+2;
    let mut arena = DecisionArena::new(flag+1);
    let zero = arena.zero();
    let mut root = arena.basis(&BigUint::zero(), BigInt::one());
    for &wire in &x { let high = arena.flip(root,wire); root = arena.sum(root,high); }
    root = arena.compare_constant(root,&x,&n,&[],flag);
    root = arena.conditional_literals(zero,root,&[(flag,true)]);
    root = arena.flip(root,flag);
    if arena.mass(root) != n { return Err("source preparation mass changed".into()); }
    let prepared = root;
    let mut state = Continuation { bank: FunctionalBank {source:source.clone(),applied_rows:0},
        observation:root };
    arena.fold_roots(&[prepared,root]);
    let preparation_seconds = started.elapsed().as_secs_f64();
    let mut rows = Vec::new();
    let mut tape = String::from("⊢≻∈");
    tape.extend(core::iter::repeat("⊤⊞").take(width).flat_map(str::chars));
    tape.push_str("⊥≺∋⊙⊡⊣");
    let mut executed = String::new();
    let mut held = false;
    let mut awaiting_cut = false;
    let mut reduced = false;
    let tape_result = vox_core::phase_word::execute_with(&tape,&source.as_word().to_string(),|mark,_| {
        // A row and its last-use cut form one operation boundary. Holding
        // between them would require a separate continuation phase.
        if matches!(mark,'⊤'|'⊥') && arena.retained_nodes()>node_budget {
            held=true; return Err("last-use retained node boundary".into());
        }
        executed.push(mark);
        match mark {
            '⊤' => {
                if awaiting_cut || state.bank.applied_rows>=width {return Err("square row order".into());}
                let i = state.bank.applied_rows;
                let clock = std::time::Instant::now();
                eprintln!("LAST_USE row={i} stage=enter nodes={}",arena.retained_nodes());
                state = crate::reentry_powerset::bind_one(state.clone(),|mut next| {
                    next.observation=arena.integer_affine_add(next.observation,&r,&x[i+1..],
                        2*i+2,&(BigUint::one()<<(2*i)),&[(x[i],true)])?;
                    next.bank.applied_rows+=1;
                    Ok::<_,String>(next)
                })?;
                rows.push(json!({"row":i,"nodes_before_cut":arena.retained_nodes(),
                    "row_seconds":clock.elapsed().as_secs_f64()}));
                awaiting_cut=true;
            }
            '⊞' => {
                if !awaiting_cut {return Err("last-use cut without applied row".into());}
                let i=state.bank.applied_rows-1;
                let clock=std::time::Instant::now();
                state = crate::reentry_powerset::bind_one(state.clone(),|mut next| {
                    next.observation=arena.contract_internal_wire(next.observation,x[i]);
                    next.observation=arena.conditional_literals(zero,next.observation,&[(x[i],false)]);
                    Ok::<_,String>(next)
                })?;
                arena.fold_roots(&[prepared,state.observation]);
                rows[i]["nodes_after_cut"]=json!(arena.retained_nodes());
                rows[i]["cut_seconds"]=json!(clock.elapsed().as_secs_f64());
                eprintln!("LAST_USE row={i} stage=return nodes={}",arena.retained_nodes());
                awaiting_cut=false;
            }
            '⊥' => {
                if awaiting_cut || state.bank.applied_rows!=width {return Err("early modular reduction".into());}
                state.observation=arena.reduce_register(state.observation,&r,&n)?;
                arena.fold_roots(&[prepared,state.observation]);
                reduced=true;
            }
            _ => {}
        }
        Ok(())
    });
    if !held { tape_result?; }
    let mut result=json!({"schema":"last-use-source-square/v1","N":n.to_string(),
        "input_boundary":"N-only","factors":null,"status":if held{"held"}else{"readout"},
        "planned_tape":tape,"executed_tape":executed,"source_rows":rows,
        "functional_joint_bank":state.bank.record()?,
        "applied_rows":state.bank.applied_rows,"node_budget":node_budget,
        "retained_nodes":arena.retained_nodes(),"preparation_seconds":preparation_seconds,
        "powerset_operation":"bind_one on source-bound functional bank and observation root",
        "Y_evaluator_executed":false,"reduced":reduced});
    if !held {
        let gram=arena.control_gram(&[state.observation,zero,zero,zero]);
        let frame=FixedQuquartSic::new(&FixedPointFormat::for_modulus_bits(n.bits())?)?;
        let masses=frame.gram_mass_numerators(&gram)?;
        let recovered=frame.exact_gram_from_masses(&masses)?;
        let numerator=&recovered.numerators[0].0;
        if numerator%&recovered.denominator!=BigInt::zero(){return Err("nonintegral collision readout".into());}
        let count=(numerator/&recovered.denominator).to_biguint().ok_or("negative collision count")?;
        let top=&n*4u8+BigUint::one();
        if count<=top && ((&top-&count)%2u8).is_zero() {
            let sum=(&top-&count)/2u8;
            if &sum*&sum>=&n*4u8 {
                let delta=&sum*&sum-&n*4u8;
                let gap=delta.sqrt();
                if &gap*&gap==delta && sum>=gap && ((&sum-&gap)%2u8).is_zero() {
                    let p=WordTape::from_biguint(&((&sum-&gap)/2u8));
                    if p>WordTape::one() && p<source {
                        let(q,rem)=source.divmod(&p).ok_or("zero divisor")?;
                        if rem.is_zero() && p.mul(&q)==source {
                            result["factors"]=json!({"p":p.to_biguint().to_string(),
                                "q":q.to_biguint().to_string(),"p_word":p.as_word(),"q_word":q.as_word(),
                                "exact_word_product":true});
                            result["status"]=json!("closed");
                        }
                    }
                }
            }
        }
        result["collision_count"]=json!(count.to_string());
        result["sic_mass_numerators"]=json!(masses.map(|v|v.to_string()));
        result["sic_denominator"]=json!(recovered.denominator.to_string());
    }
    result["seconds_before_serialization"]=json!(started.elapsed().as_secs_f64());
    result["retained"]=retain(&arena,&[prepared,state.observation])?;
    result["seconds"]=json!(started.elapsed().as_secs_f64());
    Ok(result)
}
