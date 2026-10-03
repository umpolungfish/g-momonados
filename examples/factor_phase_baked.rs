//! Silent prepared coherent pair measurement using the kernel interface.
#![deny(warnings)]
fn main() {
    let output=g_momonados::factor_phase::execute_baked()
        .expect("prepared coherent factor measurement");
    use std::io::{self,Write};
    io::stdout().lock().write_all(output.as_bytes()).unwrap();
}
