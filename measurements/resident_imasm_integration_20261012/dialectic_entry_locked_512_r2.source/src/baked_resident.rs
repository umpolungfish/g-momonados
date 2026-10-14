//! Cargo's typed transport of the existing prepared word and register.
//! Pool coordinates address instructions; values come from the N-only build.
use alloc::{string::String,vec::Vec};
use num_bigint::{BigInt,BigUint,Sign};
use crate::{word_tape::{WordTape,SignedWordTape},phase_unbraid::FixedComplex};

#[derive(Clone,Copy)]
pub struct BakedInteger {pub negative:bool,pub digits:&'static[u32]}
impl BakedInteger {
    pub fn value(self)->BigInt {
        BigInt::from_biguint(if self.negative {Sign::Minus} else {Sign::Plus},BigUint::new(self.digits.to_vec()))
    }
}
#[derive(Clone,Copy)]
pub struct BakedComplex {pub re:BakedInteger,pub im:BakedInteger}
impl BakedComplex {
    pub fn value(self)->FixedComplex {FixedComplex {re:self.re.value(),im:self.im.value()}}
}
#[derive(Clone,Copy)]
pub struct BakedSignedWord {pub negative:bool,pub word:&'static str}
impl BakedSignedWord {
    pub(crate) fn value(self)->Result<SignedWordTape,String> {
        Ok(SignedWordTape::from_magnitude(WordTape::from_compiled_word(self.word)
            .ok_or("compiled coefficient lost its canonical word")?,self.negative))
    }
}
#[derive(Clone,Copy)]
pub struct BakedWordComplex {pub re:BakedSignedWord,pub im:BakedSignedWord}
#[derive(Clone,Copy)]
pub enum BakedNode {
    Leaf(BakedWordComplex),
    Branch {wire:usize,low:usize,high:usize},
}
pub struct BakedSupport {
    pub limbs: &'static [u32],
    pub spans: &'static [(usize,usize)],
    pub fixed: &'static [(bool, usize, usize)],
}
#[derive(Clone,Copy)]
pub enum BakedOperation {
    Toggle {target:usize,controls:usize},
    ModularAdd {register:usize,digit:usize,value:usize,controls:usize},
}
pub struct BakedWork {
    pub source:&'static[u32],
    pub digit_bits:usize,
    pub values:&'static[&'static[u32]],
    pub wires:&'static[&'static[usize]],
    pub controls:&'static[&'static[(usize,bool)]],
    pub operations:&'static[BakedOperation],
    pub stages:&'static[(usize,&'static[usize])],
    pub lane_offsets:&'static[usize],
}
impl BakedWork {
    /// Decode one already prepared instruction without rebuilding its pool.
    pub fn operation(&self,id:usize)->Result<crate::reversible_modular::NestedOperation,String> {
        use crate::reversible_modular::{NestedOperation,ControlledX};
        let wires=|id:usize|self.wires.get(id).map(|v|v.to_vec()).ok_or_else(||String::from("compiled wire reference escaped its word"));
        let controls=|id:usize|self.controls.get(id).map(|v|v.to_vec()).ok_or_else(||String::from("compiled control reference escaped its word"));
        Ok(match *self.operations.get(id).ok_or("compiled instruction escaped its pool")? {
            BakedOperation::Toggle {target,controls:id}=>NestedOperation::Toggle(ControlledX {target,controls:controls(id)?}),
            BakedOperation::ModularAdd {register,digit,value,controls:id}=>NestedOperation::ModularAdd {
                register:wires(register)?,digit:wires(digit)?,
                value:BigUint::new(self.values.get(value).ok_or("compiled value escaped its source")?.to_vec()),
                modulus:BigUint::new(self.source.to_vec()),controls:controls(id)?,
            },
        })
    }
    pub fn lane_offsets(&self)->Result<Vec<usize>,String> {
        if self.lane_offsets.len()!=self.stages.len() {return Err(String::from("compiled lane partition lost its stack height"));}
        self.stages.iter().zip(self.lane_offsets).map(|(&(value,refs),&offset)| {
            if value>=self.values.len() {return Err(String::from("compiled stage value escaped its source"));}
            if offset>refs.len() {return Err(String::from("compiled lane partition escaped its word"));}
            // Cargo certifies every reference. Recheck the lane seams at import;
            // each executing instruction also checks all of its operands.
            for position in [0,offset,offset.saturating_sub(1),refs.len().saturating_sub(1)] {
                let Some(&id)=refs.get(position) else {continue;};
                let controls=match *self.operations.get(id).ok_or("compiled instruction escaped its pool")? {
                    BakedOperation::Toggle {controls,..}=>controls,
                    BakedOperation::ModularAdd {register,digit,value,controls}=>{
                        if register>=self.wires.len() || digit>=self.wires.len() || value>=self.values.len() {
                            return Err(String::from("compiled operand escaped its pool"));
                        }
                        controls
                    }
                };
                let first=self.controls.get(controls).ok_or("compiled control escaped its pool")?.iter().any(|&(wire,_)|wire==0);
                if first!=(position<offset) {return Err(String::from("compiled lanes changed their prepared order"));}
            }
            Ok(offset)
        }).collect()
    }
    pub fn recover(&self)->Result<crate::ququart_folded_work::PreparedModularWork,String> {
        use crate::reversible_modular::{NestedOperation,ControlledX};
        let source=BigUint::new(self.source.to_vec());
        let values:Vec<_>=self.values.iter().map(|digits| BigUint::new(digits.to_vec())).collect();
        let wires=|id:usize| self.wires.get(id).map(|v|v.to_vec()).ok_or_else(||String::from("compiled wire reference escaped its word"));
        let controls=|id:usize| self.controls.get(id).map(|v|v.to_vec()).ok_or_else(||String::from("compiled control reference escaped its word"));
        let value=|id:usize| values.get(id).cloned().ok_or_else(||String::from("compiled value reference escaped its source"));
        let operations=self.operations.iter().map(|op| Ok(match *op {
            BakedOperation::Toggle {target,controls:id}=>NestedOperation::Toggle(ControlledX {target,controls:controls(id)?}),
            BakedOperation::ModularAdd {register,digit,value:id,controls:c}=>NestedOperation::ModularAdd {
                register:wires(register)?,digit:wires(digit)?,value:value(id)?,modulus:source.clone(),controls:controls(c)?,
            },
        })).collect::<Result<Vec<_>,String>>()?;
        let stages=self.stages.iter().map(|&(id,refs)| {
            if refs.iter().any(|&id|id>=operations.len()) {return Err(String::from("compiled instruction reference escaped its pool"));}
            Ok((value(id)?,refs.to_vec()))
        }).collect::<Result<Vec<_>,String>>()?;
        Ok(crate::ququart_folded_work::PreparedModularWork {source,digit_bits:self.digit_bits,operations,stages})
    }
}
pub struct BakedSic {
    pub modulus_bits:u64,
    pub rays:&'static[[BakedComplex;4];16],
    pub projectors:&'static[[BakedWordComplex;16];16],
}
pub struct BakedPhase {
    pub source_word:&'static str,
    pub base_word:&'static str,
    pub width:usize,
    pub modulus_bits:u64,
    pub work_wires:&'static[usize],
    pub hadamard:BakedComplex,
    pub fourier:&'static[BakedComplex],
    pub cells:usize,
    pub root:usize,
    pub nodes:&'static[BakedNode],
    pub support:&'static BakedSupport,
    pub work:&'static BakedWork,
}
pub struct BakedResident {
    pub source_word:&'static str,
    pub initial_mass_word:&'static str,
    pub wire_map:&'static[usize],
    pub constraint_gates:usize,
    pub product_closure_prepared:bool,
    pub phase_source_relation:bool,
    pub sic_modulus_bits:u64,
    pub sic:&'static BakedSic,
    pub phase:&'static BakedPhase,
    pub reentry_json:&'static str,
    pub preparation_sic_json:&'static str,
    pub preparation_nanoseconds:u64,
}
