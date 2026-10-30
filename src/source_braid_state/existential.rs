//! Exact support contraction and reverse witness recovery for the source
//! proper-axis family. This reader does not count amplitudes or enumerate
//! divisor candidates. Boolean cofactors are internal elimination work.
use std::collections::{HashMap,HashSet,BinaryHeap};
use std::cmp::Reverse;
use std::time::{Instant,Duration};
use serde_json::{json,Value};
use crate::word_tape::WordTape;
use crate::source_braid_contraction::circuit::Wire;
use super::axis_tensor::AxisCircuit;

#[derive(Clone,Copy,serde::Serialize)]
enum Node { Zero,One,Branch(usize,usize,usize),Dead }
struct Arena {
    nodes:Vec<Node>,unique:HashMap<(usize,usize,usize),usize>,
    apply:HashMap<(bool,usize,usize),usize>, deadline:Instant, limit:usize, visits:u64,
    free:Vec<usize>,
}
impl Arena {
    fn check(&mut self)->Result<(),String>{
        self.visits+=1;
        if self.live_nodes()>=self.limit{return Err("retained-node boundary".into());}
        if self.visits%1024==0 && Instant::now()>=self.deadline{return Err("execution deadline".into());}
        Ok(())
    }
    fn branch(&mut self,v:usize,l:usize,h:usize)->Result<usize,String>{
        if l==h{return Ok(l);}
        if let Some(&id)=self.unique.get(&(v,l,h)){return Ok(id);}
        self.check()?;
        let id=if let Some(id)=self.free.pop(){self.nodes[id]=Node::Branch(v,l,h);id}
            else{let id=self.nodes.len();self.nodes.push(Node::Branch(v,l,h));id};
        self.unique.insert((v,l,h),id);Ok(id)
    }
    fn top(&self,id:usize)->usize{match self.nodes[id]{Node::Branch(v,_,_)=>v,_=>usize::MAX}}
    fn split(&self,id:usize,v:usize)->(usize,usize){match self.nodes[id]{Node::Branch(w,l,h)if w==v=>(l,h),_=>(id,id)}}
    fn combine(&mut self,or:bool,mut a:usize,mut b:usize)->Result<usize,String>{
        self.check()?;
        if a>b{std::mem::swap(&mut a,&mut b);}
        if a==b{return Ok(a);}
        if or {if b==1||a==1{return Ok(1);}if a==0{return Ok(b);}}
        else {if a==0{return Ok(0);}if a==1{return Ok(b);}}
        if let Some(&r)=self.apply.get(&(or,a,b)){return Ok(r);}
        let v=self.top(a).min(self.top(b));
        let(al,ah)=self.split(a,v);let(bl,bh)=self.split(b,v);
        let l=self.combine(or,al,bl)?;let h=self.combine(or,ah,bh)?;
        let r=self.branch(v,l,h)?;self.apply.insert((or,a,b),r);Ok(r)
    }
    fn table(&mut self,scope:&[usize],rows:&[usize])->Result<usize,String>{
        if scope.is_empty(){return Ok(usize::from(!rows.is_empty()));}
        let(low,high):(Vec<_>,Vec<_>)=rows.iter().copied().partition(|r|r&1==0);
        let low:Vec<_>=low.into_iter().map(|r|r>>1).collect();
        let high:Vec<_>=high.into_iter().map(|r|r>>1).collect();
        let l=self.table(&scope[1..],&low)?;let h=self.table(&scope[1..],&high)?;
        self.branch(scope[0],l,h)
    }
    fn exists_family(&mut self,roots:Vec<usize>,v:usize)->Result<usize,String>{
        fn visit(a:&mut Arena,mut roots:Vec<usize>,v:usize,cut:bool,
            cache:&mut HashMap<(Vec<usize>,bool),usize>)->Result<usize,String>{
            a.check()?;
            if roots.contains(&0){return Ok(0);}
            roots.retain(|&r|r!=1);roots.sort_unstable();roots.dedup();
            if roots.is_empty(){return Ok(1);}
            let key=(roots.clone(),cut);
            if let Some(&r)=cache.get(&key){return Ok(r);}
            let next=roots.iter().map(|&r|a.top(r)).min().unwrap();
            if cut && next>v{return visit(a,roots,v,false,cache);}
            let(mut ls,mut hs)=(Vec::new(),Vec::new());
            for r in roots{let(l,h)=a.split(r,next);ls.push(l);hs.push(h);}
            let eliminate=cut && next==v;
            let l=visit(a,ls,v,cut&&!eliminate,cache)?;
            let h=visit(a,hs,v,cut&&!eliminate,cache)?;
            let r=if eliminate{a.combine(true,l,h)?}else{a.branch(next,l,h)?};
            cache.insert(key,r);Ok(r)
        }
        visit(self,roots,v,true,&mut HashMap::new())
    }
    fn eval(&self,mut root:usize,values:&[bool],override_at:Option<(usize,bool)>)->bool{
        loop{match self.nodes[root]{Node::Zero=>return false,Node::One=>return true,
            Node::Branch(v,l,h)=>root=if override_at.filter(|&(at,_)|at==v).map_or(values[v],|(_,b)|b){h}else{l},
            Node::Dead=>unreachable!("recovery referenced reclaimed node")}}
    }
    fn support(&self,root:usize)->Vec<usize>{
        let mut pending=vec![root];let mut seen=HashSet::new();let mut vars=HashSet::new();
        while let Some(r)=pending.pop(){if seen.insert(r){if let Node::Branch(v,l,h)=self.nodes[r]{vars.insert(v);pending.extend([l,h]);}}}
        let mut vars:Vec<_>=vars.into_iter().collect();vars.sort_unstable();vars
    }
    fn low_cofactor(&mut self,id:usize,v:usize,memo:&mut HashMap<usize,usize>)->Result<usize,String>{
        self.check()?;
        if let Some(&r)=memo.get(&id){return Ok(r);}
        let r=match self.nodes[id]{
            Node::Branch(w,l,_)if w==v=>l,
            Node::Branch(w,l,h)if w<v=>{
                let l=self.low_cofactor(l,v,memo)?;let h=self.low_cofactor(h,v,memo)?;
                self.branch(w,l,h)?
            },
            _=>id,
        };
        memo.insert(id,r);Ok(r)
    }
    fn live_nodes(&self)->usize{self.nodes.len()-self.free.len()}
    fn reclaim(&mut self,factors:&[(Vec<usize>,usize,bool)],history:&[(usize,Vec<usize>)])->usize{
        let before=self.live_nodes();let mut seen=vec![false;self.nodes.len()];let mut stack=vec![0,1];
        stack.extend(factors.iter().filter(|f|f.2).map(|f|f.1));
        for (_,roots) in history{stack.extend(roots);}
        while let Some(id)=stack.pop(){if seen[id]{continue;}seen[id]=true;
            match self.nodes[id]{Node::Branch(_,l,h)=>stack.extend([l,h]),Node::Dead=>panic!("live bank references dead slot"),_=>{}}}
        self.apply.clear();self.unique.clear();self.free.clear();
        for(id,node)in self.nodes.iter_mut().enumerate(){
            if !seen[id]{*node=Node::Dead;self.free.push(id);}
            else if let Node::Branch(v,l,h)=*node{self.unique.insert((v,l,h),id);}
        }
        before-self.live_nodes()
    }
}

