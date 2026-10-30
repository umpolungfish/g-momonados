//! Local tensor constraints for E²=E+KN. No residual truth table is produced.
use crate::ququart_decision::DecisionArena;
use num_bigint::{BigInt,BigUint};
use num_traits::One;
use std::collections::{BTreeMap,BTreeSet,BinaryHeap};
use std::cmp::Reverse;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
enum Bit { Constant(bool), Wire(usize) }
impl Bit {
    fn value(self,assignment:usize,scope:&[usize])->bool {
        match self {Self::Constant(v)=>v,Self::Wire(w)=>assignment&(1<<scope.binary_search(&w).unwrap())!=0}
    }
}
struct Factor { root:usize, scope:BTreeSet<usize> }
struct Builder { next:usize, constraints:Vec<(Vec<Bit>,Vec<usize>)>, column:usize, columns:Vec<usize> }
impl Builder {
    fn fresh(&mut self)->Bit {let w=self.next;self.next+=1;self.columns.push(self.column);Bit::Wire(w)}
    fn and(&mut self,a:Bit,b:Bit)->Bit {
        match (a,b) {
            (Bit::Constant(false),_)|(_,Bit::Constant(false))=>Bit::Constant(false),
            (Bit::Constant(true),v)|(v,Bit::Constant(true))=>v,
            _ if a==b=>a,
            _=>{let out=self.fresh();self.constraint(vec![a,b,out],|v|v[2]==(v[0]&&v[1]));out}
        }
    }
    fn constraint(&mut self,bits:Vec<Bit>,accept:impl Fn(&[bool])->bool) {
        let scope:Vec<_>=bits.iter().filter_map(|b|if let Bit::Wire(w)=b{Some(*w)}else{None})
            .collect::<BTreeSet<_>>().into_iter().collect();
        let rows=(0..1usize<<scope.len()).filter(|&row|{
            let values:Vec<_>=bits.iter().map(|&b|b.value(row,&scope)).collect();accept(&values)
        }).collect();
        self.constraints.push((scope.into_iter().map(Bit::Wire).collect(),rows));
    }
    fn adder(&mut self,a:Bit,b:Bit,c:Bit)->(Bit,Bit) {
        let sum=self.fresh();self.column+=1;let carry=self.fresh();self.column-=1;
        self.constraint(vec![a,b,c,sum,carry],|v|{
            let n=usize::from(v[0])+usize::from(v[1])+usize::from(v[2]);
            v[3]==(n&1!=0)&&v[4]==(n>=2)
        });
        (sum,carry)
    }
    fn compress(&mut self,mut columns:Vec<Vec<Bit>>)->Vec<Bit> {
        let width=columns.len();
        for i in 0..width {
            self.column=i;
            while columns[i].len()>1 {
                let a=columns[i].pop().unwrap();let b=columns[i].pop().unwrap();
                let c=columns[i].pop().unwrap_or(Bit::Constant(false));
                let (sum,carry)=self.adder(a,b,c);columns[i].push(sum);
                if i+1<width {columns[i+1].push(carry)}
                else {self.constraint(vec![carry],|v|!v[0]);}
            }
        }
        columns.into_iter().map(|mut c|c.pop().unwrap_or(Bit::Constant(false))).collect()
    }
}

