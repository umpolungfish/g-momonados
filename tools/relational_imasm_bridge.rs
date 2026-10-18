//! Source-bound relational supports. The compiled product circuit stays compact;
//! no decision diagram or prospective operand table is materialized.
use std::{collections::VecDeque, time::{Duration, Instant}};
use crate::factor_relation::RelationProgram;
use crate::fixed_point_imasm::ImasmNumeralWord;
use crate::fixed_point_word_arithmetic as arithmetic;
use serde_json::{json, Value};

// Each wire's alternatives are the classical slice of the native carrier:
// bit 0 = constructive truth T, bit 1 = constructive falsity F. A gate row
// retains the JOINT assignment, rather than multiplying marginal supports.
const T: u8 = 1;
const F: u8 = 2;
const B: u8 = T | F;

pub struct Engine {
    ops: Vec<[u32;4]>, domains: Vec<u8>, uses: Vec<Vec<usize>>,
    inputs: Vec<(usize,usize)>, queue: VecDeque<usize>, queued: Vec<bool>,
    trail: Vec<(usize,u8)>, decisions: Vec<(usize,u8,usize)>,
    /// Each hop acts on the PREVIOUS hop's value. Original assignments remain
    /// explicit inverse fibres when an image merges T and F.
    hops: Vec<[u8;16]>, image: [u8;2], phase: usize,
    cells: Vec<[usize;5]>, cell_rows: [[u8;5];8],
    pub contractions: u64, pub rows_examined: u64, pub branches: u64,
    pub bridge_applications: u64,
    pub joint_visits:u64, pub joint_contractions:u64,
}

