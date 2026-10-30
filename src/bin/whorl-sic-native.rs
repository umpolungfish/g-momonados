//! Executable entry point for crate-local WordTape/SIC transport.
use g_momonados::whorl_sic_native;
fn main() {
    let args:Vec<String>=std::env::args().skip(1).collect();
    if let Err(e)=whorl_sic_native::command_line(&args) {
        eprintln!("ERROR: {e}");
        std::process::exit(2);
    }
}
