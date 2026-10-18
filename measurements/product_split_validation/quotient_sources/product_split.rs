//! Source-bound product splitting: low carry and high bounds act on one branch.
use super::{SignedWord, WordTape};
use alloc::{format, string::String, vec::Vec};
use serde_json::{json, Value};
use std::{path::PathBuf, time::{Duration, Instant}};

#[derive(Clone)]
struct Branch {
    p: Vec<i8>, q: Vec<i8>, low: usize,
    lp: WordTape, lq: WordTape, carry: SignedWord, low_turn: bool,
}
fn tape(bits: &[i8], upper: bool) -> WordTape {
    let cells: Vec<char> = bits.iter().map(|&b| if b == 1 || (upper && b < 0) { '⊥' } else { '⊤' }).collect();
    WordTape::from_parity_cells(&cells).unwrap()
}
impl Branch {
    fn initial(n: &WordTape, p: usize, q: usize) -> Self {
        let mut a=vec![-1;p]; let mut b=vec![-1;q];
        a[0]=1; a[p-1]=1; b[0]=1; b[q-1]=1;
        Self {p:a,q:b,low:1,lp:WordTape::one(),lq:WordTape::one(),
            carry:SignedWord::negative(n.sub(&WordTape::one()).unwrap().shr1()),low_turn:true}
    }
    fn valid(&self,n:&WordTape) -> bool {
        let width=n.bit_len();
        if self.p.len()<2 || self.q.len()<self.p.len() || ![width,width+1].contains(&(self.p.len()+self.q.len())) ||
            self.low<1 || self.low>self.q.len() || self.p[0]!=1 || self.q[0]!=1 ||
            self.p.last()!=Some(&1) || self.q.last()!=Some(&1) ||
            self.p.iter().chain(&self.q).any(|&b| !(-1..=1).contains(&b)) {return false;}
        let pl=self.low.min(self.p.len()); let ql=self.low.min(self.q.len());
        if self.p[..pl].contains(&-1) || self.q[..ql].contains(&-1) || tape(&self.p[..pl],false)!=self.lp || tape(&self.q[..ql],false)!=self.lq {return false;}
        let product=self.lp.mul(&self.lq); let residual=self.carry.magnitude.shl(self.low);
        if self.carry.negative {product.add(&residual)==*n} else {n.add(&residual)==product}
    }
    fn encode(&self)->Value {json!({"p":self.p,"q":self.q,"low":self.low,"lp":self.lp.as_word(),"lq":self.lq.as_word(),"carry_negative":self.carry.negative,"carry_word":self.carry.magnitude.as_word(),"low_turn":self.low_turn})}
    fn decode(v:&Value)->Option<Self> {
        Some(Self {p:serde_json::from_value(v["p"].clone()).ok()?,q:serde_json::from_value(v["q"].clone()).ok()?,
            low:usize::try_from(v["low"].as_u64()?).ok()?,lp:WordTape::from_canonical_word(v["lp"].as_str()?)?,lq:WordTape::from_canonical_word(v["lq"].as_str()?)?,
            carry:SignedWord{negative:v["carry_negative"].as_bool()?,magnitude:WordTape::from_canonical_word(v["carry_word"].as_str()?)?},low_turn:v["low_turn"].as_bool()?})
    }
    fn bounds_admit(&self,n:&WordTape)->bool {
        let pmin=tape(&self.p,false);let pmax=tape(&self.p,true);
        let qmin=tape(&self.q,false);let qmax=tape(&self.q,true);
        qmax.ge(&pmin) && n.ge(&pmin.mul(&qmin)) && pmax.mul(&qmax).ge(n)
    }
    fn tighten(&mut self,n:&WordTape)->Option<u64> {
        let ceil=|d:&WordTape| {let(q,r)=n.divmod(d).unwrap();if r.is_zero(){q}else{q.add(&WordTape::one())}};
        let mut total=0;
        loop {
            let pl=tape(&self.p,false);let ph=tape(&self.p,true);
            let ql=tape(&self.q,false);let qh=tape(&self.q,true);
            let plo=ceil(&qh);let phi=n.divmod(&ql).unwrap().0;
            let qlo=ceil(&ph);let qhi=n.divmod(&pl).unwrap().0;
            let plo=if plo.ge(&pl){plo}else{pl};let phi=if ph.ge(&phi){phi}else{ph};
            let qlo=if qlo.ge(&ql){qlo}else{ql};let qhi=if qh.ge(&qhi){qhi}else{qh};
            if !phi.ge(&plo)||!qhi.ge(&qlo){return None;}
            let mut changed=0;
            for (bits,lo,hi) in [(&mut self.p,&plo,&phi),(&mut self.q,&qlo,&qhi)] {
                for i in (0..bits.len()).rev() {
                    let bit=lo.bit_at(i);if bit!=hi.bit_at(i){break;}
                    let bit=if bit{1}else{0};
                    if bits[i]>=0 && bits[i]!=bit{return None;}
                    if bits[i]<0 {bits[i]=bit;changed+=1;}
                }
            }
            total+=changed;if changed==0{return Some(total);}
        }
    }
    fn low_children(&self)->Vec<Self> {
        let k=self.low;let step=WordTape::one_at(k);let parity=self.carry.magnitude.is_odd() as i8;
        let pa=self.p.get(k).copied().unwrap_or(0);let qa=self.q.get(k).copied().unwrap_or(0);
        let mut children=Vec::new();
        for pb in [0i8,1] {
            let qb=parity ^ pb;
            if (pa>=0 && pa!=pb)||(qa>=0 && qa!=qb) {continue;}
            let mut c=self.clone();
            if k<c.p.len(){c.p[k]=pb;} if k<c.q.len(){c.q[k]=qb;}
            let mut contribution=WordTape::zero();
            if pb==1 {contribution=contribution.add(&self.lq);c.lp=self.lp.add(&step);}
            if qb==1 {contribution=contribution.add(&self.lp);c.lq=self.lq.add(&step);}
            if pb==1 && qb==1 {contribution=contribution.add(&step);}
            let numerator=self.carry.add_magnitude(&contribution);
            if numerator.magnitude.is_odd(){continue;}
            c.carry=numerator.halve_even();c.low+=1;c.low_turn=false;children.push(c);
        }
        children
    }
    fn high_children(&self)->Vec<Self> {
        let pos=self.p.iter().rposition(|&v|v<0).into_iter().chain(self.q.iter().rposition(|&v|v<0)).max();
        let Some(k)=pos else{return self.low_children();};
        let pa=self.p.get(k).copied().unwrap_or(0);let qa=self.q.get(k).copied().unwrap_or(0);
        let mut children=Vec::new();
        for pb in [0i8,1] {for qb in [0i8,1] {
            if (pa>=0 && pa!=pb)||(qa>=0 && qa!=qb){continue;}
            let mut c=self.clone();if k<c.p.len(){c.p[k]=pb;}if k<c.q.len(){c.q[k]=qb;}
            c.low_turn=true;children.push(c);
        }}
        children
    }
}

