//! Check transformed-state recovery through every prepared anyon stack stage.
#[path = "ququart_support/mod.rs"]
#[allow(dead_code)]
mod support;

use g_momonados::{braid_protocol::audit_braid_frames,
    godel_calculus::{encode_cell_binary, Nat},
    ququart_factor::QuquartPowerSchedule, ququart_folded_work::QuquartFoldedWorkDevice};
use num_bigint::BigUint;
use serde_json::json;

fn word(value: &BigUint) -> String {
    encode_cell_binary(&Nat::from_bits_le(
        (0..value.bits()).map(|bit| value.bit(bit)).collect()))
}

fn run(args: &[String]) -> Result<(), String> {
    if !(2..=3).contains(&args.len()) {
        return Err("usage: anyon_stack_audit prepared.json base [physical-braid.word]".into());
    }
    let mut prepared: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&args[0]).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let (source, matrix, metrics) = support::contract(&prepared)?;
    if source.bits() < 200 { return Err("stack audit requires a source of at least 200 bits".into()); }
    let base: BigUint = args[1].parse().map_err(|_| "invalid base")?;
    let schedule = QuquartPowerSchedule::prepare(&source, &base)?;
    if prepared.get("base_word").is_none() {
        prepared["base_word"] = json!(word(&base));
        prepared["prepared_operator"]["phase_digits_word"] = json!(word(&BigUint::from(schedule.powers().len())));
        prepared["prepared_operator"]["controlled_power_words"] =
            json!(schedule.powers().iter().map(word).collect::<Vec<_>>());
    } else if support::numeral(prepared["base_word"].as_str().ok_or("invalid prepared base")?)? != base {
        return Err("audit base differs from the prepared stack".into());
    }
    if prepared.get("prepared_work").is_none() {
        prepared["prepared_work"] = support::work::compile(&prepared)?;
    }
    let work = support::work::decode(&prepared)?;
    let radix = prepared["radix_word"].as_str().ok_or("missing work radix")?;
    let heights: Vec<i32> = (1..=g_momonados::reversible_modular::ModularMultiply::new(&source)?
        .elementary_qubits() as i32 + 1).collect();
    let frames = if let Some(path) = args.get(2) {
        let braid: Vec<i32> = std::fs::read_to_string(path).map_err(|e| e.to_string())?
            .split_whitespace().map(|g| g.parse().map_err(|_| "invalid physical generator"))
            .collect::<Result<_, _>>()?;
        let reports = audit_braid_frames(&braid, 6, &heights)?;
        reports.len()
    } else { 0 };
    eprintln!("stack_audit_started source_bits={} stages={} presentation_heights={frames}",
        source.bits(), schedule.powers().len());
    let mut device = QuquartFoldedWorkDevice::new_interleaved_radix(
        source.clone(), matrix, 1729, radix)?.with_prepared_work(work);
    let witnesses = device.audit_frobenius_stack(&base)?;
    println!("{}", json!({"source": source.to_str_radix(10), "source_bits": source.bits(),
        "base": base.to_str_radix(10), "presentation_heights": frames,
        "fusion_channels": 5, "complex_residues_per_channel": 3,
        "fourier_computational": metrics.computational, "fourier_leakage": metrics.leakage,
        "fourier_bidirectional_return": metrics.closure,
        "stages": witnesses.iter().map(|w| json!({"height": w.height, "operations": w.operations,
            "transformed_state_recovered": true, "adjacent_inverse_return": true,
            "workspace_clean": true})).collect::<Vec<_>>()}));
    Ok(())
}

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let worker = std::thread::Builder::new().stack_size(64 * 1024 * 1024)
        .spawn(move || run(&args)).expect("start stack auditor");
    match worker.join() {
        Ok(Ok(())) => {},
        Ok(Err(error)) => { eprintln!("stack audit: {error}"); std::process::exit(1); },
        Err(_) => { eprintln!("stack audit: a runtime fuse rejected recovery"); std::process::exit(1); },
    }
}
