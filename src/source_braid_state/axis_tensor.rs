//! Exact-product slice compiled directly into native Belnap arithmetic tensors.
//! Only the definite numeral slice is used by this existential observer.
use crate::source_braid_contraction::circuit::Wire;
use crate::word_tape::WordTape;
use crate::belnap::B4;
use vox_core::belnap_relation::{AND,ADD,BelnapSet};
use serde::Serialize;
use std::collections::VecDeque;

#[derive(Clone,Serialize)]
pub struct Constraint {pub scope:Vec<usize>,pub rows:Vec<usize>}
#[derive(Serialize)]
pub struct AxisCircuit {
    pub bits:usize,pub variables:usize,pub constraints:Vec<Constraint>,
    pub left:Vec<usize>,pub right:Vec<usize>,
}
#[derive(Serialize)]
pub struct SourceRestriction {
    pub domains:Vec<u8>,pub constraints:Vec<Constraint>,
    pub updates:Vec<(usize,usize,u8)>,pub revisions:usize,
}
struct Builder {next:usize,constraints:Vec<Constraint>}
impl Builder {
    fn fresh(&mut self)->Wire{let w=Wire(self.next+2);self.next+=1;w}
    fn relation(&mut self,ports:&[Wire],rows:impl IntoIterator<Item=Vec<bool>>){
        let mut scope:Vec<_>=ports.iter().filter_map(|w|w.variable()).collect();scope.sort_unstable();scope.dedup();
        let mut accepted=Vec::new();
        for row in rows{
            let mut seen=vec![None;scope.len()];let mut valid=true;let mut mask=0;
            for (&w,value) in ports.iter().zip(row){
                if let Some(v)=w.variable(){let i=scope.binary_search(&v).unwrap();
                    if seen[i].is_some_and(|old|old!=value){valid=false;break;}
                    seen[i]=Some(value);if value{mask|=1<<i;}
                }else if (w.0==1)!=value{valid=false;break;}
            }
            if valid{accepted.push(mask);}
        }
        accepted.sort_unstable();accepted.dedup();
        if !scope.is_empty()||accepted.is_empty(){self.constraints.push(Constraint{scope,rows:accepted});}
    }
    fn and(&mut self,a:Wire,b:Wire)->Wire{
        if a.0==0||b.0==0{return Wire(0);}if a.0==1||a==b{return b;}if b.0==1{return a;}
        let out=self.fresh();let table=AND.numeral_supports();
        self.relation(&[a,b,out],(0..4).map(|i|(0..3).map(|p|table[p][1]&(1<<i)!=0).collect::<Vec<_>>()));out
    }
    fn or(&mut self,a:Wire,b:Wire)->Wire{
        if a.0==1||b.0==1{return Wire(1);}if a.0==0||a==b{return b;}if b.0==0{return a;}
        let out=self.fresh();
        self.relation(&[a,b,out],(0..4).map(|i|{
            let x=i&2!=0;let y=i&1!=0;
            let left=if x{B4::T}else{B4::F};let right=if y{B4::T}else{B4::F};
            vec![x,y,left.truth_or(right)==B4::T]
        }));out
    }
    fn full_add(&mut self,a:Wire,b:Wire,c:Wire)->(Wire,Wire){
        if a==b{return(c,a);}
        if a.0==0&&b.0==0{return(c,Wire(0));}
        if c.0==0&&a.0==0{return(b,Wire(0));}
        if c.0==0&&b.0==0{return(a,Wire(0));}
        if a.0<2&&b.0<2&&c.0<2{let s=a.0+b.0+c.0;return(Wire(s&1),Wire(s>>1));}
        let sum=self.fresh();let carry=self.fresh();let table=ADD.numeral_supports();
        self.relation(&[a,b,c,sum,carry],(0..8).map(|i|(0..5).map(|p|table[p][1]&(1<<i)!=0).collect::<Vec<_>>()));
        (sum,carry)
    }
}
impl AxisCircuit {
    /// Saturate the source-pinned local relations before joining any of them.
    /// A domain is a powerset of Belnap values, not a single B4 glut. Every
    /// accepted tensor row remains correlated in the resulting constraints.
    pub fn restrict_source(&self)->SourceRestriction{
        let mut domains=vec![BelnapSet::NUMERAL;self.variables];
        let mut rows:Vec<_>=self.constraints.iter().map(|c|c.rows.clone()).collect();
        let mut incidence=vec![Vec::new();self.variables];
        for(i,c)in self.constraints.iter().enumerate(){for &v in &c.scope{incidence[v].push(i);}}
        let mut queue:VecDeque<_>=(0..self.constraints.len()).collect();
        let mut queued=vec![true;self.constraints.len()];let mut updates=Vec::new();let mut revisions=0;
        while let Some(i)=queue.pop_front(){
            queued[i]=false;revisions+=1;let scope=&self.constraints[i].scope;
            rows[i].retain(|row|scope.iter().enumerate().all(|(at,&v)|
                domains[v].contains(if row&(1<<at)!=0{B4::T}else{B4::F})));
            for(at,&v)in scope.iter().enumerate(){
                let mut supported=BelnapSet::EMPTY;
                for row in &rows[i]{supported=supported.union(BelnapSet::singleton(
                    if row&(1<<at)!=0{B4::T}else{B4::F}));}
                let restricted=domains[v].intersection(supported);
                if restricted!=domains[v]{
                    domains[v]=restricted;updates.push((i,v,restricted.mask()));
                    for &other in &incidence[v]{if !queued[other]{queue.push_back(other);queued[other]=true;}}
                }
            }
        }
        let mut constraints=Vec::new();
        for(c,rows)in self.constraints.iter().zip(rows){
            let positions:Vec<_>=c.scope.iter().enumerate().filter_map(|(i,&v)|
                (domains[v].only().is_none()).then_some(i)).collect();
            let scope=positions.iter().map(|&i|c.scope[i]).collect::<Vec<_>>();
            let mut rows:Vec<_>=rows.into_iter().map(|row|positions.iter().enumerate()
                .fold(0,|out,(at,&old)|out|(((row>>old)&1)<<at))).collect();
            rows.sort_unstable();rows.dedup();
            if rows.len()!=1usize<<scope.len(){constraints.push(Constraint{scope,rows});}
        }
        SourceRestriction{domains:domains.into_iter().map(|d|d.mask()).collect(),constraints,updates,revisions}
    }
    /// Internal wires are deterministic functions of the two retained input
    /// coordinates. Recover them in their original construction order instead
    /// of banking duplicate cofactors for their existential elimination.
    pub fn recover_internal(&self,values:&mut [bool])->Result<(),String>{
        let mut known=vec![false;self.variables];
        for &w in self.left.iter().chain(&self.right){
            if let Some(v)=Wire(w).variable(){known[v]=true;}
        }
        for constraint in &self.constraints{
            let rows:Vec<_>=constraint.rows.iter().copied().filter(|row|
                constraint.scope.iter().enumerate().all(|(i,&v)|
                    !known[v]||values[v]==(row&(1<<i)!=0))).collect();
            let Some(&first)=rows.first() else{return Err("recovered inputs violate native tensor".into());};
            for(i,&v)in constraint.scope.iter().enumerate(){
                if known[v]{continue;}
                let value=first&(1<<i)!=0;
                if !rows.iter().all(|row|(row&(1<<i)!=0)==value){
                    return Err("internal tensor is not functional in its retained inputs".into());
                }
                values[v]=value;known[v]=true;
            }
        }
        if known.iter().any(|&v|!v){return Err("unrecovered internal coordinate".into());}
        Ok(())
    }
    pub fn prepare(n:&WordTape)->Result<Self,String>{
        let bits=n.bit_len();if bits<3{return Err("proper-axis reader requires N >= 4".into());}
        let aw=bits.div_ceil(2);let bw=bits-1;let width=aw+bw;
        let mut b=Builder{next:0,constraints:Vec::new()};let(mut left,mut right)=(Vec::new(),Vec::new());
        for i in 0..bw.max(aw){
            if i<aw{left.push(if i==0&&n.bit_at(0){Wire(1)}else{b.fresh()});}
            if i<bw{right.push(if i==0&&n.bit_at(0){Wire(1)}else{b.fresh()});}
        }
        // Keep partial products in their source column. Three-to-two native
        // ADD contractions preserve the exact weighted sum without sending a
        // ripple carry across every partial row. Each layer consumes only
        // the previous layer; it does not serialise newly produced carries.
        let mut columns=vec![Vec::new();width];
        for(i,&x)in left.iter().enumerate(){
            for(j,&y)in right.iter().enumerate(){
                let p=b.and(x,y);if p.0!=0{columns[i+j].push(p);}
            }
        }
        while columns.iter().any(|c|c.len()>2){
            let mut next=vec![Vec::new();width];
            for(k,column)in columns.into_iter().enumerate(){
                let mut triples=column.chunks_exact(3);
                for t in &mut triples{
                    let(s,c)=b.full_add(t[0],t[1],t[2]);
                    if s.0!=0{next[k].push(s);}
                    if c.0!=0{
                        if k+1<width{next[k+1].push(c);}
                        else{b.relation(&[c],[vec![false]]);}
                    }
                }
                next[k].extend_from_slice(triples.remainder());
            }
            columns=next;
        }
        let mut carry=Wire(0);let mut product=Vec::with_capacity(width);
        for column in columns{
            let x=column.first().copied().unwrap_or(Wire(0));
            let y=column.get(1).copied().unwrap_or(Wire(0));
            let(s,c)=b.full_add(x,y,carry);product.push(s);carry=c;
        }
        b.relation(&[carry],[vec![false]]);
        for(i,&w)in product.iter().enumerate(){b.relation(&[w],[vec![n.bit_at(i)]]);}
        for lane in [&left,&right]{
            let any=lane.iter().skip(1).copied().fold(Wire(0),|x,y|b.or(x,y));
            b.relation(&[any],[vec![true]]);
        }
        Ok(Self{bits,variables:b.next,constraints:b.constraints,
            left:left.iter().map(|w|w.0).collect(),right:right.iter().map(|w|w.0).collect()})
    }
}
