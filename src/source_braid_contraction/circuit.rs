//! N-only, acyclic native-cell circuit for the joint square-return relation.
use std::collections::HashMap;
use std::io::{Write,BufWriter};
use std::path::Path;
use crate::word_tape::WordTape;

#[derive(Clone,Copy,Debug,PartialEq,Eq,Hash)]
pub struct Wire(pub usize);
impl Wire {
    pub fn variable(self)->Option<usize>{self.0.checked_sub(2)}
    fn constant(self)->Option<bool>{if self.0<2 {Some(self.0==1)}else{None}}
}
#[derive(Clone,Copy,Debug)]
pub struct Gate {pub a:Wire,pub b:Wire,pub out:usize}
pub struct Circuit {
    pub bits:usize,pub variables:usize,pub gates:Vec<Gate>,pub output:Wire,
    pub elimination:Vec<(usize,usize)>,
}
struct Builder {next:usize,gates:Vec<Gate>,cache:HashMap<(usize,usize),Wire>}
impl Builder {
    fn nand(&mut self,a:Wire,b:Wire)->Wire {
        if a.constant()==Some(false)||b.constant()==Some(false){return Wire(1);}
        if let (Some(x),Some(y))=(a.constant(),b.constant()){return Wire(usize::from(!(x&&y)));}
        let key=if a.0<=b.0{(a.0,b.0)}else{(b.0,a.0)};
        if let Some(&w)=self.cache.get(&key){return w;}
        let out=self.next;self.next+=1;let w=Wire(out+2);
        self.gates.push(Gate{a:Wire(key.0),b:Wire(key.1),out});self.cache.insert(key,w);w
    }
    fn not(&mut self,a:Wire)->Wire{self.nand(a,a)}
    fn and(&mut self,a:Wire,b:Wire)->Wire{if a==b{return a;}let p=self.nand(a,b);self.not(p)}
    fn or(&mut self,a:Wire,b:Wire)->Wire{let x=self.not(a);let y=self.not(b);self.nand(x,y)}
    fn full_add(&mut self,a:Wire,b:Wire,c:Wire)->(Wire,Wire){
        let p=self.nand(a,b);let q=self.nand(a,p);let r=self.nand(b,p);let t=self.nand(q,r);
        let u=self.nand(t,c);let v=self.nand(t,u);let w=self.nand(c,u);
        (self.nand(v,w),self.nand(p,u))
    }
    fn add(&mut self,a:&[Wire],b:&[Wire],width:usize,mut carry:Wire)->(Vec<Wire>,Wire){
        let mut out=Vec::with_capacity(width);
        for i in 0..width {let(s,c)=self.full_add(*a.get(i).unwrap_or(&Wire(0)),*b.get(i).unwrap_or(&Wire(0)),carry);out.push(s);carry=c;}
        (out,carry)
    }
    fn sub(&mut self,a:&[Wire],b:&[Wire],width:usize)->(Vec<Wire>,Wire){
        let inverted:Vec<_>=(0..width).map(|i|self.not(*b.get(i).unwrap_or(&Wire(0)))).collect();
        self.add(a,&inverted,width,Wire(1))
    }
    fn select(&mut self,c:Wire,yes:Wire,no:Wire)->Wire{
        if yes==no{return yes;}if let Some(b)=c.constant(){return if b{yes}else{no};}
        let nc=self.not(c);let a=self.nand(c,yes);let b=self.nand(nc,no);self.nand(a,b)
    }
    fn square(&mut self,x:&[Wire])->Vec<Wire>{
        let width=2*x.len();let mut acc=vec![Wire(0);width];
        for i in 0..x.len(){
            let mut row=vec![Wire(0);width];
            for j in 0..x.len(){row[i+j]=self.and(x[i],x[j]);}
            acc=self.add(&acc,&row,width,Wire(0)).0;
        }acc
    }
    fn reduce(&mut self,product:&[Wire],n:&[Wire])->Vec<Wire>{
        let mut r=vec![Wire(0);n.len()];
        for &bit in product.iter().rev(){
            let mut t=vec![bit];t.extend_from_slice(&r);
            let(difference,no_borrow)=self.sub(&t,n,n.len()+1);
            r=(0..n.len()).map(|i|self.select(no_borrow,difference[i],t[i])).collect();
        }r
    }
    fn all(&mut self,items:impl IntoIterator<Item=Wire>)->Wire{items.into_iter().fold(Wire(1),|a,b|self.and(a,b))}
    fn sic_pair(&mut self,a:[Wire;2],b:[Wire;2],kernel:&[[bool;4];4])->Wire{
        let wires=[a[0],a[1],b[0],b[1]];let mut terms=Vec::new();
        for u in 0..4 {for v in 0..4 {if kernel[u][v]{
            let label=u|(v<<2);
            let literals:Vec<_>=wires.iter().enumerate().map(|(i,&w)|if label&(1<<i)!=0{w}else{self.not(w)}).collect();
            terms.push(self.all(literals));
        }}}
        terms.into_iter().fold(Wire(0),|a,b|self.or(a,b))
    }
}
impl Circuit {
    /// Exact-product slice of the nonzero zero-product family. Widths follow
    /// from N: one arm is at most sqrt(N), the other at most N/2. No instance
    /// factors, orders or externally supplied widths enter this construction.
    pub fn prepare_proper_axis(n:&WordTape)->Result<(Self,usize,usize),String>{
        let bits=n.bit_len();
        if bits<3{return Err("proper-axis reader requires N >= 4".into());}
        let aw=bits.div_ceil(2);let bw=bits-1;let width=aw+bw;
        let mut b=Builder{next:width,gates:Vec::new(),cache:HashMap::new()};
        let x:Vec<_>=(0..aw).map(|i|Wire(i+2)).collect();
        let y:Vec<_>=(0..bw).map(|i|Wire(aw+i+2)).collect();
        let mut product=vec![Wire(0);width];
        for (i,&a) in x.iter().enumerate(){
            let mut row=vec![Wire(0);width];
            for (j,&c) in y.iter().enumerate(){row[i+j]=b.and(a,c);}
            product=b.add(&product,&row,width,Wire(0)).0;
        }
        let mut predicates=Vec::new();
        for (i,&w) in product.iter().enumerate(){predicates.push(if n.bit_at(i){w}else{b.not(w)});}
        let nontrivial_x=x.iter().skip(1).copied().fold(Wire(0),|a,c|b.or(a,c));
        let nontrivial_y=y.iter().skip(1).copied().fold(Wire(0),|a,c|b.or(a,c));
        predicates.extend([nontrivial_x,nontrivial_y]);
        let output=b.all(predicates);let variables=b.next;
        Ok((Self{bits,variables,gates:b.gates,output,elimination:Vec::new()},aw,bw))
    }
    pub fn prepare(n:&WordTape,kernel:&[[bool;4];4])->Result<Self,String>{
        let bits=n.bit_len();if bits<2{return Err("source circuit requires a positive non-unit numeral".into());}
        let mut b=Builder{next:2*bits,gates:Vec::new(),cache:HashMap::new()};
        let x:Vec<_>=(0..bits).map(|i|Wire(i+2)).collect();let y:Vec<_>=(0..bits).map(|i|Wire(bits+i+2)).collect();
        let source:Vec<_>=(0..bits).map(|i|Wire(usize::from(n.bit_at(i)))).collect();
        let xcarry=b.sub(&x,&source,bits).1;let valid_x=b.not(xcarry);
        let ycarry=b.sub(&y,&source,bits).1;let valid_y=b.not(ycarry);
        let xx=b.square(&x);let rx=b.reduce(&xx,&source);
        let yy=b.square(&y);let ry=b.reduce(&yy,&source);
        let mut predicates=vec![valid_x,valid_y];
        for i in (0..bits).step_by(2){
            predicates.push(b.sic_pair([rx[i],*rx.get(i+1).unwrap_or(&Wire(0))],
                [ry[i],*ry.get(i+1).unwrap_or(&Wire(0))],kernel));
        }
        let output=b.all(predicates);let variables=b.next;
        let mut last=vec![0;variables];
        for (i,g) in b.gates.iter().enumerate(){
            for w in [g.a,g.b,Wire(g.out+2)]{if let Some(v)=w.variable(){last[v]=i;}}
        }
        if let Some(v)=output.variable(){last[v]=b.gates.len();}
        let mut elimination:Vec<_>=last.into_iter().enumerate().map(|(v,i)|(i,v)).collect();elimination.sort_unstable();
        Ok(Self{bits,variables,gates:b.gates,output,elimination})
    }
    pub fn write(&self,path:&Path)->Result<(),String>{
        let mut f=BufWriter::new(std::fs::File::create(path).map_err(|e|e.to_string())?);
        f.write_all(b"BSC1").map_err(|e|e.to_string())?;
        for x in [self.bits,self.variables,self.gates.len(),self.output.0]{f.write_all(&(x as u64).to_le_bytes()).map_err(|e|e.to_string())?;}
        for g in &self.gates {for x in [g.a.0,g.b.0,g.out]{f.write_all(&(x as u64).to_le_bytes()).map_err(|e|e.to_string())?;}}
        f.flush().map_err(|e|e.to_string())
    }
    #[cfg(test)]
    pub fn evaluate(&self,x:&WordTape,y:&WordTape)->bool{
        let mut values=vec![false;self.variables];
        for i in 0..self.bits{values[i]=x.bit_at(i);values[self.bits+i]=y.bit_at(i);}
        let read=|w:Wire,v:&[bool]|w.variable().map_or(w.0==1,|i|v[i]);
        for g in &self.gates{values[g.out]=!(read(g.a,&values)&&read(g.b,&values));}
        read(self.output,&values)
    }
}