impl Engine {
    fn new(ops: Vec<[u32;4]>, domains: Vec<u8>, hops: Vec<[u8;16]>,
        cells:Vec<[usize;5]>, cell_rows:[[u8;5];8]) -> Self {
        let mut uses=vec![Vec::new();ops.len()]; let mut inputs=Vec::new();
        for (i, &[op,a,b,_]) in ops.iter().enumerate() {
            if op==8 { inputs.push((a as usize,i)); }
            if [3,4,7].contains(&op) {
                uses[a as usize].push(i);
                if op!=3 { uses[b as usize].push(i); }
                uses[i].push(i);
            }
        }
        for (cell,ports) in cells.iter().enumerate() {
            for &wire in ports {uses[wire].push(ops.len()+cell);}
        }
        inputs.sort_unstable();
        let count=ops.len()+cells.len();
        let mut result=Self { ops,domains,uses,inputs,queue:VecDeque::new(),
            queued:vec![false;count],trail:Vec::new(),decisions:Vec::new(),
            hops,image:[T,F],phase:0,contractions:0,rows_examined:0,branches:0,
            bridge_applications:0,cells,cell_rows,joint_visits:0,joint_contractions:0 };
        result.hop(); result.enqueue_all(); result
    }
    fn hop(&mut self) {
        let table=&self.hops[self.phase % self.hops.len()];
        self.image=self.image.map(|value|table[value as usize]);
        self.phase+=1; self.bridge_applications+=1;
    }
    fn enqueue_all(&mut self) {
        for i in 0..self.ops.len() {
            if [3,4,7].contains(&self.ops[i][0]) { self.enqueue(i); }
        }
        for i in 0..self.cells.len() {self.enqueue(self.ops.len()+i);}
    }
    fn enqueue(&mut self, i:usize) {
        if !self.queued[i] {self.queued[i]=true;self.queue.push_back(i);}
    }
    fn intersect(&mut self, wire:usize, allowed:u8) -> bool {
        let old=self.domains[wire];let next=old & allowed;
        if old!=next {
            self.trail.push((wire,old));self.domains[wire]=next;self.contractions+=1;
            let readers=self.uses[wire].clone();
            for reader in readers {self.enqueue(reader);}
        }
        next!=0
    }
    fn compatible(&self, wire:usize, fibre:u8) -> bool {
        // Compare transported labels AND their retained source fibre. Dropping
        // the second component would accept false equalities after a lossy hop.
        (0..2).any(|alternative| self.domains[wire] & (1<<alternative)!=0
            && self.image[alternative]==self.image[fibre as usize]
            && alternative==fibre as usize)
    }
    fn propagate(&mut self, deadline:Instant) -> Option<bool> {
        while let Some(i)=self.queue.pop_front() {
            self.queued[i]=false;
            if self.rows_examined & 4095==0 && Instant::now()>=deadline {
                self.queue.push_front(i);self.queued[i]=true;return None;
            }
            if i>=self.ops.len() {
                let ports=self.cells[i-self.ops.len()];
                let domains=ports.map(|wire|self.domains[wire]);
                let allowed=crate::joint_cells::supported(&self.cell_rows,domains,self.image);
                self.rows_examined+=8;self.joint_visits+=1;
                let before=self.contractions;
                for port in 0..5 {
                    if !self.intersect(ports[port],allowed[port]) {return Some(false);}
                }
                self.joint_contractions+=self.contractions-before;
                continue;
            }
            let [op,a,b,_]=self.ops[i];let(a,b)=(a as usize,b as usize);
            let mut allowed=[0u8;3];
            for av in 0..2u8 {for bv in 0..if op==3 {1}else{2} {
                let truth=av==0; let other=bv==0;
                let output=match op {3=>!truth,4=>truth&&other,7=>truth||other,_=>unreachable!()};
                let ov=if output {0}else{1};self.rows_examined+=1;
                // A repeated operand is ONE variable, not two independent
                // alternatives. Preserve that correlation in the finite row.
                if op!=3 && a==b && av!=bv {continue;}
                if self.compatible(a,av) && (op==3 || self.compatible(b,bv))
                    && self.compatible(i,ov) {
                    allowed[0]|=1<<av;allowed[1]|=1<<bv;allowed[2]|=1<<ov;
                }
            }}
            if !self.intersect(a,allowed[0]) || (op!=3 && !self.intersect(b,allowed[1]))
                || !self.intersect(i,allowed[2]) {return Some(false);}
        }
        Some(true)
    }
    fn restore(&mut self, at:usize) {
        while self.trail.len()>at {let(wire,old)=self.trail.pop().unwrap();self.domains[wire]=old;}
        self.queue.clear();self.queued.fill(false);
    }
    fn next_branch(&mut self) -> bool {
        while let Some((wire,alternative,at))=self.decisions.pop() {
            self.restore(at);self.hop();
            if alternative!=0 {
                self.decisions.push((wire,0,at));
                return self.intersect(wire,alternative);
            }
        }
        false
    }
    fn search(&mut self, seconds:u64) -> &'static str {
        let deadline=Instant::now()+Duration::from_secs(seconds);
        loop {
            match self.propagate(deadline) {
                None=>return "held",
                Some(false)=>if !self.next_branch(){return "empty";}else{continue;},
                Some(true)=>{}
            }
            let unresolved=self.inputs.iter().find(|(_,wire)|self.domains[*wire]==B).copied();
            let Some((_,wire))=unresolved else {return "assigned";};
            // The existing powerset unit/bind returns the two possible updates
            // on this same retained arena. The unexecuted sibling stays in the
            // continuation with its trail boundary; no producer is restarted.
            let alternatives=crate::reentry_powerset::bind(
                crate::reentry_powerset::unit(wire),
                |_|Ok::<_,()>([T,F].into_iter().collect())).unwrap();
            let mut alternatives=alternatives.into_iter();
            let selected=alternatives.next().unwrap();let sibling=alternatives.next().unwrap();
            self.decisions.push((wire,sibling,self.trail.len()));
            self.branches+=1;self.hop();self.intersect(wire,selected);
        }
    }
}

fn joint_bank(ops:&[[u32;4]],out:&std::path::Path,joint:bool) -> (Vec<[usize;5]>,[[u8;5];8]) {
    let rows=crate::joint_cells::full_adder_rows();
    let ports=if joint {crate::joint_cells::full_adder_ports(ops)}else{Vec::new()};
    crate::joint_cells::check_lowering(ops,&ports,&rows);
    let mut bytes=Vec::with_capacity(ports.len()*20);
    for cell in &ports {for &wire in cell {bytes.extend_from_slice(&(wire as u32).to_le_bytes());}}
    std::fs::write(out.join("relation.cells"),bytes).unwrap();
    std::fs::write(out.join("cell.rows"),rows.iter().flatten().copied().collect::<Vec<_>>()).unwrap();
    let words=crate::joint_cells::native_row_words(&rows);
    std::fs::write(out.join("native_cell_bank.json"),serde_json::to_vec_pretty(&json!({
        "ports":ports.len(),"native_rows":rows,"native_row_words":words.iter().map(|w|w.as_str()).collect::<Vec<_>>(),
        "native_adder_word":arithmetic::WORD_ADD_CELL.iter().collect::<String>(),
        "lowering_assignments_checked":ports.len()*8})).unwrap()).unwrap();
    (ports,rows)
}

