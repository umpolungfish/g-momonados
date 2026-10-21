//! Source-bound idempotent fixed-point projection on one resident register.
//! All residue alternatives are carried together; no candidate walk or external
//! factor producer is called. Numerical readout follows the complete residual.
use num_bigint::{BigInt,BigUint};
use num_traits::{One,Zero};
use serde_json::{json,Value};
use vox_core::resident_y::ResidentY;
use crate::{conventional_translation::BinaryTree,numeral_encodings::SourceForms,
    ququart_decision::DecisionArena,word_tape::WordTape};
use std::time::{Duration,Instant};

struct PointMembrane {
    n:BigUint,
    forms:SourceForms,
    arena:DecisionArena,
    root:usize,
    prepared:usize,
    before_projection:usize,
    e:Vec<usize>,
    residual:Vec<usize>,
    powers:Vec<BigUint>,
    row:usize,
    column:usize,
    subtract:bool,
    projected:bool,
    y:ResidentY,
    operations:u64,
}
impl PointMembrane {
    fn fold(&mut self) {
        let z=self.arena.zero();
        let mut roots=[self.root,self.prepared,self.before_projection,z,z];
        self.arena.fold(&mut roots);
        self.root=roots[0];self.prepared=roots[1];self.before_projection=roots[2];
    }
    fn prepare(n:BigUint)->Result<Self,String> {
        let forms=SourceForms::from_n(&n)?;
        let width=usize::try_from(n.bits()).map_err(|_|"source exceeds cell indexing")?;
        let e:Vec<_>=(0..width).map(|i|2*i).collect();
        let residual:Vec<_>=(0..width).map(|i|2*i+1).collect();
        let flag=2*width;
        let mut arena=DecisionArena::new(flag+1);
        let mut root=arena.basis(&BigUint::zero(),BigInt::one());
        // Equal unnormalised amplitudes on every E. DD sharing retains the
        // full family without visiting its individual residue assignments.
        for &wire in &e {
            let flipped=arena.flip(root,wire);root=arena.sum(root,flipped);
            let z=arena.zero();let mut roots=[root,z,z,z,z];arena.fold(&mut roots);root=roots[0];
        }
        root=arena.compare_constant(root,&e,&n,&[],flag);
        let z=arena.zero();root=arena.conditional_literals(z,root,&[(flag,true)]);
        root=arena.flip(root,flag);
        let mut literals:Vec<_>=e.iter().map(|&wire|(wire,false)).collect();
        root=arena.conditional_literals(root,z,&literals); // exclude E=0
        literals[0].1=true;
        root=arena.conditional_literals(root,z,&literals); // exclude E=1
        let expected=&n-BigUint::from(2u8);
        if arena.mass(root)!=expected{return Err("quotient support does not contain exactly N-2 nontrivial residues".into());}
        let mut powers=Vec::with_capacity(2*width);
        let mut power=BigUint::one();
        for _ in 0..2*width {powers.push(power.clone());power=(power<<1usize)%&n;}
        let mut state=Self{n,forms,arena,root,prepared:root,before_projection:root,e,residual,powers,
            row:0,column:0,subtract:false,projected:false,
            y:ResidentY::from_certified_program(ResidentY::certified_program())?,operations:0};
        state.fold();Ok(state)
    }
    fn arithmetic_step(&mut self)->Result<bool,String> {
        if self.projected {
            let zero=self.arena.zero();
            let literals:Vec<_>=self.residual.iter().map(|&wire|(wire,false)).collect();
            let returned=self.arena.conditional_literals(zero,self.root,&literals);
            if returned!=self.root {return Err("idempotent-locus projection did not reach its fixed point".into());}
            return Ok(true);
        }
        let width=self.e.len();
        if self.row==width {
            self.before_projection=self.root;
            let zero=self.arena.zero();
            let literals:Vec<_>=self.residual.iter().map(|&wire|(wire,false)).collect();
            self.root=self.arena.conditional_literals(zero,self.root,&literals);
            self.projected=true;self.fold();
            return Ok(false);
        }
        let (value,controls)=if self.subtract {
            let value=if self.powers[self.row].is_zero(){BigUint::zero()}else{&self.n-&self.powers[self.row]};
            (value,vec![(self.e[self.row],true)])
        }else{
            let i=self.row;let j=self.column;
            let value=if i==j{self.powers[i+j].clone()}else{(&self.powers[i+j]<<1usize)%&self.n};
            let controls=if i==j{vec![(self.e[i],true)]}else{vec![(self.e[i],true),(self.e[j],true)]};
            (value,controls)
        };
        self.root=self.arena.modular_add(self.root,&self.residual,&value,&self.n,&controls);
        self.operations+=1;self.fold();
        if self.subtract {self.row+=1;self.column=self.row;self.subtract=false;}
        else {self.column+=1;if self.column==width{self.subtract=true;}}
        Ok(false)
    }
    fn advance(&mut self)->Result<bool,String> {
        // Keep the same source-bound object in the certified Y payload.
        // The powerset bind transforms its complete correlated diagram.
        let mut y=self.y.clone();
        let done=y.advance(|| {
            let mut done=false;
            self.root=crate::reentry_powerset::bind_one(self.root,|root| {
                self.root=root;done=self.arithmetic_step()?;Ok::<_,String>(self.root)
            })?;
            Ok(done)
        })?;
        self.y=y;Ok(done)
    }
    fn snapshot(&self)->Value {
        json!({"schema":"conventional-point-continuation/v1","N":self.n.to_string(),
            "register":self.arena.prepared_record(self.root),
            "prepared_bank":self.arena.prepared_record(self.prepared),
            "pre_measurement_bank":self.arena.prepared_record(self.before_projection),
            "row":self.row,"column":self.column,"subtract":self.subtract,"projected":self.projected,
            "operations":self.operations,"Y_program":ResidentY::certified_program(),
            "Y_residual":self.y.residual_program(),"Y_calls":self.y.calls,"Y_contractions":self.y.contractions})
    }
}

