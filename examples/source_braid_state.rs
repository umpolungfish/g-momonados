//! N-only file entry for the retained square-winding execution.
use g_momonados::{source_braid_state,word_tape::WordTape};
use num_bigint::BigUint;

fn main()->Result<(),Box<dyn std::error::Error>>{
    let mut args=std::env::args().skip(1);
    let input=args.next().ok_or("expected N-only input file and receipt path")?;
    let output=args.next().ok_or("expected receipt path")?;
    let last_use=match args.next().as_deref(){None=>false,Some("--last-use")=>true,
        _=>return Err("unexpected extra input".into())};
    if args.next().is_some(){return Err("unexpected extra input".into());}
    let raw=std::fs::read_to_string(input)?;
    let source=WordTape::from_canonical_word(raw.trim()).or_else(||
        BigUint::parse_bytes(raw.trim().as_bytes(),10).and_then(|n|
            WordTape::from_canonical_word(&vox_core::combined_factor::native_word(&n))))
        .ok_or("invalid N-only numeral")?;
    let node_budget=std::env::var("SOURCE_BRAID_NODE_BUDGET")
        .map(|s|s.parse::<usize>()).unwrap_or(Ok(8_000_000))?;
    let checkpoint=std::path::Path::new(&output).with_extension("state.json");
    std::env::set_var("SOURCE_BRAID_CHECKPOINT_PATH",checkpoint);
    let result=if last_use{source_braid_state::last_use::execute(source,node_budget)?}
        else{source_braid_state::execute_with_node_budget(source,node_budget)?};
    std::fs::write(output,serde_json::to_vec_pretty(&result)?)?;
    println!("N={} factors={} computation_seconds={}",result["N"],result["factors"],
        result.get("seconds").or_else(||result.get("seconds_before_serialization")).unwrap_or(&serde_json::Value::Null));
    if result["factors"].is_null(){return Err("factor boundary remains open; retained state saved".into());}
    Ok(())
}
