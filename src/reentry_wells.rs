//! Executing well-space operations corresponding to ReEntryConstructions.lean.
//! Binary atoms generate additive place values. Multiplicative leaves retain
//! their own composition and are not declared prime from their binary shape.
use crate::word_tape::WordTape;
use num_bigint::BigUint;
use serde_json::{json,Value};

#[derive(Clone,Debug,PartialEq,Eq)]
pub struct BinaryWell { digits:Vec<char> }
impl BinaryWell {
    pub fn from_word(word:&WordTape)->Self {Self{digits:word.parity_cells()}}
    pub fn evaluate(&self)->WordTape {
        WordTape::from_parity_cells(&self.digits).expect("binary well retains its parity arms")
    }
    /// binAdd v = 0 :: v. Keep the added cell even when evaluation is zero.
    pub fn reenter(&self)->Self {
        let mut digits=vec!['⊤'];digits.extend_from_slice(&self.digits);Self{digits}
    }
    pub fn depth(&self)->usize {self.digits.iter().filter(|&&cell|cell=='⊥').count()}
    pub fn width(&self)->usize {self.digits.len()}
    pub(crate) fn prepared_digits(&self)->&[char]{&self.digits}
    pub fn record(&self)->Value {json!({"digits_low_first":self.digits.iter().map(|&cell|u8::from(cell=='⊥')).collect::<Vec<_>>(),
        "depth":self.depth(),"evaluation":self.evaluate().to_biguint().to_string()})}
}

#[derive(Clone,Debug,PartialEq,Eq)]
pub struct MultiplicativeWell { leaves:Vec<WordTape> }
impl MultiplicativeWell {
    pub fn singleton(word:WordTape)->Self {Self{leaves:vec![word]}}
    /// primeAdd v w = v ++ w. This does not assert that the leaves are prime.
    pub fn reenter(&self,other:&Self)->Self {
        let mut leaves=self.leaves.clone();leaves.extend_from_slice(&other.leaves);Self{leaves}
    }
    pub fn evaluate(&self)->WordTape {self.leaves.iter().fold(WordTape::from_small(1),|value,leaf|value.mul(leaf))}
    pub fn record(&self)->Value {json!({"leaves":self.leaves.iter().map(|v|v.to_biguint().to_string()).collect::<Vec<_>>(),
        "depth":self.leaves.len(),"evaluation":self.evaluate().to_biguint().to_string(),
        "prime_atomicity_established":false})}
}

#[derive(Clone)]
pub struct SourceWells { source:WordTape, pub binary:BinaryWell, named_product:MultiplicativeWell }
impl SourceWells {
    pub(crate) fn from_baked(source:&'static str,digits:&[char])->Result<Self,String>{
        let source=WordTape::from_compiled_word(source).ok_or("compiled source well lost its word")?;
        Ok(Self{binary:BinaryWell{digits:digits.to_vec()},
            named_product:MultiplicativeWell::singleton(source.clone()),source})
    }
    pub fn new(source:WordTape)->Result<Self,String>{
        if source.is_zero(){return Err("zero has no proper factor boundary".into());}
        let binary=BinaryWell::from_word(&source);
        let named_product=MultiplicativeWell::singleton(source.clone());
        if binary.evaluate()!=source || named_product.evaluate()!=source {
            return Err("source well evaluations diverged".into());
        }
        Ok(Self{source,binary,named_product})
    }
    /// Source-reduced reading of the same binary-well place values.
    pub fn powers_mod_source(&self,count:usize)->Result<Vec<BigUint>,String>{
        self.powers_integer(count)?.into_iter().map(|value|{
            WordTape::from_biguint(&value).divmod(&self.source)
                .map(|(_,remainder)|remainder.to_biguint()).ok_or_else(||"well basis lost its nonzero source".into())
        }).collect()
    }
    /// AFWD consumes these exact place values. Every step performs binary
    /// re-entry before evaluation; source reduction follows the complete rows.
    pub fn powers_integer(&self,count:usize)->Result<Vec<BigUint>,String>{
        let mut well=BinaryWell::from_word(&WordTape::from_small(1));
        let mut powers=Vec::with_capacity(count);
        for _ in 0..count {
            let value=well.evaluate();
            if well.depth()!=1{return Err("binary basis lost its single distinction".into());}
            powers.push(value.to_biguint());well=well.reenter();
        }
        Ok(powers)
    }
    pub fn record(&self)->Value {json!({"definitions":"Imscribing.ReEntryConstructions",
        "binary":self.binary.record(),"named_product":self.named_product.record(),
        "decomposition_complete":false,"shared_evaluation":self.source.to_biguint().to_string(),
        "binary_atomicity_is_arithmetic_atomicity":false})}
    /// Consumes produced legs only after point closure. It never supplies a
    /// leg, factor width, seed or candidate to numerical preparation.
    pub fn close(&self,divisor:&WordTape,quotient:&WordTape)->Result<Value,String>{
        let left=MultiplicativeWell::singleton(divisor.clone());
        let right=MultiplicativeWell::singleton(quotient.clone());
        let product=left.reenter(&right);
        if product.evaluate()!=self.source || self.binary.evaluate()!=self.source {
            return Err("binary and multiplicative re-entry did not close on the same source".into());
        }
        Ok(json!({"binary":self.binary.record(),"multiplicative":product.record(),
            "shared_evaluation":self.source.to_biguint().to_string(),"exact_product":true,
            "decomposition_complete":true,"operation":"primeAdd [d] [q]; primeEval = binEval"}))
    }
}

pub fn source_control(n:BigUint)->Result<Value,String>{
    if n.bits()<200{return Err("well control requires a source of at least 200 bits".into());}
    let word=WordTape::from_biguint(&n);let wells=SourceWells::new(word.clone())?;
    let empty_binary=BinaryWell{digits:Vec::new()};
    let entered_zero=empty_binary.reenter();
    if empty_binary==entered_zero || empty_binary.evaluate()!=entered_zero.evaluate() {
        return Err("zero-valued binary words were collapsed before evaluation".into());
    }
    let empty_product=MultiplicativeWell{leaves:Vec::new()};
    if !empty_binary.evaluate().is_zero() || !empty_product.evaluate().is_one(){return Err("well units were conflated".into());}
    if wells.binary.reenter().evaluate()!=word.shl(1){return Err("binary re-entry did not double the common source".into());}
    let source_pair=wells.named_product.reenter(&wells.named_product);
    if source_pair.evaluate()!=word.mul(&word){return Err("multiplicative concatenation changed its evaluation".into());}
    let powers=wells.powers_mod_source(2*wells.binary.width())?;
    let mut expected=BigUint::from(1u8);
    for power in &powers{
        if power!=&expected{return Err("executing well basis differs from exact binary powers".into());}
        expected=(expected<<1usize)%&n;
    }
    Ok(json!({"N":n.to_string(),"source_bits":n.bits(),"input_boundary":"N-only",
        "source_wells":wells.record(),"binAdd_doubles":true,"primeAdd_multiplies":true,
        "distinct_empty_units":true,"zero_word_distinction":true,"executing_powers_match":true,
        "power_count":powers.len(),"factor_inputs":0}))
}