pub struct Projector {
    factors:Vec<Option<Factor>>,
    incidence:BTreeMap<usize,BTreeSet<usize>>,
    internal:BTreeSet<usize>,
    first_internal:usize,
    pending:BinaryHeap<Reverse<(usize,usize,usize)>>,
    versions:BTreeMap<usize,usize>,
    pub constraints:usize,
    pub contractions:usize,
    // (eliminated coordinate, retained coordinate, complement). This is the
    // exact copy tensor needed to lift the returned state back to its arms.
    pub copy_returns:Vec<(usize,usize,bool)>,
}
impl Projector {
    /// Lift the already prepared Gödel relations without rebuilding a product
    /// circuit or enumerating factor candidates. Cell renaming is carried by
    /// the caller; all gate rows and source pins enter the same complex arena.
    pub(crate) fn from_relations(arena:&mut DecisionArena,boundary:usize,
        tables:Vec<(Vec<usize>,Vec<usize>)>)->Result<Self,String>{
        if boundary>arena.cells{return Err("relation boundary exceeds register".into());}
        let mut unit=DecisionArena::new(0);
        let one=unit.basis(&BigUint::from(0u8),BigInt::one());
        let one=arena.import_shifted(&unit,one,0)?;let zero=arena.zero();
        let mut factors=Vec::new();let mut incidence:BTreeMap<usize,BTreeSet<usize>>=BTreeMap::new();
        for (scope,rows) in tables{
            if scope.windows(2).any(|w|w[0]>=w[1])||scope.iter().any(|&w|w>=arena.cells)
                ||scope.len()>=usize::BITS as usize{return Err("invalid prepared gate chart".into());}
            let mut root=zero;
            for row in rows.into_iter().collect::<BTreeSet<_>>(){
                if row>>scope.len()!=0{return Err("gate row exceeds its chart".into());}
                let literals:Vec<_>=scope.iter().enumerate().map(|(i,&w)|(w,row&(1<<i)!=0)).collect();
                let arm=arena.conditional_literals(zero,one,&literals);root=arena.sum(root,arm);
            }
            let index=factors.len();for &w in &scope{incidence.entry(w).or_default().insert(index);}
            factors.push(Some(Factor{root,scope:scope.into_iter().collect()}));
        }
        let internal=incidence.keys().copied().filter(|&w|w>=boundary).collect();
        let constraints=factors.len();
        let mut result=Self{factors,incidence,internal,first_internal:boundary,
            pending:BinaryHeap::new(),versions:BTreeMap::new(),constraints,contractions:0,copy_returns:Vec::new()};
        for w in result.internal.iter().copied().collect::<Vec<_>>(){result.schedule(w);}
        Ok(result)
    }
    pub(crate) fn enclosing_banks(&self)->Vec<(usize,Vec<usize>)>{
        self.factors.iter().flatten().map(|f|(f.root,f.scope.iter().copied().collect())).collect()
    }
    pub(crate) fn enclosing_arm(&self,index:usize)->Result<(usize,Vec<usize>),String>{
        self.factors.get(index).and_then(|f|f.as_ref()).map(|f|
            (f.root,f.scope.iter().copied().collect())).ok_or_else(||"lost catalytic arm".into())
    }
    /// Split the whole incident family on its shared coordinate. These are
    /// the executing tensors, including their Gaussian gauges and spectators.
    pub(crate) fn joint_boundary_arm(&self,arena:&mut DecisionArena,wire:usize)
        ->Result<(usize,Vec<usize>),String>{
        let indices=self.incidence.get(&wire).ok_or("lost shared measurement coordinate")?;
        let mut root=None;let mut scope=BTreeSet::new();
        for &index in indices{
            let factor=self.factors[index].as_ref().ok_or("lost incident measurement arm")?;
            root=Some(match root{None=>factor.root,Some(r)=>arena.amplitude_product(r,factor.root)});
            scope.extend(&factor.scope);
        }
        Ok((root.ok_or("empty incident measurement family")?,scope.into_iter().collect()))
    }
    /// Return SIC-observed support to every incident arm. The zero/one mask
    /// changes no surviving complex coefficient. A neighbor sees the
    /// existential projection onto the focus coordinates that it actually
    /// carries, so no new independent coordinate or producer is introduced.
    pub(crate) fn return_measured_support(&mut self,arena:&mut DecisionArena,
        focus:&[usize],labels:u8)->Result<usize,String>{
        if focus.is_empty()||focus.len()>2||focus.windows(2).any(|w|w[0]>=w[1])
            ||labels==0||labels>> (1usize<<focus.len())!=0{
            return Err("invalid shared SIC support return".into());
        }
        let neighbors:BTreeSet<_>=focus.iter().flat_map(|w|
            self.incidence.get(w).into_iter().flatten().copied()).collect();
        let mut changed=Vec::new();let mut affected=BTreeSet::new();
        let zero=arena.zero();
        for index in neighbors{
            let factor=self.factors[index].as_mut().ok_or("lost support-return arm")?;
            let projected:Vec<_>=focus.iter().enumerate().filter(|(_,w)|factor.scope.contains(w))
                .map(|(at,&wire)|(at,wire)).collect();
            let before=factor.root;
            for assignment in 0..1usize<<projected.len(){
                let supported=(0..1usize<<focus.len()).any(|label|labels&(1<<label)!=0&&
                    projected.iter().enumerate().all(|(bit,(original,_))|
                        ((label>>original)&1)==((assignment>>bit)&1)));
                if !supported{
                    let controls:Vec<_>=projected.iter().enumerate().map(|(at,&(_,wire))|
                        (wire,assignment&(1<<at)!=0)).collect();
                    factor.root=arena.conditional_literals(factor.root,zero,&controls);
                }
            }
            if factor.root!=before{
                affected.extend(factor.scope.iter().filter(|w|self.internal.contains(w)).copied());
                changed.push(index);
            }
        }
        let count=changed.len();
        for index in changed{self.fixed_point_return(arena,index,true);}
        for wire in affected{if self.internal.contains(&wire){self.schedule(wire);}}
        Ok(count)
    }
    pub(crate) fn retain_boundary_literals(&mut self,arena:&mut DecisionArena,literals:&[(usize,bool)])
        ->Result<(),String>{
        if literals.iter().any(|&(w,_)|w>=self.first_internal)
            ||literals.windows(2).any(|w|w[0].0>=w[1].0){return Err("returned frame escaped its factor boundary".into());}
        let mut unit=DecisionArena::new(0);let one=unit.basis(&BigUint::from(0u8),BigInt::one());
        let one=arena.import_shifted(&unit,one,0)?;let zero=arena.zero();
        let root=arena.conditional_literals(zero,one,literals);
        let scope:BTreeSet<_>=literals.iter().map(|v|v.0).collect();let index=self.factors.len();
        for &w in &scope{self.incidence.entry(w).or_default().insert(index);}
        self.factors.push(Some(Factor{root,scope}));
        self.fixed_point_return(arena,index,true);Ok(())
    }
    /// Exact Gaussian gauge exchange on two arms at their shared wire.
    /// The product is invariant pointwise, while both carried arms change.
    /// It precedes the ordinary FSPLIT/FFUSE contraction of this same bucket.
    pub(crate) fn exchange_at_boundary(&mut self,arena:&mut DecisionArena,turn:i32)
        ->Result<Option<(usize,usize,usize)>,String>{
        let wire=loop{
            let Some(Reverse((_,w,v)))=self.pending.peek().copied() else{return Ok(None);};
            if self.internal.contains(&w)&&self.versions.get(&w)==Some(&v){break w;}
            self.pending.pop();
        };
        let arms:Vec<_>=self.incidence.get(&wire).into_iter().flatten().copied().take(2).collect();
        if arms.len()!=2{return Ok(None);}
        let phase=|q:i32|{let (re,im)=match q.rem_euclid(4){0=>(1,0),1=>(0,1),2=>(-1,0),_=>(0,-1)};
            (crate::word_tape::SignedWordTape::from_bigint(&BigInt::from(re)),
             crate::word_tape::SignedWordTape::from_bigint(&BigInt::from(im)))};
        for (&index,q) in arms.iter().zip([turn,-turn]){
            let f=self.factors[index].as_mut().unwrap();let zero=arena.zero();
            let selected=arena.conditional_literals(zero,f.root,&[(wire,true)]);
            let rotated=arena.scale_words_numerator(selected,&phase(q));
            f.root=arena.conditional_literals(f.root,rotated,&[(wire,true)]);
        }
        Ok(Some((wire,arms[0],arms[1])))
    }
    pub(crate) fn from_baked(p:&crate::baked_point::BakedPointProjector,roots:&[usize])->Result<Self,String>{
        if roots.len()!=p.scopes.len(){return Err("compiled projector lost a tensor bank".into());}
        Ok(Self{
            factors:roots.iter().zip(p.scopes).map(|(&root,scope)|Some(Factor{root,scope:scope.iter().copied().collect()})).collect(),
            incidence:p.incidence.iter().map(|&(wire,indices)|(wire,indices.iter().copied().collect())).collect(),
            internal:p.internal.iter().copied().collect(),first_internal:p.first_internal,
            pending:BinaryHeap::from(p.pending.iter().copied().map(Reverse).collect::<Vec<_>>()),
            versions:p.versions.iter().copied().collect(),constraints:p.constraints,contractions:p.contractions,
            copy_returns:p.copy_returns.to_vec(),
        })
    }
    pub(crate) fn preparation_record(&self)->serde_json::Value{
        let compact:BTreeMap<_,_>=self.factors.iter().enumerate().filter(|(_,f)|f.is_some())
            .enumerate().map(|(new,(old,_))|(old,new)).collect();
        let mut pending:Vec<_>=self.pending.as_slice().iter().map(|v|v.0).collect();pending.sort_unstable();
        serde_json::json!({"scopes":self.factors.iter().flatten().map(|f|&f.scope).collect::<Vec<_>>(),
            "incidence":self.incidence.iter().filter(|(_,indices)|!indices.is_empty()).map(|(wire,indices)|(*wire,indices.iter()
                .map(|index|compact[index]).collect::<BTreeSet<_>>())).collect::<Vec<_>>(),
            "internal":self.internal,"first_internal":self.first_internal,
            "pending":pending,
            "versions":self.versions.iter().map(|(&wire,&version)|(wire,version)).collect::<Vec<_>>(),
            "constraints":self.constraints,"contractions":self.contractions,"copy_returns":self.copy_returns})
    }
    pub(crate) fn from_preparation_record(p:&serde_json::Value,roots:&[usize])->Result<Self,String>{
        let decode=|key:&str|serde_json::from_value(p[key].clone())
            .map_err(|e|format!("invalid projector {key}: {e}"));
        let scopes:Vec<BTreeSet<usize>>=decode("scopes")?;
        if scopes.len()!=roots.len(){return Err("held projector lost a tensor bank".into());}
        let incidence:Vec<(usize,BTreeSet<usize>)>=serde_json::from_value(p["incidence"].clone()).map_err(|e|e.to_string())?;
        let factors:Vec<_>=roots.iter().zip(scopes).map(|(&root,scope)|Some(Factor{root,scope})).collect();
        let mut expected:BTreeMap<usize,BTreeSet<usize>>=BTreeMap::new();
        for (index,factor) in factors.iter().enumerate(){for &wire in &factor.as_ref().unwrap().scope{
            expected.entry(wire).or_default().insert(index);
        }}
        let incidence:BTreeMap<_,_>=incidence.into_iter().filter(|(_,v)|!v.is_empty()).collect();
        if incidence!=expected{return Err("held projector incidence differs from its tensor scopes".into());}
        let read=|key:&str|p[key].as_u64().and_then(|n|usize::try_from(n).ok()).ok_or_else(||format!("invalid projector {key}"));
        let internal:BTreeSet<usize>=serde_json::from_value(p["internal"].clone()).map_err(|e|e.to_string())?;
        let first_internal=read("first_internal")?;
        if internal.iter().any(|&w|w<first_internal||!incidence.contains_key(&w)){
            return Err("held projector lost an internal coordinate".into());
        }
        let pending:Vec<(usize,usize,usize)>=serde_json::from_value(p["pending"].clone()).map_err(|e|e.to_string())?;
        let versions:Vec<(usize,usize)>=serde_json::from_value(p["versions"].clone()).map_err(|e|e.to_string())?;
        let versions:BTreeMap<_,_>=versions.into_iter().collect();
        let live_pending:BTreeSet<_>=pending.iter().filter_map(|&(_,w,v)|
            (versions.get(&w)==Some(&v)).then_some(w)).collect();
        if !internal.is_subset(&live_pending){
            return Err("held projector lost its live contraction continuation".into());
        }
        Ok(Self{factors,incidence,internal,first_internal,
            pending:BinaryHeap::from(pending.into_iter().map(Reverse).collect::<Vec<_>>()),versions,
            constraints:read("constraints")?,contractions:read("contractions")?,
            copy_returns:serde_json::from_value(p.get("copy_returns").cloned().unwrap_or_else(||serde_json::json!([]))).map_err(|e|e.to_string())?})
    }
    /// Feed newly fixed coordinates back through the same incident tensors.
    /// Every restriction is implied by an existing factor; no source point is
    /// selected and no amplitude or spectator bank is reconstructed.
    fn fixed_point_return(&mut self,arena:&mut DecisionArena,start:usize,discharge_fixed:bool) {
        let mut queue=std::collections::VecDeque::from([start]);
        let mut seen=BTreeMap::new();
        let mut affected=BTreeSet::new();
        while let Some(index)=queue.pop_front() {
            let Some(factor)=self.factors[index].as_ref() else {continue;};
            if seen.get(&index)==Some(&factor.root) {continue;}
            seen.insert(index,factor.root);
            let fixed:Vec<_>=factor.scope.iter().filter_map(|&wire|
                arena.fixed_cell_value(factor.root,wire).map(|value|(wire,value))).collect();
            for (wire,value) in fixed {
                let neighbors=self.incidence.get(&wire).cloned().unwrap_or_default();
                let discharge=discharge_fixed && self.internal.contains(&wire);
                for other in neighbors {
                    let target=self.factors[other].as_mut().unwrap();
                    let before=target.root;
                    if arena.fixed_cell_value(before,wire)!=Some(value) {
                        let zero=arena.zero();
                        target.root=arena.conditional_literals(zero,before,&[(wire,value)]);
                    }
                    if discharge {
                        // All incident tensors now carry the same proved delta.
                        // Sum each delta at its unique accepted value before
                        // multiplying: their joint sum is the same product.
                        target.root=arena.contract_internal_wire(target.root,wire);
                        target.scope.remove(&wire);
                        affected.extend(target.scope.iter().filter(|w|self.internal.contains(w)).copied());
                    }
                    if target.root!=before || discharge {
                        queue.push_back(other);
                    }
                }
                if discharge {
                    self.internal.remove(&wire);self.incidence.remove(&wire);
                    self.contractions+=1;
                }
            }
        }
        for wire in affected {if self.internal.contains(&wire){self.schedule(wire);}}
    }
    /// Enlarge the same arena, retaining the original source and spectator roots.
    pub fn compile(arena:&mut DecisionArena,root:usize,e:&[usize],n:&BigUint)->Result<Self,String> {
        Self::compile_with_fixed_elimination(arena,root,e,n,true)
    }
    fn compile_with_fixed_elimination(arena:&mut DecisionArena,root:usize,e:&[usize],n:&BigUint,
        eliminate_fixed:bool)->Result<Self,String> {
        if e.len()!=n.bits() as usize || e.iter().any(|&w|w>=arena.cells) {
            return Err("projector point layout differs from source width".into());
        }
        let first=arena.cells;let width=e.len();
        let mut b=Builder{next:first,constraints:Vec::new(),column:0,columns:Vec::new()};
        let k:Vec<_>=(0..width).map(|i|{b.column=i;b.fresh()}).collect();
        let mut left=vec![Vec::new();2*width+1];
        for i in 0..width {
            left[2*i].push(Bit::Wire(e[i]));
            for j in i+1..width {
                b.column=i+j+1;
                let term=b.and(Bit::Wire(e[i]),Bit::Wire(e[j]));left[i+j+1].push(term);
            }
        }
        let mut right=vec![Vec::new();2*width+1];
        for i in 0..width {
            right[i].push(Bit::Wire(e[i]));
            for j in 0..width {if n.bit(j as u64){right[i+j].push(k[i]);}}
        }
        let left=b.compress(left);let right=b.compress(right);
        for (a,c) in left.into_iter().zip(right){b.constraint(vec![a,c],|v|v[0]==v[1]);}
        // Column the internal coordinates by the place value they carry.
        // Source and spectator coordinates keep their existing addresses.
        // Gate rows are transported by the same bijection, including their
        // assignment-bit order, before any tensor enters the shared arena.
        let mut order:Vec<_>=(first..b.next).collect();
        order.sort_by_key(|&wire|(b.columns[wire-first],wire));
        let mut mapped=vec![0usize;b.next-first];
        for (offset,wire) in order.into_iter().enumerate(){mapped[wire-first]=first+offset;}
        for (scope,rows) in &mut b.constraints {
            let mut transport:Vec<_>=scope.iter().enumerate().map(|(index,bit)|{
                let Bit::Wire(wire)=*bit else {unreachable!()};
                (if wire<first{wire}else{mapped[wire-first]},index)
            }).collect();
            transport.sort_unstable();
            for row in rows.iter_mut(){
                let before=*row;
                *row=transport.iter().enumerate().fold(0usize,|value,(after,(_,old))|
                    value|(((before>>old)&1)<<after));
            }
            *scope=transport.into_iter().map(|(wire,_)|Bit::Wire(wire)).collect();
        }
        arena.cells=b.next;
        // Propagate both the input and terminal boundaries into local tensors.
        // This operates on at most 32 gate rows, never on candidate E values.
        let mut fixed:BTreeMap<usize,bool>=e.iter().filter_map(|&w|
            arena.fixed_cell_value(root,w).map(|v|(w,v))).collect();
        let mut touches:BTreeMap<usize,Vec<usize>>=BTreeMap::new();
        for (i,(scope,_)) in b.constraints.iter().enumerate(){for bit in scope {
            if let Bit::Wire(w)=bit {touches.entry(*w).or_default().push(i);}
        }}
        let mut queue:std::collections::VecDeque<_>=(0..b.constraints.len()).collect();
        while let Some(index)=queue.pop_front() {
            let (scope,rows)=&mut b.constraints[index];
            rows.retain(|row|scope.iter().enumerate().all(|(i,bit)|match bit {
                Bit::Wire(w)=>fixed.get(w).is_none_or(|v|*v==(*row&(1<<i)!=0)),_=>true}));
            if rows.is_empty(){
                let zero=arena.zero();
                return Ok(Self{factors:vec![Some(Factor{root:zero,scope:BTreeSet::new()})],
                    incidence:BTreeMap::new(),internal:BTreeSet::new(),first_internal:first,
                    constraints:b.constraints.len(),contractions:0,copy_returns:Vec::new(),pending:BinaryHeap::new(),versions:BTreeMap::new()});
            }
            for (i,bit) in scope.iter().enumerate(){if let Bit::Wire(w)=bit {
                let value=rows[0]&(1<<i)!=0;
                if !fixed.contains_key(w) && rows.iter().all(|row|(*row&(1<<i)!=0)==value) {
                    fixed.insert(*w,value);
                    if let Some(indices)=touches.get(w){queue.extend(indices);}
                }
            }}
        }
        // A zero-cell unit is a constant amplitude, not a basis enumerator.
        let mut unit=DecisionArena::new(0);let one=unit.basis(&BigUint::from(0u8),BigInt::one());
        let one=arena.import_shifted(&unit,one,0)?;let zero=arena.zero();
        // Reverse carry propagation can prove a previously open source bit.
        // Keep that proof on the actual source bank before substituting the
        // value out of the gate rows. The prepared spectator bank is untouched.
        let source_literals:Vec<_>=fixed.iter().filter(|(w,_)|**w<first)
            .map(|(&w,&value)|(w,value)).collect();
        let projected_source=arena.conditional_literals(zero,root,&source_literals);
        let mut factors=vec![Some(Factor{root:projected_source,scope:e.iter().copied().collect()})];
        // All gate rows below have already substituted these exact values.
        // Summing a remaining unary delta is exactly one, so discharge it now
        // rather than baking an already-resolved carry into the trigger queue.
        for (&w,&value) in &fixed {if w>=first && !eliminate_fixed {
            let relation=arena.conditional_literals(zero,one,&[(w,value)]);
            factors.push(Some(Factor{root:relation,scope:[w].into_iter().collect()}));
        }}
        for (scope,rows) in b.constraints {
            let full:Vec<_>=scope.into_iter().map(|bit|match bit{Bit::Wire(w)=>w,_=>unreachable!()}).collect();
            let kept:Vec<_>=full.iter().enumerate().filter(|(_,w)|!fixed.contains_key(w)).map(|(i,&w)|(i,w)).collect();
            if kept.is_empty(){continue;}
            let rows:BTreeSet<_>=rows.into_iter().map(|row|kept.iter().enumerate().fold(0usize,|v,(j,(i,_))|
                v|(((row>>i)&1)<<j))).collect();
            let scope:Vec<_>=kept.into_iter().map(|(_,w)|w).collect();
            let mut relation=zero;
            for row in rows {
                let literals:Vec<_>=scope.iter().enumerate().map(|(i,&w)|(w,row&(1<<i)!=0)).collect();
                let cube=arena.conditional_literals(zero,one,&literals);relation=arena.sum(relation,cube);
            }
            factors.push(Some(Factor{root:relation,scope:scope.into_iter().collect()}));
        }
        let mut incidence:BTreeMap<usize,BTreeSet<usize>>=BTreeMap::new();
        for (i,factor) in factors.iter().enumerate(){for &w in &factor.as_ref().unwrap().scope {
            incidence.entry(w).or_default().insert(i);
        }}
        let constraints=factors.len()-1;
        let internal:BTreeSet<_>=(first..b.next)
            .filter(|w|!eliminate_fixed || !fixed.contains_key(w)).collect();
        let mut projector=Self{factors,incidence,internal,first_internal:first,
            constraints,contractions:0,copy_returns:Vec::new(),pending:BinaryHeap::new(),versions:BTreeMap::new()};
        for w in projector.internal.iter().copied().collect::<Vec<_>>() {projector.schedule(w);}
        Ok(projector)
    }
    pub fn internal_count(&self)->usize {self.internal.len()}
    pub(crate) fn check_carrier(&self,cells:usize,source:&[usize])->Result<(),String>{
        if self.first_internal>cells||source.iter().any(|&w|w>=self.first_internal)
            ||source.iter().copied().collect::<BTreeSet<_>>().len()!=source.len()
            ||self.internal.iter().any(|&w|w>=cells)
            ||self.factors.iter().flatten().any(|f|f.scope.iter().any(|&w|w>=cells)){
            return Err("held tensor coordinates escaped their source/register boundary".into());
        }
        Ok(())
    }
    pub fn roots(&self)->Vec<usize>{self.factors.iter().filter_map(|f|f.as_ref().map(|f|f.root)).collect()}
    pub(crate) fn carries_internal(&self,wire:usize)->bool{self.internal.contains(&wire)}
    fn schedule(&mut self,wire:usize) {
        let scope:BTreeSet<_>=self.incidence.get(&wire).into_iter().flatten().flat_map(|&i|
            self.factors[i].as_ref().unwrap().scope.iter().copied()).collect();
        let version=self.versions.entry(wire).or_default();*version+=1;
        self.pending.push(Reverse((scope.len(),wire,*version)));
    }
    pub fn record(&self)->serde_json::Value {
        serde_json::json!({"equation":"E*E=E+K*N","constraints":self.constraints,
            "internal_coordinate_order":"arithmetic-place-value-columns",
            "contractions":self.contractions,"internal_wires":self.internal,
            "copy_returns":self.copy_returns,
            "first_internal":self.first_internal,"factor_banks":self.factors.iter().enumerate()
                .filter_map(|(i,f)|f.as_ref().map(|f|serde_json::json!({"index":i,"root":f.root,"scope":f.scope})))
                .collect::<Vec<_>>()})
    }
    /// One exact bucket contraction. All factors touching this internal wire
    /// participate before it is eliminated; source E is never summed out.
    pub fn advance(&mut self,arena:&mut DecisionArena)->Result<bool,String> {
        self.advance_with_fixed_return(arena,true)
    }
    /// Pull each incident arm through a proved unit copy tensor before fusion:
    /// sum_x delta(x,f(y)) product_i F_i = product_i F_i(f(y)).
    /// The delta is used separately on every arm and retained as ancestry.
    /// No source coordinate is eliminated and complex coefficients are exact.
    fn copy_return(&mut self,arena:&mut DecisionArena,wire:usize)->bool {
        let bucket=self.incidence.get(&wire).cloned().unwrap_or_default();
        let relation=bucket.iter().find_map(|&index|{
            let factor=self.factors[index].as_ref()?;
            if factor.scope.len()!=2 {return None;}
            let other=*factor.scope.iter().find(|&&w|w!=wire)?;
            arena.phase_copy_relation(factor.root,wire,other)
                .map(|(complement,weights)|(index,factor.root,other,complement,weights))
        });
        let Some((index,weighted_delta,other,complement,weights))=relation else {return false;};
        // Extract the copy phase once. Pullback through each arm uses only the
        // unit delta, otherwise a degree-k bucket would multiply its phase k times.
        let mut delta=weighted_delta;
        for (x,weight) in weights.iter().enumerate(){
            if weight.0==crate::word_tape::SignedWordTape::from_small(1)&&weight.1.is_zero(){continue;}
            let zero=arena.zero();let controls=[(wire,x!=0)];
            let selected=arena.conditional_literals(zero,delta,&controls);
            let unphased=arena.scale_words_numerator(selected,&(weight.0.clone(),weight.1.negate()));
            delta=arena.conditional_literals(delta,unphased,&controls);
        }
        self.factors[index]=None;
        for &w in &[wire,other] {
            if let Some(indices)=self.incidence.get_mut(&w){indices.remove(&index);}
        }
        let mut changed=Vec::new();let mut affected=BTreeSet::new();
        for target_index in bucket.into_iter().filter(|&i|i!=index) {
            let target=self.factors[target_index].as_mut().unwrap();
            target.root=arena.contract_amplitude_product(delta,target.root,wire);
            target.scope.remove(&wire);target.scope.insert(other);
            self.incidence.entry(other).or_default().insert(target_index);
            affected.extend(target.scope.iter().filter(|w|self.internal.contains(w)).copied());
            changed.push(target_index);
        }
        if changed.is_empty() {
            // An isolated copy still leaves the other coordinate free. Keep
            // its unit tensor so a later sum retains its branch multiplicity.
            let root=arena.contract_internal_wire(weighted_delta,wire);
            self.factors[index]=Some(Factor{root,scope:[other].into_iter().collect()});
            self.incidence.entry(other).or_default().insert(index);
            changed.push(index);affected.insert(other);
        }else{
            // sum_x w(x) delta(x,f(y)) product_i F_i(x)
            //   = w(f(y)) product_i F_i(f(y)). Restore w on one arm only.
            let target=self.factors[changed[0]].as_mut().unwrap();
            for (x,weight) in weights.iter().enumerate(){
                if weight.0==crate::word_tape::SignedWordTape::from_small(1)&&weight.1.is_zero(){continue;}
                let controls=[(other,(x!=0)^complement)];let zero=arena.zero();
                let selected=arena.conditional_literals(zero,target.root,&controls);
                let phased=arena.scale_words_numerator(selected,weight);
                target.root=arena.conditional_literals(target.root,phased,&controls);
            }
        }
        self.internal.remove(&wire);self.incidence.remove(&wire);
        self.contractions+=1;self.copy_returns.push((wire,other,complement));
        for index in changed {self.fixed_point_return(arena,index,true);}
        for w in affected {if self.internal.contains(&w){self.schedule(w);}}
        true
    }
    fn advance_with_fixed_return(&mut self,arena:&mut DecisionArena,discharge_fixed:bool)->Result<bool,String> {
        if self.internal.is_empty(){return Ok(true);}
        let wire=loop {
            let Reverse((_,wire,version))=self.pending.pop().ok_or("missing contraction queue")?;
            if self.internal.contains(&wire)&&self.versions.get(&wire)==Some(&version){break wire;}
        };
        self.contract_selected_wire(arena,wire,discharge_fixed)
    }
    /// Fuse the coordinate whose transformed arms were measured. A support
    /// return may already have discharged it by its exact delta; in that case
    /// do not silently substitute another bucket for the recorded ancestry.
    pub(crate) fn advance_at_boundary(&mut self,arena:&mut DecisionArena,wire:usize)->Result<bool,String>{
        if !self.internal.contains(&wire){return Ok(self.internal.is_empty());}
        self.contract_selected_wire(arena,wire,true)
    }
    fn contract_selected_wire(&mut self,arena:&mut DecisionArena,wire:usize,discharge_fixed:bool)->Result<bool,String>{
        if discharge_fixed&&self.copy_return(arena,wire){return Ok(self.internal.is_empty());}
        let bucket=self.incidence.get(&wire).cloned().unwrap_or_default();
        let mut roots=Vec::with_capacity(bucket.len());let mut scope=BTreeSet::new();
        for &i in &bucket {
            let factor=self.factors[i].take().ok_or("missing projector factor")?;
            roots.push(factor.root);
            for &w in &factor.scope {if let Some(set)=self.incidence.get_mut(&w){set.remove(&i);}}
            scope.extend(factor.scope);
        }
        if roots.is_empty(){return Err("unconstrained internal projector wire".into());}
        let root=arena.contract_amplitude_family(&roots,wire);
        scope.remove(&wire);self.internal.remove(&wire);self.incidence.remove(&wire);
        let index=self.factors.len();
        for &w in &scope {self.incidence.entry(w).or_default().insert(index);}
        let affected:Vec<_>=scope.iter().filter(|w|self.internal.contains(w)).copied().collect();
        self.factors.push(Some(Factor{root,scope}));self.contractions+=1;
        self.fixed_point_return(arena,index,discharge_fixed);
        for w in affected {if self.internal.contains(&w){self.schedule(w);}}
        Ok(self.internal.is_empty())
    }
    fn join_boundary(self,arena:&mut DecisionArena)->Result<usize,String> {
        if !self.internal.is_empty(){return Err("projector retains open internal wires".into());}
        let mut joined=None;
        for f in self.factors.into_iter().flatten(){joined=Some(match joined{
            None=>f.root,Some(root)=>arena.amplitude_product(root,f.root)});}
        joined.ok_or_else(||"projector has no source boundary".into())
    }
    pub fn finish(self,arena:&mut DecisionArena)->Result<usize,String> {
        self.finish_banked(arena,&[])
    }
    pub fn finish_banked(self,arena:&mut DecisionArena,banks:&[usize])->Result<usize,String> {
        let first_internal=self.first_internal;
        let root=self.join_boundary(arena)?;
        // Auxiliary variables were summed out, not retained as free qubits.
        // Verify their absence and restore the incoming carrier's dimension.
        // Recreating them as a zero chain would enlarge every original bank
        // and force subsequent observers through already-eliminated wires.
        let mut retained=vec![root];retained.extend_from_slice(banks);
        arena.return_contracted_boundary(&retained,first_internal)?;
        Ok(root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::word_tape::SignedWordTape;

    #[test]
    fn source_202_complex_exchange_and_return_share_one_carrier(){
        let n=BigUint::parse_bytes(b"4479106148067194240380817417142593728490011937516752269508037",10).unwrap();
        let width=n.bits() as usize;assert!(width>=200);
        for complement in [false,true]{for turn in [-1,1]{
        let mut arena=DecisionArena::new(width+3);
        // Build the spectator directly in the shared register's source chart.
        let mut unit=DecisionArena::new(0);let one=unit.basis(&BigUint::from(0u8),BigInt::one());
        let one=arena.import_shifted(&unit,one,0).unwrap();let zero=arena.zero();
        let pins:Vec<_>=(0..width).map(|w|(w,n.bit(w as u64))).collect();
        let bank=arena.conditional_literals(zero,one,&pins);
        let mut p=Projector::from_relations(&mut arena,width+2,vec![
            (vec![width,width+2],if complement{vec![1,2]}else{vec![0,3]}),
            (vec![width+1,width+2],vec![1,2,3]),
        ]).unwrap();
        let before=p.roots();let product=arena.amplitude_product(before[0],before[1]);
        let expected=arena.contract_amplitude_family(&before,width+2);
        assert!(p.exchange_at_boundary(&mut arena,turn).unwrap().is_some());
        let after=p.roots();assert_ne!(before,after);
        assert_eq!(product,arena.amplitude_product(after[0],after[1]));
        assert!(p.advance(&mut arena).unwrap());
        assert_eq!(p.copy_returns.len(),1,"phased catalyst disabled the exact copy return");
        let root=p.finish_banked(&mut arena,&[bank]).unwrap();
        assert_eq!(root,expected);assert_eq!(arena.cells,width+2);
        let expected_bank=arena.conditional_literals(zero,one,&pins);
        assert_eq!(bank,expected_bank);
        }}
    }

    #[test]
    fn source_sized_copy_return_preserves_complex_arms() {
        let n=BigUint::parse_bytes(b"894334587051813008271775013505301739404417062575564752132471",10).unwrap();
        let width=n.bits() as usize;assert!(width>=200);
        for complement in [false,true] {
            let mut arena=DecisionArena::new(width+1);
            let real=arena.basis(&n,BigInt::from(3));
            let imaginary=arena.flip(real,width);
            let imaginary=arena.scale_words(imaginary,&(SignedWordTape::from_bigint(&BigInt::from(-2)),
                SignedWordTape::from_bigint(&BigInt::from(5))),0);
            let source=arena.sum(real,imaginary);
            let (y,z,x)=(width+1,width+2,width+3);let first=x;
            arena.cells=x+1;
            let mut unit=DecisionArena::new(0);
            let one=unit.basis(&BigUint::from(0u8),BigInt::one());
            let one=arena.import_shifted(&unit,one,0).unwrap();let zero=arena.zero();
            let low=arena.conditional_literals(zero,one,&[(y,false),(x,complement)]);
            let high=arena.conditional_literals(zero,one,&[(y,true),(x,!complement)]);
            let delta=arena.sum(low,high);
            assert_eq!(arena.unit_copy_relation(delta,x,y),Some(complement));
            let weighted=arena.sum(delta,delta);
            assert_eq!(arena.unit_copy_relation(weighted,x,y),None,"weighted tensor accepted as unit copy");
            let phase=arena.scale_words(source,&(SignedWordTape::from_bigint(&BigInt::from(-1)),
                SignedWordTape::from_bigint(&BigInt::from(2))),0);
            let a=arena.conditional_literals(source,phase,&[(x,true)]);
            let seven=arena.scale_words(one,&(SignedWordTape::from_bigint(&BigInt::from(7)),SignedWordTape::zero()),0);
            let b=arena.conditional_literals(one,seven,&[(z,true),(x,true)]);
            let joined=arena.amplitude_product(a,b);
            let joined=arena.amplitude_product(delta,joined);
            let expected=arena.contract_internal_wire(joined,x);
            let factors=vec![Some(Factor{root:a,scope:(0..=x).collect()}),
                Some(Factor{root:delta,scope:[x,y].into_iter().collect()}),
                Some(Factor{root:b,scope:[x,z].into_iter().collect()})];
            let mut incidence:BTreeMap<usize,BTreeSet<usize>>=BTreeMap::new();
            for (i,factor) in factors.iter().enumerate(){for &wire in &factor.as_ref().unwrap().scope{
                incidence.entry(wire).or_default().insert(i);
            }}
            let mut projector=Projector{factors,incidence,internal:[x].into_iter().collect(),first_internal:first,
                pending:BinaryHeap::new(),versions:BTreeMap::new(),constraints:2,contractions:0,copy_returns:Vec::new()};
            projector.schedule(x);
            let gram=arena.control_gram(&[source,real,imaginary,expected]);
            assert!(projector.advance(&mut arena).unwrap());
            assert_eq!(projector.copy_returns,vec![(x,y,complement)]);
            assert_eq!(projector.preparation_record()["copy_returns"],serde_json::json!([[x,y,complement]]));
            let actual=projector.finish_banked(&mut arena,&[source,real,imaginary]).unwrap();
            assert_eq!(actual,expected,"distributed return changed complex amplitudes");
            // Both reference and returned roots now share the incoming dimension.
            assert_eq!(arena.control_gram(&[source,real,imaginary,actual]),
                gram.map(|(re,im)|(re/2,im/2)));
            assert_eq!(arena.cells,first);
            // A detached copy pair has two accepted alternatives. Returning it
            // must retain that multiplicity rather than discard the free arm.
            let (u,v)=(first,first+1);arena.cells=first+2;
            let low=arena.conditional_literals(zero,one,&[(u,false),(v,complement)]);
            let high=arena.conditional_literals(zero,one,&[(u,true),(v,!complement)]);
            let delta=arena.sum(low,high);
            let factors=vec![Some(Factor{root:source,scope:(0..first).collect()}),
                Some(Factor{root:delta,scope:[u,v].into_iter().collect()})];
            let mut incidence:BTreeMap<usize,BTreeSet<usize>>=BTreeMap::new();
            for (i,factor) in factors.iter().enumerate(){for &wire in &factor.as_ref().unwrap().scope{
                incidence.entry(wire).or_default().insert(i);
            }}
            let mut projector=Projector{factors,incidence,internal:[u,v].into_iter().collect(),first_internal:first,
                pending:BinaryHeap::new(),versions:BTreeMap::new(),constraints:1,contractions:0,copy_returns:Vec::new()};
            for wire in [u,v]{projector.schedule(wire);}
            while !projector.advance(&mut arena).unwrap() {}
            assert_eq!(projector.copy_returns.len(),1);
            let actual=projector.finish_banked(&mut arena,&[source,real,imaginary]).unwrap();
            assert_eq!(actual,arena.sum(source,source),"isolated copy lost its free alternative");
        }
    }

    #[test]
    fn source_sized_dynamic_carry_return_matches_joint_contraction() {
        let n=BigUint::parse_bytes(b"894334587051813008271775013505301739404417062575564752132471",10).unwrap();
        let width=n.bits() as usize;assert!(width>=200);
        let build=||{
            let first=width+1;
            let mut arena=DecisionArena::new(first);
            let real=arena.basis(&n,BigInt::from(3));
            let imaginary=arena.flip(real,width);
            let imaginary=arena.scale_words(imaginary,&(SignedWordTape::from_bigint(&BigInt::from(-2)),
                SignedWordTape::from_bigint(&BigInt::from(5))),0);
            let source=arena.sum(real,imaginary);
            arena.cells+=3;
            let (x,z,w)=(first,first+1,first+2);
            let mut unit=DecisionArena::new(0);
            let one=unit.basis(&BigUint::from(0u8),BigInt::one());
            let one=arena.import_shifted(&unit,one,0).unwrap();
            let mut relation=|scope:Vec<usize>,accept:&dyn Fn(usize)->bool|{
                let zero=arena.zero();let mut root=zero;
                for row in 0..1usize<<scope.len(){if accept(row){
                    let literals:Vec<_>=scope.iter().enumerate().map(|(i,&wire)|(wire,row&(1<<i)!=0)).collect();
                    let cube=arena.conditional_literals(zero,one,&literals);
                    root=arena.sum(root,cube);
                }}
                Factor{root,scope:scope.into_iter().collect()}
            };
            let parity=usize::from(n.bit(0));
            let a=relation(vec![x,z,w],&|row|((row>>2)&1)==((row&1)^((row>>1)&1)^parity));
            let b=relation(vec![x,z],&|row|(row&1)==((row>>1)&1));
            let c=relation(vec![0,w],&|row|(row&1)==((row>>1)&1));
            let factors=vec![Some(Factor{root:source,scope:(0..first).collect()}),Some(a),Some(b),Some(c)];
            let mut incidence:BTreeMap<usize,BTreeSet<usize>>=BTreeMap::new();
            for (i,factor) in factors.iter().enumerate(){for &wire in &factor.as_ref().unwrap().scope{
                incidence.entry(wire).or_default().insert(i);
            }}
            let mut projector=Projector{factors,incidence,internal:[x,z,w].into_iter().collect(),first_internal:first,
                pending:BinaryHeap::new(),versions:BTreeMap::new(),constraints:3,contractions:0,copy_returns:Vec::new()};
            for wire in [x,z,w]{projector.schedule(wire);}
            (arena,projector,source,real,imaginary,w)
        };
        let (mut reference_arena,mut reference,reference_source,reference_real,reference_imaginary,_)=build();
        let mut reference_steps=0;
        loop {reference_steps+=1;if reference.advance_with_fixed_return(&mut reference_arena,false).unwrap(){break;}}
        let expected=reference.finish_banked(&mut reference_arena,&[reference_source]).unwrap();
        let (mut arena,mut projector,source,real,imaginary,w)=build();
        let mut steps=1;
        let mut done=projector.advance(&mut arena).unwrap();
        assert!(!projector.internal.contains(&w),"newly proved carry remained pending");
        let held=projector.preparation_record();
        assert!(held["scopes"].as_array().unwrap().len()<projector.factors.len(),"control did not exercise removed tensor banks");
        let mut roots=vec![source,real,imaginary];roots.extend(projector.roots());
        let register=arena.compiled_record_many(&roots);
        let (mut restored_arena,restored_roots)=DecisionArena::from_prepared_record_many(&register).unwrap();
        let mut restored=Projector::from_preparation_record(&held,&restored_roots[3..]).unwrap();
        assert_eq!(restored.preparation_record(),held,"held tensor/queue ancestry changed");
        let mut corrupt=held.clone();
        corrupt["incidence"][0][1]=serde_json::json!([usize::MAX]);
        assert!(Projector::from_preparation_record(&corrupt,&restored_roots[3..]).is_err());
        while !restored.advance(&mut restored_arena).unwrap() {}
        let restored_result=restored.finish_banked(&mut restored_arena,&restored_roots[..3]).unwrap();
        while !done {done=projector.advance(&mut arena).unwrap();steps+=1;}
        assert_eq!(projector.contractions,3);
        let actual=projector.finish_banked(&mut arena,&[source,real,imaginary]).unwrap();
        let imported=arena.import_shifted(&reference_arena,expected,0).unwrap();
        assert_eq!(actual,imported);
        assert_eq!(actual,arena.import_shifted(&restored_arena,restored_result,0).unwrap(),
            "resumed tensor computation diverged from uninterrupted return");
        let doubled=arena.scale_words(source,&(SignedWordTape::from_bigint(&BigInt::from(2)),SignedWordTape::zero()),0);
        assert_eq!(actual,doubled,"free auxiliary branch multiplicity changed");
        assert_eq!(arena.control_gram(&[actual,source,real,imaginary]),
            reference_arena.control_gram(&[expected,reference_source,reference_real,reference_imaginary]));
        assert!(steps<reference_steps);
        eprintln!("dynamic-carry calibration bits={width} payload_steps_before={reference_steps} payload_steps_after={steps} logical_contractions=3");
    }

    #[test]
    fn source_sized_fixed_carry_elimination_preserves_complex_banks() {
        let n=BigUint::parse_bytes(b"894334587051813008271775013505301739404417062575564752132471",10).unwrap();
        let width=n.bits() as usize;assert!(width>=200);
        let build=||{
            let mut arena=DecisionArena::new(width+1);
            let first=arena.basis(&BigUint::one(),BigInt::from(3));
            let other=arena.flip(first,width);
            let other=arena.scale_words(other,&(SignedWordTape::from_bigint(&BigInt::from(-2)),
                SignedWordTape::from_bigint(&BigInt::from(5))),0);
            let valid=arena.sum(first,other);
            let nonpoint=arena.flip(valid,1); // E=3 has E²-E=6, not 0 mod N.
            let root=arena.sum(valid,nonpoint);
            (arena,root,first,other)
        };
        let (mut arena,root,first,other)=build();
        let original_cells=arena.cells;
        let original_gram=arena.control_gram(&[root,root,first,other]);
        let wires:Vec<_>=(0..width).collect();
        let (mut reference_arena,reference_root,reference_first,reference_other)=build();
        assert_eq!((root,first,other),(reference_root,reference_first,reference_other));
        let mut reference=Projector::compile_with_fixed_elimination(&mut reference_arena,root,&wires,&n,false).unwrap();
        let reference_count=reference.internal_count();
        while !reference.advance(&mut reference_arena).unwrap() {}
        let expected=reference.finish(&mut reference_arena).unwrap();
        let mut compiled=Projector::compile(&mut arena,root,&wires,&n).unwrap();
        let compiled_count=compiled.internal_count();
        assert!(compiled_count<reference_count,"fixed carries were not discharged");
        let expanded_cells=arena.cells;
        let zero=arena.zero();
        let open_bank=arena.conditional_literals(zero,root,&[(original_cells,true)]);
        assert!(arena.return_contracted_boundary(&[root,open_bank],original_cells).is_err());
        assert_eq!(arena.cells,expanded_cells,"rejected return changed the carrier");
        while !compiled.advance(&mut arena).unwrap() {}
        let actual=compiled.finish_banked(&mut arena,&[root,first,other]).unwrap();
        assert_eq!(arena.cells,original_cells);
        assert_eq!(reference_arena.cells,original_cells);
        let valid=arena.sum(first,other);
        assert_eq!(actual,valid,"calibration failed to reject the E=3 arm");
        // Importing into the same canonical arena compares every complex leaf
        // and branch without serializing a quadratic set of support masks.
        let imported=arena.import_shifted(&reference_arena,expected,0).unwrap();
        assert_eq!(actual,imported);
        for bank in [root,first,other] {
            assert_eq!(bank,arena.import_shifted(&reference_arena,bank,0).unwrap());
        }
        assert_eq!(arena.control_gram(&[root,root,first,other]),original_gram);
        assert_eq!(arena.control_gram(&[actual,root,first,other]),reference_arena.control_gram(&[expected,root,first,other]));
        eprintln!("fixed-carry calibration bits={width} queued_before={reference_count} queued_after={compiled_count}");
    }
}
