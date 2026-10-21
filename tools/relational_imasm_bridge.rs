//! Source-bound relational supports. The compiled product circuit stays compact;
//! no decision diagram or prospective operand table is materialized.
use std::{collections::{VecDeque,HashMap,hash_map::DefaultHasher}, hash::{Hash,Hasher}, time::{Duration, Instant}};
use crate::factor_relation::RelationProgram;
use crate::fixed_point_imasm::ImasmNumeralWord;
use crate::fixed_point_word_arithmetic as arithmetic;
use serde_json::{json, Value};
use std::io::{Read,Write};
#[path="resident_dialect_relations.rs"]
mod resident_dialect_relations;
use resident_dialect_relations::Resident;
#[path="resident_product_return.rs"]
mod resident_product_return;
#[path="resident_reader_index.rs"]
mod resident_reader_index;
use resident_reader_index::ReaderIndex;
#[path="resident_evidence_store.rs"]
mod resident_evidence_store;
use resident_evidence_store::Archive;
// The compilation prefix and retained runtime cursors operate on one source
// carrier. The verification tail returns that same register before fixation.
const CONTROL_WORD:&str="⊢⋈⋈⋈⋈⋈⋈⋈⊙≻∈⊤≻⋈≻⋈≻⋈≺∈⊤≻⋈≻⋈∋⋈≺≻∈⊤≻⋈≺⊞∋⊤⋈⊤⊤⊤⊙∋⊡⊣";
type NestedReceipt=(usize,usize,usize,usize,bool,bool,usize,[u8;2],Vec<(usize,u8)>);
#[derive(Clone)]
struct HoldingFrame {
    entry_bank:usize,entry_trail:usize,entry_phase:usize,
    returned_bank:Option<usize>,fixed:bool,
}
impl HoldingFrame {
    fn report(&self)->Value {json!({"entry_bank":self.entry_bank,"entry_trail":self.entry_trail,
        "entry_phase":self.entry_phase,"returned_bank":self.returned_bank,"fixed":self.fixed})}
    fn load(value:&Value)->Self {Self{
        entry_bank:value["entry_bank"].as_u64().unwrap() as usize,
        entry_trail:value["entry_trail"].as_u64().unwrap() as usize,
        entry_phase:value["entry_phase"].as_u64().unwrap() as usize,
        returned_bank:value["returned_bank"].as_u64().map(|v|v as usize),fixed:value["fixed"].as_bool().unwrap()}}
}
struct Checkpoint {directory:std::path::PathBuf,source:String,started:Instant,last:Instant}

struct VerificationTail {
    pc:usize,divisor:ImasmNumeralWord,paired:ImasmNumeralWord,
    quotient:Option<ImasmNumeralWord>,remainder:Option<ImasmNumeralWord>,
    product:Option<ImasmNumeralWord>,returned_bank:Option<usize>,
    verified_seconds:Option<f64>,steps:Vec<usize>,
}
impl VerificationTail {
    fn new(divisor:ImasmNumeralWord,paired:ImasmNumeralWord)->Self {
        Self{pc:36,divisor,paired,quotient:None,remainder:None,product:None,
            returned_bank:None,verified_seconds:None,steps:Vec::new()}
    }
    fn report(&self)->Value {json!({"pc":self.pc,"divisor_word":self.divisor.as_str(),
        "paired_word":self.paired.as_str(),"quotient_word":self.quotient.as_ref().map(|w|w.as_str()),
        "remainder_word":self.remainder.as_ref().map(|w|w.as_str()),
        "product_word":self.product.as_ref().map(|w|w.as_str()),"returned_bank":self.returned_bank,
        "trigger_to_verified_factor_seconds":self.verified_seconds,"executed_positions":self.steps})}
    fn load(value:&Value)->Self {
        let word=|key:&str|value[key].as_str().map(|s|ImasmNumeralWord::checked(s).unwrap());
        Self{pc:value["pc"].as_u64().unwrap() as usize,divisor:word("divisor_word").unwrap(),
            paired:word("paired_word").unwrap(),quotient:word("quotient_word"),
            remainder:word("remainder_word"),product:word("product_word"),
            returned_bank:value["returned_bank"].as_u64().map(|v|v as usize),
            verified_seconds:value["trigger_to_verified_factor_seconds"].as_f64(),
            steps:serde_json::from_value(value["executed_positions"].clone()).unwrap()}
    }
}

// Each wire's alternatives are the classical slice of the native carrier:
// bit 0 = constructive truth T, bit 1 = constructive falsity F. A gate row
// retains the JOINT assignment, rather than multiplying marginal supports.
const T: u8 = 1;
const F: u8 = 2;
const B: u8 = T | F;

pub struct Engine<'a> {
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
    nested_enabled:bool,
    control_enabled:bool,control_pc:usize,control_blocks:u64,holding:Option<HoldingFrame>,
    nested_control_pc:usize,nested_consistent:bool,nested_block:u8,
    outer_pc:usize,outer_seams:u64,
    verification_tail:Option<VerificationTail>,
    preparation_control:Option<Value>,
    legacy_phase:Option<usize>,
    reversal_receipts:Archive<Value>,
    // wire, parent trail boundary, parent bank, child stage, first bank, first consistency
    nested:Option<(usize,usize,usize,u8,usize,bool)>,
    nested_cursor:usize, nested_pass_changed:bool,
    banks:Archive<(usize,Vec<u8>)>, bank_index:HashMap<u64,Vec<usize>>, bank_cache:Option<usize>,
    // wire, parent, T bank, F bank, T/F consistency, phase, image, common deductions
    nested_receipts:Archive<NestedReceipt>,
    nested_entries:u64, nested_common:u64, nested_factor_common:u64,
    resident:Option<Resident>,
    product_source:Option<ImasmNumeralWord>,product_banks:Archive<Value>,
    product_visits:u64,product_contractions:u64,preparing:bool,
    prepared_readers:Option<ReaderIndex<'a>>,checkpoint:Option<Checkpoint>,checkpoint_ticks:u64,
}