pub fn prepare(n:BigUint)->Result<Value,String> {
    if n<=BigUint::from(2u8){return Err("proper-point construction requires N greater than two".into());}
    let started=Instant::now();
    let source=json!({"schema":"conventional-n-only/v1","N":n.to_string()});
    let model=crate::parasm::SelfModel::prepare()?;
    let bytes=serde_json::to_vec(&source).map_err(|e|e.to_string())?;
    if model.reflect(&bytes)!=bytes{return Err("native self-model changed the source".into());}
    eprintln!("conventional point stage=prepare source_bits={}",n.bits());
    let mut state=PointMembrane::prepare(n.clone())?;
    let register_initialization_seconds=started.elapsed().as_secs_f64();
    let trigger=Instant::now();
    eprintln!("conventional point stage=arithmetic nodes={} support=N-2",state.arena.retained_nodes());
    let done=loop {
        if state.advance()? {break true;}
        if state.operations%16==0 {
            eprintln!("conventional point op={} row={} column={} nodes={} elapsed={:.3}",
                state.operations,state.row,state.column,state.arena.retained_nodes(),trigger.elapsed().as_secs_f64());
        }
    };
    let mut factors=None;let mut reading=None;
    let mass=state.arena.mass(state.root);
    if done && !mass.is_zero() {
        // One complete joint readout, after the residual-zero projection.
        let joint=state.arena.sample_prefix(state.root,state.arena.cells,&BigUint::zero(),&BigUint::one())?;
        let mut point=BigUint::zero();
        for (i,&wire) in state.e.iter().enumerate(){if joint.bit(wire as u64){point|=BigUint::one()<<i;}}
        let source_word=WordTape::from_canonical_word(&state.forms.native).ok_or("lost native source word")?;
        let split=crate::idempotent_point::close(&source_word,&WordTape::from_biguint(&point))?;
        factors=Some([split.divisor.to_biguint().to_string(),split.quotient.to_biguint().to_string()]);
        reading=Some(json!({"point":point.to_string(),"complement":split.complement.to_biguint().to_string(),
            "divisor_word":split.divisor.as_word(),"quotient_word":split.quotient.as_word(),
            "zero_remainder":true,"exact_word_product":true,"paired_boundary":true}));
    }
    let status=if factors.is_some(){"arithmetic-closed"}else if done{"empty-point-support"}else{"held"};
    let binary=BinaryTree::from_value(&n);
    let retained_membrane=state.snapshot();
    let continuation=if done{None}else{Some(retained_membrane.clone())};
    Ok(json!({"schema":"conventional-prepared-point/v1","source":source,"input_boundary":"N-only",
        "status":status,"preparation_complete":done,"factors":factors,"product_verified":factors.is_some(),"reading":reading,
        "numeral_forms":{"native_word":state.forms.native,"hex_digit_word":state.forms.hex,
            "native_evaluation":state.forms.source_value()?.to_string(),"hex_evaluation":n.to_string(),
            "encoder_source":"shared with tfactor read","retained_through_Y":true},
        "binary_tree":{"positions":binary.positions,"evaluation":binary.evaluate().to_string()},
        "multiplicative_tree":{"operator":"proper-product-relation","equation":"P*Q=N",
            "evaluation_constraint":n.to_string(),"evaluated":factors.is_some(),"leaves":factors},
        "point_projection":{"operator":"retain E with E*E-E=0 mod N; exclude 0 and 1",
            "arithmetic_operations":state.operations,"complete":done,"support_mass":mass.to_string(),
            "retained_nodes":state.arena.retained_nodes(),"row":state.row,"column":state.column,
            "return_boundary":if done{"projection-complete"}else{"held"}},
        "Y":{"operator_calls":state.y.calls,"contractions":state.y.contractions,
            "residual_program":state.y.residual_program(),"continuation_serialized":continuation.is_some()},
        "continuation":continuation,"retained_membrane":retained_membrane,
        "register_initialization_seconds":register_initialization_seconds,
        "preparation_arithmetic_seconds":trigger.elapsed().as_secs_f64(),
        "preparation_seconds":started.elapsed().as_secs_f64(),
        "read_off_arithmetic_operations":0}))
}

