//! Source-braid collision contraction. Numerals and signed coefficients remain
//! WordTape/SignedWordTape; indices address one retained arithmetic graph.
pub(crate) mod circuit;
mod coefficients;
use std::path::{Path,PathBuf};
use std::time::Instant;
use serde_json::{json,Value};
use num_bigint::BigInt;
use num_traits::{Zero,One};
use crate::word_tape::{WordTape,SignedWordTape};
use crate::phase_unbraid::FixedPointFormat;
use crate::anyon_ququart::FixedQuquartSic;
use circuit::{Circuit,Wire};
use coefficients::{Arena,Node,Op,Id};

#[derive(Clone,Debug)]
enum Instruction {Force(usize,usize),Neg(usize),Double(usize),Add(usize,usize),Eliminate(usize,usize)}
struct Plan {kind:String,factor:usize,before:Id,instructions:Vec<Instruction>,slots:Vec<Id>,cursor:usize,executed:Vec<Value>}
impl Plan {
    fn new(kind:&str,factor:usize,root:Id)->Self{Self{kind:kind.into(),factor,before:root,instructions:Vec::new(),slots:vec![root,0],cursor:0,executed:Vec::new()}}
    fn push(&mut self,i:Instruction)->usize{let slot=2+self.instructions.len();self.instructions.push(i);slot}
    fn monomial(&mut self,support:Option<Vec<usize>>)->usize{
        let Some(mut support)=support else{return 1;};support.sort_unstable();support.dedup();
        let mut slot=0;for w in support{slot=self.push(Instruction::Force(slot,w));}slot
    }
    fn op(&self)->Op{match self.instructions[self.cursor]{
        Instruction::Force(s,w)=>Op::Force(w,self.slots[s]),Instruction::Neg(s)=>Op::Neg(self.slots[s]),
        Instruction::Double(s)=>Op::Double(self.slots[s]),Instruction::Add(a,b)=>Op::add(self.slots[a],self.slots[b]),
        Instruction::Eliminate(s,w)=>Op::Eliminate(w,self.slots[s])}}
    fn output(&self)->Id{*self.slots.last().unwrap()}
    fn snapshot(&self)->Value{json!({"kind":self.kind,"factor":self.factor,"before":self.before,
        "cursor":self.cursor,"steps":self.executed,"slots":self.slots,
        "instructions":self.instructions.iter().map(|i|match i{
            Instruction::Force(s,w)=>json!(["force",s,w]),Instruction::Neg(s)=>json!(["neg",s]),
            Instruction::Double(s)=>json!(["double",s]),Instruction::Add(a,b)=>json!(["add",a,b]),
            Instruction::Eliminate(s,w)=>json!(["eliminate",s,w])}).collect::<Vec<_>>()})}
}
fn support(wires:&[Wire])->Option<Vec<usize>>{
    let mut out=Vec::new();for w in wires{if w.0==0{return None;}if let Some(i)=w.variable(){out.push(i);}}Some(out)
}
fn exact_sic_kernel(bits:usize)->Result<([[bool;4];4],Value),String>{
    let format=FixedPointFormat::for_modulus_bits(bits as u64)?;
    let frame=FixedQuquartSic::new(&format)?;let dual=frame.prepare_exact_dual()?;
    let mut inputs:Vec<[(BigInt,BigInt);16]>=Vec::new();
    for i in 0..4{let mut g=core::array::from_fn(|_|(BigInt::zero(),BigInt::zero()));g[5*i].0=BigInt::one();inputs.push(g);}
    for i in 0..4{for j in i+1..4{
        for imaginary in [false,true]{
            let mut g=core::array::from_fn(|_|(BigInt::zero(),BigInt::zero()));g[5*i].0=BigInt::one();g[5*j].0=BigInt::one();
            if imaginary{g[4*i+j].1=BigInt::one();g[4*j+i].1=-BigInt::one();}
            else{g[4*i+j].0=BigInt::one();g[4*j+i].0=BigInt::one();}inputs.push(g);
        }
    }}
    let mut recovered=Vec::new();let mut records=Vec::new();
    for g in &inputs{
        let masses=frame.gram_mass_numerators(g)?;let back=dual.apply(&frame,&masses)?;
        if g.iter().zip(&back.numerators).any(|(a,b)|&a.0*&back.denominator!=b.0||&a.1*&back.denominator!=b.1){return Err("source SIC basis did not return exactly".into());}
        records.push(json!({"input":g.iter().map(|(r,i)|[r.to_string(),i.to_string()]).collect::<Vec<_>>(),
            "masses":masses.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "numerators":back.numerators.iter().map(|(r,i)|[r.to_string(),i.to_string()]).collect::<Vec<_>>(),"denominator":back.denominator.to_string()}));
        recovered.push(back);
    }
    let mut kernel=[[false;4];4];
    for i in 0..4{for j in 0..4{
        let mut value=BigInt::zero();for a in 0..4{for b in 0..4{
            let(x,y)=&recovered[i].numerators[4*a+b];let(u,v)=&recovered[j].numerators[4*b+a];value+=x*u-y*v;
        }}
        let denominator=&recovered[i].denominator*&recovered[j].denominator;
        if value.is_zero(){kernel[i][j]=false;}else if value==denominator{kernel[i][j]=true;}else{return Err("SIC equality kernel is not a closed exact cell relation".into());}
    }}
    let one=SignedWordTape::from_small(1);let minus=one.negate();
    let plus=[one.clone(),one.clone()];let negative=[minus.clone(),one.clone()];
    let returned=[plus[0].sub(&negative[0]).div(&SignedWordTape::from_small(2)),plus[1].sub(&negative[1]).div(&SignedWordTape::from_small(2))];
    if returned!=[one,SignedWordTape::zero()]{return Err("inner horn direction return failed".into());}
    Ok((kernel,json!({"fractional_bits":format.w_bits,"kernel":kernel,"basis":records,
        "rays":frame.rays().iter().map(|r|r.iter().map(|z|[z.re.to_string(),z.im.to_string()]).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "source_generator":"ig-docs/torus_process.py:square_wave_angles",
        "address_rule":{"ordinal":"120*x","azimuth_turns":"x/N","tube_turns":[11,40],"square_return":"120*(x*x mod N)"},
        "inner_return":{"direction":["1","0"],"normal":["0","1"],"plus":["1","1"],"minus":["-1","1"],
            "returned":returned.iter().map(|c|json!({"word":c.magnitude().as_word(),"negative":c.is_negative()})).collect::<Vec<_>>()}})))
}

pub struct SourceBraidContraction {
    source:WordTape,circuit:Circuit,arena:Arena,root:Id,plan:Option<Plan>,factor:usize,elimination_at:usize,
    pub events:Vec<Value>,banks:Vec<Value>,preparation:Value,prepared_dir:PathBuf,
    pub result:Option<Value>,pub error:Option<String>,pub exhausted:bool,
    negative_chart:bool,requested_chart:bool,pub reentry_mask:u32,pub reentries:Vec<Value>,
    chart_factor:Option<usize>,
}
impl SourceBraidContraction {
    pub fn prepare(source:WordTape,directory:&Path)->Result<Self,String>{
        let start=Instant::now();let(kernel,sic)=exact_sic_kernel(source.bit_len())?;
        let circuit=Circuit::prepare(&source,&kernel)?;
        std::fs::create_dir_all(directory).map_err(|e|e.to_string())?;
        circuit.write(&directory.join("circuit.bin"))?;
        std::fs::write(directory.join("source.imasm"),source.as_word()).map_err(|e|e.to_string())?;
        let preparation=json!({"seconds":start.elapsed().as_secs_f64(),"source_word":source.as_word(),"source_bits":source.bit_len(),
            "variables":circuit.variables,"nand_gates":circuit.gates.len(),"factors":circuit.gates.len()+1,
            "output":circuit.output.0,"circuit":directory.join("circuit.bin"),"sic":sic,
            "coefficient_bit_bound":2*(circuit.gates.len()+1)+circuit.variables+2,
            "numeral_arithmetic":"WordTape/SignedWordTape","residue_pairs_enumerated":0});
        let mut arena=Arena::new();let root=arena.leaf(SignedWordTape::from_small(1));
        Ok(Self{source,circuit,arena,root,plan:None,factor:0,elimination_at:0,events:Vec::new(),banks:Vec::new(),
            preparation,prepared_dir:directory.into(),result:None,error:None,exhausted:false,
            negative_chart:false,requested_chart:false,reentry_mask:0,reentries:Vec::new(),chart_factor:None})
    }
    pub fn reenter(&mut self,a:u32,b:u32,c:u32,k:u32)->Result<(),String>{
        let permutation=((a&1)<<1)|((a&2)>>1)|((a&4)<<1)|((a&8)>>1);
        let next=if k==2{(!( ((a&1)<<3)|((a&2)<<1)|((a&4)>>1)|((a&8)>>3) ))&15}
            else{(0..4).find(|i|a&(1<<i)==0).map_or(a,|i|a|(1<<i))};
        if a!=self.reentry_mask||a>15||b!=permutation||c!=next||!matches!(k,1|2)||(k==2)!=(a==15){return Err("source contraction reentry transport".into());}
        self.reentry_mask=c;self.requested_chart=c&4!=0;
        self.reentries.push(json!({"before":a,"permuted":b,"after":c,"kind":k,"at_operation":self.events.len(),
            "requested_negative_chart":self.requested_chart,"root_at_request":self.root}));Ok(())
    }
    fn next_plan(&mut self)->Option<Plan>{
        // At most one chart action per source factor: continued logical
        // feedback cannot starve the fixed arithmetic contraction schedule.
        if self.chart_factor!=Some(self.factor){
            self.chart_factor=Some(self.factor);
            if self.requested_chart!=self.negative_chart{
                let mut p=Plan::new("chart-involution",self.factor,self.root);p.push(Instruction::Neg(0));return Some(p);
            }
        }
        if self.factor>self.circuit.gates.len(){return None;}
        let mut p=Plan::new("source-factor",self.factor,self.root);
        let out=if let Some(g)=self.circuit.gates.get(self.factor){
            let xy=p.monomial(support(&[g.a,g.b]));let z=p.monomial(Some(vec![g.out]));
            let xyz=p.monomial(support(&[g.a,g.b,Wire(g.out+2)]));
            let sum=p.push(Instruction::Add(xy,z));let twice=p.push(Instruction::Double(xyz));
            let negative=p.push(Instruction::Neg(twice));p.push(Instruction::Add(sum,negative))
        }else{
            p.monomial(support(&[self.circuit.output]))
        };
        // Ensure the final slot is the factor output even for a constant term.
        let mut slot=p.push(Instruction::Add(out,1));
        let mut at=self.elimination_at;
        while let Some(&(last,wire))=self.circuit.elimination.get(at){
            if last!=self.factor{break;}slot=p.push(Instruction::Eliminate(slot,wire));at+=1;
        }
        Some(p)
    }
    fn finish_readout(&mut self)->Result<(),String>{
        let count=match self.arena.nodes[self.root as usize].clone(){Node::Zero=>SignedWordTape::zero(),Node::Leaf(c)=>c,
            _=>return Err("closed contraction still carries unresolved variable indices".into())};
        let count=if self.negative_chart{count.negate()}else{count};
        if count.is_negative(){return Err("source collision count returned negative".into());}
        let n=SignedWordTape::from_magnitude(self.source.clone(),false);
        let numerator=n.shl(2).sub(&count).sub(&SignedWordTape::from_small(3));
        if numerator.is_negative(){return Err("source prime-arm trace is negative".into());}
        let(tau,remainder)=numerator.magnitude().divmod(&WordTape::from_small(4)).ok_or("trace division")?;
        if !remainder.is_zero(){return Err("source prime-arm trace denominator did not close".into());}
        let u=tau.add(&WordTape::one());let square=u.mul(&u);
        let delta=square.sub(&self.source).ok_or("negative horn gap")?;let v=delta.isqrt();
        if v.mul(&v)!=delta{return Err("source horn gap is not an exact square".into());}
        let p=u.sub(&v).ok_or("negative source factor")?;let q=u.add(&v);
        let(quotient,rem)=self.source.divmod(&p).ok_or("factor division")?;
        if !p.gt(&WordTape::one())||!self.source.gt(&p)||!rem.is_zero()||q!=quotient||p.mul(&q)!=self.source{return Err("source factor tapes do not close the exact product".into());}
        let decimal=|w:&WordTape|vox_core::morphism_factor::dec_of(&w.to_imasm_tape());
        self.result=Some(json!({"p":decimal(&p),"q":decimal(&q),"p_word":p.as_word(),"q_word":q.as_word(),
            "remainder":"0","product":decimal(&self.source),"exact_product":true,"paired_boundary":true,
            "producer":"source-braid-contraction","collision_count_word":count.magnitude().as_word(),
            "tau_word":tau.as_word(),"midpoint_word":u.as_word(),"gap_word":v.as_word()}));Ok(())
    }
    pub fn advance(&mut self,port:usize,deadline:Option<Instant>){
        if self.exhausted||self.error.is_some()||self.result.is_some(){return;}
        let started=Instant::now();let before=(self.factor,self.root,self.arena.steps,self.arena.nodes.len());
        for _ in 0..1_000_000{
            if deadline.is_some_and(|d|Instant::now()>=d){self.exhausted=true;break;}
            if started.elapsed().as_millis()>=750{break;}
            if self.plan.is_none(){
                self.plan=self.next_plan();
                if self.plan.is_none(){if let Err(e)=self.finish_readout(){self.error=Some(e);}break;}
            }
            let plan=self.plan.as_mut().unwrap();
            if plan.cursor==plan.instructions.len(){
                let p=self.plan.take().unwrap();self.root=p.output();
                let mut bank=p.snapshot();bank["after"]=json!(self.root);self.banks.push(bank);
                if p.kind=="chart-involution"{self.negative_chart=!self.negative_chart;}
                else{while self.circuit.elimination.get(self.elimination_at).is_some_and(|(last,_)|*last==self.factor){self.elimination_at+=1;}self.factor+=1;}
                continue;
            }
            let op=plan.op();
            if let Some(&id)=self.arena.cache.get(&op){plan.executed.push(json!([op.json(),id]));plan.slots.push(id);plan.cursor+=1;}
            else{self.arena.start(op);self.arena.step();}
        }
        self.events.push(json!({"port":port,"seconds":started.elapsed().as_secs_f64(),"before_factor":before.0,"factor":self.factor,
            "before_root":before.1,"root":self.root,"primitive_steps":self.arena.steps-before.2,
            "before_nodes":before.3,"nodes":self.arena.nodes.len(),"eliminated":self.elimination_at,
            "negative_chart":self.negative_chart,"requested_negative_chart":self.requested_chart,"reentry_state":self.reentry_mask,
            "produced":self.result.is_some()}));
    }
    pub fn context_values(&self)->Vec<(&'static str,WordTape)>{vec![
        ("source",self.source.clone()),("factor",WordTape::from_small(self.factor as u64)),
        ("root",WordTape::from_small(self.root as u64)),("nodes",WordTape::from_small(self.arena.nodes.len() as u64)),
        ("eliminated",WordTape::from_small(self.elimination_at as u64)),("reentry_mask",WordTape::from_small(self.reentry_mask as u64)),
        ("negative_chart",WordTape::from_small(u64::from(self.negative_chart)))]}
    pub fn snapshot(&self)->Result<Value,String>{
        let state=json!({"source_word":self.source.as_word(),"factor":self.factor,"root":self.root,
            "elimination_cursor":self.elimination_at,"negative_chart":self.negative_chart,"requested_negative_chart":self.requested_chart,"chart_factor":self.chart_factor,
            "arena":self.arena.snapshot(),"banks":self.banks,"pending_plan":self.plan.as_ref().map(Plan::snapshot)});
        let path=self.prepared_dir.join("retained.json");let bytes=serde_json::to_vec(&state).map_err(|e|e.to_string())?;
        std::fs::write(&path,&bytes).map_err(|e|e.to_string())?;
        Ok(json!({"schema":"source-braid-contraction-v1","construction":"source-braid-contraction","input_boundary":"N alone",
            "source":vox_core::morphism_factor::dec_of(&self.source.to_imasm_tape()),"source_word":self.source.as_word(),
            "preparation_seconds":self.preparation["seconds"],"preparation":self.preparation,"operations":self.events,
            "logical_reentry":self.reentries,"retained_state":{"path":path,"bytes":bytes.len()},
            "factor_cursor":self.factor,"eliminated_variables":self.elimination_at,"coefficient_nodes":self.arena.nodes.len(),
            "result":self.result,"error":self.error,"budget_exhausted":self.exhausted,
            "boundary":if self.result.is_some(){"closed"}else{"held"}}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_200_circuit_and_retained_coefficient_operations(){
        let path=std::env::var("SOURCE_BRAID_TEST_SOURCE").expect("N-only 200-bit source file");
        let raw=std::fs::read_to_string(path).unwrap();let cells=vox_core::morphism_factor::decimal_to_tape(raw.trim()).unwrap();
        let n=WordTape::from_imasm_tape(&cells).unwrap();assert!(n.bit_len()>=200);
        let(kernel,_)=exact_sic_kernel(n.bit_len()).unwrap();
        let circuit=Circuit::prepare(&n,&kernel).unwrap();
        for i in 1..=4 {
            let x=n.shr(i).add(&WordTape::from_small(13*i as u64));let y=n.shr(i+1).add(&WordTape::from_small(7*i as u64));
            let expected=x.compare(&n).is_lt()&&y.compare(&n).is_lt()&&x.mul(&x).divmod(&n).unwrap().1==y.mul(&y).divmod(&n).unwrap().1;
            assert_eq!(circuit.evaluate(&x,&y),expected);
            assert!(circuit.evaluate(&x,&x));
        }
        let mut a=Arena::new();let one=a.leaf(SignedWordTape::from_small(1));
        for op in [Op::Force(0,one),Op::Force(1,one)]{a.start(op);while a.step(){}}
        let x=a.cache[&Op::Force(0,one)];let y=a.cache[&Op::Force(1,one)];
        let op=Op::Force(0,y);a.start(op);while a.step(){}let xy=a.cache[&op];
        let op=Op::Eliminate(0,xy);a.start(op);while a.step(){}assert_eq!(a.cache[&op],y);
        let op=Op::Eliminate(1,x);a.start(op);while a.step(){}let twice_x=a.cache[&op];
        let op=Op::Eliminate(0,twice_x);a.start(op);while a.step(){}
        assert_eq!(a.nodes[a.cache[&op] as usize],Node::Leaf(SignedWordTape::from_small(2)));
    }
}
