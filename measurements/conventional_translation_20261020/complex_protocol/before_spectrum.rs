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
    protocol_banks:Vec<usize>,
    protocol_returns:Vec<Value>,
}
impl PointMembrane {
    fn fold(&mut self) {
        let mut roots=vec![self.root,self.prepared,self.before_projection];
        roots.extend_from_slice(&self.protocol_banks);
        self.arena.fold_roots(&roots);
    }
    fn prepare(n:BigUint)->Result<Self,String> {
        let forms=SourceForms::from_n(&n)?;
        let width=usize::try_from(n.bits()).map_err(|_|"source exceeds cell indexing")?;
        let e:Vec<_>=(0..width).map(|i|2*i).collect();
        let residual:Vec<_>=(0..2*width).map(|i|2*i+1).collect();
        let flag=4*width;
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
            y:ResidentY::from_certified_program(ResidentY::certified_program())?,operations:0,protocol_banks:Vec::new(),protocol_returns:Vec::new()};
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
        // e_i²=e_i combines the diagonal and linear terms. All cross
        // terms of this row form one shifted source-register addition.
        let i=self.row;let row_started=Instant::now();
        eprintln!("conventional affine_row={i} stage=enter nodes={}",self.arena.retained_nodes());
        let correction=(&self.powers[2*i]+&self.n-&self.powers[i])%&self.n;
        self.root=self.arena.modular_affine_add(self.root,&self.residual,&self.e[i+1..],
            2*i+2,&correction,&self.n,&[(self.e[i],true)])?;
        self.operations+=1;self.row+=1;self.column=self.row;self.subtract=false;
        if self.residual[width..].iter().any(|&wire|!self.arena.wire_is_zero(self.root,wire)) {
            return Err("composed affine row left a noncanonical modular residue".into());
        }
        self.fold();
        eprintln!("conventional affine_row={i} stage=return nodes={} elapsed={:.9}",
            self.arena.retained_nodes(),row_started.elapsed().as_secs_f64());
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
    fn complement(&mut self)->Result<(),String> {
        // F(E)=E²-E is invariant under E -> 1-E modulo N. On 2<=E<N,
        // its canonical representative is N+1-E. Keep residual and all banks.
        for &wire in &self.e { self.root=self.arena.flip(self.root,wire); }
        self.root=self.arena.add_constant(self.root,&self.e,&(&self.n+2u8),&[]);
        self.fold();Ok(())
    }
    fn project_point(&mut self) {
        self.before_projection=self.root;
        let zero=self.arena.zero();
        let literals:Vec<_>=self.residual.iter().map(|&wire|(wire,false)).collect();
        self.root=self.arena.conditional_literals(zero,self.root,&literals);
        self.projected=true;self.fold();
    }
    fn protocol(&mut self,kind:&str)->Result<(),String> {
        use imasm_core::{check::Graph,classic::Token,flow,imasm16_3::Reg16_3};
        use crate::reentry_powerset::{Held,unit};
        let raw=match kind {
            "srfp"=>include_str!("../../ob3ect/digital/self_reading_fixed_point/self_reading_fixed_point_ob3ect.json"),
            "dnn"=>include_str!("../../ob3ect/digital/dual_named_numerals/dual_named_numerals_ob3ect.json"),
            _=>return Err("unknown numerical preparation protocol".into()),
        };
        let definition:Value=serde_json::from_str(raw).map_err(|e|e.to_string())?;
        let wiring=&definition["wiring_check"];
        let mut graph=Graph::new();
        for node in wiring["nodes"].as_array().ok_or("protocol has no native nodes")? {
            graph.add(Token::parse(node.as_str().ok_or("invalid native node")?).ok_or("unsupported native protocol node")?);
        }
        let word=graph.nodes.iter().map(|op|op.code()).collect::<String>();
        if definition["glyph_word"].as_str()!=Some(word.as_str()) {
            return Err("protocol word differs from native node sequence".into());
        }
        for edge in wiring["wires"].as_array().ok_or("protocol has no native wiring")? {
            let from=edge[0].as_u64().ok_or("invalid native edge source")? as usize;
            let to=edge[2].as_u64().ok_or("invalid native edge return")? as usize;
            if from>=to || to>=graph.nodes.len(){return Err("protocol edge escaped its ordered graph".into());}
            graph.connect(from,to);
        }
        if !graph.validate().is_empty() || !matches!(graph.closure_state(),imasm_core::check::ClosureState::Closed(_)) {
            return Err("native protocol wiring failed closure validation".into());
        }
        let seed=Reg16_3::from_name("A").ok_or("native carrier lost A")?;
        let (reference_inputs,reference_edges)=flow::flow_values(&graph,seed);
        let mut edge_values=vec![Reg16_3::default();graph.edges.len()];
        let mut edge_payload:Vec<Held<usize>>=vec![Held::new();graph.edges.len()];
        let entry=self.root;self.protocol_banks.push(entry);self.fold();
        let mut trace=Vec::new();let mut returned=Held::new();let mut fixed=false;
        for (node,&op) in graph.nodes.iter().enumerate() {
            let mut input=Reg16_3::default();let mut incoming=Held::new();
            for (edge,&(_,to)) in graph.edges.iter().enumerate(){if to==node {
                input=input.union(edge_values[edge]);incoming.extend(edge_payload[edge].iter().copied());
            }}
            if input!=reference_inputs[node] {return Err("executed node diverged from native graph flow".into());}
            if op==Token::Vinit {incoming=unit(entry);}
            if incoming.is_empty(){return Err("protocol node received no resident payload".into());}
            self.protocol_banks.extend(incoming.iter().copied());self.fold();
            let before=self.operations;let before_y=self.y.calls;
            let mut output=Held::new();
            eprintln!("conventional protocol={kind} node={node} opcode={} stage=enter",op.name());
            for root in incoming.iter().copied() {
                self.root=root;
                match op {
                    Token::Vinit|Token::Tanch|Token::Fsplit|Token::Ffuse|Token::Engagr=>{},
                    Token::Imscrib|Token::Clink=>{
                        if self.forms.source_value()?!=self.n {return Err("protocol changed its source binding".into());}
                    },
                    Token::Afwd=>{
                        // The native forward node applies the numerical operator
                        // through the retained Y payload. Projection belongs to
                        // the executed evaluation nodes, rather than this loop.
                        while self.row<self.e.len() {
                            self.advance()?;
                            if self.operations%16==0 {
                                eprintln!("conventional protocol={kind} op={} row={} column={} nodes={}",
                                    self.operations,self.row,self.column,self.arena.retained_nodes());
                            }
                        }
                    },
                    Token::Arev=>self.complement()?,
                    Token::Evalt=>self.project_point(),
                    Token::Evalf=>{self.complement()?;self.project_point();},
                    Token::Ifix=>{
                        if !self.projected || !self.arithmetic_step()? {
                            return Err("protocol fixation preceded numerical projection".into());
                        }
                        self.advance()?;fixed=true;
                    },
                    _=>return Err("unimplemented native numerical protocol operation".into()),
                }
                self.root=crate::reentry_powerset::bind_one(self.root,Ok::<_,String>)?;
                output.insert(self.root);self.protocol_banks.push(self.root);self.fold();
            }
            if op==Token::Tanch {
                if !fixed{return Err("protocol returned before fixation".into());}
                returned=output.clone();
            }
            let fan=graph.out_degree(node);let mut slot=0;
            for (edge,&(from,_)) in graph.edges.iter().enumerate(){if from==node {
                edge_values[edge]=flow::gate_out(op,input,seed,slot,fan);
                edge_payload[edge]=output.clone();slot+=1;
            }}
            trace.push(json!({"node":node,"opcode":op.name(),"input_carrier":input.name(),
                "incoming_roots":incoming,"returned_roots":output,
                "arithmetic_operations":self.operations-before,"Y_calls":self.y.calls-before_y,
                "retained_nodes":self.arena.retained_nodes()}));
        }
        if edge_values!=reference_edges{return Err("executed edges diverged from native graph flow".into());}
        if returned.is_empty(){return Err("native protocol omitted its terminal return".into());}
        self.root=*returned.iter().next().unwrap();
        self.protocol_banks.extend(returned.iter().copied());self.fold();
        self.protocol_returns.push(json!({"kind":kind,"word":word,"graph_edges":graph.edges,
            "source":self.n.to_string(),"nodes":trace,"terminal_roots":returned,
            "edge_carriers":edge_values.iter().map(|v|v.name()).collect::<Vec<_>>(),
            "edge_payloads":edge_payload,"fixation_executed":fixed,"native_flow_agreement":true}));
        Ok(())
    }
    fn snapshot(&self)->Value {
        let mut roots=vec![self.root,self.prepared,self.before_projection];
        roots.extend_from_slice(&self.protocol_banks);
        json!({"schema":"conventional-point-continuation/v1","N":self.n.to_string(),
            "register":self.arena.prepared_record(self.root),
            "prepared_bank":self.arena.prepared_record(self.prepared),
            "pre_measurement_bank":self.arena.prepared_record(self.before_projection),
            "row":self.row,"column":self.column,"subtract":self.subtract,"projected":self.projected,
            "point_wires":self.e,"residual_wires":self.residual,"arithmetic_unit":"composed-affine-row",
            "operations":self.operations,"Y_program":ResidentY::certified_program(),
            "Y_residual":self.y.residual_program(),"Y_calls":self.y.calls,"Y_contractions":self.y.contractions,
            "protocol_bank_roots":self.protocol_banks,"retained_root_order":roots,
            "shared_protocol_register":self.arena.prepared_record_many(&roots),
            "protocol_execution":self.protocol_returns})
    }
}