pub(super) fn run(n:&WordTape,seconds:u64,node_cap:u64)->String {
    if n.is_zero() || n.is_one(){return String::from("UNIT: no proper factor split");}
    if !n.is_odd(){
        let p=WordTape::from_small(2);let Some((q,r))=n.divmod(&p) else{return String::from("division failed");};
        return if !q.is_one() && r.is_zero() && p.mul(&q)==*n {closed(n,&p,&q,0,0,0)} else {String::from("no proper two-lane split")};
    }
    let mut hash=14695981039346656037u64;
    for b in n.as_word().bytes(){hash=(hash ^ u64::from(b)).wrapping_mul(1099511628211);}
    let path=PathBuf::from(format!("measurements/product_split/{hash:016x}.json"));
    let mut frontier=Vec::new();let(mut nodes,mut lows,mut highs,mut pruned,mut forced)=(0u64,0u64,0u64,0u64,0u64);
    if path.exists(){
        let loaded=std::fs::read_to_string(&path).ok().and_then(|s|serde_json::from_str::<Value>(&s).ok());
        let Some(v)=loaded else{return format!("invalid continuation: {}",path.display());};
        if v["format"]!="joint-product-split-v1" || v["source_word"].as_str()!=Some(n.as_word()){return String::from("continuation source mismatch");}
        if let Some(pair)=v["closed_pair"].as_array(){
            if pair.len()==2 {if let (Some(p),Some(q))=(pair[0].as_str().and_then(WordTape::from_canonical_word),pair[1].as_str().and_then(WordTape::from_canonical_word)){
                if !p.is_zero() && !p.is_one() && !q.is_zero() && !q.is_one() && p.mul(&q)==*n{return closed(n,&p,&q,v["nodes"].as_u64().unwrap_or(0),v["low_steps"].as_u64().unwrap_or(0),v["high_steps"].as_u64().unwrap_or(0));}
            }}return String::from("invalid closed continuation");
        }
        let Some(items)=v["frontier"].as_array() else{return String::from("missing frontier");};
        for item in items {let Some(b)=Branch::decode(item) else{return String::from("invalid retained branch");};if !b.valid(n){return String::from("retained carry/source invariant failed");}frontier.push(b);}
        nodes=v["nodes"].as_u64().unwrap_or(0);lows=v["low_steps"].as_u64().unwrap_or(0);highs=v["high_steps"].as_u64().unwrap_or(0);pruned=v["interval_rejections"].as_u64().unwrap_or(0);
        forced=v["quotient_forced_bits"].as_u64().unwrap_or(0);
    } else {
        let width=n.bit_len();
        for p in 2..=(width/2+1) {for q in [width.saturating_sub(p),width+1-p] {if q>=p {frontier.push(Branch::initial(n,p,q));}}}
    }
    let start=Instant::now();let first_nodes=nodes;let mut found=None;
    while !frontier.is_empty() && start.elapsed()<Duration::from_secs(seconds) && nodes-first_nodes<node_cap {
        let mut b=frontier.pop().unwrap();nodes+=1;
        if !b.bounds_admit(n){pruned+=1;continue;}
        match b.tighten(n){Some(count)=>forced+=count,None=>{pruned+=1;continue;}}
        if b.low==b.q.len(){
            if b.carry.is_zero() && b.lp.mul(&b.lq)==*n && !b.lp.is_one() && !b.lq.is_one(){found=Some((b.lp,b.lq));break;}continue;
        }
        let children=if b.low_turn || !b.p.iter().chain(&b.q).any(|&v|v<0) {lows+=1;b.low_children()}else{highs+=1;b.high_children()};
        frontier.extend(children.into_iter().rev());
    }
    let mut state=json!({"format":"joint-product-split-v1","source_word":n.as_word(),"nodes":nodes,"low_steps":lows,"high_steps":highs,"interval_rejections":pruned,"quotient_forced_bits":forced,"elapsed_this_run_seconds":start.elapsed().as_secs_f64(),"frontier":frontier.iter().map(Branch::encode).collect::<Vec<_>>()});
    let answer=if let Some((p,q))=found {state["closed_pair"]=json!([p.as_word(),q.as_word()]);closed(n,&p,&q,nodes,lows,highs)}else{format!("{}\nsource={}\nnodes={} low_steps={} high_steps={} interval_rejections={} quotient_forced_bits={} retained_branches={}\ncontinuation={}",if frontier.is_empty(){"EXHAUSTED: no proper pair in the enumerated widths"}else{"OPEN: product split retained"},n.as_word(),nodes,lows,highs,pruned,forced,frontier.len(),path.display())};
    if let Some(parent)=path.parent(){if let Err(e)=std::fs::create_dir_all(parent){return format!("{answer}\ncontinuation save failed: {e}");}}
    let pending=path.with_extension("pending");
    if let Err(e)=std::fs::write(&pending,serde_json::to_vec_pretty(&state).unwrap()).and_then(|_|std::fs::rename(&pending,&path)){return format!("{answer}\ncontinuation save failed: {e}");}
    answer
}
fn closed(n:&WordTape,p:&WordTape,q:&WordTape,nodes:u64,lows:u64,highs:u64)->String {
    let (quotient,remainder)=n.divmod(p).unwrap();let packed=p.interlace(q);let(back_p,back_q)=packed.deinterlace();
    format!("CLOSED: joint product split\np={}\nq={}\nproper_divisor={}\nzero_remainder={}\nquotient_matches={}\nexact_word_product={}\npaired_boundary={}\nnodes={} low_steps={} high_steps={}",p.as_word(),q.as_word(),!p.is_one()&&!q.is_one(),remainder.is_zero(),quotient==*q,p.mul(q)==*n,back_p==*p&&back_q==*q,nodes,lows,highs)
}