pub fn compile_joint_bank(packed:&[u8],out:&std::path::Path,joint:bool) {
    let ops:Vec<[u32;4]>=packed.chunks_exact(16).map(|r|core::array::from_fn(|i|
        u32::from_le_bytes(r[4*i..4*i+4].try_into().unwrap()))).collect();
    joint_bank(&ops,out,joint);
}

pub fn prepare(source:&str, out:&std::path::Path, hops:Vec<[u8;16]>,joint:bool) {
    let started=Instant::now();
    let word=ImasmNumeralWord::checked(source.trim()).unwrap();
    assert_eq!(arithmetic::add(&word,&arithmetic::zero()).unwrap(),word,
        "source must use canonical native numeral cells");
    let mut n=num_bigint::BigUint::from(0u8);let mut place=num_bigint::BigUint::from(1u8);
    for mark in word.as_str().chars().filter(|c|matches!(c,'⊤'|'⊥')) {
        if mark=='⊥' {n+=&place;} place<<=1;
    }
    let program=RelationProgram::proper_product(&n).unwrap();
    let mut domains=vec![B;program.ops.len()];
    for (i,&[op,_,_,required]) in program.ops.iter().enumerate() {
        if op==0 {domains[i]=F;} if op==5 || required!=0 {domains[i]=T;}
    }
    domains[program.root as usize]=T;
    let(cells,rows)=joint_bank(&program.ops,out,joint);
    let mut engine=Engine::new(program.ops,domains,hops,cells,rows);
    // Preparation really contracts the source-bound register to a fixed point.
    // The one-hour instrument boundary is not a production cutoff.
    let consistent=engine.propagate(Instant::now()+Duration::from_secs(3600));
    assert_eq!(consistent,Some(true),"source relation inconsistent or preparation unfinished");
    let mut bytes=Vec::with_capacity(engine.ops.len()*16);
    for op in &engine.ops {for field in op {bytes.extend_from_slice(&field.to_le_bytes());}}
    std::fs::write(out.join("relation.ops"),bytes).unwrap();
    std::fs::write(out.join("register.cells"),&engine.domains).unwrap();
    std::fs::write(out.join("prepared.json"),serde_json::to_vec_pretty(&json!({
        "source_word":source.trim(),"source_bits":n.bits(),"wires":engine.ops.len(),
        "factor_lane_width":program.width,"width_boundary":"both lanes use N bits; no supplied factor width",
        "prepared_contractions":engine.contractions,"prepared_rows_examined":engine.rows_examined,
        "prepared_bridge_image":engine.image,"preparation_seconds":started.elapsed().as_secs_f64(),
        "joint_cells":engine.cells.len(),"prepared_joint_cell_visits":engine.joint_visits,
        "prepared_joint_cell_contractions":engine.joint_contractions,
        "producing_method":"compact source relations with powerset branching"
    })).unwrap()).unwrap();
}

fn numeral(bits:&[u8]) -> ImasmNumeralWord {
    let mut word=String::from("⊢");
    let end=bits.iter().rposition(|&b|b==T).map_or(0,|i|i+1);
    for &bit in &bits[..end] {word.push_str(if bit==T {"≻⋈∈⊥∋"}else{"≻⋈∈⊤∋"});}
    word.push_str("⊙⊡⊣");ImasmNumeralWord::checked(&word).unwrap()
}

