//! Changed-operator control on complete N-only source self-application.
use g_momonados::parasm::SelfModel;
use serde_json::json;
use num_bigint::BigUint;
fn main() {
    let n=include_str!("../measurements/conventional_translation_20261020/n_only/inputs/user_896.txt").trim();
    let number:BigUint=n.parse().unwrap();assert!(number.bits()>=200);
    let forms=g_momonados::numeral_encodings::SourceForms::from_n(&number).unwrap();
    assert_eq!(forms.source_value().unwrap(),number);
    let mut changed=forms.clone();
    changed.hex=g_momonados::numeral_encodings::digit_encode(&(number.clone()+1u8).to_string());
    assert!(changed.source_value().is_err());
    let source=json!({"schema":"conventional-n-only/v1","N":n});
    let bytes=serde_json::to_vec(&source).unwrap();
    let mut model=SelfModel::prepare().unwrap();
    assert!(model.executed_trie_steps>0&&model.program_self_return&&model.contains(&bytes));
    assert_eq!(model.reflect(&bytes),bytes);
    model.backward[model.forward[0]]^=1;
    assert_ne!(model.reflect(&bytes),bytes);
    println!("complete N-only self-application and changed-operator control passed; source bits={}",number.bits());
}
