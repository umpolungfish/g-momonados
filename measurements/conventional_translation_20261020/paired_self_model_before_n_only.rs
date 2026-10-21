//! Changed-operator control on the actual shared self-hosting implementation.
use g_momonados::{conventional_translation::PairedNumeral,parasm::SelfModel};
use serde_json::Value;
fn main() {
    let value:Value=serde_json::from_str(include_str!("../measurements/conventional_translation_20261020/paired_repl.json")).unwrap();
    let source=PairedNumeral::from_source(&value).unwrap();
    assert!(source.name.bits()>=200);
    let bytes=serde_json::to_vec(&source.source()).unwrap();
    let mut model=SelfModel::prepare().unwrap();
    assert!(model.executed_trie_steps>0&&model.program_self_return&&model.contains(&bytes));
    let returned=model.reflect(&bytes);
    assert_eq!(returned,bytes);
    assert_eq!(PairedNumeral::from_source(&serde_json::from_slice(&returned).unwrap()).unwrap(),source);
    model.backward[model.forward[0]]^=1;
    assert_ne!(model.reflect(&bytes),bytes);
    println!("complete native self-application and changed-operator control passed; source bits={}",source.name.bits());
}