/// Read the already evaluated factor boundary. This function cannot invoke
/// PointMembrane, ResidentY, modular additions, a projection or a factor producer.
/// Preparation and loading precede the read clock. Product replay follows it.
pub fn read_prepared(kind:&str,prepared:Value)->Result<Value,String> {
    if prepared["schema"]!="conventional-prepared-point/v1"
        || prepared["input_boundary"]!="N-only" {
        return Err("read requires a native N-only prepared-point artifact".into());
    }
    if prepared["preparation_complete"]!=true || prepared["status"]!="arithmetic-closed" {
        return Err("preparation has not produced numerical factor boundaries".into());
    }
    let n:BigUint=prepared["source"]["N"].as_str().ok_or("prepared source lost N")?
        .parse().map_err(|_|"invalid prepared N")?;
    let leaves=prepared["multiplicative_tree"]["leaves"].as_array()
        .filter(|leaves|leaves.len()==2).ok_or("prepared product tree has no two evaluated leaves")?;
    if prepared["factors"]!=prepared["multiplicative_tree"]["leaves"]
        || prepared["binary_tree"]["evaluation"]!=prepared["source"]["N"]
        || prepared["multiplicative_tree"]["evaluation_constraint"]!=prepared["source"]["N"]
        || prepared["point_projection"]["complete"]!=true
        || prepared["Y"]["residual_program"]!="DONE" {
        return Err("prepared factor boundary lost its source or completed continuation".into());
    }
    // Source-only preparation has already closed the multiplicative tree.
    // The read itself retrieves its two retained terminal marks.
    let trigger=Instant::now();
    let factors=[leaves[0].as_str().ok_or("invalid prepared divisor mark")?.to_owned(),
        leaves[1].as_str().ok_or("invalid prepared quotient mark")?.to_owned()];
    let read_off_seconds=trigger.elapsed().as_secs_f64();
    // Verification consumes emitted factors after read-off, with its own clock.
    let verification=Instant::now();
    let d:BigUint=factors[0].parse().map_err(|_|"invalid emitted divisor")?;
    let q:BigUint=factors[1].parse().map_err(|_|"invalid emitted quotient")?;
    if d<=BigUint::one() || d>=n || q<=BigUint::one() || &d*&q!=n || &n%&d!=BigUint::zero() {
        return Err("emitted factors failed exact source-product verification".into());
    }
    let product_verification_seconds=verification.elapsed().as_secs_f64();
    let names=if kind=="both"{vec!["srfp","dnn"]}else{vec![kind]};
    let mut result=prepared;
    result["schema"]=json!("conventional-n-only-return/v3");
    result["reports"]=json!(names.into_iter().map(|name|json!({"structure":name,
        "status":"arithmetic-closed","all_predicates_hold":false,
        "reading":result["reading"],"factor_product_verified":true})).collect::<Vec<_>>());
    result["factors"]=json!(factors);
    result["product_verified"]=json!(true);
    result["read_off_seconds"]=json!(read_off_seconds);
    result["product_verification_seconds"]=json!(product_verification_seconds);
    result["trigger_to_return_seconds"]=json!(read_off_seconds+product_verification_seconds);
    result["read_off_arithmetic_operations"]=json!(0);
    result["read_off_boundary"]=json!("two evaluated product-tree leaves; numerical preparation already complete");
    Ok(result)
}

