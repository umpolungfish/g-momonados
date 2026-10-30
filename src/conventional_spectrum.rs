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


struct PreparedNativeGraph {
    graph:imasm_core::check::Graph,
    word:String,
    reference_inputs:Vec<imasm_core::imasm16_3::Reg16_3>,
    reference_edges:Vec<imasm_core::imasm16_3::Reg16_3>,
}
fn prepare_native_graph(kind:&str)->Result<PreparedNativeGraph,String>{
    use imasm_core::{check::Graph,classic::Token,flow,imasm16_3::Reg16_3};
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
        Ok(PreparedNativeGraph{graph,word,reference_inputs,reference_edges})

}

struct PointMembrane {
    copy_return_ancestry:Vec<(usize,usize,bool)>,
    n:BigUint,
    forms:SourceForms,
    wells:crate::reentry_wells::SourceWells,
    arena:DecisionArena,
    root:usize,
    prepared:usize,
    before_projection:usize,
    e:Vec<usize>,
    residual:Vec<usize>,
    coordinates:[usize;2],
    fork_history:[usize;2],
    engagement_history:[[usize;2];2],
    powers:Vec<BigUint>,
    row:usize,
    column:usize,
    subtract:bool,
    reduced:bool,
    projected:bool,
    y:ResidentY,
    operations:u64,
    logical_contractions:usize,
    protocol_banks:Vec<usize>,
    protocol_returns:Vec<Value>,
    projector:Option<crate::idempotent_projector::Projector>,
    prepared_protocol:crate::conventional_protocol::PreparedProtocol,
    native_graphs:[PreparedNativeGraph;2],
    moment_operator:crate::idempotent_moment::PreparedMoment,
    complement_word:WordTape,
}
impl PointMembrane {
    fn fold(&mut self) {
        let mut roots=vec![self.root,self.prepared,self.before_projection];
        roots.extend_from_slice(&self.protocol_banks);
        if let Some(projector)=&self.projector {roots.extend(projector.roots());}
        self.arena.fold_roots(&roots);
    }
    fn prepare(n:BigUint)->Result<Self,String> {
        let forms=SourceForms::from_n(&n)?;
        let source_word=WordTape::from_canonical_word(&forms.native).ok_or("source well lost its native word")?;
        let wells=crate::reentry_wells::SourceWells::new(source_word)?;
        let width=wells.binary.width();
        let coordinates=[0,1];
        let e:Vec<_>=(0..width).map(|i|2*i+2).collect();
        let residual:Vec<_>=(0..2*width+1).map(|i|2*i+3).collect();
        let flag=4*width+4;
        let fork_history=[flag+1,flag+2];
        let engagement_history=[[flag+3,flag+4],[flag+5,flag+6]];
        let mut arena=DecisionArena::new(flag+7);
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
        root=crate::conventional_protocol::prepare(&mut arena,root,coordinates)?;
        for bit in 0..2.min(width){root=arena.controlled_flip(root,coordinates[bit],&[e[bit]]);}
        if arena.mass(root)!=expected*4u8{return Err("complex coordinate preparation lost its source correlation".into());}
        let powers=wells.powers_integer(2*width)?;
        let prepared_protocol=crate::conventional_protocol::PreparedProtocol::prepare(n.bits())?;
        let moment_operator=crate::idempotent_moment::PreparedMoment::prepare(&n,e.len());
        let complement_word=WordTape::from_biguint(&(&n+2u8));
        let mut state=Self{copy_return_ancestry:Vec::new(),n,forms,wells,arena,root,prepared:root,before_projection:root,e,residual,coordinates,fork_history,engagement_history,powers,
            row:0,column:0,subtract:false,reduced:false,projected:false,
            y:ResidentY::from_certified_program(ResidentY::certified_program())?,operations:0,logical_contractions:0,protocol_banks:Vec::new(),protocol_returns:Vec::new(),projector:None,
            prepared_protocol,
            native_graphs:[prepare_native_graph("srfp")?,prepare_native_graph("dnn")?],moment_operator,complement_word};
        state.fold();Ok(state)
    }
    fn arithmetic_step(&mut self)->Result<bool,String> {
        if let Some(projector)=self.projector.as_mut() {
            let done=projector.advance(&mut self.arena)?;
            self.logical_contractions=projector.contractions;
            self.operations+=1;
            if done {
                self.copy_return_ancestry=projector.copy_returns.clone();
                self.before_projection=self.root;
                let mut banks=vec![self.prepared,self.before_projection];
                banks.extend_from_slice(&self.protocol_banks);
                self.root=self.projector.take().unwrap().finish_banked(&mut self.arena,&banks)?;
                self.projected=true;self.reduced=true;self.row=self.e.len();
            }
            if done||self.operations%32==0 {self.fold();}
            return Ok(done);
        }
        if self.projected {
            let zero=self.arena.zero();
            let literals:Vec<_>=self.residual.iter().map(|&wire|(wire,false)).collect();
            let returned=self.arena.conditional_literals(zero,self.root,&literals);
            if returned!=self.root {return Err("idempotent-locus projection did not reach its fixed point".into());}
            return Ok(true);
        }
        let width=self.e.len();
        if self.row==width {
            if !self.reduced {
                self.protocol_banks.push(self.root);self.fold();
                self.root=self.arena.reduce_register(self.root,&self.residual,&self.n)?;
                self.reduced=true;self.operations+=1;self.fold();
                if self.residual[width..].iter().any(|&wire|!self.arena.wire_is_zero(self.root,wire)) {
                    return Err("terminal source reduction left a noncanonical residue".into());
                }
                return Ok(false);
            }
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
        let correction=&self.powers[2*i]-&self.powers[i];
        self.root=self.arena.integer_affine_add(self.root,&self.residual,&self.e[i+1..],
            2*i+2,&correction,&[(self.e[i],true)])?;
        self.operations+=1;self.row+=1;self.column=self.row;self.subtract=false;
        if !self.arena.wire_is_zero(self.root,*self.residual.last().unwrap()) {
            return Err("integer square-minus-self row overflowed its source square".into());
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
        self.root=self.arena.add_constant_word(self.root,&self.e,&self.complement_word,&[]);
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
        use imasm_core::{classic::Token,flow,imasm16_3::Reg16_3};
        use crate::reentry_powerset::{Held,unit};
        let seed=Reg16_3{big_t:true,big_f:true,small_t:true,small_f:true};
        let selected=&self.native_graphs[usize::from(kind=="dnn")];
        if !matches!(kind,"srfp"|"dnn"){return Err("unknown numerical preparation protocol".into());}
        let graph=selected.graph.clone();let word=selected.word.clone();
        let reference_inputs=selected.reference_inputs.clone();
        let reference_edges=selected.reference_edges.clone();
        let mut edge_values=vec![Reg16_3::default();graph.edges.len()];
        let mut edge_payload:Vec<Held<usize>>=vec![Held::new();graph.edges.len()];
        let protocol_slot=usize::from(kind=="dnn");
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
            let ancestral_incoming=incoming.clone();
            let mut effects=Vec::new();
            if op==Token::Ffuse {
                let mut joined=self.arena.zero();let mut mass=BigUint::zero();
                for &root in &incoming{mass+=self.arena.mass(root);joined=self.arena.sum(joined,root);}
                if self.arena.mass(joined)!=mass{return Err("ancestry-tagged fuse lost its disjoint carried arms".into());}
                let recovered=crate::conventional_protocol::fuse(&mut self.arena,joined,self.fork_history[protocol_slot]);
                effects.push(json!({"operation":"coherent direct-sum retraction mu","input_roots":incoming,
                    "tagged_root":joined,"output_root":recovered,"ancestry_retained_in_bank":true,
                    "history_wire":self.fork_history[protocol_slot],"history_cleared":self.arena.wire_is_zero(recovered,self.fork_history[protocol_slot]),
                    "tagged_mass_preserved":true}));
                incoming=unit(recovered);self.protocol_banks.extend([joined,recovered]);self.fold();
            }
            let before=self.operations;let before_y=self.y.calls;let before_contractions=self.logical_contractions;
            let mut output=Held::new();
            eprintln!("conventional protocol={kind} node={node} opcode={} stage=enter",op.name());
            for root in incoming.iter().copied() {
                self.root=root;
                match op {
                    Token::Vinit|Token::Tanch|Token::Fsplit|Token::Ffuse=>{},
                    Token::Imscrib=>{
                        effects.push(crate::conventional_protocol::source_reflect_prepared(&self.arena,self.root,&self.n,&word,&self.prepared_protocol.model)?);
                    },
                    Token::Clink=>{
                        if self.forms.source_value()?!=self.n {return Err("protocol changed its source binding".into());}
                        let (root,banks,witness)=crate::conventional_protocol::clink_prepared(&mut self.arena,self.root,self.coordinates,&self.prepared_protocol)?;
                        self.root=root;self.protocol_banks.extend(banks);effects.push(witness);
                    },
                    Token::Engagr=>{
                        let (root,witness)=crate::conventional_protocol::engage_prepared(&mut self.arena,self.root,self.coordinates,
                            self.engagement_history[protocol_slot],&self.prepared_protocol)?;
                        self.root=root;effects.push(witness);
                    },
                    Token::Afwd=>{
                        // The native forward node applies the numerical operator
                        // through the retained Y payload. Projection belongs to
                        // the executed evaluation nodes, rather than this loop.
                        while !self.reduced {
                            self.advance()?;
                            if self.operations%16==0 {
                                eprintln!("conventional protocol={kind} op={} row={} column={} nodes={}",
                                    self.operations,self.row,self.column,self.arena.retained_nodes());
                            }
                        }
                    },
                    Token::Arev=>{
                        self.complement()?;self.root=self.arena.flip(self.root,self.coordinates[0]);
                        effects.push(json!({"operation":"numerical complement and native coordinate involution",
                            "coordinate_flip_wire":self.coordinates[0]}));
                    },
                    Token::Evalt=>self.project_point(),
                    Token::Evalf=>{self.complement()?;self.project_point();},
                    Token::Ifix=>{
                        if !self.projected || !self.arithmetic_step()? {
                            return Err("protocol fixation preceded numerical projection".into());
                        }
                        self.advance()?;
                        let (root,banks,witness)=crate::conventional_protocol::fix_phase_prepared(&mut self.arena,self.root,self.coordinates,&self.prepared_protocol)?;
                        self.root=root;self.protocol_banks.extend(banks);effects.push(witness);fixed=true;
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
                edge_payload[edge]=if op==Token::Fsplit {
                    let carrier=edge_values[edge];
                    let mask=u8::from(carrier.big_t)|(u8::from(carrier.big_f)<<1)
                        |(u8::from(carrier.small_t)<<2)|(u8::from(carrier.small_f)<<3);
                    let mut family=Held::new();
                    for &root in &output{
                        let history=self.fork_history[protocol_slot];
                        let (arms,tagged,witness)=crate::conventional_protocol::split(&mut self.arena,root,self.coordinates,history)?;
                        let arm=match mask{5=>0,10=>1,_=>return Err("native fork is not the complementary truth/falsity split".into())};
                        let selected=arms[arm];
                        effects.push(witness);
                        family.insert(selected);self.protocol_banks.extend([arms[0],arms[1],tagged]);
                    }
                    effects.push(json!({"operation":"native coordinate fork projection","edge":edge,"slot":slot,
                        "coordinate_mask":mask,"history_wire":self.fork_history[protocol_slot],"returned_roots":family}));
                    self.fold();family
                }else{output.clone()};slot+=1;
            }}
            trace.push(json!({"node":node,"opcode":op.name(),"input_carrier":input.name(),
                "incoming_roots":ancestral_incoming,"returned_roots":output,"executed_operations":effects,
                "arithmetic_operations":self.operations-before,"Y_calls":self.y.calls-before_y,
                "logical_contractions":self.logical_contractions-before_contractions,
                "retained_nodes":self.arena.retained_nodes()}));
        }
        if edge_values!=reference_edges{return Err("executed edges diverged from native graph flow".into());}
        if returned.is_empty(){return Err("native protocol omitted its terminal return".into());}
        if returned.len()!=1{return Err("terminal return did not contain one complete correlated register".into());}
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
        if let Some(projector)=&self.projector {roots.extend(projector.roots());}
        json!({"schema":"conventional-point-continuation/v1","N":self.n.to_string(),
            "reentry_wells":self.wells.record(),
            "register":self.arena.prepared_record(self.root),
            "prepared_bank":self.arena.prepared_record(self.prepared),
            "pre_measurement_bank":self.arena.prepared_record(self.before_projection),
            "row":self.row,"column":self.column,"subtract":self.subtract,"reduced":self.reduced,"projected":self.projected,
            "point_wires":self.e,"residual_wires":self.residual,"coordinate_wires":self.coordinates,
            "fork_history_wires":self.fork_history,"engagement_history_wires":self.engagement_history,
            "arithmetic_unit":"integer-affine-row-single-reduction",
            "contracted_projector":self.projector.as_ref().map(|p|p.record()),
            "copy_return_ancestry":self.projector.as_ref().map_or(&self.copy_return_ancestry,|p|&p.copy_returns),
            "operations":self.operations,"logical_contractions":self.logical_contractions,"Y_program":ResidentY::certified_program(),
            "Y_residual":self.y.residual_program(),"Y_calls":self.y.calls,"Y_contractions":self.y.contractions,
            "protocol_bank_roots":self.protocol_banks,"retained_root_order":roots,
            "shared_protocol_register":self.arena.prepared_record_many(&roots),
            "protocol_execution":self.protocol_returns})
    }
}

pub fn prepare(n:BigUint)->Result<Value,String> {prepare_for("both",n)}

/// Contract the zero boundary together with its local arithmetic tensors.
/// The source register, original banks and Y evaluator belong to one carrier.
/// Zero seconds leaves execution uncapped; positive limits are explicit diagnostics.
fn compile_point(n:BigUint)->Result<PointMembrane,String>{
    if n.bits()<100{return Err("contracted source requires at least 100 bits".into());}
    let mut state=PointMembrane::prepare(n.clone())?;
    state.projector=Some(crate::idempotent_projector::Projector::compile(
        &mut state.arena,state.root,&state.e,&n)?);
    state.fold();Ok(state)
}
pub fn prepare_contracted(n:BigUint,seconds:u64)->Result<Value,String>{
    let started=Instant::now();let state=compile_point(n.clone())?;
    run_contracted(state,n,seconds,started.elapsed().as_secs_f64())
}
/// Complete numerical preparation inside Cargo. A final switch image may only
/// be emitted after the N-only computation has produced an exact factor pair.
pub fn compile_final_switch(n:BigUint)->Result<Value,String>{
    let started=Instant::now();
    let mut returned=prepare_contracted(n.clone(),0)?;
    let factors=returned["factors"].as_array().filter(|v|v.len()==2)
        .ok_or("numerical preparation returned without two factor marks")?;
    let parse=|v:&Value|v.as_str().ok_or("factor mark is not a numeral")?
        .parse::<BigUint>().map_err(|_|"invalid factor mark");
    let d=parse(&factors[0])?;let q=parse(&factors[1])?;
    if d<=BigUint::one() || d>=n || q<=BigUint::one()
        || &n%&d!=BigUint::zero() || &d*&q!=n
        || returned["product_verified"]!=true || returned["projection_complete"]!=true {
        return Err("Cargo numerical preparation failed exact factor closure".into());
    }
    returned["schema"]=json!("cargo-evaluated-point-switch/v1");
    returned["preparation_seconds"]=json!(started.elapsed().as_secs_f64());
    returned["preparation_boundary"]=json!("projection, protocol, SIC moments and exact factor closure completed in Cargo");
    returned["source_inputs_at_trigger"]=json!(0);
    returned["numerical_operations_at_trigger"]=json!(0);
    // The original contraction clock now belongs to compilation.
    let evaluation_seconds=returned.as_object_mut().unwrap().remove("trigger_to_return_seconds").unwrap();
    returned["compilation_evaluation_seconds"]=evaluation_seconds;
    Ok(returned)
}
fn run_contracted(mut state:PointMembrane,n:BigUint,seconds:u64,compilation_seconds:f64)->Result<Value,String>{
    eprintln!("prepared-switch constraints={} internal={} nodes={}",
        state.projector.as_ref().unwrap().constraints,state.projector.as_ref().unwrap().internal_count(),state.arena.retained_nodes());
    let trigger=Instant::now();
    while !state.projected && (seconds==0 || trigger.elapsed()<Duration::from_secs(seconds)) {
        state.advance()?;
        if state.operations%32==0 {eprintln!("projector contractions={} payload_steps={} internal={} nodes={} copy_returns={}",
            state.logical_contractions,state.operations,
            state.projector.as_ref().map_or(0,|p|p.internal_count()),state.arena.retained_nodes(),
            state.projector.as_ref().map_or(state.copy_return_ancestry.len(),|p|p.copy_returns.len()));}
    }
    let mut factors=None;let mut observation=None;
    if state.projected && !state.arena.mass(state.root).is_zero() {
        state.protocol("srfp")?;
        let moment=crate::idempotent_moment::read_prepared_operator(&mut state.arena,state.root,&state.e,&n,&state.prepared_protocol.frame,&state.moment_operator,Some(&state.prepared_protocol.dual))?;
        state.protocol_banks.extend([moment.weighted_root,moment.squared_root]);
        if let Some(point)=&moment.point {
            if let Ok(split)=crate::idempotent_point::close(&WordTape::from_biguint(&n),&WordTape::from_biguint(point)) {
                state.wells.close(&split.divisor,&split.quotient)?;
                factors=Some([split.divisor.to_biguint().to_string(),split.quotient.to_biguint().to_string()]);
            }
        }
        observation=Some(moment.evidence);
    }
    let factor_verified=factors.is_some();
    Ok(json!({"schema":"contracted-idempotent-moment/v1","N":n.to_string(),"input_boundary":"N-only",
        "execution_limit_seconds":if seconds==0 {None}else{Some(seconds)},
        "factors":factors,"product_verified":factor_verified,"observable":observation,
        "projection_complete":state.projected,"compilation_seconds":compilation_seconds,
        "trigger_to_return_seconds":trigger.elapsed().as_secs_f64(),"retained_membrane":state.snapshot()}))
}

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
    let mut factors=None;let mut reading=None;let mut reentry_closure=None;
    let mass=state.arena.mass(state.root);
    if done && !mass.is_zero() {
        let moment=crate::idempotent_moment::read_prepared_operator(&mut state.arena,state.root,&state.e,&n,&state.prepared_protocol.frame,&state.moment_operator,Some(&state.prepared_protocol.dual))?;
        state.protocol_banks.push(moment.weighted_root);
        state.protocol_banks.push(moment.squared_root);
        let point=moment.point.ok_or("projected family second moment did not return an integral point")?;
        let source_word=WordTape::from_canonical_word(&state.forms.native).ok_or("lost native source word")?;
        let split=crate::idempotent_point::close(&source_word,&WordTape::from_biguint(&point))?;
        reentry_closure=Some(state.wells.close(&split.divisor,&split.quotient)?);
        factors=Some([split.divisor.to_biguint().to_string(),split.quotient.to_biguint().to_string()]);
        reading=Some(json!({"observable":moment.evidence,"point":point.to_string(),"complement":split.complement.to_biguint().to_string(),
            "divisor_word":split.divisor.as_word(),"quotient_word":split.quotient.as_word(),
            "zero_remainder":true,"exact_word_product":true,"paired_boundary":true}));
    }
    let status=if factors.is_some(){"arithmetic-closed"}else if done{"empty-point-support"}else{"held"};
    let binary=BinaryTree::from_value(&n);
    let retained_membrane=state.snapshot();
    let continuation=if done{None}else{Some(retained_membrane.clone())};
    Ok(json!({"schema":"conventional-prepared-point/v3","source":source,"input_boundary":"N-only",
        "reentry_wells":state.wells.record(),"reentry_closure":reentry_closure,
        "status":status,"preparation_complete":done,"protocol_kind":kind,"protocol_execution":state.protocol_returns,"factors":factors,"product_verified":factors.is_some(),"reading":reading,
        "numeral_forms":{"native_word":state.forms.native,"hex_digit_word":state.forms.hex,
            "native_evaluation":state.forms.source_value()?.to_string(),"hex_evaluation":n.to_string(),
            "encoder_source":"shared with tfactor read","retained_through_Y":true},
        "binary_tree":{"positions":binary.positions,"evaluation":binary.evaluate().to_string()},
        "multiplicative_tree":{"operator":"proper-product-relation","equation":"P*Q=N",
            "evaluation_constraint":n.to_string(),"evaluated":factors.is_some(),"leaves":factors},
        "point_projection":{"operator":"retain E with E*E-E=0 mod N; exclude 0 and 1",
            "arithmetic_operations":state.operations,"arithmetic_unit":"integer-affine-row-single-reduction","complete":done,"support_mass":mass.to_string(),
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
    if prepared["schema"]!="conventional-prepared-point/v3"
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
    let trigger_to_verified_factor_seconds=trigger.elapsed().as_secs_f64();
    let names=if kind=="both"{expected.into_iter().map(str::to_owned).collect::<Vec<_>>()}else{vec![kind.to_owned()]};
    let mut result=prepared;
    result["schema"]=json!("conventional-n-only-return/v3");
    result["reports"]=json!(names.into_iter().map(|name|json!({"structure":name,
        "status":"arithmetic-closed","all_predicates_hold":false,
        "reading":result["reading"],"factor_product_verified":true})).collect::<Vec<_>>());
    result["factors"]=json!(factors);
    result["product_verified"]=json!(true);
    result["read_off_seconds"]=json!(read_off_seconds);
    result["trigger_to_readout_seconds"]=json!(read_off_seconds);
    result["product_verification_seconds"]=json!(product_verification_seconds);
    result["trigger_to_verified_factor_seconds"]=json!(trigger_to_verified_factor_seconds);
    result["trigger_to_return_seconds"]=json!(trigger_to_verified_factor_seconds);
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
    if state.arena.mass(state.root)!=BigUint::from(4u8){return Err("source-derived control lost its basis state".into());}
    let started=Instant::now();
    while !state.reduced {
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
        if state.arena.mass(state.root)!=BigUint::from(4u8){return Err("control lost its source-derived basis state".into());}
        let initial=state.root;
        state.complement()?;state.complement()?;
        if state.root!=initial{return Err("numerical parity reversal failed involution".into());}
        state.protocol(kind)?;
        if !state.projected || !state.arena.mass(state.root).is_zero() {
            return Err("executed protocol admitted the N-1 nonpoint".into());
        }
        let forward=state.protocol_returns[0]["nodes"][2]["returned_roots"][0].as_u64()
            .ok_or("forward trace lost its numerical bank")? as usize;
        if state.arena.mass(forward)!=BigUint::from(4u8){return Err("forward operation lost the control state".into());}
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
        let before=state.root;state.protocol_banks.push(before);state.fold();
        let started=Instant::now();state.advance()?;
        let integer=state.root;state.protocol_banks.push(integer);state.fold();
        let composed=state.arena.reduce_register(integer,&state.residual,&n)?;
        let composed_seconds=started.elapsed().as_secs_f64();
        state.protocol_banks.push(composed);state.fold();
        let mut baseline=state.arena.reduce_register(before,&state.residual,&n)?;
        state.protocol_banks.push(baseline);state.fold();
        let slot=state.protocol_banks.len()-1;let started=Instant::now();
        for j in i..state.e.len() {
            let value=if i==j{&state.powers[i+j]%&n}else{(&state.powers[i+j]<<1usize)%&n};
            let controls=if i==j{vec![(state.e[i],true)]}else{vec![(state.e[i],true),(state.e[j],true)]};
            baseline=state.arena.modular_add(baseline,&state.residual,&value,&n,&controls);
            state.protocol_banks[slot]=baseline;state.fold();
        }
        let value=(&n-(&state.powers[i]%&n))%&n;
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

/// Nonvacuous operation and bank instruments on the complete N-only family.
/// This is not a factor-production receipt or a substitute source witness.
pub fn source_complex_protocol_control(n:BigUint)->Result<Value,String>{
    if n.bits()<200{return Err("complex protocol control requires at least 200 bits".into());}
    let mut state=PointMembrane::prepare(n.clone())?;
    let started=Instant::now();let original=state.root;
    let reflected=crate::conventional_protocol::source_reflect(&state.arena,original,&n,"⊢⊙≻≺⋈∈⊤⊥∋⊞⊡⊣")?;
    let (changed,polynomial_banks,clink)=crate::conventional_protocol::clink(&mut state.arena,original,state.coordinates,n.bits())?;
    state.root=changed;state.protocol_banks.extend([original,changed]);state.protocol_banks.extend(polynomial_banks);state.fold();
    let (phase_changed,coherence)=crate::conventional_protocol::phase_coherence_control(&mut state.arena,changed,state.coordinates,n.bits())?;
    state.protocol_banks.push(phase_changed);state.fold();
    let (arms,joined,fork_witness)=crate::conventional_protocol::split(&mut state.arena,changed,state.coordinates,state.fork_history[0])?;
    if arms.iter().any(|&arm|state.arena.mass(arm).is_zero()){return Err("fork control did not exercise both arms".into());}
    let recovered=crate::conventional_protocol::fuse(&mut state.arena,joined,state.fork_history[0]);
    if recovered!=changed{return Err("production split/fuse lost complete complex ancestry".into());}
    // Removing the falsity arm must fail identity on this full-support state.
    let damaged=crate::conventional_protocol::fuse(&mut state.arena,arms[0],state.fork_history[0]);
    if damaged==changed{return Err("missing-arm control did not distinguish lost state".into());}
    state.protocol_banks.extend([arms[0],arms[1],joined,recovered,damaged]);state.fold();
    let (engaged,engagement)=crate::conventional_protocol::engage(&mut state.arena,joined,state.coordinates,state.engagement_history[0])?;
    state.root=engaged;state.protocol_banks.push(engaged);state.fold();
    let (fixed,banks,winding)=crate::conventional_protocol::fix_phase(&mut state.arena,engaged,state.coordinates,n.bits())?;
    state.root=fixed;state.protocol_banks.extend(banks);state.fold();
    let snapshot=state.snapshot();let record=&snapshot["shared_protocol_register"];
    let (restored,roots)=DecisionArena::from_prepared_record_many(record)?;
    if restored.prepared_record_many(&roots)!=*record{return Err("complex protocol return changed a retained bank".into());}
    let mut model=crate::parasm::SelfModel::prepare()?;
    let bytes=serde_json::to_vec(&snapshot).map_err(|e|e.to_string())?;
    if model.reflect(&bytes)!=bytes{return Err("complex state did not self-return".into());}
    model.backward[model.forward[0]]^=1;
    if model.reflect(&bytes)==bytes{return Err("changed self-model inverse was not observed".into());}
    Ok(json!({"schema":"conventional-complex-protocol-control/v1","N":n.to_string(),"source_bits":n.bits(),
        "input_boundary":"N-only complete family","self_reference":reflected,"CLINK":clink,"coherence":coherence,
        "fork_fuse":{"both_arms_nonempty":true,"complete_complex_ancestry_return":true,"missing_arm_rejected":true,"executed":fork_witness},
        "ENGAGR":engagement,"IFIX_phase":winding,"all_banks_roundtrip":true,
        "changed_self_model_rejected":true,"retained_banks":snapshot["retained_root_order"],
        "elapsed_seconds":started.elapsed().as_secs_f64()}))
}


/// Cargo calls this from N alone; no producer, seed or factor input is admitted.
pub fn compile_switch_record(n:BigUint)->Result<Value,String>{
    let started=Instant::now();let state=compile_point(n)?;
    let mut record=prepared_point_record(&state,0)?;
    record["preparation_nanoseconds"]=json!(u64::try_from(started.elapsed().as_nanos()).map_err(|_|"preparation clock overflow")?);
    Ok(record)
}
fn prepared_point_record(state:&PointMembrane,preparation_nanoseconds:u64)->Result<Value,String>{
    let mut roots=vec![state.root,state.prepared,state.before_projection];
    roots.extend_from_slice(&state.protocol_banks);
    roots.extend(state.projector.as_ref().unwrap().roots());
    let graph=|g:&PreparedNativeGraph|{
        let carrier=|v:imasm_core::imasm16_3::Reg16_3|[v.big_t,v.big_f,v.small_t,v.small_f];
        json!({"node_variants":g.graph.nodes.iter().map(|op|format!("{op:?}")).collect::<Vec<_>>(),
            "edges":g.graph.edges,"word":g.word,
            "reference_inputs":g.reference_inputs.iter().copied().map(carrier).collect::<Vec<_>>(),
            "reference_edges":g.reference_edges.iter().copied().map(carrier).collect::<Vec<_>>()})
    };
    let matrix=|m:&[(crate::word_tape::SignedWordTape,crate::word_tape::SignedWordTape);16]|
        m.iter().map(|(re,im)|json!({"re":re.prepared_record(),"im":im.prepared_record()})).collect::<Vec<_>>();
    let y=state.y.prepared_state();let p=&state.prepared_protocol;let model=&p.model;
    let register=state.arena.compiled_record_many(&roots);
    Ok(json!({"schema":"cargo-prepared-point-switch/v1","input_boundary":"N-only",
        "source_digits":state.n.to_u32_digits(),"source_word":state.forms.native,"hex_word":state.forms.hex,
        "binary_digits":state.wells.binary.prepared_digits(),
        "powers":state.powers.iter().map(BigUint::to_u32_digits).collect::<Vec<_>>(),
        "complement_word":state.complement_word.as_word(),
        "moment_offset":state.moment_operator.offset.prepared_record(),
        "moment_weights":state.moment_operator.weights.iter().map(|v|v.prepared_record()).collect::<Vec<_>>(),
        "register":register,"point_wires":state.e,"residual_wires":state.residual,
        "coordinates":state.coordinates,"fork_history":state.fork_history,"engagement_history":state.engagement_history,
        "projector":state.projector.as_ref().unwrap().preparation_record(),
        "protocol":{"sic":p.frame.prepared_record(),"modulus_bits":p.format.modulus_bits,"w_bits":p.format.w_bits,
            "central_step":matrix(&p.central_step),"engagement":matrix(&p.engagement),
            "dual_denominator":crate::word_tape::SignedWordTape::from_bigint(&p.dual.denominator).prepared_record(),
            "dual_columns":p.dual.columns.iter().map(|column|column.iter().map(|(re,im)|json!({
                "re":crate::word_tape::SignedWordTape::from_bigint(re).prepared_record(),
                "im":crate::word_tape::SignedWordTape::from_bigint(im).prepared_record()})).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "disassembler":model.disassembler,"recompiler":model.recompiler,"forward":model.forward,"backward":model.backward,
            "program_self_return":model.program_self_return,"executed_trie_steps":model.executed_trie_steps},
        "graphs":state.native_graphs.iter().map(graph).collect::<Vec<_>>(),
        "y":{"postfix":y.postfix,"prefix_trace":y.prefix_trace,"prefix_contractions":y.prefix_contractions,
            "contractions":y.contractions,"calls":y.calls,"done":y.done},
        "logical_contractions":state.logical_contractions,
        "runtime_state":{"row":state.row,"column":state.column,"subtract":state.subtract,
            "reduced":state.reduced,"projected":state.projected,"operations":state.operations,
            "protocol_bank_count":state.protocol_banks.len(),"protocol_returns":state.protocol_returns},
        "preparation_nanoseconds":preparation_nanoseconds}))
}

fn held_field<T:serde::de::DeserializeOwned>(record:&Value,key:&str)->Result<T,String>{
    serde_json::from_value(record[key].clone()).map_err(|e|format!("invalid held {key}: {e}"))
}

pub struct PreparedPointSwitch {state:PointMembrane,preparation_nanoseconds:u64}
impl PreparedPointSwitch {
    /// Restore the carried object, including the reduced tensors and pending Y
    /// program. No preparation, candidate producer or source reset is called.
    pub fn from_record(record:&Value)->Result<Self,String>{
        use crate::word_tape::SignedWordTape;
        use imasm_core::{check::Graph,classic::Token,flow,imasm16_3::Reg16_3};
        if record["schema"]!="cargo-prepared-point-switch/v1"||record["input_boundary"]!="N-only"{
            return Err("unsupported held membrane boundary".into());
        }
        let n=BigUint::new(held_field(record,"source_digits")?);
        let forms=SourceForms{native:held_field(record,"source_word")?,hex:held_field(record,"hex_word")?};
        if forms.source_value()?!=n{return Err("held membrane changed its source binding".into());}
        let wells=crate::reentry_wells::SourceWells::new(WordTape::from_canonical_word(&forms.native).ok_or("held source word")?)?;
        if wells.binary.prepared_digits()!=held_field::<Vec<char>>(record,"binary_digits")?{
            return Err("held binary well differs from the source".into());
        }
        let (arena,roots)=DecisionArena::from_prepared_record_many(&record["register"])?;
        let runtime=&record["runtime_state"];
        let bank_count=runtime.get("protocol_bank_count").and_then(Value::as_u64).unwrap_or(0) as usize;
        if roots.len()<3||bank_count>roots.len()-3{return Err("held membrane lost its bank boundary".into());}
        let projector=crate::idempotent_projector::Projector::from_preparation_record(&record["projector"],&roots[3+bank_count..])?;
        let p=&record["protocol"];
        let frame=crate::anyon_ququart::FixedQuquartSic::from_prepared_record(&p["sic"])?;
        let format=crate::phase_unbraid::FixedPointFormat::for_modulus_bits(n.bits())?;
        if p["modulus_bits"]!=n.bits()||p["w_bits"]!=format.w_bits{return Err("held SIC precision changed".into());}
        let complex=|z:&Value|Ok::<_,String>((SignedWordTape::from_prepared_record(&z["re"])?,SignedWordTape::from_prepared_record(&z["im"])?));
        let matrix=|rows:&Value|rows.as_array().ok_or("held matrix missing")?.iter().map(complex)
            .collect::<Result<Vec<_>,_>>()?.try_into().map_err(|_|String::from("held matrix dimension changed"));
        let columns=p["dual_columns"].as_array().ok_or("held SIC dual missing")?.iter().map(|row|{
            row.as_array().ok_or("held SIC dual column missing")?.iter().map(|z|complex(z).map(|(re,im)|(re.to_bigint(),im.to_bigint())))
                .collect::<Result<Vec<_>,String>>()?.try_into().map_err(|_|String::from("held SIC dual dimension changed"))
        }).collect::<Result<Vec<[(BigInt,BigInt);16]>,String>>()?;
        if columns.len()!=16{return Err("held SIC dual lost a column".into());}
        let prepared_protocol=crate::conventional_protocol::PreparedProtocol{frame,format,
            central_step:matrix(&p["central_step"])?,engagement:matrix(&p["engagement"])?,
            dual:crate::anyon_ququart::PreparedSicDual{denominator:SignedWordTape::from_prepared_record(&p["dual_denominator"])?.to_bigint(),columns},
            model:crate::parasm::SelfModel{disassembler:held_field(p,"disassembler")?,recompiler:held_field(p,"recompiler")?,
                forward:held_field(p,"forward")?,backward:held_field(p,"backward")?,
                program_self_return:held_field(p,"program_self_return")?,executed_trie_steps:held_field(p,"executed_trie_steps")?}};
        let graphs=record["graphs"].as_array().ok_or("held protocol graphs missing")?.iter().map(|g|{
            let names:Vec<String>=held_field(g,"node_variants")?;
            let nodes=names.iter().map(|name|Token::parse(name).ok_or_else(||format!("invalid held opcode {name}"))).collect::<Result<Vec<_>,_>>()?;
            let graph=Graph{nodes,edges:held_field(g,"edges")?};
            if !graph.validate().is_empty(){return Err("held graph wiring changed".into());}
            let word:String=held_field(g,"word")?;
            if word!=graph.nodes.iter().map(|op|op.code()).collect::<String>(){return Err("held word differs from graph".into());}
            let carrier=|key:&str|held_field::<Vec<[bool;4]>>(g,key).map(|values|values.into_iter().map(|v|Reg16_3{big_t:v[0],big_f:v[1],small_t:v[2],small_f:v[3]}).collect::<Vec<_>>());
            let reference_inputs=carrier("reference_inputs")?;let reference_edges=carrier("reference_edges")?;
            if flow::flow_values(&graph,Reg16_3::from_name("A").unwrap())!=(reference_inputs.clone(),reference_edges.clone()){
                return Err("held native flow changed".into());
            }
            Ok(PreparedNativeGraph{graph,word,reference_inputs,reference_edges})
        }).collect::<Result<Vec<_>,String>>()?;
        let native_graphs=graphs.try_into().map_err(|_|"held protocol graph count changed")?;
        let y=&record["y"];
        let y=ResidentY::from_prepared_state(vox_core::resident_y::PreparedResidentY{
            postfix:held_field(y,"postfix")?,prefix_trace:held_field(y,"prefix_trace")?,
            prefix_contractions:held_field(y,"prefix_contractions")?,contractions:held_field(y,"contractions")?,
            calls:held_field(y,"calls")?,done:held_field(y,"done")?})?;
        let e:Vec<usize>=held_field(record,"point_wires")?;
        if e.len()!=n.bits() as usize{return Err("held source coordinate map changed".into());}
        projector.check_carrier(arena.cells,&e)?;
        let powers:Vec<Vec<u32>>=held_field(record,"powers")?;
        let row=runtime.get("row").and_then(Value::as_u64).unwrap_or(0) as usize;
        let column=runtime.get("column").and_then(Value::as_u64).unwrap_or(0) as usize;
        let flag=|key|runtime.get(key).and_then(Value::as_bool).unwrap_or(false);
        if flag("projected"){return Err("closed image is not an open continuation".into());}
        let state=PointMembrane{copy_return_ancestry:Vec::new(),n,forms,wells,arena,
            root:roots[0],prepared:roots[1],before_projection:roots[2],e,
            residual:held_field(record,"residual_wires")?,coordinates:held_field(record,"coordinates")?,
            fork_history:held_field(record,"fork_history")?,engagement_history:held_field(record,"engagement_history")?,
            powers:powers.into_iter().map(BigUint::new).collect(),row,column,subtract:flag("subtract"),reduced:flag("reduced"),projected:false,y,
            operations:runtime.get("operations").and_then(Value::as_u64).unwrap_or(0),logical_contractions:projector.contractions,
            protocol_banks:roots[3..3+bank_count].to_vec(),protocol_returns:runtime.get("protocol_returns").cloned().map(serde_json::from_value)
                .transpose().map_err(|e|e.to_string())?.unwrap_or_default(),projector:Some(projector),prepared_protocol,native_graphs,
            moment_operator:crate::idempotent_moment::PreparedMoment{offset:SignedWordTape::from_prepared_record(&record["moment_offset"])?,
                weights:record["moment_weights"].as_array().ok_or("held moment missing")?.iter().map(SignedWordTape::from_prepared_record).collect::<Result<_,_>>()?},
            complement_word:WordTape::from_canonical_word(record["complement_word"].as_str().ok_or("held complement missing")?).ok_or("invalid held complement word")?};
        Ok(Self{state,preparation_nanoseconds:held_field(record,"preparation_nanoseconds")?})
    }
    pub fn from_baked(b:&crate::baked_point::BakedPoint)->Result<Self,String>{
        use crate::conventional_protocol::PreparedProtocol;
        use crate::phase_unbraid::FixedPointFormat;
        let (arena,roots)=DecisionArena::from_baked_sparse_nodes(b.cells,b.roots,b.nodes,b.support)?;
        let runtime=b.runtime;let bank_count=runtime.protocol_bank_count;
        if roots.len()<3||bank_count>roots.len()-3{return Err("compiled membrane lost its prepared banks".into());}
        if runtime.projected{return Err("closed image is not an open compiled continuation".into());}
        let projector=crate::idempotent_projector::Projector::from_baked(b.projector,&roots[3+bank_count..])?;
        projector.check_carrier(arena.cells,b.point_wires)?;
        let p=b.protocol;let format=FixedPointFormat{modulus_bits:p.modulus_bits,w_bits:p.w_bits};
        let matrix=|rows:&[crate::baked_resident::BakedWordComplex;16]|{
            rows.iter().map(|z|Ok((z.re.value()?,z.im.value()?))).collect::<Result<Vec<_>,String>>()?
                .try_into().map_err(|_|String::from("compiled operator lost its dimensions"))
        };
        let prepared_protocol=PreparedProtocol{frame:crate::anyon_ququart::FixedQuquartSic::from_baked_format(p.sic,format.clone())?,format,
            central_step:matrix(p.central_step)?,engagement:matrix(p.engagement)?,
            dual:crate::anyon_ququart::PreparedSicDual{denominator:p.dual_denominator.value(),
                columns:p.dual_columns.iter().map(|row|row.map(|z|{let v=z.value();(v.re,v.im)})).collect()},
            model:crate::parasm::SelfModel{disassembler:p.disassembler.into(),recompiler:p.recompiler.into(),
                forward:p.forward,backward:p.backward,program_self_return:p.program_self_return,
                executed_trie_steps:p.executed_trie_steps}};
        let native_graphs=b.graphs.each_ref().map(|g|PreparedNativeGraph{
            graph:imasm_core::check::Graph{nodes:g.nodes.to_vec(),edges:g.edges.to_vec()},word:g.word.into(),
            reference_inputs:g.reference_inputs.to_vec(),reference_edges:g.reference_edges.to_vec()});
        let y=ResidentY::from_prepared_state(vox_core::resident_y::PreparedResidentY{
            postfix:b.y.postfix.to_vec(),prefix_trace:b.y.prefix_trace.iter().map(|s|(*s).into()).collect(),
            prefix_contractions:b.y.prefix_contractions,contractions:b.y.contractions,calls:b.y.calls,done:b.y.done})?;
        let state=PointMembrane{copy_return_ancestry:Vec::new(),n:BigUint::new(b.source_digits.to_vec()),
            forms:SourceForms{native:b.source_word.into(),hex:b.hex_word.into()},
            wells:crate::reentry_wells::SourceWells::from_baked(b.source_word,b.binary_digits)?,
            arena,root:roots[0],prepared:roots[1],before_projection:roots[2],
            e:b.point_wires.to_vec(),residual:b.residual_wires.to_vec(),coordinates:b.coordinates,
            fork_history:b.fork_history,engagement_history:b.engagement_history,
            powers:b.powers.iter().map(|v|BigUint::new(v.to_vec())).collect(),
            row:runtime.row,column:runtime.column,subtract:runtime.subtract,reduced:runtime.reduced,projected:false,y,
            operations:runtime.operations,logical_contractions:b.projector.contractions,
            protocol_banks:roots[3..3+bank_count].to_vec(),
            protocol_returns:serde_json::from_str(runtime.protocol_returns).map_err(|e|e.to_string())?,projector:Some(projector),
            prepared_protocol,native_graphs,
            moment_operator:crate::idempotent_moment::PreparedMoment{offset:b.moment_offset.value()?,
                weights:b.moment_weights.iter().map(|v|v.value()).collect::<Result<Vec<_>,_>>()?},
            complement_word:WordTape::from_compiled_word(b.complement_word).ok_or("compiled complement lost its word")?};
        Ok(Self{state,preparation_nanoseconds:b.preparation_nanoseconds})
    }
    pub fn trigger(self)->Result<Value,String>{
        let n=self.state.n.clone();run_contracted(self.state,n,0,self.preparation_nanoseconds as f64/1e9)
    }
    pub fn hold(mut self,steps:u64,path:&std::path::Path)->Result<Value,String>{
        let before=self.state.operations;let started=Instant::now();
        for _ in 0..steps {
            self.state.advance()?;
            if self.state.projected {
                let n=self.state.n.clone();
                let returned=run_contracted(self.state,n,0,self.preparation_nanoseconds as f64/1e9)?;
                let mut file=std::io::BufWriter::new(std::fs::File::create(path).map_err(|e|e.to_string())?);
                serde_json::to_writer(&mut file,&returned).map_err(|e|e.to_string())?;
                return Ok(json!({"projection_complete":true,"factors":returned["factors"],"return_file":path}));
            }
        }
        self.state.fold();
        let transformation_seconds=started.elapsed().as_secs_f64();
        let record=self.prepared_record()?;
        let mut file=std::io::BufWriter::new(std::fs::File::create(path).map_err(|e|e.to_string())?);
        serde_json::to_writer(&mut file,&record).map_err(|e|e.to_string())?;
        use std::io::Write;
        file.flush().map_err(|e|e.to_string())?;
        let mut report=self.loading_record();
        report["schema"]=json!("held-point-return/v1");report["held"]=json!(true);
        report["continuation_file"]=json!(path);report["payload_steps_executed"]=json!(self.state.operations-before);
        report["total_payload_steps"]=json!(self.state.operations);report["Y_calls"]=json!(self.state.y.calls);
        report["logical_contractions"]=json!(self.state.logical_contractions);
        report["copy_returns"]=json!(self.state.projector.as_ref().unwrap().copy_returns.len());
        report["open_internal_coordinates"]=json!(self.state.projector.as_ref().unwrap().internal_count());
        report["transformation_seconds"]=json!(transformation_seconds);
        report["return_serialization_seconds"]=json!(started.elapsed().as_secs_f64()-transformation_seconds);
        Ok(report)
    }
    pub fn prepared_record(&self)->Result<Value,String>{prepared_point_record(&self.state,self.preparation_nanoseconds)}
    pub fn loading_record(&self)->Value{
        json!({"N":self.state.n.to_string(),"preparation_nanoseconds":self.preparation_nanoseconds,
            "register_cells":self.state.arena.cells,"nodes":self.state.arena.retained_nodes(),
            "tensor_banks":self.state.projector.as_ref().unwrap().roots().len(),
            "prepared_Y_residual":self.state.y.residual_program(),"source_inputs_at_trigger":0})
    }
}

pub fn prepare_held(n:BigUint,steps:u64,path:&std::path::Path)->Result<Value,String>{
    let started=Instant::now();let state=compile_point(n)?;
    let preparation_nanoseconds=u64::try_from(started.elapsed().as_nanos()).map_err(|_|"preparation clock overflow")?;
    PreparedPointSwitch{state,preparation_nanoseconds}.hold(steps,path)
}