#[cfg(test)]
mod tests {
    use super::*;
    const ROOT:&str=concat!(env!("CARGO_MANIFEST_DIR"),"/measurements/product_split_validation/");
    #[test]
    fn joint_carry_and_high_bounds_preserve_fresh_208_bit_witness() {
        let load=|name:&str|WordTape::from_canonical_word(&std::fs::read_to_string(format!("{ROOT}{name}")).unwrap().trim()).unwrap();
        let n=load("inputs/control.imasm");let mut p=load("reference/p.imasm");let mut q=load("reference/q.imasm");
        if p.ge(&q){std::mem::swap(&mut p,&mut q);}
        assert!(n.bit_len()>=200);assert_eq!(p.bit_len(),104);assert_eq!(q.bit_len(),104);assert_eq!(p.mul(&q),n);
        let compatible=|b:&Branch|b.p.iter().enumerate().all(|(i,&v)|v<0 || (v==1)==p.bit_at(i)) && b.q.iter().enumerate().all(|(i,&v)|v<0 || (v==1)==q.bit_at(i));
        let mut b=Branch::initial(&n,p.bit_len(),q.bit_len());let(mut lows,mut highs)=(0,0);
        while b.low<b.q.len(){
            assert!(b.valid(&n));assert!(b.bounds_admit(&n));
            assert!(b.tighten(&n).is_some());assert!(compatible(&b));assert!(b.valid(&n));
            let children=if b.low_turn || !b.p.iter().chain(&b.q).any(|&v|v<0){lows+=1;b.low_children()}else{highs+=1;b.high_children()};
            b=children.into_iter().find(|c|compatible(c)).expect("joint transformations must retain the reference witness");
            let encoded=b.encode();let restored=Branch::decode(&encoded).unwrap();assert!(restored.valid(&n));b=restored;
        }
        assert!(lows>0 && highs>0);assert!(b.carry.is_zero());assert_eq!(b.lp.mul(&b.lq),n);assert_eq!(b.lp,p);assert_eq!(b.lq,q);
        let mut bad=b.clone();bad.carry.magnitude=WordTape::one();assert!(!bad.valid(&n));
        let mut bad=b;bad.p.fill(1);bad.q.fill(1);assert!(!bad.bounds_admit(&n));
    }
}