pub fn execute(kind:&str,n:BigUint)->Result<Value,String> {
    read_prepared(kind,prepare(n)?)
}

/// A source-only negative control for the actual modular oracle. N-1 is
/// derived inside this instrument; no factor, idempotent or order is input.
pub fn source_residual_control(n:BigUint)->Result<Value,String> {
    if n.bits()<200 {return Err("source control requires at least 200 bits".into());}
    let mut state=PointMembrane::prepare(n.clone())?;
    let point=&n-BigUint::one();
    let zero=state.arena.zero();
    let literals:Vec<_>=state.e.iter().enumerate().map(|(i,&wire)|(wire,point.bit(i as u64))).collect();
    state.root=state.arena.conditional_literals(zero,state.root,&literals);
    state.fold();
    if state.arena.mass(state.root)!=BigUint::one(){return Err("source-derived control lost its basis state".into());}
    let started=Instant::now();
    while state.row<state.e.len() {
        state.advance()?;
        if started.elapsed()>Duration::from_secs(300){return Err("source residual control reached its execution bound".into());}
    }
    let sample=state.arena.sample_prefix(state.root,state.arena.cells,&BigUint::zero(),&BigUint::one())?;
    let mut residual=BigUint::zero();
    for (i,&wire) in state.residual.iter().enumerate(){if sample.bit(wire as u64){residual|=BigUint::one()<<i;}}
    let expected=BigUint::from(2u8)%&n;
    if residual!=expected {return Err("native square-minus-self oracle failed on the source-derived N-1 state".into());}
    while !state.advance()? {}
    if !state.arena.mass(state.root).is_zero(){return Err("zero-residual projection admitted the N-1 nonpoint".into());}
    Ok(json!({"N":n.to_string(),"source_derived_control":"E=N-1","native_residual":residual.to_string(),
        "expected_residual":expected.to_string(),"postselection_rejected":true,
        "arithmetic_operations":state.operations,"Y_calls":state.y.calls,
        "elapsed_seconds":started.elapsed().as_secs_f64()}))
}