impl<'a> Engine<'a> {
    fn new(ops: Vec<[u32;4]>, domains: Vec<u8>, hops: Vec<[u8;16]>,
        cells:Vec<[usize;5]>, cell_rows:[[u8;5];8]) -> Self {
        Self::new_indexed(ops,domains,hops,cells,cell_rows,None)
    }
    fn new_indexed(ops: Vec<[u32;4]>, domains: Vec<u8>, hops: Vec<[u8;16]>,
        cells:Vec<[usize;5]>, cell_rows:[[u8;5];8],readers:Option<&'a[u8]>) -> Self {
        Self::new_indexed_seeded(ops,domains,hops,cells,cell_rows,readers,true)
    }
    fn new_indexed_seeded(ops: Vec<[u32;4]>, domains: Vec<u8>, hops: Vec<[u8;16]>,
        cells:Vec<[usize;5]>, cell_rows:[[u8;5];8],readers:Option<&'a[u8]>,advance:bool) -> Self {
        let prepared_readers=readers.map(ReaderIndex::new);
        if let Some(index)=&prepared_readers{assert_eq!(index.wires(),ops.len());}
        let mut uses=if prepared_readers.is_none(){vec![Vec::new();ops.len()]}else{Vec::new()};
        let mut inputs=Vec::new();
        for (i, &[op,a,b,_]) in ops.iter().enumerate() {
            if op==8 { inputs.push((a as usize,i)); }
            if prepared_readers.is_none() && [3,4,7].contains(&op) {
                uses[a as usize].push(i);
                if op!=3 { uses[b as usize].push(i); }
                uses[i].push(i);
            }
        }
        if prepared_readers.is_none(){for (cell,ports) in cells.iter().enumerate() {
            for &wire in ports {uses[wire].push(ops.len()+cell);}
        }
        }
        inputs.sort_unstable();
        let count=ops.len()+cells.len();
        let mut result=Self { ops,domains,uses,inputs,queue:VecDeque::new(),
            queued:vec![false;count],trail:Vec::new(),decisions:Vec::new(),
            hops,image:[T,F],phase:0,contractions:0,rows_examined:0,branches:0,
            bridge_applications:0,cells,cell_rows,joint_visits:0,joint_contractions:0,
            nested_enabled:false,control_enabled:false,control_pc:12,control_blocks:0,holding:None,nested_control_pc:20,nested_consistent:true,nested_block:0,reversal_receipts:Archive::new(),nested:None,nested_cursor:0,nested_pass_changed:false,
            outer_pc:18,outer_seams:0,verification_tail:None,preparation_control:None,legacy_phase:None,
            banks:{let mut bank=Archive::new();bank.push((0,Vec::new()));bank},bank_index:HashMap::new(),bank_cache:Some(0),
            nested_receipts:Archive::new(),nested_entries:0,nested_common:0,nested_factor_common:0,
            resident:None,product_source:None,product_banks:Archive::new(),
            product_visits:0,product_contractions:0,preparing:false,prepared_readers,checkpoint:None,checkpoint_ticks:0 };
        result.nested_cursor=result.inputs.len();
        if advance{result.hop();}if result.prepared_readers.is_none(){result.enqueue_all();}result
    }
    fn assert_live(&self) {
        assert!(!self.holding.as_ref().is_some_and(|frame|frame.fixed),"resident register is fixed after verified closure");
    }
    fn open_holding(&mut self) {
        assert!(self.holding.is_none());
        let entry_bank=self.parent_bank();
        self.holding=Some(HoldingFrame{entry_bank,entry_trail:self.trail.len(),entry_phase:self.phase,
            returned_bank:None,fixed:false});
    }
    fn fix_verified_register(&mut self) {
        self.assert_live();assert!(self.holding.is_some());
        // A complete source-rooted overlay includes wire, joint-row and pair
        // slots, even when the selected path came through a nested child.
        let selected=self.intern_bank(0,self.overlay(0));
        let frame=self.holding.as_mut().unwrap();
        frame.returned_bank=Some(selected);frame.fixed=true;
    }
    fn readout_words(&self)->(ImasmNumeralWord,ImasmNumeralWord) {
        assert!(!self.inputs.is_empty() && self.inputs.len()%2==0);
        let width=self.inputs.len()/2;let mut p=vec![F;width];let mut q=vec![F;width];
        for &(variable,wire) in &self.inputs {
            assert!(matches!(self.domains[wire],T|F),"factor readout retains unresolved alternatives");
            if variable&1==0{p[variable/2]=self.domains[wire];}else{q[variable/2]=self.domains[wire];}
        }
        (numeral(&p),numeral(&q))
    }
    fn preparation_link(&mut self) {
        loop {
            assert_eq!(self.propagate(None),Some(true),"prepared source relation is inconsistent");
            match self.return_product(){
                Ok(true)=>continue,Ok(false)=>break,Err(())=>panic!("prepared source product is inconsistent"),
            }
        }
    }
    fn prepare_word_prefix(&mut self,source:&str,root:usize) {
        assert_eq!(self.phase,0);assert!(self.holding.is_none());
        assert!(self.decisions.is_empty() && self.nested.is_none());
        let word:Vec<char>=CONTROL_WORD.chars().collect();let mut steps=Vec::new();
        for pc in 0..12 {
            let before=(self.contractions,self.product_visits,self.phase,self.queue.len());
            let mut native_before=Value::Null;let mut native_after=Value::Null;
            match word[pc] {
                '⊢'=>{
                    assert_eq!(pc,0);assert_eq!(self.domains[root],T);
                    assert!(ImasmNumeralWord::checked(source).is_ok());
                }
                '⋈'=>{assert!((1..=7).contains(&pc));self.preparation_link();}
                '⊙'=>{
                    assert_eq!(pc,8);let n=ImasmNumeralWord::checked(source).unwrap();
                    assert_eq!(arithmetic::add(&n,&arithmetic::zero()).unwrap(),n);
                    if let Some(bound)=&self.product_source{assert_eq!(bound,&n);}
                }
                '≻'=>{
                    assert_eq!(pc,9);
                    native_before=self.resident.as_ref().map_or(Value::Null,|r|json!(r.native_types));
                    self.hop();self.enqueue_all();self.preparation_link();
                    native_after=self.resident.as_ref().map_or(Value::Null,|r|json!(r.native_types));
                }
                '∈'=>{
                    assert_eq!(pc,10);
                    // The compiled register becomes the source-root bank. Its
                    // preparation contractions remain in the prefix receipts;
                    // execution trails subsequent changes from this boundary.
                    self.trail.clear();self.bank_cache=Some(0);self.open_holding();
                }
                '⊤'=>{
                    assert_eq!(pc,11);assert_eq!(self.domains[root],T);
                    assert!(self.domains.iter().all(|&mask|mask!=0));
                    if let Some(r)=&self.resident {
                        assert!(r.state[..r.frames.len()].iter().all(|&mask|mask!=0));
                        assert!((0..r.pairs.len()).all(|pair|r.mask(pair)!=0));
                    }
                }
                _=>panic!("unexpected preparation-prefix operation"),
            }
            steps.push(json!({"pc":pc,"glyph":word[pc].to_string(),
                "phase_before":before.2,"phase_after":self.phase,
                "contractions_before":before.0,"contractions_after":self.contractions,
                "product_visits_before":before.1,"product_visits_after":self.product_visits,
                "pending_before":before.3,"pending_after":self.queue.len(),
                "native_types_before":native_before,"native_types_after":native_after}));
        }
        self.preparation_control=Some(json!({"word":CONTROL_WORD,"source_word":source,"next_pc":12,
            "executed_positions":(0..12).collect::<Vec<_>>(),"steps":steps,
            "phase":self.phase,"carried_T_F_images":self.image,"root":root,
            "holding_frame":self.holding.as_ref().unwrap().report()}));
    }
    fn control_verify(&mut self,source:&str,started:Instant,deadline:Option<Instant>)->Option<Value> {
        if self.verification_tail.is_none(){
            let (p,q)=self.readout_words();self.verification_tail=Some(VerificationTail::new(p,q));
        }
        loop {
            let pc=self.verification_tail.as_ref().unwrap().pc;
            assert!((36..=44).contains(&pc));
            // Once fixation has fired, the terminal readout follows without
            // exposing a fixed carrier as an open checkpoint.
            if pc<=43 && deadline.is_some_and(|at|Instant::now()>=at){return None;}
            let closure=self.control_verify_step(source,started);
            if closure.is_some(){return closure;}
            if pc<42{self.maybe_checkpoint();}
        }
    }
    fn control_verify_step(&mut self,source:&str,started:Instant)->Option<Value> {
            let n=ImasmNumeralWord::checked(source.trim()).unwrap();
            let word:Vec<char>=CONTROL_WORD.chars().collect();
            let mut tail=self.verification_tail.take().unwrap();let pc=tail.pc;
            assert_eq!(tail.steps,(36..pc).collect::<Vec<_>>(),"verification cursor lost its executed operations");
            match pc {
                36=>{
                    assert_eq!(word[pc],'⊤');
                    assert!(arithmetic::compare(&tail.divisor,&arithmetic::one()).unwrap().is_gt()
                        && arithmetic::compare(&tail.divisor,&n).unwrap().is_lt(),"readout is not a proper divisor");
                }
                37=>{
                    assert_eq!(word[pc],'⋈');
                    let (q,r)=arithmetic::divide(&n,&tail.divisor).unwrap();
                    tail.quotient=Some(q);tail.remainder=Some(r);
                }
                38=>{
                    assert_eq!(word[pc],'⊤');
                    assert_eq!(tail.remainder.as_ref().unwrap(),&arithmetic::zero(),"source division has a nonzero remainder");
                }
                39=>{
                    assert_eq!(word[pc],'⊤');
                    let product=arithmetic::multiply(&tail.divisor,tail.quotient.as_ref().unwrap()).unwrap();
                    assert_eq!(product,n,"native product differs from the exact source");tail.product=Some(product);
                }
                40=>{
                    assert_eq!(word[pc],'⊤');
                    assert_eq!(tail.quotient.as_ref().unwrap(),&tail.paired,"derived quotient differs from the paired lane");
                }
                41=>{
                    assert_eq!(word[pc],'⊙');
                    assert_eq!(self.readout_words(),(tail.divisor.clone(),tail.paired.clone()),"selected factor register changed during verification");
                    assert_eq!(tail.product.as_ref().unwrap().as_str(),source.trim(),"source identity changed before enclosing return");
                }
                42=>{
                    assert_eq!(word[pc],'∋');
                    // Return the actual complete selected register. The holding
                    // frame remains unfixed until the next word operation.
                    tail.returned_bank=Some(self.intern_bank(0,self.overlay(0)));
                }
                43=>{
                    assert_eq!(word[pc],'⊡');
                    assert_eq!(self.readout_words(),(tail.divisor.clone(),tail.paired.clone()));
                    assert!(arithmetic::compare(&tail.divisor,&arithmetic::one()).unwrap().is_gt()
                        && arithmetic::compare(&tail.divisor,&n).unwrap().is_lt());
                    assert_eq!(tail.remainder.as_ref().unwrap(),&arithmetic::zero());
                    assert_eq!(tail.quotient.as_ref().unwrap(),&tail.paired);
                    assert_eq!(arithmetic::multiply(&tail.divisor,&tail.paired).unwrap(),n);
                    let bank=tail.returned_bank.unwrap();
                    assert_eq!(ungzip_bytes(&self.banks.get(bank).1),self.overlay(0),"selected register changed after enclosing return");
                    self.fix_verified_register();
                    assert_eq!(self.holding.as_ref().unwrap().returned_bank,tail.returned_bank);
                    tail.verified_seconds=Some(started.elapsed().as_secs_f64());
                }
                44=>{
                    assert_eq!(word[pc],'⊣');
                    assert!(self.holding.as_ref().unwrap().fixed);
                }
                _=>unreachable!(),
            }
            tail.steps.push(pc);tail.pc+=1;
            let closure=if pc==44{Some(json!({"divisor_word":tail.divisor.as_str(),
                "quotient_word":tail.quotient.as_ref().unwrap().as_str(),"zero_remainder":true,
                "exact_native_imasm_product":true,"paired_boundary_closure":true,
                "trigger_to_verified_factor_seconds":tail.verified_seconds.unwrap()}))}else{None};
            self.verification_tail=Some(tail);
            closure
    }
    fn hop(&mut self) {
        self.assert_live();
        let table=&self.hops[self.phase % self.hops.len()];
        self.image=self.image.map(|value|table[value as usize]);
        self.phase+=1; self.bridge_applications+=1;
        if let Some(r)=&mut self.resident {r.advance();}
    }
    fn enqueue_all(&mut self) {
        for i in 0..self.ops.len() {
            if [3,4,7].contains(&self.ops[i][0]) { self.enqueue(i); }
        }
        for i in 0..self.cells.len() {self.enqueue(self.ops.len()+i);}
        if let Some(r)=&self.resident {
            let count=r.pairs.len();let start=self.ops.len()+self.cells.len();
            for i in 0..count {self.enqueue(start+i);}
        }
    }
    fn attach_resident(&mut self, encoded:&str, prepared:Option<(&[u8],&[u8])>) {
        let building=prepared.is_none();
        let mut r=if let Some((topology,bank))=prepared {
            Resident::restore_topology(&self.ops,self.cells.len(),encoded,topology,bank)
        } else {Resident::new(&self.ops,&self.cells,&self.cell_rows,encoded)};
        r.seek(self.phase);
        self.queued.resize(self.ops.len()+self.cells.len()+r.pairs.len(),false);
        self.resident=Some(r);if building{self.enqueue_all();}
    }
    fn value(&self, slot:usize) -> u8 {
        if slot<self.ops.len(){self.domains[slot]}
        else {self.resident.as_ref().unwrap().state[slot-self.ops.len()]}
    }
    fn set_value(&mut self, slot:usize, value:u8) {
        if slot<self.ops.len(){self.domains[slot]=value;}
        else {self.resident.as_mut().unwrap().state[slot-self.ops.len()]=value;}
    }
    fn restrict_resident(&mut self, slot:usize, allowed:u8) {
        self.assert_live();
        let old=self.resident.as_ref().unwrap().state[slot];let next=old&allowed;
        if old==next {return;}
        self.bank_cache=None;self.trail.push((self.ops.len()+slot,old));
        let r=self.resident.as_mut().unwrap();r.state[slot]=next;
        if slot<r.frames.len() {
            r.row_removals+=(old.count_ones()-next.count_ones()) as u64;
            let gate=r.frames[slot].gate;let readers=r.adjacent[slot].clone();
            self.enqueue(gate);
            let start=self.ops.len()+self.cells.len();
            for edge in readers {self.enqueue(start+edge);}
        }
    }
    fn propagate_resident(&mut self, gate:usize) -> bool {
        let start=self.ops.len()+self.cells.len();
        if gate>=start {
            let edge=gate-start;
            let r=self.resident.as_ref().unwrap();
            let (mask,left,right)=r.pair_projection(edge);
            let pair=&r.pairs[edge];let (a,b)=(pair.left,pair.right);
            let slot=r.frames.len()+8*edge;let old=r.mask(edge);
            for (offset,byte) in mask.to_le_bytes().into_iter().enumerate() {self.restrict_resident(slot+offset,byte);}
            let r=self.resident.as_mut().unwrap();r.pair_visits+=1;
            r.pair_removals+=(old.count_ones()-mask.count_ones()) as u64;
            self.restrict_resident(a,left);self.restrict_resident(b,right);
            self.rows_examined+=64;
            mask!=0
        } else {
            let r=self.resident.as_ref().unwrap();let frame=r.frame_by_gate[gate];
            assert_ne!(frame,usize::MAX);
            let (mask,supports)=r.frame_projection(frame,&self.domains);
            let ports=r.frames[frame].ports.clone();let rows=r.frames[frame].rows.len();
            self.restrict_resident(frame,mask);
            self.resident.as_mut().unwrap().native_visits+=1;
            self.rows_examined+=rows as u64;
            if gate>=self.ops.len(){self.joint_visits+=1;}
            for (wire,support) in ports.into_iter().zip(supports) {
                if !self.intersect(wire,support){return false;}
            }
            mask!=0
        }
    }
    fn enqueue(&mut self, i:usize) {
        if !self.queued[i] {self.queued[i]=true;self.queue.push_back(i);}
    }
    fn intersect(&mut self, wire:usize, allowed:u8) -> bool {
        self.assert_live();
        let old=self.domains[wire];let next=old & allowed;
        if old!=next {
            self.bank_cache=None;
            self.trail.push((wire,old));self.domains[wire]=next;self.contractions+=1;
            if self.prepared_readers.is_some(){
                let count=self.prepared_readers.as_ref().unwrap().len(wire);
                for at in 0..count {let reader=self.prepared_readers.as_ref().unwrap().get(wire,at);self.enqueue(reader);}
            } else {
                let readers=self.uses[wire].clone();for reader in readers {self.enqueue(reader);}
            }
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
    fn return_product(&mut self)->Result<bool,()> {
        let Some(source)=self.product_source.as_ref() else{return Ok(false);};
        let width=self.inputs.len()/2;let mut p=vec![B;width];let mut q=p.clone();
        for &(variable,wire) in &self.inputs {
            if variable&1==0{p[variable/2]=self.domains[wire];}else{q[variable/2]=self.domains[wire];}
        }
        let returned=resident_product_return::refine(source,&p,&q);
        self.product_visits+=1;
        self.product_banks.push(json!({"bank":returned.bank,"p_before":p,"q_before":q,
            "p_after":returned.p,"q_after":returned.q,"phase":self.phase,
            "trail_boundary":self.trail.len(),"stage":if self.preparing{"preparation"}else{"execution"}}));
        let before=self.contractions;
        // Both quotient arms act on this same retained register. Their support
        // returns enter the ordinary wire/row/pair queue and its common trail.
        for index in 0..self.inputs.len(){let(variable,wire)=self.inputs[index];
            let support=if variable&1==0{returned.p[variable/2]}else{returned.q[variable/2]};
            if !self.intersect(wire,support){self.product_contractions+=self.contractions-before;return Err(());}
        }
        self.product_contractions+=self.contractions-before;
        Ok(self.contractions!=before)
    }
    fn propagate(&mut self, deadline:Option<Instant>) -> Option<bool> {
        while !self.queue.is_empty() {
            self.checkpoint_ticks+=1;if self.checkpoint_ticks & 65535==0 {self.maybe_checkpoint();}
            let i=self.queue.pop_front().unwrap();
            self.queued[i]=false;
            if deadline.is_some_and(|limit|Instant::now()>=limit) {
                self.queue.push_front(i);self.queued[i]=true;return None;
            }
            if self.resident.is_some() {
                if !self.propagate_resident(i){return Some(false);}
                continue;
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
        self.assert_live();
        while self.trail.len()>at {let(wire,old)=self.trail.pop().unwrap();self.set_value(wire,old);}
        self.queue.clear();self.queued.fill(false);
        self.bank_cache=None;
    }
    fn overlay(&self, at:usize) -> Vec<u8> {
        let mut wires:Vec<_>=self.trail[at..].iter().map(|&(wire,_)|wire).collect();
        wires.sort_unstable();wires.dedup();let mut bytes=Vec::new();let mut previous=0usize;
        for wire in wires {
            let bits=if self.resident.is_some(){8}else{2};
            let mut code=(((wire+1-previous) as u64)<<bits)|self.value(wire) as u64;
            previous=wire+1;
            loop {let byte=(code&127) as u8;code>>=7;bytes.push(byte|if code!=0 {128}else{0});if code==0 {break;}}
        }
        bytes
    }
    fn decode(bytes:&[u8],bits:u32) -> Vec<(usize,u8)> {
        let mut result=Vec::new();let(mut code,mut shift,mut previous)=(0u64,0u32,0usize);
        for &byte in bytes {
            code|=((byte&127) as u64)<<shift;
            if byte&128!=0 {shift+=7;assert!(shift<64);continue;}
            let gap=(code>>bits) as usize;assert!(gap>0);let wire=previous+gap-1;
            result.push((wire,(code&((1<<bits)-1)) as u8));previous=wire+1;code=0;shift=0;
        }
        assert_eq!(shift,0);result
    }
    fn intern_bank(&mut self,parent:usize,bytes:Vec<u8>) -> usize {
        if parent==0 && bytes.is_empty(){return 0;}
        let mut hash=DefaultHasher::new();parent.hash(&mut hash);bytes.hash(&mut hash);let key=hash.finish();
        if let Some(ids)=self.bank_index.get(&key) {
            for &id in ids {if self.banks.get(id).0==parent && ungzip_bytes(&self.banks.get(id).1)==bytes {return id;}}
        }
        let id=self.banks.len();self.banks.push((parent,gzip_bytes(&bytes)));self.bank_index.entry(key).or_default().push(id);id
    }
    fn parent_bank(&mut self) -> usize {
        if let Some(bank)=self.bank_cache{return bank;}
        let bytes=self.overlay(0);let bank=self.intern_bank(0,bytes);self.bank_cache=Some(bank);bank
    }
    fn begin_nested(&mut self,wire:usize) {
        let bank=self.parent_bank();let at=self.trail.len();
        let alternatives=crate::reentry_powerset::bind(crate::reentry_powerset::unit(bank),
            |_|Ok::<_,()>([(wire,T),(wire,F)].into_iter().collect())).unwrap();
        assert_eq!(alternatives.len(),2);
        // One environment acts on the complete correlated family. Both
        // children share that frame, rather than resampling its clock.
        if self.control_enabled {
            assert_eq!(CONTROL_WORD.chars().nth(if self.nested_block==0{19}else{29}),Some('∈'));
            self.nested_control_pc=if self.nested_block==0{20}else{30};self.nested_consistent=true;
        }else{self.hop();}
        self.nested=Some((wire,at,bank,0,0,false));self.nested_entries+=1;
        self.intersect(wire,T);
    }
    fn finish_nested(&mut self,consistent:bool) -> bool {
        self.control_pc=12;
        let(wire,at,parent,stage,first,first_ok)=self.nested.take().unwrap();
        let overlay=self.overlay(at);let child=self.intern_bank(parent,overlay);
        self.restore(at);self.bank_cache=Some(parent);
        if stage==0 {
            self.nested=Some((wire,at,parent,1,child,consistent));self.intersect(wire,F);return true;
        }
        let bits=if self.resident.is_some(){8}else{2};
        let a=Self::decode(&ungzip_bytes(&self.banks.get(first).1),bits);let b=Self::decode(&ungzip_bytes(&self.banks.get(child).1),bits);
        let mut affected:Vec<_>=a.iter().chain(&b).map(|&(wire,_)|wire).collect();
        affected.sort_unstable();affected.dedup();let(mut ai,mut bi)=(0,0);let mut common=Vec::new();
        if first_ok || consistent {
            for target in affected {
                while ai<a.len() && a[ai].0<target {ai+=1;}
                while bi<b.len() && b[bi].0<target {bi+=1;}
                let base=self.value(target);
                let av=if !first_ok {0}else if ai<a.len() && a[ai].0==target {a[ai].1}else{base};
                let bv=if !consistent {0}else if bi<b.len() && b[bi].0==target {b[bi].1}else{base};
                let merged=av|bv;assert!(merged & !base==0);
                if merged!=base {
                    common.push((target,merged));
                    if target<self.ops.len(){self.intersect(target,merged);}
                    else {self.restrict_resident(target-self.ops.len(),merged);}
                    self.nested_common+=1;
                    if target<self.ops.len() && self.ops[target][0]==8 {self.nested_factor_common+=1;}
                    self.nested_pass_changed=true;
                }
            }
        }
        self.nested_receipts.push((wire,parent,first,child,first_ok,consistent,self.phase,self.image,common));
        first_ok || consistent
    }
    fn next_branch(&mut self) -> bool {
        self.control_pc=12;
        self.outer_pc=if self.nested_block==0{18}else{26};
        self.nested_cursor=self.inputs.len();self.nested_pass_changed=false;
        while let Some((wire,alternative,at))=self.decisions.pop() {
            self.restore(at);self.hop();
            if alternative!=0 {
                self.decisions.push((wire,0,at));
                return self.intersect(wire,alternative);
            }
        }
        false
    }
    fn control_propagate(&mut self,deadline:Option<Instant>)->Option<bool> {
        if !self.control_enabled {return self.propagate(deadline);}
        let word:Vec<char>=CONTROL_WORD.chars().collect();
        while self.control_pc<18 {
            match word[self.control_pc] {
                '≻'=>{
                    self.hop();self.enqueue_all();self.control_pc+=1;
                }
                '⋈'=>{
                    let outcome=self.propagate(deadline);
                    if outcome!=Some(true){return outcome;}
                    self.control_pc+=1;
                }
                _=>panic!("unexpected opcode in the resident dialect block"),
            }
        }
        self.control_pc=12;self.control_blocks+=1;Some(true)
    }
    fn reverse_banked(&mut self) {
        self.reverse_banked_at(None);
    }
    fn reverse_banked_at(&mut self,word_pc:Option<usize>) {
        self.assert_live();
        let (boundary,parent)=self.nested.map_or((0,0),|(_,at,bank,_,_,_)|(at,bank));
        let bytes=self.overlay(boundary);let bank=self.intern_bank(parent,bytes.clone());
        let held_trail=self.trail[boundary..].to_vec();
        let held_queue=std::mem::take(&mut self.queue);let cached=self.bank_cache;
        self.restore(boundary);
        let bits=if self.resident.is_some(){8}else{2};
        let restored=Self::decode(&ungzip_bytes(&self.banks.get(bank).1),bits);
        let cleared_slots=restored.iter().filter(|&&(slot,value)|self.value(slot)!=value).count();
        let split_recovery=self.nested.map(|(wire,_,_,_,_,_)|json!({"wire":wire,
            "cleared":self.domains[wire],"recovered":restored.iter().find(|&&(slot,_)|slot==wire).unwrap().1}));
        for &(slot,value) in &restored {self.set_value(slot,value);}
        self.trail.extend(held_trail);self.queue=held_queue;self.queued.fill(false);
        for &gate in &self.queue{self.queued[gate]=true;}
        self.bank_cache=cached;
        assert_eq!(self.overlay(boundary),bytes,"banked reversal changed a wire, row or pair");
        self.reversal_receipts.push(json!({"parent_bank":parent,"before_bank":bank,"after_bank":bank,
            "trail_boundary":boundary,"trail_length":self.trail.len(),"pending_gates":self.queue.len(),
            "phase":self.phase,"image":self.image,"correlated_state_recovered":true,
            "cleared_slots":cleared_slots,"split_recovery":split_recovery,"word_pc":word_pc}));
    }
    fn control_seam(&mut self,deadline:Option<Instant>)->Option<bool> {
        assert!(self.nested.is_none());
        let word:Vec<char>=CONTROL_WORD.chars().collect();
        loop {
            match self.outer_pc {
                18|27=>{
                    assert_eq!(word[self.outer_pc],'≺');
                    self.reverse_banked_at(Some(self.outer_pc));self.outer_pc+=1;
                }
                26=>{
                    assert_eq!(word[26],'⋈');
                    match self.propagate(deadline) {
                        Some(true)=>self.outer_pc=27,
                        other=>return other,
                    }
                }
                28=>{
                    assert_eq!(word[28],'≻');self.hop();self.enqueue_all();
                    self.outer_pc=29;self.outer_seams+=1;
                }
                19|29=>return Some(true),
                _=>panic!("invalid retained surrounding-word cursor"),
            }
        }
    }
    fn continue_legacy_work(&mut self,deadline:Option<Instant>)->Option<bool> {
        let phase=self.legacy_phase.unwrap();assert_eq!(self.phase,phase);
        loop {
            let mut outcome=self.propagate(deadline);
            if outcome.is_none(){return None;}
            if outcome==Some(true) {
                match self.return_product(){Ok(true)=>continue,Err(())=>outcome=Some(false),Ok(false)=>{}}
            }
            assert_eq!(self.phase,phase,"legacy propagation repeated a dialect advance");
            self.legacy_phase=None;
            if self.nested.is_some(){self.nested_consistent=outcome.unwrap();self.nested_control_pc=20;}
            return outcome;
        }
    }
    fn control_nested(&mut self,deadline:Option<Instant>)->Option<bool> {
        assert!(self.control_enabled && self.nested.is_some());
        let word:Vec<char>=CONTROL_WORD.chars().collect();
        loop {
            match self.nested_control_pc {
                // Old checkpoints may stop inside the former dialect block.
                // Finish that pending block once before entering the new body.
                19=>{
                    if self.legacy_phase.is_some(){
                        if self.continue_legacy_work(deadline).is_none(){return None;}
                        continue;
                    }
                    let result=self.control_propagate(deadline);
                    if result.is_none(){return None;}
                    self.nested_consistent=result.unwrap();self.nested_control_pc=20;
                }
                20|30=>{
                    assert_eq!(word[self.nested_control_pc],'⊤');
                    // The truth condition belongs to the retained arithmetic
                    // child, whose T/F selection was made at the fork.
                    self.nested_control_pc=if self.nested_consistent{if self.nested_block==0{21}else{31}}else{if self.nested_block==0{25}else{35}};
                }
                21|23|31=>{
                    assert_eq!(word[self.nested_control_pc],'≻');
                    self.hop();self.enqueue_all();self.nested_control_pc+=1;
                }
                22=>{
                    assert_eq!(word[22],'⋈');
                    if self.return_product().is_err(){self.nested_consistent=false;self.nested_control_pc=25;}
                    else{self.nested_control_pc=23;}
                }
                24|32=>{
                    assert_eq!(word[self.nested_control_pc],'⋈');
                    match self.propagate(deadline) {
                        None=>return None,
                        Some(ok)=>{self.nested_consistent=ok;self.nested_control_pc=if self.nested_block==0{25}else if ok{33}else{35};}
                    }
                }
                33=>{
                    assert_eq!(word[33],'≺');self.reverse_banked_at(Some(33));self.nested_control_pc=34;
                }
                34=>{
                    assert_eq!(word[34],'⊞');
                    // Retain the complete correlated child in a source-rooted
                    // bank. The hold does not latch or discard alternatives.
                    let (parent,at)=self.nested.map(|(_,at,parent,_,_,_)|(parent,at)).unwrap();
                    self.intern_bank(parent,self.overlay(at));self.nested_control_pc=35;
                }
                25|35=>{
                    assert_eq!(word[self.nested_control_pc],'∋');
                    let consistent=self.finish_nested(self.nested_consistent);
                    if self.nested.is_none(){
                        self.nested_block^=1;
                        self.outer_pc=if self.nested_block==1{26}else{18};
                    }
                    self.nested_control_pc=if self.nested_block==0{20}else{30};self.nested_consistent=true;
                    if self.nested.is_none(){return Some(consistent);}
                }
                _=>panic!("invalid retained inner-word cursor"),
            }
        }
    }
    fn search(&mut self, seconds:u64) -> &'static str {
        let deadline=if seconds==0 {None}else{Some(Instant::now()+Duration::from_secs(seconds))};
        loop {
            self.maybe_checkpoint();
            if self.legacy_phase.is_some() {
                match self.continue_legacy_work(deadline) {
                    None=>return "held",
                    Some(false) if self.nested.is_none()=>if !self.next_branch(){return "empty";}else{continue;},
                    Some(_)=>{}
                }
            }
            if self.control_enabled && self.nested.is_some() {
                match self.control_nested(deadline) {
                    None=>return "held",
                    Some(false)=>if !self.next_branch(){return "empty";},
                    Some(true)=>{}
                }
                continue;
            }
            // A paused surrounding CLINK owns its pending queue. Complete it
            // before entering another dialect block or choosing another child.
            if self.control_enabled && self.nested_enabled && self.outer_pc==26 {
                match self.control_seam(deadline) {
                    None=>return "held",
                    Some(false)=>if !self.next_branch(){return "empty";}else{continue;},
                    Some(true)=>{}
                }
            }
            let mut outcome=self.control_propagate(deadline);
            if outcome==Some(true) {
                match self.return_product(){Ok(true)=>continue,Err(())=>outcome=Some(false),Ok(false)=>{}}
            }
            if self.nested.is_some() {
                match outcome {
                    None=>return "held",
                    Some(true) if self.inputs.iter().all(|&(_,w)|self.domains[w]!=B)=>return "assigned",
                    Some(ok)=>{
                        if !self.finish_nested(ok) && !self.next_branch(){return "empty";}
                        continue;
                    }
                }
            }
            match outcome {
                None=>return "held",
                Some(false)=>if !self.next_branch(){return "empty";}else{continue;},
                Some(true)=>{}
            }
            let unresolved=self.inputs.iter().find(|(_,wire)|self.domains[*wire]==B).copied();
            let Some((_,wire))=unresolved else {return "assigned";};
            if self.nested_enabled {
                let mut next=None;
                while self.nested_cursor>0 {
                    self.nested_cursor-=1;let candidate=self.inputs[self.nested_cursor].1;
                    if self.domains[candidate]==B {next=Some(candidate);break;}
                }
                if let Some(candidate)=next {
                    if self.control_enabled {
                        match self.control_seam(deadline) {
                            None=>{self.nested_cursor+=1;return "held";},
                            Some(false)=>if !self.next_branch(){return "empty";}else{continue;},
                            Some(true)=>{}
                        }
                    }
                    self.begin_nested(candidate);continue;
                }
                if self.nested_pass_changed {
                    self.nested_pass_changed=false;self.nested_cursor=self.inputs.len();continue;
                }
                self.nested_cursor=self.inputs.len();
            }
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
            self.nested_pass_changed=false;
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
    let(cells,rows)=joint_bank(&ops,out,joint);
    let engine=Engine::new(ops,Vec::new(),vec![core::array::from_fn(|i|i as u8)],cells,rows);
    std::fs::write(out.join("reader.index"),ReaderIndex::encode(&engine.uses)).unwrap();
}

pub fn prepare(source:&str, out:&std::path::Path, hops:Vec<[u8;16]>,joint:bool) {
    prepare_resident(source,out,hops,joint,None)
}

pub fn prepare_resident(source:&str, out:&std::path::Path, hops:Vec<[u8;16]>,joint:bool,native:Option<&str>) {
    prepare_with_product(source,out,hops,joint,native,false)
}

pub fn prepare_with_product(source:&str, out:&std::path::Path, hops:Vec<[u8;16]>,joint:bool,native:Option<&str>,product:bool) {
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
    let mut engine=Engine::new_indexed_seeded(program.ops,domains,hops,cells,rows,None,false);
    if let Some(program)=native {engine.attach_resident(program,None);}
    engine.preparing=true;
    if product{engine.product_source=Some(word.clone());}
    engine.prepare_word_prefix(source.trim(),program.root as usize);
    if product {
        std::fs::write(out.join("product.prepared.json"),serde_json::to_vec_pretty(&json!({
            "banks":engine.product_banks,"visits":engine.product_visits,"contractions":engine.product_contractions})).unwrap()).unwrap();
    }
    let mut bytes=Vec::with_capacity(engine.ops.len()*16);
    for op in &engine.ops {for field in op {bytes.extend_from_slice(&field.to_le_bytes());}}
    std::fs::write(out.join("relation.ops"),bytes).unwrap();
    std::fs::write(out.join("register.cells"),&engine.domains).unwrap();
    std::fs::write(out.join("reader.index"),ReaderIndex::encode(&engine.uses)).unwrap();
    if let Some(r)=&engine.resident {
        std::fs::write(out.join("correlated.cells"),&r.state).unwrap();
        std::fs::write(out.join("correlated.topology"),r.topology()).unwrap();
        std::fs::write(out.join("correlated_prepared.json"),serde_json::to_vec_pretty(&json!({
            "frames":r.frames.len(),"pairs":r.pairs.len(),"native_visits":r.native_visits,
            "pair_visits":r.pair_visits,"row_removals":r.row_removals,
            "pair_removals":r.pair_removals,"route":r.route})).unwrap()).unwrap();
    }
    std::fs::write(out.join("prepared.json"),serde_json::to_vec_pretty(&json!({
        "source_word":source.trim(),"source_bits":n.bits(),"wires":engine.ops.len(),
        "factor_lane_width":program.width,"width_boundary":"both lanes use N bits; no supplied factor width",
        "prepared_contractions":engine.contractions,"prepared_rows_examined":engine.rows_examined,
        "prepared_bridge_image":engine.image,"preparation_seconds":started.elapsed().as_secs_f64(),
        "control_prefix":engine.preparation_control,
        "prepared_product_return":product,
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

fn hex(bytes:&[u8]) -> String {
    const DIGITS:&[u8]=b"0123456789abcdef";
    let mut out=String::with_capacity(bytes.len()*2);
    for &byte in bytes {out.push(DIGITS[(byte>>4) as usize] as char);out.push(DIGITS[(byte&15) as usize] as char);}
    out
}
fn unhex(text:&str) -> Vec<u8> {
    assert_eq!(text.len()%2,0);
    text.as_bytes().chunks_exact(2).map(|pair|u8::from_str_radix(std::str::from_utf8(pair).unwrap(),16).unwrap()).collect()
}
fn gzip_bytes(bytes:&[u8]) -> Vec<u8> {
    let mut encoded=flate2::write::GzEncoder::new(Vec::new(),flate2::Compression::fast());
    encoded.write_all(bytes).unwrap();encoded.finish().unwrap()
}
fn ungzip_bytes(bytes:&[u8]) -> Vec<u8> {
    if bytes.is_empty(){return Vec::new();}
    let mut reader=flate2::read::GzDecoder::new(bytes);let mut result=Vec::new();
    reader.read_to_end(&mut result).unwrap();result
}
fn gzip_hex(bytes:&[u8]) -> String {
    let mut encoded=flate2::write::GzEncoder::new(Vec::new(),flate2::Compression::fast());
    encoded.write_all(bytes).unwrap();hex(&encoded.finish().unwrap())
}
fn ungzip_hex(text:&str) -> Vec<u8> {
    let bytes=unhex(text);let mut reader=flate2::read::GzDecoder::new(bytes.as_slice());
    let mut result=Vec::new();reader.read_to_end(&mut result).unwrap();result
}

pub fn execute(source:&str, packed:&[u8], prepared:&[u8], hops:Vec<[u8;16]>, seconds:u64,
    resumed:Option<&str>,packed_cells:&[u8],packed_rows:&[u8],nested:bool) -> Value {
    execute_resident(source,packed,prepared,hops,seconds,resumed,packed_cells,packed_rows,nested,None)
}

pub fn execute_resident(source:&str, packed:&[u8], prepared:&[u8], hops:Vec<[u8;16]>, seconds:u64,
    resumed:Option<&str>,packed_cells:&[u8],packed_rows:&[u8],nested:bool,native:Option<(&str,&[u8],&[u8])>) -> Value {
    execute_with_product(source,packed,prepared,hops,seconds,resumed,packed_cells,packed_rows,nested,native,None)
}

pub fn execute_with_product(source:&str, packed:&[u8], prepared:&[u8], hops:Vec<[u8;16]>, seconds:u64,
    resumed:Option<&str>,packed_cells:&[u8],packed_rows:&[u8],nested:bool,native:Option<(&str,&[u8],&[u8])>,product:Option<&str>) -> Value {
    execute_indexed(source,packed,prepared,hops,seconds,resumed,packed_cells,packed_rows,nested,native,product,None)
}

pub fn execute_indexed(source:&str, packed:&[u8], prepared:&[u8], hops:Vec<[u8;16]>, seconds:u64,
    resumed:Option<&str>,packed_cells:&[u8],packed_rows:&[u8],nested:bool,native:Option<(&str,&[u8],&[u8])>,product:Option<&str>,readers:Option<&[u8]>) -> Value {
    execute_archived(source,packed,prepared,hops,seconds,resumed,packed_cells,packed_rows,nested,native,product,readers,None)
}

pub fn execute_archived(source:&str, packed:&[u8], prepared:&[u8], hops:Vec<[u8;16]>, seconds:u64,
    resumed:Option<&str>,packed_cells:&[u8],packed_rows:&[u8],nested:bool,native:Option<(&str,&[u8],&[u8])>,product:Option<&str>,readers:Option<&[u8]>,evidence:Option<&str>) -> Value {
    execute_embedded(source,packed,prepared,hops,seconds,resumed,packed_cells,packed_rows,nested,native,product,readers,evidence,&[])
}

pub fn execute_embedded(source:&str,packed:&[u8],prepared:&[u8],hops:Vec<[u8;16]>,seconds:u64,
    resumed:Option<&str>,packed_cells:&[u8],packed_rows:&[u8],nested:bool,native:Option<(&str,&[u8],&[u8])>,product:Option<&str>,readers:Option<&[u8]>,evidence:Option<&str>,embedded:&[(&str,&'static[u8])]) -> Value {
    execute_prepared_embedded(source,packed,prepared,hops,seconds,resumed,packed_cells,packed_rows,nested,native,product,readers,evidence,embedded,None)
}

pub fn execute_prepared_embedded(source:&str,packed:&[u8],prepared:&[u8],hops:Vec<[u8;16]>,seconds:u64,
    resumed:Option<&str>,packed_cells:&[u8],packed_rows:&[u8],nested:bool,native:Option<(&str,&[u8],&[u8])>,product:Option<&str>,readers:Option<&[u8]>,evidence:Option<&str>,embedded:&[(&str,&'static[u8])],prepared_control:Option<&str>) -> Value {
    let started=Instant::now();assert_eq!(packed.len()%16,0);
    let ops=packed.chunks_exact(16).map(|row|core::array::from_fn(|i|
        u32::from_le_bytes(row[4*i..4*i+4].try_into().unwrap()))).collect();
    assert_eq!(packed_cells.len()%20,0);assert_eq!(packed_rows.len(),40);
    let cells=packed_cells.chunks_exact(20).map(|r|core::array::from_fn(|i|
        u32::from_le_bytes(r[4*i..4*i+4].try_into().unwrap()) as usize)).collect();
    let rows=core::array::from_fn(|i|core::array::from_fn(|j|packed_rows[5*i+j]));
    let mut engine=Engine::new_indexed(ops,prepared.to_vec(),hops,cells,rows,readers);
    if let Some((program,topology,bank))=native {engine.attach_resident(program,Some((topology,bank)));}
    if let Some(bank)=product {
        engine.product_source=Some(ImasmNumeralWord::checked(source.trim()).unwrap());
        let prepared:Value=serde_json::from_str(bank).unwrap();
        engine.product_banks=Archive::load_bound(&prepared["banks"],embedded);
    }
    // The embedded register and joint banks are already at their preparation
    // fixed point. A fresh trigger starts with pending transformations; an
    // exact continuation restores its own pending queue below.
    if engine.resident.is_some(){engine.queue.clear();engine.queued.fill(false);}
    engine.nested_enabled=nested;engine.control_enabled=true;
    if let Some(encoded)=resumed {
        let previous:Value=serde_json::from_str(encoded).unwrap();
        assert_eq!(previous["source_word"].as_str(),Some(source.trim()));
        assert_eq!(previous["status"].as_str(),Some("held"));
        if let Some(control)=previous.get("control_word").filter(|v|!v.is_null()) {
            assert_eq!(control["word"],CONTROL_WORD);
            engine.control_pc=control["dialect_pc"].as_u64().unwrap() as usize;
            assert!((12..18).contains(&engine.control_pc));
            engine.control_blocks=control["completed_dialect_blocks"].as_u64().unwrap();
        }
        engine.domains=serde_json::from_value(previous["register"].clone()).unwrap();
        assert_eq!(engine.domains.len(),engine.ops.len());
        if let Some(packed)=previous["continuation"]["trail_gzip_hex"].as_str() {
            let raw=ungzip_hex(packed);assert_eq!(raw.len()%9,0);
            engine.trail=raw.chunks_exact(9).map(|row|
                (u64::from_le_bytes(row[..8].try_into().unwrap()) as usize,row[8])).collect();
        } else {engine.trail=serde_json::from_value(previous["continuation"]["trail"].clone()).unwrap();}
        engine.decisions=serde_json::from_value(previous["continuation"]["decisions"].clone()).unwrap();
        if let Some(packed)=previous["continuation"]["pending_gates_gzip_hex"].as_str() {
            let raw=ungzip_hex(packed);assert_eq!(raw.len()%8,0);
            engine.queue=raw.chunks_exact(8).map(|row|u64::from_le_bytes(row.try_into().unwrap()) as usize).collect();
        } else {engine.queue=serde_json::from_value(previous["continuation"]["pending_gates"].clone()).unwrap();}
        engine.queued.fill(false);for &gate in &engine.queue {engine.queued[gate]=true;}
        engine.image=serde_json::from_value(previous["carried_T_F_images"].clone()).unwrap();
        engine.phase=previous["bridge_phase"].as_u64().unwrap() as usize;
        engine.contractions=previous["contractions"].as_u64().unwrap();
        engine.rows_examined=previous["rows_examined"].as_u64().unwrap();
        engine.branches=previous["branches"].as_u64().unwrap();
        engine.bridge_applications=previous["bridge_applications"].as_u64().unwrap();
        if product.is_some(){
            if let Some(saved)=previous.get("source_product_return").filter(|v|!v.is_null()) {
                engine.product_banks=Archive::load_bound(&saved["banks"],embedded);
                engine.product_visits=saved["visits"].as_u64().unwrap();
                engine.product_contractions=saved["contractions"].as_u64().unwrap();
            }
        }
        if let Some(r)=&mut engine.resident {
            assert_eq!(previous["bank_value_bits"],8);
            let saved=&previous["resident_dialect"];
            assert_eq!(saved["route"],r.route);
            let state=if let Some(packed)=saved["state_gzip_hex"].as_str(){ungzip_hex(packed)}
                else {unhex(saved["state_hex"].as_str().unwrap())};
            assert_eq!(state.len(),r.state.len());
            assert!(state.iter().zip(&r.state).all(|(&a,&b)|a & !b==0));
            r.state=state;
            r.seek(engine.phase);
            assert_eq!(saved["native_types"],serde_json::to_value(&r.native_types).unwrap());
            r.native_visits=saved["native_visits"].as_u64().unwrap();
            r.pair_visits=saved["pair_visits"].as_u64().unwrap();
            r.row_removals=saved["row_removals"].as_u64().unwrap();
            r.pair_removals=saved["pair_removals"].as_u64().unwrap();
        }
        engine.bank_cache=None;
        if let Some(value)=previous.get("register_banks") {
            if previous["register_banks_encoding"]=="gzip-bytes" {
                engine.banks=Archive::load_bound(value,embedded);
                assert_eq!(engine.banks.get(0),(0,Vec::new()));
                engine.bank_index.clear();
            }else{
            let banks=Archive::<(usize,Value)>::load_bound(value,embedded);assert!(banks.get(0).1==json!("") || banks.get(0).1==json!([]));
            for (id,(parent,encoded)) in banks.iter().enumerate().skip(1) {
                assert!(parent<id);
                let bytes=if previous["register_banks_encoding"]=="gzip-bytes" {ungzip_bytes(&serde_json::from_value::<Vec<u8>>(encoded).unwrap())}
                    else if previous["register_banks_encoding"]=="gzip-hex" {ungzip_hex(encoded.as_str().unwrap())}else{unhex(encoded.as_str().unwrap())};
                assert_eq!(engine.intern_bank(parent,bytes),id);
            }
            }
        }
        engine.nested=serde_json::from_value(previous["continuation"].get("nested_frame").cloned().unwrap_or(Value::Null)).unwrap();
        if let Some(inner)=previous.get("inner_word_control") {
            engine.nested_control_pc=inner["pc"].as_u64().unwrap() as usize;
            engine.nested_consistent=inner["consistent"].as_bool().unwrap();
            engine.nested_block=inner["block"].as_u64().unwrap_or(0) as u8;
            assert!(engine.nested_block<2);
            assert!((19..=25).contains(&engine.nested_control_pc) || (30..=35).contains(&engine.nested_control_pc));
        }else if engine.nested.is_some(){engine.nested_control_pc=19;}
        if let Some(outer)=previous.get("outer_word_control") {
            engine.outer_pc=outer["pc"].as_u64().unwrap() as usize;
            engine.outer_seams=outer["completed_interblock_seams"].as_u64().unwrap();
            assert!(matches!(engine.outer_pc,18|19|26|27|28|29));
        }else{
            // Retain the old child's execution point. New surrounding work
            // starts only after that child's existing fuse has returned.
            engine.outer_pc=match (engine.nested.is_some(),engine.nested_block) {
                (true,0)=>19,(true,1)=>29,(false,0)=>18,(false,1)=>26,_=>unreachable!(),
            };
        }
        engine.nested_cursor=previous["continuation"]["nested_cursor"].as_u64().map_or(engine.inputs.len(),|v|v as usize);
        assert!(engine.nested_cursor<=engine.inputs.len());
        engine.nested_pass_changed=previous["continuation"]["nested_pass_changed"].as_bool().unwrap_or(false);
        engine.nested_receipts=Archive::load_bound(previous.get("nested_receipts").unwrap_or(&json!([])),embedded);
        engine.reversal_receipts=Archive::load_bound(previous.get("reversal_receipts").unwrap_or(&json!([])),embedded);
        engine.nested_entries=previous["nested_entries"].as_u64().unwrap_or(0);
        engine.nested_common=previous["nested_common_contractions"].as_u64().unwrap_or(0);
        engine.nested_factor_common=previous["nested_factor_bit_contractions"].as_u64().unwrap_or(0);
        let parent_cells=previous["joint_cells"].as_u64().unwrap_or(0) as usize;
        assert!(parent_cells==0 || parent_cells==engine.cells.len());
        engine.joint_visits=previous["joint_cell_visits"].as_u64().unwrap_or(0);
        engine.joint_contractions=previous["joint_cell_contractions"].as_u64().unwrap_or(0);
        if parent_cells==0 {
            for cell in 0..engine.cells.len(){engine.enqueue(engine.ops.len()+cell);}
        }
    }
    if let Some(encoded)=resumed {
        let previous:Value=serde_json::from_str(encoded).unwrap();
        if let Some(frame)=previous.get("holding_frame").filter(|v|!v.is_null()) {
            let held=HoldingFrame::load(frame);
            assert!(!held.fixed && held.returned_bank.is_none());
            assert!(held.entry_bank<engine.banks.len() && held.entry_phase<=engine.phase);
            engine.holding=Some(held);
        }
    }
    let compiled_control=prepared_control.map(|s|serde_json::from_str::<Value>(s).unwrap());
    if let Some(encoded)=resumed {
        let previous:Value=serde_json::from_str(encoded).unwrap();
        if let Some(prefix)=previous.get("preparation_word_control") {
            if let Some(compiled)=&compiled_control{assert_eq!(prefix,&compiled["control_prefix"]);}
            engine.preparation_control=Some(prefix.clone());
        }else if let Some(prefix)=compiled_control.as_ref().and_then(|v|v.get("control_prefix")).filter(|v|!v.is_null()) {
            // Cargo re-executed this prefix and checked every prepared baseline
            // byte before attaching it to an older, fully retained continuation.
            assert_eq!(prefix["word"],CONTROL_WORD);assert_eq!(prefix["source_word"],source.trim());
            if engine.holding.is_none(){engine.holding=Some(HoldingFrame::load(&prefix["holding_frame"]));}
            engine.preparation_control=Some(prefix.clone());
        }
    }else if let Some(prefix)=compiled_control.as_ref().and_then(|v|v.get("control_prefix")).filter(|v|!v.is_null()) {
        assert_eq!(prefix["word"],CONTROL_WORD);assert_eq!(prefix["source_word"],source.trim());
        assert_eq!(prefix["phase"],engine.phase);assert_eq!(prefix["carried_T_F_images"],json!(engine.image));
        assert_eq!(prefix["next_pc"],12);
        let held=HoldingFrame::load(&prefix["holding_frame"]);
        assert_eq!(held.entry_bank,0);assert_eq!(held.entry_trail,0);
        assert!(!held.fixed && held.returned_bank.is_none());engine.holding=Some(held);
        engine.preparation_control=Some(prefix.clone());
    }
    if engine.holding.is_none(){engine.open_holding();}
    if let Some(encoded)=resumed {
        let previous:Value=serde_json::from_str(encoded).unwrap();
        if let Some(pending)=previous.get("legacy_pending_work") {
            assert_eq!(pending["action"],"propagate_source_product");
            engine.legacy_phase=Some(pending["phase"].as_u64().unwrap() as usize);
            assert_eq!(engine.legacy_phase,Some(engine.phase));
        }else if previous.get("control_word").is_none_or(Value::is_null){
            // The old search checkpoint precedes propagate/return_product.
            // Its branch advance has already happened and must not be replayed.
            engine.legacy_phase=Some(engine.phase);
        }
    }
    if let Some(encoded)=resumed {
        let previous:Value=serde_json::from_str(encoded).unwrap();
        if let Some(saved)=previous.get("verification_word_control").filter(|v|!v.is_null()) {
            let tail=VerificationTail::load(saved);
            assert!((36..=43).contains(&tail.pc));
            assert_eq!(engine.readout_words(),(tail.divisor.clone(),tail.paired.clone()));
            engine.verification_tail=Some(tail);
        }
    }
    if let Some(directory)=evidence {
        // Execution may select its output archive location. All computational
        // inputs, including resumed archive prefixes, remain in the image.
        let directory=std::env::var_os("RESIDENT_EVIDENCE_DIR").map(std::path::PathBuf::from)
            .unwrap_or_else(||std::path::PathBuf::from(directory));
        assert!(!directory.as_os_str().is_empty());std::fs::create_dir_all(&directory).unwrap();
        engine.banks.migrate(&directory.join("register.records"));
        engine.nested_receipts.migrate(&directory.join("nested.records"));
        engine.reversal_receipts.migrate(&directory.join("reversal.records"));
        engine.product_banks.migrate(&directory.join("product.records"));
        engine.checkpoint=Some(Checkpoint{directory,source:source.trim().into(),started,last:Instant::now()});
        engine.write_checkpoint();
    }
    let status=if engine.verification_tail.is_some(){"assigned"}else{engine.search(seconds)};
    let mut closure=Value::Null;
    if status=="assigned" {
        closure=engine.control_verify(source,started,None).unwrap();
    }
    engine.report(source,status,closure,started.elapsed().as_secs_f64())
}

impl<'a> Engine<'a> {
    fn full_word_observed(&self)->bool {
        if !self.control_enabled || self.preparation_control.is_none()
            || !self.verification_tail.as_ref().is_some_and(|tail|tail.pc==45)
            || self.control_blocks==0 || self.outer_seams==0 || self.nested_block!=0
            || self.nested.is_some() || self.nested_receipts.len()<2 {return false;}
        let mut observed=[false;3];
        for event in self.reversal_receipts.iter(){
            match event["word_pc"].as_u64(){Some(18)=>observed[0]=true,Some(27)=>observed[1]=true,
                Some(33)=>observed[2]=true,_=>{}}
        }
        observed.into_iter().all(|present|present)
    }
    fn report(&self,source:&str,status:&str,closure:Value,readout:f64)->Value {
    let resident=self.resident.as_ref().map(|r|json!({
        "route":r.route,"stage":(self.phase-1)%r.route.as_array().unwrap().len(),
        "frames":r.frames.len(),"pairs":r.pairs.len(),"state_gzip_hex":gzip_hex(&r.state),
        "native_visits":r.native_visits,"pair_visits":r.pair_visits,
        "native_steps":r.native_steps,"native_types":r.native_types,
        "row_removals":r.row_removals,"pair_removals":r.pair_removals}));
    let packed_trail=self.resident.as_ref().map(|_| {
        let mut raw=Vec::with_capacity(self.trail.len()*9);
        for &(slot,old) in &self.trail {raw.extend_from_slice(&(slot as u64).to_le_bytes());raw.push(old);}
        gzip_hex(&raw)
    });
    let packed_queue=self.resident.as_ref().map(|_| {
        let mut raw=Vec::with_capacity(self.queue.len()*8);
        for &gate in &self.queue {raw.extend_from_slice(&(gate as u64).to_le_bytes());}
        gzip_hex(&raw)
    });
    let mut report=json!({"source_word":source.trim(),"status":status,"closure":closure,
        "prepared_dependency_incidence":self.prepared_readers.is_some(),
        "holding_frame":self.holding.as_ref().map(HoldingFrame::report),
        "inner_word_control":{"pc":self.nested_control_pc,"consistent":self.nested_consistent,"block":self.nested_block,
            "implemented_ranges_zero_based":[[19,26],[29,36]]},
        "outer_word_control":{"pc":self.outer_pc,"completed_interblock_seams":self.outer_seams,
            "implemented_positions_zero_based":[18,26,27,28]},
        "reversal_receipts":self.reversal_receipts.view(),
        "control_word":if self.control_enabled {json!({"word":CONTROL_WORD,
            "dialect_pc":self.control_pc,"completed_dialect_blocks":self.control_blocks,
            "implemented_range_zero_based":[12,18],"full_word_execution":self.full_word_observed()})}else{Value::Null},
        "source_product_return":if self.product_source.is_some(){json!({"banks":self.product_banks.view(),
            "visits":self.product_visits,"contractions":self.product_contractions})}else{Value::Null},
        "resident_dialect":resident,"bank_value_bits":if self.resident.is_some(){8}else{2},
        "contractions":self.contractions,"rows_examined":self.rows_examined,"branches":self.branches,
        "bridge_applications":self.bridge_applications,"bridge_phase":self.phase,
        "joint_cells":self.cells.len(),"joint_cell_visits":self.joint_visits,
        "joint_cell_contractions":self.joint_contractions,
        "nested_entries":self.nested_entries,"nested_common_contractions":self.nested_common,
        "nested_factor_bit_contractions":self.nested_factor_common,
        "nested_receipts":self.nested_receipts.view(),
        "register_banks_encoding":"gzip-bytes",
        "register_banks":self.banks.view(),
        "carried_T_F_images":self.image,"transport_factor_readouts":0,
        "trigger_to_readout_seconds":readout,"register":self.domains,
        "continuation":{"trail":if packed_trail.is_some(){json!([])}else{json!(self.trail)},
            "trail_gzip_hex":packed_trail,"trail_length":self.trail.len(),
            "decisions":self.decisions,"pending_gates":if packed_queue.is_some(){json!([])}else{json!(self.queue)},
            "pending_gates_gzip_hex":packed_queue,"pending_gates_length":self.queue.len(),
            "nested_frame":self.nested,"nested_cursor":self.nested_cursor,
            "nested_pass_changed":self.nested_pass_changed},
        "source_bank_retained":true,"inverse_fibres_retained":true,
        "scope":"source relation, arithmetic register and retained continuation"});
    if let Some(tail)=&self.verification_tail {report["verification_word_control"]=tail.report();}
    if let Some(prefix)=&self.preparation_control {report["preparation_word_control"]=prefix.clone();}
    if let Some(phase)=self.legacy_phase{report["legacy_pending_work"]=json!({"phase":phase,"action":"propagate_source_product"});}
    report
    }
    fn maybe_checkpoint(&mut self) {
        if self.checkpoint.as_ref().is_some_and(|c|c.last.elapsed()>=Duration::from_secs(60)){self.write_checkpoint();}
    }
    fn write_checkpoint(&mut self) {
        let Some(c)=&self.checkpoint else{return;};
        let directory=c.directory.clone();let source=c.source.clone();let elapsed=c.started.elapsed().as_secs_f64();
        self.banks.sync();self.nested_receipts.sync();self.reversal_receipts.sync();self.product_banks.sync();
        let report=self.report(&source,"held",Value::Null,elapsed);
        let mut file=std::io::BufWriter::with_capacity(1024*1024,std::fs::File::create(directory.join("checkpoint.next")).unwrap());
        serde_json::to_writer(&mut file,&report).unwrap();file.flush().unwrap();file.get_ref().sync_all().unwrap();
        std::fs::rename(directory.join("checkpoint.next"),directory.join("checkpoint.json")).unwrap();
        self.checkpoint.as_mut().unwrap().last=Instant::now();
    }
}

#[cfg(test)]
mod resident_tests {
    use super::*;
    #[test]
    fn resident_legacy_pending_work_finishes_before_new_word_advances() {
        let native=std::fs::read_to_string(std::env::var("NATIVE_ARITHMETIC_PROGRAM").unwrap()).unwrap();
        let ops=vec![[8,0,0,0],[8,1,0,0],[4,0,1,0],[7,0,1,0]];
        let mut engine=Engine::new(ops,vec![B,B,F,T],vec![core::array::from_fn(|i|i as u8)],Vec::new(),crate::joint_cells::full_adder_rows());
        engine.attach_resident(&native,None);assert_eq!(engine.propagate(None),Some(true));
        let parent=engine.resident.as_ref().unwrap().state.clone();
        // The frozen old begin_nested advances once before selecting its T
        // child. Resume its pending queue at that same native phase.
        engine.begin_nested(0);let phase=engine.phase;
        engine.control_enabled=true;engine.nested_control_pc=19;engine.outer_pc=19;
        engine.legacy_phase=Some(phase);let queue=engine.queue.clone();
        assert_eq!(engine.continue_legacy_work(Some(Instant::now())),None);
        assert_eq!(engine.queue,queue);assert_eq!(engine.phase,phase);
        assert_eq!(engine.legacy_phase,Some(phase));assert_eq!(engine.nested_control_pc,19);
        assert_eq!(engine.report("legacy-pending-instrument","held",Value::Null,0.0)["legacy_pending_work"]["phase"],phase);
        assert_eq!(engine.continue_legacy_work(None),Some(true));
        assert_eq!(engine.phase,phase);assert_eq!(engine.control_blocks,0);
        assert!(engine.legacy_phase.is_none());assert_eq!(engine.nested_control_pc,20);
        assert_eq!(engine.control_nested(None),Some(true));
        assert_eq!(engine.phase,phase+4);assert!(engine.nested.is_none());
        assert_eq!(engine.domains,vec![B,B,F,T]);assert_eq!(engine.resident.as_ref().unwrap().state,parent);
    }
    // These are verification-only readout slots, loaded from a completed
    // historical production receipt. No factor is supplied to preparation or
    // search. The factor source in that historical receipt remains MPQS.
    fn emitted_verification_register()->(Engine<'static>,String) {
        let receipt:Value=serde_json::from_str(&std::fs::read_to_string(
            std::env::var("VERIFIED_FACTOR_RECEIPT").unwrap()).unwrap()).unwrap();
        let n=receipt["n"].as_str().unwrap().parse::<num_bigint::BigUint>().unwrap();
        assert!(n.bits()>=200);assert_eq!(receipt["product_verified"],true);
        let line=receipt["stdout"].as_str().unwrap().lines().find(|s|s.starts_with("factor=")).unwrap();
        let fields:HashMap<_,_>=line.split_whitespace().filter_map(|s|s.split_once('=')).collect();
        let p=fields["factor"].parse::<num_bigint::BigUint>().unwrap();
        let q=fields["cofactor"].parse::<num_bigint::BigUint>().unwrap();assert_eq!(&p*&q,n);
        let width=n.bits() as usize;let mut ops=Vec::new();let mut domains=Vec::new();
        for bit in 0..width {
            for value in [&p,&q] {
                ops.push([8,ops.len() as u32,0,0]);domains.push(if value.bit(bit as u64){T}else{F});
            }
        }
        let n=numeral(&(0..width).map(|bit|if n.bit(bit as u64){T}else{F}).collect::<Vec<_>>());
        let mut engine=Engine::new(ops,domains,vec![core::array::from_fn(|i|i as u8)],Vec::new(),crate::joint_cells::full_adder_rows());
        engine.open_holding();(engine,n.as_str().to_owned())
    }
    #[test]
    fn resident_verification_tail_resumes_native_checks_before_return_and_fixation() {
        let (mut engine,source)=emitted_verification_register();let started=Instant::now();
        assert_eq!(engine.control_verify(&source,started,Some(Instant::now())),None);
        assert_eq!(engine.verification_tail.as_ref().unwrap().pc,36);
        for _ in 36..39{assert_eq!(engine.control_verify_step(&source,started),None);}
        let saved=engine.verification_tail.as_ref().unwrap().report();
        let pending=engine.report(&source,"held",Value::Null,started.elapsed().as_secs_f64());
        assert_eq!(saved["pc"],39);assert_eq!(saved["executed_positions"],json!([36,37,38]));
        assert!(saved["quotient_word"].is_string());assert!(saved["product_word"].is_null());
        engine.verification_tail=Some(VerificationTail::load(&saved));
        for _ in 39..43{assert_eq!(engine.control_verify_step(&source,started),None);}
        assert_eq!(engine.verification_tail.as_ref().unwrap().pc,43);
        assert!(!engine.holding.as_ref().unwrap().fixed);
        assert!(engine.holding.as_ref().unwrap().returned_bank.is_none());
        assert!(engine.verification_tail.as_ref().unwrap().returned_bank.is_some());
        let returned=engine.report(&source,"held",Value::Null,started.elapsed().as_secs_f64());
        let closure=engine.control_verify(&source,started,None).unwrap();
        assert_eq!(closure["zero_remainder"],true);assert_eq!(closure["paired_boundary_closure"],true);
        assert_eq!(engine.verification_tail.as_ref().unwrap().steps,(36..45).collect::<Vec<_>>());
        assert!(engine.holding.as_ref().unwrap().fixed);
        let closed=engine.report(&source,"assigned",closure,started.elapsed().as_secs_f64());
        let out=std::path::PathBuf::from(std::env::var("RESIDENT_EVIDENCE_TEST_DIR").unwrap());
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("verification_tail.json"),serde_json::to_vec_pretty(&json!({
            "scope":"verification of a previously emitted 208-bit readout; no producer invocation",
            "input_wires":engine.inputs,"snapshots":[pending,returned,closed]})).unwrap()).unwrap();
    }
    #[test]
    fn resident_verification_tail_rejects_changed_pair_and_changed_return_before_fixation() {
        let (mut engine,source)=emitted_verification_register();let started=Instant::now();
        engine.domains[1]=if engine.domains[1]==T{F}else{T};
        let rejected=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||engine.control_verify(&source,started,None)));
        assert!(rejected.is_err());assert!(!engine.holding.as_ref().unwrap().fixed);
        let (mut engine,source)=emitted_verification_register();
        let (p,q)=engine.readout_words();engine.verification_tail=Some(VerificationTail::new(p,q));
        for _ in 36..43{assert_eq!(engine.control_verify_step(&source,started),None);}
        // A live contraction after the fuse must not be silently fixed.
        engine.intersect(0,0);
        let rejected=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||engine.control_verify_step(&source,started)));
        assert!(rejected.is_err());assert!(!engine.holding.as_ref().unwrap().fixed);
    }
    #[test]
    fn resident_surrounding_link_resumes_before_one_reversal_and_advance() {
        let native=std::fs::read_to_string(std::env::var("NATIVE_ARITHMETIC_PROGRAM").unwrap()).unwrap();
        let ops=vec![[8,0,0,0],[8,1,0,0],[4,0,1,0],[7,0,1,0]];
        let mut engine=Engine::new(ops,vec![B,B,F,T],vec![core::array::from_fn(|i|i as u8)],Vec::new(),crate::joint_cells::full_adder_rows());
        engine.attach_resident(&native,None);assert_eq!(engine.propagate(None),Some(true));
        engine.open_holding();engine.control_enabled=true;
        let state=engine.resident.as_ref().unwrap().state.clone();
        let trail=engine.trail.clone();let phase=engine.phase;
        assert_eq!(engine.control_seam(None),Some(true));assert_eq!(engine.outer_pc,19);
        assert_eq!(engine.reversal_receipts.get(0)["word_pc"],18);
        assert_eq!(engine.domains,vec![B,B,F,T]);assert_eq!(engine.trail,trail);
        engine.nested_block=1;engine.outer_pc=26;engine.enqueue_all();
        let queue=engine.queue.clone();
        assert_eq!(engine.control_seam(Some(Instant::now())),None);
        assert_eq!(engine.outer_pc,26);assert_eq!(engine.queue,queue);
        assert_eq!(engine.phase,phase);assert_eq!(engine.reversal_receipts.len(),1);
        let saved=engine.report("seam-instrument","held",Value::Null,0.0);
        assert_eq!(saved["outer_word_control"]["pc"],26);
        assert_eq!(engine.control_seam(None),Some(true));
        assert_eq!(engine.outer_pc,29);assert_eq!(engine.phase,phase+1);
        assert_eq!(engine.outer_seams,1);assert_eq!(engine.reversal_receipts.len(),2);
        assert_eq!(engine.reversal_receipts.get(1)["word_pc"],27);
        assert_eq!(engine.domains,vec![B,B,F,T]);assert_eq!(engine.resident.as_ref().unwrap().state,state);
        // Entering the ready fork again cannot repeat either seam operation.
        assert_eq!(engine.control_seam(None),Some(true));
        assert_eq!(engine.phase,phase+1);assert_eq!(engine.reversal_receipts.len(),2);
        assert!(!engine.holding.as_ref().unwrap().fixed);
    }
    #[test]
    fn resident_inner_word_resumes_child_then_fuses_correlated_alternatives() {
        let native=std::fs::read_to_string(std::env::var("NATIVE_ARITHMETIC_PROGRAM").unwrap()).unwrap();
        let ops=vec![[8,0,0,0],[8,1,0,0],[4,0,1,0],[7,0,1,0]];
        let mut engine=Engine::new(ops,vec![B,B,F,T],vec![core::array::from_fn(|i|i as u8)],Vec::new(),crate::joint_cells::full_adder_rows());
        engine.attach_resident(&native,None);assert_eq!(engine.propagate(None),Some(true));
        let state=engine.resident.as_ref().unwrap().state.clone();let phase=engine.phase;
        engine.control_enabled=true;engine.begin_nested(0);
        assert_eq!(engine.phase,phase);
        assert_eq!(engine.control_nested(Some(Instant::now())),None);
        assert_eq!(engine.nested_control_pc,24);assert_eq!(engine.phase,phase+2);
        assert_eq!(engine.nested.unwrap().3,0);
        assert_eq!(engine.report("inner-instrument","held",Value::Null,0.0)["inner_word_control"]["pc"],24);
        assert_eq!(engine.control_nested(None),Some(true));
        assert!(engine.nested.is_none());assert_eq!(engine.phase,phase+4);
        assert_eq!(engine.nested_control_pc,30);assert_eq!(engine.nested_receipts.len(),1);
        assert_eq!(engine.domains,vec![B,B,F,T]);
        assert_eq!(engine.resident.as_ref().unwrap().state,state);
        let receipt=engine.nested_receipts.get(0);assert!(receipt.4 && receipt.5);
    }
    #[test]
    fn resident_second_inner_word_recovers_both_children_without_fixing() {
        let native=std::fs::read_to_string(std::env::var("NATIVE_ARITHMETIC_PROGRAM").unwrap()).unwrap();
        let ops=vec![[8,0,0,0],[8,1,0,0],[4,0,1,0],[7,0,1,0]];
        let mut engine=Engine::new(ops,vec![B,B,F,T],vec![core::array::from_fn(|i|i as u8)],Vec::new(),crate::joint_cells::full_adder_rows());
        engine.attach_resident(&native,None);assert_eq!(engine.propagate(None),Some(true));
        engine.open_holding();let state=engine.resident.as_ref().unwrap().state.clone();
        engine.control_enabled=true;engine.nested_block=1;engine.begin_nested(0);
        let phase=engine.phase;
        assert_eq!(engine.control_nested(Some(Instant::now())),None);
        assert_eq!(engine.nested_control_pc,32);assert_eq!(engine.phase,phase+1);
        assert_eq!(engine.control_nested(None),Some(true));
        assert_eq!(engine.phase,phase+2);assert_eq!(engine.nested_control_pc,20);
        assert!(engine.nested.is_none());assert_eq!(engine.reversal_receipts.len(),2);
        assert_eq!(engine.domains,vec![B,B,F,T]);assert_eq!(engine.resident.as_ref().unwrap().state,state);
        assert!(!engine.holding.as_ref().unwrap().fixed);
        for receipt in engine.reversal_receipts.iter(){
            assert!(receipt["cleared_slots"].as_u64().unwrap()>0);
            assert_eq!(receipt["split_recovery"]["cleared"],3);
            assert!(matches!(receipt["split_recovery"]["recovered"].as_u64(),Some(1)|Some(2)));
        }
    }
    #[test]
    fn resident_banked_reversal_preserves_live_queue_and_continuation() {
        let native=std::fs::read_to_string(std::env::var("NATIVE_ARITHMETIC_PROGRAM").unwrap()).unwrap();
        let ops=vec![[8,0,0,0],[8,1,0,0],[4,0,1,0],[7,0,1,0]];
        let mut engine=Engine::new(ops,vec![B,B,F,T],vec![core::array::from_fn(|i|i as u8)],Vec::new(),crate::joint_cells::full_adder_rows());
        engine.attach_resident(&native,None);assert_eq!(engine.propagate(None),Some(true));
        engine.control_enabled=true;engine.begin_nested(0);assert_eq!(engine.propagate(None),Some(true));
        engine.enqueue_all();
        let (trail,queue,decisions,nested)=(engine.trail.clone(),engine.queue.clone(),engine.decisions.clone(),engine.nested);
        let (domains,state)=(engine.domains.clone(),engine.resident.as_ref().unwrap().state.clone());
        let (image,phase)=(engine.image,engine.phase);
        engine.reverse_banked();
        assert_eq!(engine.trail,trail);assert_eq!(engine.queue,queue);assert_eq!(engine.decisions,decisions);
        assert_eq!(engine.nested,nested);assert_eq!(engine.domains,domains);
        assert_eq!(engine.resident.as_ref().unwrap().state,state);assert_eq!((engine.image,engine.phase),(image,phase));
        assert!(engine.reversal_receipts.get(0)["cleared_slots"].as_u64().unwrap()>2);
    }
    #[test]
    fn resident_holding_bank_retains_selected_state_and_latches_mutations() {
        let ops=vec![[8,0,0,0],[8,1,0,0],[4,0,1,0]];
        let mut engine=Engine::new(ops,vec![B,B,F],vec![core::array::from_fn(|i|i as u8)],Vec::new(),crate::joint_cells::full_adder_rows());
        engine.open_holding();let entry=engine.holding.as_ref().unwrap().report();
        assert!(!entry["fixed"].as_bool().unwrap());
        engine.intersect(0,T);assert_eq!(engine.propagate(None),Some(true));
        assert_eq!(engine.domains,vec![T,F,F]);
        // Exercise the latch after the computation; arithmetic verification is
        // checked separately by the production caller before this operation.
        engine.fix_verified_register();
        let held=engine.holding.as_ref().unwrap();let bank=held.returned_bank.unwrap();
        let saved=HoldingFrame::load(&held.report());assert!(saved.fixed);
        assert_eq!(saved.entry_bank,0);
        let pairs=Engine::decode(&ungzip_bytes(&engine.banks.get(bank).1),2);
        assert_eq!(pairs,vec![(0,T),(1,F)]);
        let before=engine.domains.clone();
        let rejected=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||engine.intersect(0,F)));
        assert!(rejected.is_err());assert_eq!(engine.domains,before);
    }
    #[test]
    fn resident_control_cursor_preserves_interrupted_dialect_action() {
        let native=std::fs::read_to_string(std::env::var("NATIVE_ARITHMETIC_PROGRAM").unwrap()).unwrap();
        let ops=vec![[8,0,0,0],[8,1,0,0],[4,0,1,0],[7,0,1,0]];
        let mut engine=Engine::new(ops,vec![B,B,F,T],vec![core::array::from_fn(|i|i as u8)],Vec::new(),crate::joint_cells::full_adder_rows());
        engine.attach_resident(&native,None);engine.control_enabled=true;
        let before=engine.phase;
        assert_eq!(engine.control_propagate(Some(Instant::now())),None);
        assert_eq!(engine.phase,before+1);assert_eq!(engine.control_pc,13);
        assert!(!engine.queue.is_empty());
        let snapshot=engine.report("cursor-instrument","held",Value::Null,0.0);
        assert_eq!(snapshot["control_word"]["dialect_pc"],13);
        assert_eq!(snapshot["control_word"]["completed_dialect_blocks"],0);
        assert_eq!(engine.control_propagate(None),Some(true));
        assert_eq!(engine.phase,before+3);assert_eq!(engine.control_pc,12);
        assert_eq!(engine.control_blocks,1);
        assert_eq!(engine.domains,vec![B,B,F,T]);
        assert_eq!(engine.resident.as_ref().unwrap().state[0].count_ones(),2);
        assert_eq!(engine.resident.as_ref().unwrap().state[1].count_ones(),2);
    }
    #[test]
    fn native_correlation_rollback_and_static_recovery() {
        let native=std::fs::read_to_string(std::env::var("NATIVE_ARITHMETIC_PROGRAM").unwrap()).unwrap();
        // Two arithmetic predicates share both inputs. Marginal supports
        // retain all four bits; their source-bound join retains x != y.
        let ops=vec![[8,0,0,0],[8,1,0,0],[4,0,1,0],[7,0,1,0]];
        let mut engine=Engine::new(ops,vec![B,B,F,T],vec![core::array::from_fn(|i|i as u8)],Vec::new(),crate::joint_cells::full_adder_rows());
        engine.attach_resident(&native,None);
        assert_eq!(engine.propagate(Some(Instant::now()+Duration::from_secs(10))),Some(true));
        assert_eq!(engine.domains,vec![B,B,F,T]);
        let resident=engine.resident.as_ref().unwrap();
        assert_eq!(resident.state[0].count_ones(),2);
        assert_eq!(resident.state[1].count_ones(),2);
        let saved=resident.state.clone();let topology=resident.topology();
        let restored=Resident::restore_topology(&engine.ops,0,&native,&topology,&saved);
        assert_eq!(restored.state,saved);assert_eq!(restored.topology(),topology);
        let at=engine.trail.len();engine.intersect(0,T);
        assert_eq!(engine.propagate(Some(Instant::now()+Duration::from_secs(10))),Some(true));
        assert_eq!(engine.domains[1],F);
        engine.restore(at);
        assert_eq!(engine.domains,vec![B,B,F,T]);
        assert_eq!(engine.resident.as_ref().unwrap().state,saved);
        // Both byte-level pair fibres and their frame supports survive a
        // nested split/fuse on the same parent, rather than being rebuilt.
        engine.begin_nested(0);
        let first=engine.propagate(Some(Instant::now()+Duration::from_secs(10))).unwrap();
        assert!(engine.finish_nested(first));
        let second=engine.propagate(Some(Instant::now()+Duration::from_secs(10))).unwrap();
        assert!(engine.finish_nested(second));
        assert_eq!(engine.resident.as_ref().unwrap().state,saved);
    }
}
