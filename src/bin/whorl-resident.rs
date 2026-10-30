//! Real resident DAG capture CLI.
use g_momonados::whorl_resident_bridge;
fn main() {
    let args:Vec<String>=std::env::args().skip(1).collect();
    if let Err(e)=whorl_resident_bridge::command_line(&args){
        eprintln!("ERROR: {e}");std::process::exit(2);
    }
}