pub fn execute(source:&str, packed:&[u8], prepared:&[u8], hops:Vec<[u8;16]>, seconds:u64,
    resumed:Option<&str>,packed_cells:&[u8],packed_rows:&[u8]) -> Value {
    let started=Instant::now();assert_eq!(packed.len()%16,0);
    let ops=packed.chunks_exact(16).map(|row|core::array::from_fn(|i|
        u32::from_le_bytes(row[4*i..4*i+4].try_into().unwrap()))).collect();
    assert_eq!(packed_cells.len()%20,0);assert_eq!(packed_rows.len(),40);
    let cells=packed_cells.chunks_exact(20).map(|r|core::array::from_fn(|i|
        u32::from_le_bytes(r[4*i..4*i+4].try_into().unwrap()) as usize)).collect();
    let rows=core::array::from_fn(|i|core::array::from_fn(|j|packed_rows[5*i+j]));
    let mut engine=Engine::new(ops,prepared.to_vec(),hops,cells,rows);
    if let Some(encoded)=resumed {
        let previous:Value=serde_json::from_str(encoded).unwrap();
        assert_eq!(previous["source_word"].as_str(),Some(source.trim()));
        assert_eq!(previous["status"].as_str(),Some("held"));
        engine.domains=serde_json::from_value(previous["register"].clone()).unwrap();
        assert_eq!(engine.domains.len(),engine.ops.len());
        engine.trail=serde_json::from_value(previous["continuation"]["trail"].clone()).unwrap();
        engine.decisions=serde_json::from_value(previous["continuation"]["decisions"].clone()).unwrap();
        engine.queue=serde_json::from_value(previous["continuation"]["pending_gates"].clone()).unwrap();
        engine.queued.fill(false);for &gate in &engine.queue {engine.queued[gate]=true;}
        engine.image=serde_json::from_value(previous["carried_T_F_images"].clone()).unwrap();
        engine.phase=previous["bridge_phase"].as_u64().unwrap() as usize;
        engine.contractions=previous["contractions"].as_u64().unwrap();
        engine.rows_examined=previous["rows_examined"].as_u64().unwrap();
        engine.branches=previous["branches"].as_u64().unwrap();
        engine.bridge_applications=previous["bridge_applications"].as_u64().unwrap();
        let parent_cells=previous["joint_cells"].as_u64().unwrap_or(0) as usize;
        assert!(parent_cells==0 || parent_cells==engine.cells.len());
        engine.joint_visits=previous["joint_cell_visits"].as_u64().unwrap_or(0);
        engine.joint_contractions=previous["joint_cell_contractions"].as_u64().unwrap_or(0);
        if parent_cells==0 {
            for cell in 0..engine.cells.len(){engine.enqueue(engine.ops.len()+cell);}
        }
    }
    let status=engine.search(seconds);let readout=started.elapsed().as_secs_f64();
    let mut closure=Value::Null;
    if status=="assigned" {
        let width=engine.inputs.len()/2;let mut p=vec![F;width];let mut q=vec![F;width];
        for &(variable,wire) in &engine.inputs {
            if variable&1==0 {p[variable/2]=engine.domains[wire];}else{q[variable/2]=engine.domains[wire];}
        }
        let (p,q)=(numeral(&p),numeral(&q));let n=ImasmNumeralWord::checked(source.trim()).unwrap();
        let (derived,remainder)=arithmetic::divide(&n,&p).unwrap();
        let product=arithmetic::multiply(&p,&derived).unwrap();
        let valid=arithmetic::compare(&p,&arithmetic::one()).unwrap().is_gt()
            && arithmetic::compare(&p,&n).unwrap().is_lt()
            && remainder==arithmetic::zero() && product==n && derived==q;
        assert!(valid,"assigned relation did not close against the exact source");
        closure=json!({"divisor_word":p.as_str(),"quotient_word":derived.as_str(),
            "zero_remainder":true,"exact_native_imasm_product":true,"paired_boundary_closure":true,
            "trigger_to_verified_factor_seconds":started.elapsed().as_secs_f64()});
    }
    json!({"source_word":source.trim(),"status":status,"closure":closure,
        "contractions":engine.contractions,"rows_examined":engine.rows_examined,"branches":engine.branches,
        "bridge_applications":engine.bridge_applications,"bridge_phase":engine.phase,
        "joint_cells":engine.cells.len(),"joint_cell_visits":engine.joint_visits,
        "joint_cell_contractions":engine.joint_contractions,
        "carried_T_F_images":engine.image,"transport_factor_readouts":0,
        "trigger_to_readout_seconds":readout,"register":engine.domains,
        "continuation":{"trail":engine.trail,"decisions":engine.decisions,"pending_gates":engine.queue},
        "source_bank_retained":true,"inverse_fibres_retained":true,
        "scope":"source relation, arithmetic register and retained continuation"})
}
