//! N-only quotient and constrained idempotent locus for the SRFP/DNN carrier.
//! E ranges over residues of Z/N satisfying E²=E. A function involving E
//! is not yet an evaluated point of Spec(Z/N).
use num_bigint::BigUint;
use num_traits::{One, Zero};
use serde_json::{json,Value};
use vox_core::resident_y::ResidentY;
use crate::{conventional_translation::BinaryTree,numeral_encodings::SourceForms};

#[derive(Clone,Debug,PartialEq,Eq,PartialOrd,Ord)]
struct Function { constant:BigUint, linear:BigUint }
impl Function {
    fn add(&self,rhs:&Self,n:&BigUint)->Self {
        Self{constant:(&self.constant+&rhs.constant)%n,linear:(&self.linear+&rhs.linear)%n}
    }
    fn multiply(&self,rhs:&Self,n:&BigUint)->Self {
        Self{constant:(&self.constant*&rhs.constant)%n,
            linear:(&self.constant*&rhs.linear+&self.linear*&rhs.constant+&self.linear*&rhs.linear)%n}
    }
    fn json(&self)->Value {json!({"constant":self.constant.to_string(),"E":self.linear.to_string()})}
}
#[derive(Clone,Debug,PartialEq,Eq,PartialOrd,Ord)]
struct QuotientCarrier {
    modulus:BigUint,
    source_forms:SourceForms,
    left:Function,
    right:Function,
    fused:Function,
}
impl QuotientCarrier {
    fn new(n:BigUint)->Result<Self,String> {
        let source_forms=SourceForms::from_n(&n)?;
        Ok(Self{source_forms,left:Function{constant:BigUint::zero(),linear:BigUint::one()},
            right:Function{constant:BigUint::one(),linear:&n-BigUint::one()},
            fused:Function{constant:BigUint::zero(),linear:BigUint::zero()},modulus:n})
    }
    fn reenter(mut self)->Result<Self,String> {
        if self.source_forms.source_value()?!=self.modulus {return Err("re-entry changed the source numeral".into());}
        // Both legs reference ONE constrained E, never independent producers.
        self.left=self.left.multiply(&self.left,&self.modulus);
        self.right=self.right.multiply(&self.right,&self.modulus);
        self.fused=self.left.add(&self.right,&self.modulus);
        let zero=Function{constant:BigUint::zero(),linear:BigUint::zero()};
        let one=Function{constant:BigUint::one(),linear:BigUint::zero()};
        if self.left.multiply(&self.right,&self.modulus)!=zero || self.fused!=one {
            return Err("correlated idempotent pairing lost its quotient relation".into());
        }
        Ok(self)
    }
}

/// The only assessed value is N. Normalization cannot stand in for the
/// evaluation of the nontrivial residue that specifies the two points.
pub fn execute(kind:&str,n:BigUint)->Result<Value,String> {
    if n<=BigUint::one(){return Err("SRFP/DNN proper-factor input must exceed one".into());}
    let started=std::time::Instant::now();
    let binary=BinaryTree::from_value(&n);
    if binary.evaluate()!=n{return Err("binary source return changed N".into());}
    let source=json!({"schema":"conventional-n-only/v1","N":n.to_string()});
    let model=crate::parasm::SelfModel::prepare()?;
    let bytes=serde_json::to_vec(&source).map_err(|e|e.to_string())?;
    if model.reflect(&bytes)!=bytes{return Err("native source self-model did not return N".into());}
    let mut carrier=QuotientCarrier::new(n.clone())?;
    let mut y=ResidentY::from_certified_program(ResidentY::certified_program())?;
    let mut normalized=false;
    // First incorporate the paired return, then observe stability. The Y
    // continuation remains open until point evaluation emits actual factors.
    for _ in 0..2 {
        y.advance(|| {
            let before=carrier.clone();
            carrier=crate::reentry_powerset::bind_one(carrier.clone(),QuotientCarrier::reenter)?;
            normalized=carrier==before;
            Ok(false)
        })?;
    }
    let structures=if kind=="both"{vec!["srfp","dnn"]}else{vec![kind]};
    let reports=structures.into_iter().map(|s|json!({"structure":s,
        "status":"held","all_predicates_hold":false,"reading":null,
        "source_self_return":true,"pairing_normal_form_stable":normalized,
        "factor_product_verified":false})).collect::<Vec<_>>();
    Ok(json!({"schema":"conventional-n-only-return/v1","source":source,"input_boundary":"N-only",
        "status":"held","reports":reports,"factors":null,"product_verified":false,
        "numeral_forms":{"native_word":carrier.source_forms.native,"hex_digit_word":carrier.source_forms.hex,
            "native_evaluation":carrier.source_forms.source_value()?.to_string(),
            "hex_evaluation":crate::numeral_encodings::digit_decode(&carrier.source_forms.hex).unwrap().to_string(),
            "encoder_source":"shared with tfactor read","retained_through_Y":true},
        "binary_tree":{"positions":binary.positions,"evaluation":binary.evaluate().to_string()},
        "multiplicative_tree":{"operator":"proper-product-relation","equation":"P*Q=N",
            "left":{"sort":"left-ideal-generator","value":null},
            "right":{"sort":"right-ideal-generator","value":null},
            "evaluation_constraint":n.to_string(),"evaluated":false},
        "quotient":{"modulus":n.to_string(),"ring":"Z/N","point_constraint":"E*E=E; E!=0; E!=1",
            "symbol_scope":"constrained residue in Z/N, not an evaluated residue",
            "left_projection":carrier.left.json(),"right_projection":carrier.right.json(),
            "fused_projection":carrier.fused.json(),"shared_point_symbol":true,
            "point_evaluated":false},
        "self_model":{"source_bytes":bytes.len(),"program_self_return":model.program_self_return,
            "executed_trie_steps":model.executed_trie_steps},
        "Y":{"operator_calls":y.calls,"contractions":y.contractions,"continuation":"retained open"},
        "next_operation":"evaluate the nontrivial idempotent in Z/N before reading the two ideal generators",
        "trigger_to_held_return_seconds":started.elapsed().as_secs_f64()}))
}
