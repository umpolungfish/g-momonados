//! Persistent zero-suppressed coefficient trie, with cooperative exact tasks.
use std::collections::HashMap;
use serde_json::{json,Value};
use crate::word_tape::SignedWordTape as Coeff;
pub type Id=u32;
#[derive(Clone,Debug,PartialEq,Eq,Hash)]
pub enum Node {Zero,Leaf(Coeff),Branch{wire:usize,low:Id,high:Id}}
#[derive(Clone,Copy,Debug,PartialEq,Eq,Hash)]
pub enum Op {Add(Id,Id),Neg(Id),Double(Id),Force(usize,Id),Eliminate(usize,Id)}
impl Op {
    pub fn add(a:Id,b:Id)->Self{if a<=b{Self::Add(a,b)}else{Self::Add(b,a)}}
    pub fn json(self)->Value{match self{
        Self::Add(a,b)=>json!(["add",a,b]),Self::Neg(a)=>json!(["neg",a]),Self::Double(a)=>json!(["double",a]),
        Self::Force(w,a)=>json!(["force",w,a]),Self::Eliminate(w,a)=>json!(["eliminate",w,a])}}
}
pub struct Arena {
    pub nodes:Vec<Node>,intern:HashMap<Node,Id>,pub cache:HashMap<Op,Id>,pub stack:Vec<Op>,pub steps:u64,
}
impl Arena {
    pub fn new()->Self{let mut a=Self{nodes:Vec::new(),intern:HashMap::new(),cache:HashMap::new(),stack:Vec::new(),steps:0};a.intern(Node::Zero);a}
    pub fn intern(&mut self,n:Node)->Id{
        if let Some(&id)=self.intern.get(&n){return id;}
        let id=Id::try_from(self.nodes.len()).expect("coefficient arena address exhausted");
        self.nodes.push(n.clone());self.intern.insert(n,id);id
    }
    pub fn leaf(&mut self,c:Coeff)->Id{if c.is_zero(){0}else{self.intern(Node::Leaf(c))}}
    fn head(&self,id:Id)->usize{match self.nodes[id as usize]{Node::Branch{wire,..}=>wire,_=>usize::MAX}}
    fn parts(&self,id:Id,w:usize)->(Id,Id){match self.nodes[id as usize]{Node::Branch{wire,low,high} if wire==w=>(low,high),_=>(id,0)}}
    fn branch(&mut self,w:usize,low:Id,high:Id)->Id{
        if high==0{return low;}
        assert!(self.head(low)>w&&self.head(high)>w,"coefficient trie address order");
        self.intern(Node::Branch{wire:w,low,high})
    }
    fn ready(&mut self,ops:&[Op])->Option<Vec<Id>>{
        let mut values=Vec::with_capacity(ops.len());
        for op in ops{if let Some(&id)=self.cache.get(op){values.push(id);}else{self.stack.push(*op);return None;}}
        Some(values)
    }
    pub fn start(&mut self,op:Op){if !self.cache.contains_key(&op)&&self.stack.is_empty(){self.stack.push(op);}}
    pub fn step(&mut self)->bool{
        let Some(op)=self.stack.last().copied() else{return false;};
        if self.cache.contains_key(&op){self.stack.pop();return true;}
        self.steps+=1;
        let result=match op {
            Op::Add(a,b)=>{
                if a==0{Some(b)}else if b==0{Some(a)}else{
                    match (self.nodes[a as usize].clone(),self.nodes[b as usize].clone()){
                        (Node::Leaf(x),Node::Leaf(y))=>Some(self.leaf(x.add(&y))),
                        _=>{let w=self.head(a).min(self.head(b));let(al,ah)=self.parts(a,w);let(bl,bh)=self.parts(b,w);
                            self.ready(&[Op::add(al,bl),Op::add(ah,bh)]).map(|r|self.branch(w,r[0],r[1]))}
                    }
                }
            },
            Op::Neg(a)|Op::Double(a)=>{
                match self.nodes[a as usize].clone(){
                    Node::Zero=>Some(0),
                    Node::Leaf(c)=>Some(self.leaf(if matches!(op,Op::Neg(_)){c.negate()}else{c.shl(1)})),
                    Node::Branch{wire,low,high}=>{
                        let f=|id|if matches!(op,Op::Neg(_)){Op::Neg(id)}else{Op::Double(id)};
                        self.ready(&[f(low),f(high)]).map(|r|self.branch(wire,r[0],r[1]))
                    }
                }
            },
            Op::Force(w,a)=>{
                if a==0{Some(0)}else{match self.nodes[a as usize].clone(){
                    Node::Branch{wire,low,high} if wire<w=>self.ready(&[Op::Force(w,low),Op::Force(w,high)]).map(|r|self.branch(wire,r[0],r[1])),
                    Node::Branch{wire,low,high} if wire==w=>self.ready(&[Op::add(low,high)]).map(|r|self.branch(w,0,r[0])),
                    _=>Some(self.branch(w,0,a))
                }}
            },
            Op::Eliminate(w,a)=>{
                if a==0{Some(0)}else{match self.nodes[a as usize].clone(){
                    Node::Branch{wire,low,high} if wire<w=>self.ready(&[Op::Eliminate(w,low),Op::Eliminate(w,high)]).map(|r|self.branch(wire,r[0],r[1])),
                    Node::Branch{wire,low,high} if wire==w=>{
                        if let Some(r)=self.ready(&[Op::Double(low)]){self.ready(&[Op::add(r[0],high)]).map(|s|s[0])}else{None}
                    },
                    _=>self.ready(&[Op::Double(a)]).map(|r|r[0])
                }}
            }
        };
        if let Some(id)=result{self.cache.insert(op,id);self.stack.pop();}
        true
    }
    pub fn snapshot(&self)->Value{
        let nodes:Vec<_>=self.nodes.iter().map(|n|match n{
            Node::Zero=>json!(["zero"]),Node::Leaf(c)=>json!(["coefficient",c.is_negative(),c.magnitude().as_word()]),
            Node::Branch{wire,low,high}=>json!(["branch",wire,low,high])}).collect();
        let cache:Vec<_>=self.cache.iter().map(|(op,id)|json!([op.json(),id])).collect();
        json!({"nodes":nodes,"cache":cache,"pending":self.stack.iter().map(|op|op.json()).collect::<Vec<_>>(),"steps":self.steps})
    }
}
