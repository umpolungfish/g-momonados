//! N-only executable for the exact existential proper-axis reader.
use g_momonados::{source_braid_state::existential,word_tape::WordTape};
fn main()->Result<(),Box<dyn std::error::Error>>{
    let mut args=std::env::args().skip(1);
    let path=args.next().ok_or("N-only source path")?;let output=args.next().ok_or("receipt path")?;
    let raw=std::fs::read_to_string(path)?;
    let bits=vox_core::morphism_factor::decimal_to_tape(raw.trim()).ok_or("invalid decimal N")?;
    let source=WordTape::from_imasm_tape(&bits).ok_or("invalid source")?;
    let seconds=std::env::var("AXIS_READER_SECONDS").unwrap_or("24".into()).parse()?;
    let nodes=std::env::var("AXIS_READER_NODES").unwrap_or("4000000".into()).parse()?;
    std::env::set_var("AXIS_READER_CHECKPOINT",std::path::Path::new(&output).with_extension("state.json"));
    let result=existential::select(source,seconds,nodes)?;
    std::fs::write(output,serde_json::to_vec(&result)?)?;
    println!("status={} eliminated={} nodes={} factors={}",result["status"],result["eliminated"],result["reader_nodes"],result["factors"]);
    if result["factors"].is_null(){std::process::exit(3);}Ok(())
}
