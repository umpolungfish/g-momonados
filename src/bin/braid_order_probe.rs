//! Execute a selected braid-protocol order through IMASM lowering and readout.

use g_momonados::braid_protocol::{
    braid_to_imasm_selected, read_tangle, recover_ordered_generators,
};

fn main() {
    vox_core::ordering::cli::intercept();
    let generators: Vec<i32> = std::env::args().skip(1)
        .map(|arg| arg.parse().map_err(|_| format!("invalid generator: {arg}")))
        .collect::<Result<_, _>>()
        .unwrap_or_else(|error| { eprintln!("{error}"); std::process::exit(2); });
    if generators.is_empty() {
        eprintln!("usage: braid_order_probe [--ordering ORDER] [--order-site braid-protocol] <signed-generators...>");
        std::process::exit(2);
    }
    let result = (|| -> Result<(), String> {
        let order = vox_core::ordering::selected("braid-protocol", generators.len())?;
        let ordered = order.apply(&generators)?;
        let program = braid_to_imasm_selected(&generators, 1, true)?;
        let readout = read_tangle(&program, generators.len() + 2, 1)?;
        let returned = recover_ordered_generators(&readout.generators)?;
        let verified = readout.closes && readout.generators == ordered && returned == generators;
        println!("order={} inverse={} physical={:?} returned={:?} closes={} return_verified={}",
            order.label(), order.inverse().label(), readout.generators, returned, readout.closes, verified);
        if !verified { return Err("selected braid did not return through its inverse order".into()); }
        Ok(())
    })();
    if let Err(error) = result { eprintln!("{error}"); std::process::exit(1); }
}