pub fn prepare(n:BigUint)->Result<Value,String> {prepare_for("both",n)}

pub fn prepare_for(kind:&str,n:BigUint)->Result<Value,String> {
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
    let protocols=match kind {"both"=>vec!["srfp","dnn"],"srfp"=>vec!["srfp"],"dnn"=>vec!["dnn"],_=>return Err("invalid protocol selection".into())};
    for protocol in protocols {state.protocol(protocol)?;}
    let done=state.projected;
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
    Ok(json!({"schema":"conventional-prepared-point/v2","source":source,"input_boundary":"N-only",
        "status":status,"preparation_complete":done,"protocol_kind":kind,"protocol_execution":state.protocol_returns,"factors":factors,"product_verified":factors.is_some(),"reading":reading,
        "numeral_forms":{"native_word":state.forms.native,"hex_digit_word":state.forms.hex,
            "native_evaluation":state.forms.source_value()?.to_string(),"hex_evaluation":n.to_string(),
            "encoder_source":"shared with tfactor read","retained_through_Y":true},
        "binary_tree":{"positions":binary.positions,"evaluation":binary.evaluate().to_string()},
        "multiplicative_tree":{"operator":"proper-product-relation","equation":"P*Q=N",
            "evaluation_constraint":n.to_string(),"evaluated":factors.is_some(),"leaves":factors},
        "point_projection":{"operator":"retain E with E*E-E=0 mod N; exclude 0 and 1",
            "arithmetic_operations":state.operations,"arithmetic_unit":"composed-affine-row","complete":done,"support_mass":mass.to_string(),
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
    if prepared["schema"]!="conventional-prepared-point/v2"
        || prepared["input_boundary"]!="N-only" {
        return Err("read requires a native N-only prepared-point artifact".into());
    }
    let actual=prepared["protocol_kind"].as_str().ok_or("prepared artifact omitted protocol selection")?;
    if !matches!(actual,"srfp"|"dnn"|"both") || (kind!="both" && kind!=actual && actual!="both") {
        return Err("read selection differs from executed preparation protocol".into());
    }
    let expected=if actual=="both" {vec!["srfp","dnn"]} else {vec![actual]};
    let traces=prepared["protocol_execution"].as_array().ok_or("missing executed protocol traces")?;
    if traces.len()!=expected.len() || traces.iter().zip(&expected).any(|(trace,name)|
        trace["kind"]!=*name || trace["source"]!=prepared["source"]["N"] ||
        trace["fixation_executed"]!=true || trace["nodes"].as_array().map_or(true,|nodes|nodes.len()!=12)) {
        return Err("prepared artifact omitted complete source-bound protocol execution".into());
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
    let names=if kind=="both"{expected.into_iter().map(str::to_owned).collect::<Vec<_>>()}else{vec![kind.to_owned()]};
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
    read_prepared(kind,prepare_for(kind,n)?)
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

/// Execute both saved graphs on the source-derived nonpoint N-1. This is a
/// negative numerical-path instrument, not a factor production run.
pub fn source_protocol_control(n:BigUint)->Result<Value,String> {
    if n.bits()<200{return Err("protocol control requires at least 200 bits".into());}
    let started=Instant::now();
    let mut receipts=Vec::new();
    for kind in ["srfp","dnn"] {
        let mut state=PointMembrane::prepare(n.clone())?;
        let point=&n-BigUint::one();
        let zero=state.arena.zero();
        let literals:Vec<_>=state.e.iter().enumerate().map(|(i,&wire)|(wire,point.bit(i as u64))).collect();
        state.root=state.arena.conditional_literals(zero,state.root,&literals);
        state.prepared=state.root;state.before_projection=state.root;state.fold();
        if state.arena.mass(state.root)!=BigUint::one(){return Err("control lost its source-derived basis state".into());}
        let initial=state.root;
        state.complement()?;state.complement()?;
        if state.root!=initial{return Err("numerical parity reversal failed involution".into());}
        state.protocol(kind)?;
        if !state.projected || !state.arena.mass(state.root).is_zero() {
            return Err("executed protocol admitted the N-1 nonpoint".into());
        }
        let forward=state.protocol_returns[0]["nodes"][2]["returned_roots"][0].as_u64()
            .ok_or("forward trace lost its numerical bank")? as usize;
        if state.arena.mass(forward)!=BigUint::one(){return Err("forward operation lost the control state".into());}
        let joint=state.arena.sample_prefix(forward,state.arena.cells,&BigUint::zero(),&BigUint::one())?;
        let decode=|wires:&[usize]| wires.iter().enumerate().fold(BigUint::zero(),|mut value,(i,&wire)| {
            if joint.bit(wire as u64){value|=BigUint::one()<<i;}value
        });
        let observed_point=decode(&state.e);let residual=decode(&state.residual);
        let expected=(&point*&point-&point)%&n;
        if observed_point!=point || residual!=expected || expected!=BigUint::from(2u8) {
            return Err("executed forward oracle changed the control point or its exact residual".into());
        }
        // All live banks are serialized together. Check recovery of every
        // retained root and its full complex coefficient diagram.
        let mut roots=vec![state.root,state.prepared,state.before_projection];
        roots.extend_from_slice(&state.protocol_banks);
        let record=state.arena.prepared_record_many(&roots);
        let (recovered,recovered_roots)=DecisionArena::from_prepared_record_many(&record)?;
        if record!=recovered.prepared_record_many(&recovered_roots) {
            return Err("protocol bank recovery changed its shared diagram".into());
        }
        let mut corrupt=record.clone();
        corrupt["roots"][0]=json!(record["nodes"].as_array().unwrap().len()+1);
        if DecisionArena::from_prepared_record_many(&corrupt).is_ok() {
            return Err("protocol bank recovery admitted an escaped root".into());
        }
        receipts.push(json!({"kind":kind,"execution":state.protocol_returns,
            "source_derived_control":"E=N-1","postselection_rejected":true,
            "forward_point":observed_point.to_string(),"native_residual":residual.to_string(),
            "expected_residual":expected.to_string(),"parity_reversal_involution":true,"shared_bank_roundtrip":true,
            "escaped_bank_rejected":true,"retained_banks":roots.len(),
            "arithmetic_operations":state.operations,"Y_calls":state.y.calls}));
    }
    Ok(json!({"schema":"conventional-protocol-control/v1","N":n.to_string(),
        "source_bits":n.bits(),"controls":receipts,"elapsed_seconds":started.elapsed().as_secs_f64()}))
}

/// Compare complete-family composed rows with the retained elementary
/// operator on the same arena. No factor witnesses or reference legs enter.
pub fn source_affine_prefix_control(n:BigUint,rows:usize)->Result<Value,String> {
    if n.bits()<200 || rows==0 || rows>2{return Err("prefix control requires >=200 bits and one or two rows".into());}
    let mut state=PointMembrane::prepare(n.clone())?;let mut receipts=Vec::new();
    for i in 0..rows {
        let before=state.root;let started=Instant::now();state.advance()?;
        let composed=state.root;let composed_seconds=started.elapsed().as_secs_f64();
        state.protocol_banks.push(composed);state.protocol_banks.push(before);state.fold();
        let slot=state.protocol_banks.len()-1;let started=Instant::now();let mut baseline=before;
        for j in i..state.e.len() {
            let value=if i==j{state.powers[i+j].clone()}else{(&state.powers[i+j]<<1usize)%&n};
            let controls=if i==j{vec![(state.e[i],true)]}else{vec![(state.e[i],true),(state.e[j],true)]};
            baseline=state.arena.modular_add(baseline,&state.residual,&value,&n,&controls);
            state.protocol_banks[slot]=baseline;state.fold();
        }
        let value=(&n-&state.powers[i])%&n;
        baseline=state.arena.modular_add(baseline,&state.residual,&value,&n,&[(state.e[i],true)]);
        state.protocol_banks[slot]=baseline;state.fold();
        if composed!=baseline{return Err("composed affine row differs from complete elementary family".into());}
        receipts.push(json!({"row":i,"complete_family_root_equal":true,
            "composed_seconds":composed_seconds,"elementary_seconds":started.elapsed().as_secs_f64(),
            "composed_Y_calls":1,"elementary_modular_calls":state.e.len()-i+1,
            "retained_nodes":state.arena.retained_nodes()}));
    }
    Ok(json!({"N":n.to_string(),"source_bits":n.bits(),"input_boundary":"N-only",
        "scope":"complete retained family; two operator implementations, same mathematical rows",
        "rows":receipts}))
}
