//! Source-bound Fourier membrane with one terminal readout.
#[path = "ququart_support/mod.rs"]
mod support;
include!(concat!(env!("OUT_DIR"), "/ququart_prepared.rs"));

fn execute() -> Result<String, String> {
    let prepared: serde_json::Value = serde_json::from_str(PREPARED).map_err(|e| e.to_string())?;
    if prepared.get("prepared_operator").is_none() {
        return Err("baked membrane requires a prepared operator in IMASM words".into());
    }
    let (n, _, m) = support::contract(&prepared)?;
    Ok(format!("completed ququart Fourier membrane\nsource_bits={} exchanges={}\ncomputational={:.8e} leakage={:.8e} return={:.8e}\n", n.bits(), m.exchanges, m.computational, m.leakage, m.closure))
}

fn main() {
    // No progress stream, socket, runtime input, or intermediate file writes.
    let result = execute();
    use std::io::Write;
    match result {
        Ok(report) => {
            if std::io::stdout().lock().write_all(report.as_bytes()).is_err() {
                std::process::exit(1);
            }
        }
        Err(error) => {
            let report = format!("completed with error: {error}\n");
            let _ = std::io::stderr().lock().write_all(report.as_bytes());
            std::process::exit(1);
        }
    }
}