fn read(w:Wire,values:&[bool])->bool{w.variable().map_or(w.0==1,|v|values[v])}

/// Run from N alone. A held return contains all reader nodes, remaining
/// factors and recovery banks; it is not a success receipt.
pub fn select(source:WordTape,seconds:f64,node_limit:usize)->Result<Value,String>{
    let start=Instant::now();let deadline=start+Duration::from_secs_f64(seconds);
    // The source amplitude bank remains resident while its observation is
    // contracted. An arithmetic image uses this actual arena root, not a
    // placeholder ID manufactured after witness recovery.
    let width=source.bit_len();let mut source_arena=crate::ququart_decision::DecisionArena::new(width+1);
    let n=source.to_biguint();let zero=source_arena.zero();
    let mut source_root=source_arena.basis(&num_bigint::BigUint::from(0u8),num_bigint::BigInt::from(1));
    let source_wires:Vec<_>=(0..width).collect();
    for &wire in &source_wires{let high=source_arena.flip(source_root,wire);source_root=source_arena.sum(source_root,high);}
    source_root=source_arena.compare_constant(source_root,&source_wires,&n,&[],width);
    source_root=source_arena.conditional_literals(zero,source_root,&[(width,true)]);
    source_root=source_arena.flip(source_root,width);source_arena.fold_roots(&[source_root]);
    if source_arena.mass(source_root)!=n{return Err("source bank changed during reader preparation".into());}
    let image=super::arithmetic_image::SquareImage::new(source.clone(),source_root)?;
    let circuit=AxisCircuit::prepare(&image.source)?;let aw=circuit.left.len();let bw=circuit.right.len();
    let restriction=circuit.restrict_source();
    let mut input_variables=vec![false;circuit.variables];
    for &w in circuit.left.iter().chain(&circuit.right){
        if let Some(v)=Wire(w).variable(){input_variables[v]=true;}
    }
    let circuit_path=std::env::var_os("AXIS_READER_CHECKPOINT").map(|p|
        std::path::PathBuf::from(p).with_extension("circuit.json"));
    if let Some(path)=&circuit_path{
        let file=std::io::BufWriter::new(std::fs::File::create(path).map_err(|e|e.to_string())?);
        serde_json::to_writer(file,&circuit).map_err(|e|e.to_string())?;
    }
    let mut arena=Arena{nodes:vec![Node::Zero,Node::One],unique:HashMap::new(),apply:HashMap::new(),deadline,limit:node_limit,visits:0,free:Vec::new()};
    let mut factors:Vec<(Vec<usize>,usize,bool)>=Vec::new();
    let mut incidence=vec![HashSet::new();circuit.variables];
    let mut neighbors=vec![HashSet::new();circuit.variables];
    for constraint in &restriction.constraints{
        let scope=constraint.scope.clone();let root=arena.table(&scope,&constraint.rows)?;let id=factors.len();
        for &v in &scope{incidence[v].insert(id);for &w in &scope{if v!=w{neighbors[v].insert(w);}}}
        factors.push((scope,root,true));
    }
    let prep=start.elapsed().as_secs_f64();
    // Internal coordinates are functional extensions of the input pair.
    // Quantify them first, keeping their original tensor equations for exact
    // recovery. Only the final input projection requires choice guards.
    let mut queue:BinaryHeap<_>=(0..circuit.variables).map(|v|
        Reverse((input_variables[v],neighbors[v].len(),v))).collect();
    let mut eliminated=vec![false;circuit.variables];
    let mut history:Vec<(usize,Vec<usize>)>=Vec::new();let mut events=Vec::new();
    let mut held=None;let mut impossible=factors.iter().any(|f|f.1==0);
    while let Some(Reverse((_,degree,v)))=queue.pop(){
        if impossible{break;}
        if eliminated[v]||degree!=neighbors[v].len(){continue;}
        if Instant::now()>=deadline{held=Some("execution deadline".to_string());break;}
        let ids:Vec<_>=incidence[v].iter().copied().filter(|&i|factors[i].2).collect();
        let roots:Vec<_>=ids.iter().map(|&i|factors[i].1).collect();
        let contracted=if roots.is_empty(){Some((1,Vec::new()))}else{loop{
            // A recovery bank only needs the zero-arm guard. The existential
            // message certifies that the one arm is supported if it fails.
            // Keep the original complete circuit separately, unchanged.
            let attempt=(||{
                let mut memo=HashMap::new();let mut low=Vec::new();
                if input_variables[v]{
                    for &root in &roots{
                        let r=arena.low_cofactor(root,v,&mut memo)?;
                        if r==0{low=vec![0];break;}
                        if r!=1{low.push(r);}
                    }
                }
                low.sort_unstable();low.dedup();
                Ok::<_,String>((arena.exists_family(roots.clone(),v)?,low))
            })();
            match attempt{
            Ok(r)=>break Some(r),
            Err(e)if e=="retained-node boundary"=>{
                let reclaimed=arena.reclaim(&factors,&history);
                events.push(json!({"at_variable":v,"reclaimed":reclaimed,"live_nodes":arena.live_nodes(),"seconds":start.elapsed().as_secs_f64()}));
                if reclaimed<node_limit/20{held=Some("live retained-node boundary".into());break None;}
                if Instant::now()>=deadline{held=Some("execution deadline".into());break None;}
            }
            Err(e)=>{held=Some(e);break None;}}}};
        let Some((message,low_choices))=contracted else{break;};
        if message==0{impossible=true;break;}
        let mut affected=neighbors[v].clone();
        for &i in &ids{for &w in &factors[i].0{affected.insert(w);incidence[w].remove(&i);}factors[i].2=false;}
        let scope=arena.support(message);
        if message!=1{let id=factors.len();for &w in &scope{incidence[w].insert(id);}factors.push((scope,message,true));}
        history.push((v,low_choices));eliminated[v]=true;
        for w in affected{
            if eliminated[w]{continue;}
            neighbors[w].clear();
            for &i in &incidence[w]{if factors[i].2{for &z in &factors[i].0{if z!=w{neighbors[w].insert(z);}}}}
            queue.push(Reverse((input_variables[w],neighbors[w].len(),w)));
        }
        neighbors[v].clear();
        if history.len()%1000==0{
            eprintln!("EXISTS eliminated={} variables={} nodes={} degree={} seconds={}",history.len(),circuit.variables,arena.live_nodes(),degree,start.elapsed().as_secs_f64());
            events.push(json!({"eliminated":history.len(),"nodes":arena.live_nodes(),"degree":degree,"seconds":start.elapsed().as_secs_f64()}));
        }
    }
    let mut result=None;
    if held.is_none()&&!impossible{
        let mut values:Vec<_>=restriction.domains.iter().map(|&d|
            d==vox_core::belnap_relation::BelnapSet::singleton(crate::belnap::B4::T).mask()).collect();
        for (v,roots) in history.iter().rev(){
            if input_variables[*v]&&restriction.domains[*v]==vox_core::belnap_relation::BelnapSet::NUMERAL.mask(){
                values[*v]=!roots.iter().all(|&r|arena.eval(r,&values,None));
            }
        }
        circuit.recover_internal(&mut values)?;
        for c in &circuit.constraints{
            let mask=c.scope.iter().enumerate().fold(0,|mask,(i,&v)|mask|(usize::from(values[v])<<i));
            if !c.rows.contains(&mask){return Err("recovered assignment violates retained native tensor".into());}
        }
        let numeral=|lane:&[usize]|WordTape::from_imasm_tape(&lane.iter().map(|&w|if read(Wire(w),&values){'⊥'}else{'⊤'}).collect::<Vec<_>>()).unwrap();
        let a=numeral(&circuit.left);let b=numeral(&circuit.right);
        if a<=WordTape::one()||b<=WordTape::one()||a.mul(&b)!=source{return Err("recovered axis member fails exact product closure".into());}
        let projected=image.project_proper_axis_family();
        let axis=projected.read_member(&a,&WordTape::zero())?;
        result=Some(json!({"first_word":a.as_word(),"second_word":b.as_word(),
            "p":a.to_biguint().to_string(),"q":b.to_biguint().to_string(),
            "source_left_word":axis.left.address.as_word(),"source_right_word":axis.right.address.as_word(),
            "exact_product":true,"input_boundary":"N-only"}));
    }
    #[derive(serde::Serialize)]
    struct Checkpoint<'a>{source_word:&'a str,source_image:Value,source_register:Value,left_width:usize,right_width:usize,input_variables:&'a[bool],source_restriction:&'a super::axis_tensor::SourceRestriction,circuit_path:&'a Option<std::path::PathBuf>,nodes:&'a[Node],
        factors:&'a[(Vec<usize>,usize,bool)],recovery_banks:&'a[(usize,Vec<usize>)]}
    let before_serialization=start.elapsed().as_secs_f64();
    let retained=if let Some(path)=std::env::var_os("AXIS_READER_CHECKPOINT"){
        use std::io::Write;
        let path=std::path::PathBuf::from(path);
        let mut file=std::io::BufWriter::new(std::fs::File::create(&path).map_err(|e|e.to_string())?);
        let source_register=source_arena.prepared_record_many(&[source_root]);
        let mut source_image=image.record();
        source_image["source_root"]=source_register["root"].clone();
        source_image["root_coordinate_system"]=json!("serialized-source-register");
        serde_json::to_writer(&mut file,&Checkpoint{source_word:source.as_word(),source_image,
            source_register,left_width:aw,right_width:bw,input_variables:&input_variables,source_restriction:&restriction,circuit_path:&circuit_path,
            nodes:&arena.nodes,factors:&factors,recovery_banks:&history}).map_err(|e|e.to_string())?;
        file.flush().map_err(|e|e.to_string())?;
        json!({"path":path,"nodes":arena.live_nodes(),"slots":arena.nodes.len(),"format":"streamed exact Boolean reader and reverse-recovery banks"})
    }else{json!({"checkpoint_saved":false})};
    Ok(json!({"schema":"existential-proper-axis-reader/v1","N":source.to_biguint().to_string(),
        "source_word":source.as_word(),"method":"fused Boolean existential elimination with reverse retained-choice recovery",
        "source_bank_nodes":source_arena.retained_nodes(),"source_image_root":source_root,
        "status":if result.is_some(){"closed"}else if impossible{"empty"}else{"held"},"factors":result,
        "preparation_seconds":prep,"seconds_before_serialization":before_serialization,"seconds":start.elapsed().as_secs_f64(),
        "derived_widths":[aw,bw],"variables":circuit.variables,"gates":circuit.constraints.len(),
        "constraint_representation":"native Belnap AND and layered three-to-two ADD numeral-slice tensors; interleaved input coordinates",
        "eliminated":history.len(),"reader_nodes":arena.live_nodes(),"allocated_slots":arena.nodes.len(),"cofactor_visits":arena.visits,
        "held_reason":held,"events":events,"node_limit":node_limit,
        "recovery_semantics":"factor-input zero-arm guards; internal coordinates recovered deterministically from retained native tensors",
        "choice_guard_variables":history.iter().filter(|(v,_)|input_variables[*v]).count(),
        "functional_internal_variables":input_variables.iter().filter(|&&v|!v).count(),
        "source_restriction":{"revisions":restriction.revisions,"updates":restriction.updates.len(),
            "fixed_variables":restriction.domains.iter().filter(|&&d|d==2||d==4).count(),
            "remaining_constraints":restriction.constraints.len()},
        "original_circuit":circuit_path,
        "retained":retained,
        "factor_candidates_tested":0,"Y_evaluator_executed":false,
        "full_complex_SIC_executed":false,"closed_polymer_cycle_executed":false}))
}
