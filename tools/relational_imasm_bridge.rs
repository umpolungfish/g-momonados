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
    nested_enabled:bool,
    // wire, parent trail boundary, parent bank, child stage, first bank, first consistency
    nested:Option<(usize,usize,usize,u8,usize,bool)>,
    nested_cursor:usize, nested_pass_changed:bool,
    banks:Vec<(usize,Vec<u8>)>, bank_index:HashMap<u64,Vec<usize>>, bank_cache:Option<usize>,
    // wire, parent, T bank, F bank, T/F consistency, phase, image, common deductions
    nested_receipts:Vec<(usize,usize,usize,usize,bool,bool,usize,[u8;2],Vec<(usize,u8)>)>,
    nested_entries:u64, nested_common:u64, nested_factor_common:u64,
    resident:Option<Resident>,
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
            bridge_applications:0,cells,cell_rows,joint_visits:0,joint_contractions:0,
            nested_enabled:false,nested:None,nested_cursor:0,nested_pass_changed:false,
            banks:vec![(0,Vec::new())],bank_index:HashMap::new(),bank_cache:Some(0),
            nested_receipts:Vec::new(),nested_entries:0,nested_common:0,nested_factor_common:0,
            resident:None };
        result.nested_cursor=result.inputs.len();
        result.hop(); result.enqueue_all(); result
    }
    fn hop(&mut self) {
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
        let mut r=if let Some((topology,bank))=prepared {
            Resident::restore_topology(&self.ops,self.cells.len(),encoded,topology,bank)
        } else {Resident::new(&self.ops,&self.cells,&self.cell_rows,encoded)};
        r.seek(self.phase);
        self.queued.resize(self.ops.len()+self.cells.len()+r.pairs.len(),false);
        self.resident=Some(r);self.enqueue_all();
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
        let old=self.domains[wire];let next=old & allowed;
        if old!=next {
            self.bank_cache=None;
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
            if Instant::now()>=deadline {
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
            for &id in ids {if self.banks[id].0==parent && self.banks[id].1==bytes {return id;}}
        }
        let id=self.banks.len();self.banks.push((parent,bytes));self.bank_index.entry(key).or_default().push(id);id
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
        self.hop();self.nested=Some((wire,at,bank,0,0,false));self.nested_entries+=1;
        self.intersect(wire,T);
    }
    fn finish_nested(&mut self,consistent:bool) -> bool {
        let(wire,at,parent,stage,first,first_ok)=self.nested.take().unwrap();
        let overlay=self.overlay(at);let child=self.intern_bank(parent,overlay);
        self.restore(at);self.bank_cache=Some(parent);
        if stage==0 {
            self.nested=Some((wire,at,parent,1,child,consistent));self.intersect(wire,F);return true;
        }
        let bits=if self.resident.is_some(){8}else{2};
        let a=Self::decode(&self.banks[first].1,bits);let b=Self::decode(&self.banks[child].1,bits);
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
    fn search(&mut self, seconds:u64) -> &'static str {
        let deadline=Instant::now()+Duration::from_secs(seconds);
        loop {
            let outcome=self.propagate(deadline);
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
                if let Some(candidate)=next {self.begin_nested(candidate);continue;}
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
    joint_bank(&ops,out,joint);
}

pub fn prepare(source:&str, out:&std::path::Path, hops:Vec<[u8;16]>,joint:bool) {
    prepare_resident(source,out,hops,joint,None)
}

pub fn prepare_resident(source:&str, out:&std::path::Path, hops:Vec<[u8;16]>,joint:bool,native:Option<&str>) {
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
    if let Some(program)=native {engine.attach_resident(program,None);}
    // Preparation really contracts the source-bound register to a fixed point.
    // The one-hour instrument boundary is not a production cutoff.
    let consistent=engine.propagate(Instant::now()+Duration::from_secs(3600));
    assert_eq!(consistent,Some(true),"source relation inconsistent or preparation unfinished");
    let mut bytes=Vec::with_capacity(engine.ops.len()*16);
    for op in &engine.ops {for field in op {bytes.extend_from_slice(&field.to_le_bytes());}}
    std::fs::write(out.join("relation.ops"),bytes).unwrap();
    std::fs::write(out.join("register.cells"),&engine.domains).unwrap();
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
    let started=Instant::now();assert_eq!(packed.len()%16,0);
    let ops=packed.chunks_exact(16).map(|row|core::array::from_fn(|i|
        u32::from_le_bytes(row[4*i..4*i+4].try_into().unwrap()))).collect();
    assert_eq!(packed_cells.len()%20,0);assert_eq!(packed_rows.len(),40);
    let cells=packed_cells.chunks_exact(20).map(|r|core::array::from_fn(|i|
        u32::from_le_bytes(r[4*i..4*i+4].try_into().unwrap()) as usize)).collect();
    let rows=core::array::from_fn(|i|core::array::from_fn(|j|packed_rows[5*i+j]));
    let mut engine=Engine::new(ops,prepared.to_vec(),hops,cells,rows);
    if let Some((program,topology,bank))=native {engine.attach_resident(program,Some((topology,bank)));}
    engine.nested_enabled=nested;
    if let Some(encoded)=resumed {
        let previous:Value=serde_json::from_str(encoded).unwrap();
        assert_eq!(previous["source_word"].as_str(),Some(source.trim()));
        assert_eq!(previous["status"].as_str(),Some("held"));
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
        if let Some(banks)=previous.get("register_banks").and_then(Value::as_array) {
            assert_eq!(banks[0],json!([0,""]));
            for (id,bank) in banks.iter().enumerate().skip(1) {
                let parent=bank[0].as_u64().unwrap() as usize;assert!(parent<id);
                let bytes=unhex(bank[1].as_str().unwrap());
                assert_eq!(engine.intern_bank(parent,bytes),id);
            }
        }
        engine.nested=serde_json::from_value(previous["continuation"].get("nested_frame").cloned().unwrap_or(Value::Null)).unwrap();
        engine.nested_cursor=previous["continuation"]["nested_cursor"].as_u64().map_or(engine.inputs.len(),|v|v as usize);
        assert!(engine.nested_cursor<=engine.inputs.len());
        engine.nested_pass_changed=previous["continuation"]["nested_pass_changed"].as_bool().unwrap_or(false);
        engine.nested_receipts=serde_json::from_value(previous.get("nested_receipts").cloned().unwrap_or(json!([]))).unwrap();
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
    let resident=engine.resident.as_ref().map(|r|json!({
        "route":r.route,"stage":(engine.phase-1)%r.route.as_array().unwrap().len(),
        "frames":r.frames.len(),"pairs":r.pairs.len(),"state_gzip_hex":gzip_hex(&r.state),
        "native_visits":r.native_visits,"pair_visits":r.pair_visits,
        "native_steps":r.native_steps,"native_types":r.native_types,
        "row_removals":r.row_removals,"pair_removals":r.pair_removals}));
    let packed_trail=engine.resident.as_ref().map(|_| {
        let mut raw=Vec::with_capacity(engine.trail.len()*9);
        for &(slot,old) in &engine.trail {raw.extend_from_slice(&(slot as u64).to_le_bytes());raw.push(old);}
        gzip_hex(&raw)
    });
    let packed_queue=engine.resident.as_ref().map(|_| {
        let mut raw=Vec::with_capacity(engine.queue.len()*8);
        for &gate in &engine.queue {raw.extend_from_slice(&(gate as u64).to_le_bytes());}
        gzip_hex(&raw)
    });
    json!({"source_word":source.trim(),"status":status,"closure":closure,
        "resident_dialect":resident,"bank_value_bits":if engine.resident.is_some(){8}else{2},
        "contractions":engine.contractions,"rows_examined":engine.rows_examined,"branches":engine.branches,
        "bridge_applications":engine.bridge_applications,"bridge_phase":engine.phase,
        "joint_cells":engine.cells.len(),"joint_cell_visits":engine.joint_visits,
        "joint_cell_contractions":engine.joint_contractions,
        "nested_entries":engine.nested_entries,"nested_common_contractions":engine.nested_common,
        "nested_factor_bit_contractions":engine.nested_factor_common,
        "nested_receipts":engine.nested_receipts,
        "register_banks":engine.banks.iter().map(|(parent,bytes)|json!([parent,hex(bytes)])).collect::<Vec<_>>(),
        "carried_T_F_images":engine.image,"transport_factor_readouts":0,
        "trigger_to_readout_seconds":readout,"register":engine.domains,
        "continuation":{"trail":if packed_trail.is_some(){json!([])}else{json!(engine.trail)},
            "trail_gzip_hex":packed_trail,"trail_length":engine.trail.len(),
            "decisions":engine.decisions,"pending_gates":if packed_queue.is_some(){json!([])}else{json!(engine.queue)},
            "pending_gates_gzip_hex":packed_queue,"pending_gates_length":engine.queue.len(),
            "nested_frame":engine.nested,"nested_cursor":engine.nested_cursor,
            "nested_pass_changed":engine.nested_pass_changed},
        "source_bank_retained":true,"inverse_fibres_retained":true,
        "scope":"source relation, arithmetic register and retained continuation"})
}

#[cfg(test)]
mod resident_tests {
    use super::*;
    #[test]
    fn native_correlation_rollback_and_static_recovery() {
        let native=std::fs::read_to_string(std::env::var("NATIVE_ARITHMETIC_PROGRAM").unwrap()).unwrap();
        // Two arithmetic predicates share both inputs. Marginal supports
        // retain all four bits; their source-bound join retains x != y.
        let ops=vec![[8,0,0,0],[8,1,0,0],[4,0,1,0],[7,0,1,0]];
        let mut engine=Engine::new(ops,vec![B,B,F,T],vec![core::array::from_fn(|i|i as u8)],Vec::new(),crate::joint_cells::full_adder_rows());
        engine.attach_resident(&native,None);
        assert_eq!(engine.propagate(Instant::now()+Duration::from_secs(10)),Some(true));
        assert_eq!(engine.domains,vec![B,B,F,T]);
        let resident=engine.resident.as_ref().unwrap();
        assert_eq!(resident.state[0].count_ones(),2);
        assert_eq!(resident.state[1].count_ones(),2);
        let saved=resident.state.clone();let topology=resident.topology();
        let restored=Resident::restore_topology(&engine.ops,0,&native,&topology,&saved);
        assert_eq!(restored.state,saved);assert_eq!(restored.topology(),topology);
        let at=engine.trail.len();engine.intersect(0,T);
        assert_eq!(engine.propagate(Instant::now()+Duration::from_secs(10)),Some(true));
        assert_eq!(engine.domains[1],F);
        engine.restore(at);
        assert_eq!(engine.domains,vec![B,B,F,T]);
        assert_eq!(engine.resident.as_ref().unwrap().state,saved);
        // Both byte-level pair fibres and their frame supports survive a
        // nested split/fuse on the same parent, rather than being rebuilt.
        engine.begin_nested(0);
        let first=engine.propagate(Instant::now()+Duration::from_secs(10)).unwrap();
        assert!(engine.finish_nested(first));
        let second=engine.propagate(Instant::now()+Duration::from_secs(10)).unwrap();
        assert!(engine.finish_nested(second));
        assert_eq!(engine.resident.as_ref().unwrap().state,saved);
    }
}
