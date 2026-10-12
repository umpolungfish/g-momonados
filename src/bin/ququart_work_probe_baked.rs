//! Source-only, baked probe of one phase digit on the full shared work register.
//! The existing native Fourier target is used to measure register scaling.
use g_momonados::{anyon_pair::{FibonacciPair, PairMatrix, COMPUTATIONAL_CHANNELS},
    anyon_ququart::{QuquartCarrier, QuquartDigit, QuquartSicOutcome}, ququart_factor::QuquartPhaseDevice,
    ququart_folded_work::QuquartFoldedWorkDevice, ququart_reentry_clock::RuntimeClock,
    godel_calculus::decode_cell_biguint};
use num_traits::{ToPrimitive, Zero};
include!(concat!(env!("OUT_DIR"), "/ququart_prepared.rs"));

fn execute() -> Result<serde_json::Value, String> {
    if std::env::args_os().len() != 1 { return Err("work probe accepts no runtime inputs".into()); }
    let prepared: serde_json::Value = serde_json::from_str(PREPARED).map_err(|e| e.to_string())?;
    let read = |key: &str| decode_cell_biguint(prepared[key].as_str().ok_or_else(|| format!("missing {key}"))?);
    let source = read("source_word")?;
    if source.bits() < 128 { return Err("work probe source must be at least 128 bits".into()); }
    let base = read("base_word")?;
    let seed = read("seed_word")?.to_u64().ok_or("invalid baked entropy seed")?;
    let algebra = FibonacciPair::new(&source)?;
    let mut fourier = PairMatrix::identity(algebra.format());
    for digit in 0..4 {
        let mut column = QuquartCarrier::basis(&algebra, QuquartDigit::try_from(digit as u8)?);
        column.fourier_target(false);
        for row in 0..5 { fourier.0[5 * row + COMPUTATIONAL_CHANNELS[digit]] = column.amplitudes()[row].clone(); }
    }
    let clock = RuntimeClock::from_record(&prepared["phase_clock"])?;
    if clock.retained() != 0 || clock.read(0) != 1 || clock.period() != 2 {
        return Err("phase probe requires its certified alternating Fourier clock".into());
    }
    let mut logical = clock.read(0);
    let mut device = QuquartFoldedWorkDevice::new_interleaved_radix(source.clone(), fourier, seed,
        prepared["radix_word"].as_str().ok_or("missing work radix")?)?;
    device.begin(&source, &base, 1)?;
    let mut clock_ticks = Vec::new();
    logical = clock.step(logical);
    if logical != clock.read(1) { return Err("first executed phase differs from clock".into()); }
    device.fourier(logical & 1 != 0)?;
    clock_ticks.push(logical);
    device.controlled_multiply(&base)?;
    logical = clock.step(logical);
    if logical != clock.read(2) { return Err("second executed phase differs from clock".into()); }
    device.fourier(logical & 1 != 0)?;
    clock_ticks.push(logical);
    let digit = device.measure_phase_digit()?;
    let witness = device.sic_witnesses().last().ok_or("missing full Gram witness")?.clone();
    let retained = u8::from(!witness.masses[0].is_zero()) | (u8::from(!witness.masses[1].is_zero()) << 3);
    let certificate = prepared["readout_clocks"].as_array().ok_or("missing measured readout clocks")?
        .iter().find(|c| c["retained"].as_u64() == Some(retained as u64) && c["seed"].as_u64() == Some(1))
        .ok_or("measured evidence has no baked clock")?;
    let readout_clock = RuntimeClock::from_record(certificate)?;
    let peak_nodes = device.peak_nodes;
    device.reset_control(digit)?;
    device.finish()?;

    // Carry the measured SIC coordinate support through the retained-9 clock.
    // The four nonzero coordinate labels should remain distinct and select all
    // four two-call Fourier orientation pairs via output bits 1 and 2.
    let mut coordinate_labels = witness.gram.iter().enumerate()
        .filter_map(|(label, (re, im))| (!re.is_zero() || !im.is_zero()).then_some(label as u8))
        .collect::<Vec<_>>();
    coordinate_labels.sort_unstable();
    let mut support_clock_cases = Vec::with_capacity(coordinate_labels.len());
    for label in coordinate_labels {
        let certificate = prepared["readout_clocks"].as_array().ok_or("missing measured readout clocks")?
            .iter().find(|c| c["retained"].as_u64() == Some(9) && c["seed"].as_u64() == Some(label as u64))
            .ok_or_else(|| format!("coordinate label {label} has no retained-9 clock"))?;
        let clock = RuntimeClock::from_record(certificate)?;
        let first_tick = clock.step(label);
        let second_tick = clock.step(first_tick);
        if first_tick != clock.read(1) || second_tick != clock.read(2) {
            return Err(format!("coordinate label {label} differs from certified clock ticks"));
        }
        device.begin(&source, &base, 1)?;
        let first_inverse = first_tick & 2 != 0;
        device.fourier(first_inverse)?;
        device.controlled_multiply(&base)?;
        let second_inverse = second_tick & 4 != 0;
        device.fourier(second_inverse)?;
        let measured_digit = device.measure_phase_digit()?;
        device.reset_control(measured_digit)?;
        device.finish()?;
        support_clock_cases.push(serde_json::json!({
            "coordinate_label": label,
            "clock_ticks": [first_tick, second_tick],
            "fourier_inverse": [first_inverse, second_inverse],
            "digit": measured_digit as u8,
            "clock_entry": clock.entry(),
            "clock_period": clock.period(),
        }));
    }

    // A second source-bound shot exercises the projector-outcome path itself.
    // Its Gram and measured masses are certified before the SIC collapse.
    device.begin(&source, &base, 1)?;
    device.fourier(false)?;
    device.controlled_multiply(&base)?;
    device.fourier(true)?;
    let sic_outcome = device.measure_sic_outcome()?;
    let sic_witness = device.sic_outcome_witnesses().last()
        .ok_or("missing SIC outcome reconstruction witness")?.clone();
    device.finish()?;
    let sic_mask = match sic_outcome {
        QuquartSicOutcome::Carrier(mask) => Some(mask.mask()),
        QuquartSicOutcome::OutsideCarrier => None,
    };
    Ok(serde_json::json!({"source_word":prepared["source_word"],"source_bits":source.bits(),
        "work_radix_word":prepared["radix_word"],"fixed_point_bits":algebra.format().w_bits,
        "backend":"folded-shared-work","fourier":"native-fourier-target",
        "phase_digits_measured":1,"digit":digit as u8,"clock_ticks":clock_ticks,
        "measured_retained":retained,"readout_entry":readout_clock.entry(),"readout_period":readout_clock.period(),
        "readout_distant":readout_clock.read(1_000_000_000_000_000_000),"peak_nodes":peak_nodes,
        "gram":witness.gram.iter().map(|(re,im)| [re.to_string(),im.to_string()]).collect::<Vec<_>>(),
        "masses":witness.masses.map(|m| m.to_string()),
        "reconstruction_residual":witness.reconstruction.maximum_residual.to_string(),
        "reconstruction_tolerance":witness.reconstruction.tolerance.to_string(),
        "coordinate_support_clock_cases":support_clock_cases,
        "sic_outcome_mask":sic_mask,
        "sic_outcome_gram":sic_witness.gram.iter().map(|(re,im)| [re.to_string(),im.to_string()]).collect::<Vec<_>>(),
        "sic_outcome_masses":sic_witness.masses.map(|m| m.to_string()),
        "sic_outcome_sample_masses":sic_witness.sample_masses.map(|m| m.to_string()),
        "sic_outcome_projection_residual":sic_witness.projection_residual.to_string(),
        "sic_outcome_reconstruction_residual":sic_witness.reconstruction.maximum_residual.to_string(),
        "sic_outcome_reconstruction_tolerance":sic_witness.reconstruction.tolerance.to_string(),
        "finished":true}))
}

fn main() {
    match execute() {
        Ok(record) => println!("{record}"),
        Err(error) => { eprintln!("ququart-work-probe: {error}"); std::process::exit(2); }
    }
}
