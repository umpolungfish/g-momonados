//! Persistent native arithmetic rows and their shared-wire pair fibres.
//! All mutable alternatives live in byte slots, trailed by the owning engine.
use std::collections::{HashMap, BTreeSet};
use std::sync::Arc;
use serde_json::Value;

#[derive(Clone)]
pub struct Frame {pub gate:usize, pub kind:u32, pub ports:Vec<usize>, pub rows:Arc<Vec<Vec<u8>>>}
#[derive(Clone)]
pub struct Pair {pub left:usize, pub right:usize, pub admitted:u64}
pub struct Resident {
    pub frames:Vec<Frame>, pub pairs:Vec<Pair>, pub state:Vec<u8>,
    pub frame_by_gate:Vec<usize>, pub adjacent:Vec<Vec<usize>>,
    pub native_visits:u64, pub pair_visits:u64, pub row_removals:u64,
    pub pair_removals:u64, pub route:Value,
    program:Value, pub native_types:HashMap<u32,Vec<Vec<u8>>>,
    outputs:HashMap<u32,Vec<u8>>, pub native_steps:usize,
}

impl Resident {
    fn initialize_native(&mut self) {
        for kind in [3u32,4,7] {
            self.native_types.insert(kind,serde_json::from_value(self.program["gates"][kind.to_string()]["stages"][0]["before"].clone()).unwrap());
        }
        self.native_steps=0;self.outputs.clear();
    }
    pub fn advance(&mut self) {
        let phase=self.native_steps%self.route.as_array().unwrap().len();
        let cards:Vec<usize>=serde_json::from_value(self.program["axis_cards"].clone()).unwrap();
        for kind in [3u32,4,7] {
            let gate=&self.program["gates"][kind.to_string()];
            let right:Vec<Vec<u8>>=serde_json::from_value(gate["stages"][0]["right"].clone()).unwrap();
            let truth:Vec<u8>=serde_json::from_value(gate["truth_outputs"].clone()).unwrap();
            let before=&self.native_types[&kind];let mut after=Vec::new();let mut decoder=HashMap::new();
            for ((left,right),&bit) in before.iter().zip(right).zip(&truth) {
                let mut image=Vec::new();
                for axis in 0..12 {
                    let index=left[axis] as usize*cards[axis]+right[axis] as usize;
                    let cell=&self.program["native_axis_tables"][phase][axis][index];
                    image.push(cell[0].as_u64().unwrap() as u8);
                }
                if let Some(previous)=decoder.insert(image.clone(),bit) {assert_eq!(previous,bit,"native continuation lost arithmetic return");}
                after.push(image);
            }
            let decoded=after.iter().map(|image|if decoder[image]==1 {1}else{2}).collect();
            self.outputs.insert(kind,decoded);self.native_types.insert(kind,after);
        }
        self.native_steps+=1;
    }
    pub fn seek(&mut self,steps:usize) {
        self.initialize_native();
        let start=self.program["cycle_start"].as_u64().unwrap() as usize;
        let length=self.program["cycle_length"].as_u64().unwrap() as usize;
        let replay=if steps>start {start+(steps-start)%length}else{steps};
        for _ in 0..replay {self.advance();}
        self.native_steps=steps;
    }
    pub fn topology(&self) -> Vec<u8> {
        let mut templates:Vec<Arc<Vec<Vec<u8>>>>=Vec::new();let mut ids=Vec::new();
        for frame in &self.frames {
            let id=templates.iter().position(|rows|rows==&frame.rows).unwrap_or_else(|| {
                templates.push(Arc::clone(&frame.rows));templates.len()-1
            });ids.push(id);
        }
        let mut out=Vec::new();
        for value in [self.frames.len(),self.pairs.len(),templates.len()] {out.extend_from_slice(&(value as u64).to_le_bytes());}
        for rows in &templates {
            out.push(rows[0].len() as u8);out.push(rows.len() as u8);
            for row in rows.iter(){out.extend(row);}
        }
        for (frame,id) in self.frames.iter().zip(ids) {
            for value in [frame.gate,id] {out.extend_from_slice(&(value as u32).to_le_bytes());}
            for &port in &frame.ports {out.extend_from_slice(&(port as u32).to_le_bytes());}
        }
        for pair in &self.pairs {
            for value in [pair.left,pair.right] {out.extend_from_slice(&(value as u32).to_le_bytes());}
            out.extend_from_slice(&pair.admitted.to_le_bytes());
        }
        out
    }
    pub fn restore_topology(ops:&[[u32;4]],cells:usize,encoded:&str,topology:&[u8],state:&[u8]) -> Self {
        let mut at=0;
        fn read<const N:usize>(raw:&[u8],at:&mut usize)->[u8;N]{let value=raw[*at..*at+N].try_into().unwrap();*at+=N;value}
        let count=u64::from_le_bytes(read(topology,&mut at)) as usize;
        let pair_count=u64::from_le_bytes(read(topology,&mut at)) as usize;
        let template_count=u64::from_le_bytes(read(topology,&mut at)) as usize;
        let mut templates=Vec::new();
        for _ in 0..template_count {
            let width=read::<1>(topology,&mut at)[0] as usize;
            let rows=read::<1>(topology,&mut at)[0] as usize;
            assert!((1..=5).contains(&width) && (1..=8).contains(&rows));
            let mut bank=Vec::new();for _ in 0..rows {
                let row=topology[at..at+width].to_vec();at+=width;
                assert!(row.iter().all(|v|matches!(v,1|2)));bank.push(row);
            }
            templates.push(Arc::new(bank));
        }
        let mut frames=Vec::new();let mut frame_by_gate=vec![usize::MAX;ops.len()+cells];
        for id in 0..count {
            let gate=u32::from_le_bytes(read(topology,&mut at)) as usize;
            let template=u32::from_le_bytes(read(topology,&mut at)) as usize;
            let rows=Arc::clone(&templates[template]);let mut ports=Vec::new();
            for _ in 0..rows[0].len() {let wire=u32::from_le_bytes(read(topology,&mut at)) as usize;assert!(wire<ops.len());ports.push(wire);}
            assert_eq!(frame_by_gate[gate],usize::MAX);frame_by_gate[gate]=id;
            let kind=if gate<ops.len(){ops[gate][0]}else{0};
            frames.push(Frame{gate,kind,ports,rows});
        }
        let mut pairs=Vec::new();let mut adjacent=vec![Vec::new();count];
        for id in 0..pair_count {
            let left=u32::from_le_bytes(read(topology,&mut at)) as usize;
            let right=u32::from_le_bytes(read(topology,&mut at)) as usize;
            let admitted=u64::from_le_bytes(read(topology,&mut at));
            assert!(left<right && right<count);
            adjacent[left].push(id);adjacent[right].push(id);pairs.push(Pair{left,right,admitted});
        }
        assert_eq!(at,topology.len());assert_eq!(state.len(),count+8*pair_count);
        let program:Value=serde_json::from_str(encoded).unwrap();
        let mut result=Self{frames,pairs,state:state.to_vec(),frame_by_gate,adjacent,native_visits:0,pair_visits:0,
            row_removals:0,pair_removals:0,route:program["route"].clone(),program,
            native_types:HashMap::new(),outputs:HashMap::new(),native_steps:0};
        result.initialize_native();result
    }
    pub fn new(ops:&[[u32;4]], cells:&[[usize;5]], cell_rows:&[[u8;5];8], encoded:&str) -> Self {
        let program:Value=serde_json::from_str(encoded).unwrap();
        assert_eq!(program["format"],"native-arithmetic-return-v1");
        let mut primitive:HashMap<u32,Arc<Vec<Vec<u8>>>>=HashMap::new();
        for kind in [3u32,4,7] {
            let gate=&program["gates"][kind.to_string()];
            let inputs:Vec<[u8;2]>=serde_json::from_value(gate["inputs"].clone()).unwrap();
            let mut rows=None;
            for stage in gate["stages"].as_array().unwrap() {
                let images:Vec<Vec<u8>>=serde_json::from_value(stage["images"].clone()).unwrap();
                assert_eq!(inputs.len(),images.len());
                let current:Vec<Vec<u8>>=inputs.iter().zip(images).map(|(&[a,b],image)| {
                    let key=image.iter().map(u8::to_string).collect::<Vec<_>>().join(",");
                    let output=stage["decoder"][key].as_u64().unwrap() as u8;
                    // This verifies the finite arithmetic bridge at import.
                    // Propagation below consumes the resulting native rows.
                    assert_eq!(output,match kind {3=>1-a,4=>a&b,7=>a|b,_=>unreachable!()});
                    let mark=|bit|if bit==1 {1}else{2};
                    if kind==3 {vec![mark(a),mark(output)]}
                    else {vec![mark(a),mark(b),mark(output)]}
                }).collect();
                if let Some(previous)=&rows {assert_eq!(previous,&current);}
                rows=Some(current);
            }
            primitive.insert(kind,Arc::new(rows.unwrap()));
        }
        let mut frames=Vec::new();let mut frame_by_gate=vec![usize::MAX;ops.len()+cells.len()];
        for (wire,&[op,a,b,_]) in ops.iter().enumerate() {
            if let Some(rows)=primitive.get(&op) {
                let ports=if op==3 {vec![a as usize,wire]}else{vec![a as usize,b as usize,wire]};
                let rows=if op!=3 && a==b {Arc::new(rows.iter().filter(|row|row[0]==row[1]).cloned().collect())}
                    else {Arc::clone(rows)};
                frame_by_gate[wire]=frames.len();frames.push(Frame{gate:wire,kind:op,ports,rows});
            }
        }
        let carry_rows=Arc::new(cell_rows.iter().map(|r|r.to_vec()).collect());
        for (i,ports) in cells.iter().enumerate() {
            let gate=ops.len()+i;frame_by_gate[gate]=frames.len();
            frames.push(Frame{gate,kind:0,ports:ports.to_vec(),rows:Arc::clone(&carry_rows)});
        }
        let mut incident=vec![Vec::new();ops.len()];
        for (f,frame) in frames.iter().enumerate() {for &wire in &frame.ports {incident[wire].push(f);}}
        let mut edges=BTreeSet::new();
        for ids in &incident {
            // A connected chain communicates the shared variable without a
            // quadratic clique at each high-fanout factor-bit boundary.
            for pair in ids.windows(2) {edges.insert((pair[0],pair[1]));}
        }
        // Include every actual producer/consumer overlap, including the
        // primitive lowering underneath each native carry cell.
        for (f,frame) in frames.iter().enumerate() {
            for &wire in &frame.ports {
                let producer=frame_by_gate[wire];
                if producer!=usize::MAX && producer!=f {edges.insert((producer.min(f),producer.max(f)));}
            }
        }
        let mut adjacent=vec![Vec::new();frames.len()];let mut pairs=Vec::new();
        for (left,right) in edges {
            let a=&frames[left];let b=&frames[right];
            let shared:Vec<_>=a.ports.iter().enumerate().flat_map(|(i,x)|b.ports.iter().enumerate()
                .filter_map(move |(j,y)|if x==y {Some((i,j))}else{None})).collect();
            assert!(!shared.is_empty());let mut admitted=0u64;
            for (i,ar) in a.rows.iter().enumerate() {for (j,br) in b.rows.iter().enumerate() {
                if shared.iter().all(|&(x,y)|ar[x]==br[y]) {admitted|=1u64<<(8*i+j);}
            }}
            adjacent[left].push(pairs.len());adjacent[right].push(pairs.len());
            pairs.push(Pair{left,right,admitted});
        }
        let mut state:Vec<_>=frames.iter().map(|f|((1u16<<f.rows.len())-1) as u8).collect();
        for pair in &pairs {state.extend_from_slice(&pair.admitted.to_le_bytes());}
        let mut result=Self{frames,pairs,state,frame_by_gate,adjacent,native_visits:0,pair_visits:0,
            row_removals:0,pair_removals:0,route:program["route"].clone(),program,
            native_types:HashMap::new(),outputs:HashMap::new(),native_steps:0};
        result.initialize_native();result
    }
    pub fn mask(&self, pair:usize) -> u64 {
        let at=self.frames.len()+8*pair;
        u64::from_le_bytes(self.state[at..at+8].try_into().unwrap())
    }
    pub fn frame_projection(&self, frame:usize, domains:&[u8]) -> (u8,Vec<u8>) {
        let f=&self.frames[frame];let mut mask=0;let mut supports=vec![0;f.ports.len()];
        for (i,row) in f.rows.iter().enumerate() {
            let output=if f.kind!=0 {
                let input=if f.kind==3 {usize::from(row[0]==1)}else{2*usize::from(row[0]==1)+usize::from(row[1]==1)};
                self.outputs[&f.kind][input]
            } else {*row.last().unwrap()};
            let value=|port:usize|if port+1==row.len(){output}else{row[port]};
            if self.state[frame] & (1<<i)!=0 && f.ports.iter().enumerate().all(|(p,&wire)|domains[wire]&value(p)!=0) {
                mask|=1<<i;for (p,s) in supports.iter_mut().enumerate() {*s|=value(p);}
            }
        }
        (mask,supports)
    }
    pub fn pair_projection(&self, index:usize) -> (u64,u8,u8) {
        let pair=&self.pairs[index];let mut remaining=self.mask(index);let(mut lm,mut rm)=(0,0);
        let old=remaining;
        for i in 0..8 {for j in 0..8 {
            let bit=1u64<<(8*i+j);
            if remaining&bit!=0 {
                if self.state[pair.left]&(1<<i)==0 || self.state[pair.right]&(1<<j)==0 {remaining&=!bit;}
                else {lm|=1<<i;rm|=1<<j;}
            }
        }}
        assert_eq!(remaining & !old,0);(remaining,lm,rm)
    }
}
