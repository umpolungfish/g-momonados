//! Baked ququart phase measurements and Gödel factor-product extraction.
//! All work amplitudes and phase digits remain resident until completion.
#[path = "../../../p4rakernel/bridge/client.rs"]
#[allow(dead_code)]
mod lean_kernel;
#[path = "ququart_support/mod.rs"]
mod support;
use g_momonados::{
    godel_calculus::{encode_cell_binary, Nat},
    ququart_reentry_clock::RuntimeClock,
    whole_register_phase::WholeRegisterPhaseExecutor,
};
use num_traits::ToPrimitive;
use std::io::Write;
include!(concat!(env!("OUT_DIR"), "/ququart_prepared.rs"));

// Vox reads this progress marker at native samples. Codes: 0 startup, 1 Lean
// admitted, 2 input parsed, 3 Fourier checked, 4 base checked, 5 work
// schedule prepared, 6 powers decoded, 7 schedule validated, 8 phase executor.
#[no_mangle]
pub static VOX_QUQUART_PIPELINE_STAGE: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);
#[no_mangle]
pub static VOX_QUQUART_POWER_WORDS_DECODED: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);
fn pipeline_stage(stage: u64) {
    VOX_QUQUART_PIPELINE_STAGE.store(stage, std::sync::atomic::Ordering::Relaxed);
}

fn execute(open_readout: bool) -> Result<String, String> {
    // Admit the source-bound carrier through the same Lean kernel bridge used
    // by the hosted evaluator. This checks SIXTEEN_3, kernel re-entry, and Y.
    let y_fixed_point =
        lean_kernel::ensure_default().map_err(|error| format!("Lean admission: {error}"))?;
    pipeline_stage(1);
    let prepared: serde_json::Value = serde_json::from_str(PREPARED).map_err(|e| e.to_string())?;
    if prepared.get("prepared_operator").is_none() {
        return Err("baked membrane requires a prepared operator in IMASM words".into());
    }
    pipeline_stage(2);
    let (n, _fourier, metrics) = support::contract(&prepared)?;
    pipeline_stage(3);
    support::validate_base_scaling(&prepared)?;
    pipeline_stage(4);
    let base = support::numeral(
        prepared["base_word"]
            .as_str()
            .ok_or("missing baked base word")?,
    )?;
    let seed = support::numeral(
        prepared["seed_word"]
            .as_str()
            .ok_or("missing baked seed word")?,
    )?
    .to_u64()
    .ok_or("invalid seed word")?;
    let radix_word = prepared["radix_word"]
        .as_str()
        .ok_or("missing baked nested radix word")?;
    g_momonados::ququart_factor::power_of_two_radix_word(radix_word)?;
    pipeline_stage(5);
    let power_words = prepared["prepared_operator"]["controlled_power_words"]
        .as_array()
        .ok_or("missing baked controlled power words")?
        .iter()
        .collect::<Vec<_>>();
    VOX_QUQUART_POWER_WORDS_DECODED.store(0, std::sync::atomic::Ordering::Relaxed);
    let mut powers = Vec::with_capacity(power_words.len());
    for value in power_words {
        powers.push(support::numeral(
            value
                .as_str()
                .ok_or("controlled power must be an IMASM word")?,
        )?);
        VOX_QUQUART_POWER_WORDS_DECODED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    let digits = support::numeral(
        prepared["prepared_operator"]["phase_digits_word"]
            .as_str()
            .ok_or("missing baked phase resolution word")?,
    )?;
    if digits != num_bigint::BigUint::from(powers.len()) {
        return Err("baked phase resolution differs from schedule".into());
    }
    pipeline_stage(6);
    let schedule =
        g_momonados::ququart_factor::QuquartPowerSchedule::from_prepared(&n, &base, powers)?;
    let phase_digit_count = schedule.powers().len();
    let work = prepared
        .get("prepared_work")
        .map(|_| support::work::decode(&prepared))
        .transpose()?;
    let radix = g_momonados::ququart_factor::power_of_two_radix_word(radix_word)?;
    let digit_bits = radix.bits_le().len() - 1;
    if let Some(work) = &work {
        work.verify_recovery(&n, &base, digit_bits, phase_digit_count)?;
    }
    pipeline_stage(7);
    let phase_width = phase_digit_count
        .checked_mul(2)
        .ok_or("whole-register phase width overflow")?;
    pipeline_stage(8);
    let mut shot = num_bigint::BigUint::from(0u8);
    let prepared_orientation_word = prepared
        .get("phase_orientation_pairs")
        .map(|value| -> Result<Vec<(bool, bool)>, String> {
            value
                .as_array()
                .ok_or("phase orientation word must be an array")?
                .iter()
                .map(|pair| {
                    let pair = pair
                        .as_array()
                        .ok_or("phase orientation pair must be an array")?;
                    if pair.len() != 2 {
                        return Err("phase orientation pair must contain two flags".into());
                    }
                    let first = pair[0]
                        .as_bool()
                        .ok_or("phase orientation flag must be Boolean")?;
                    let second = pair[1]
                        .as_bool()
                        .ok_or("phase orientation flag must be Boolean")?;
                    Ok((first, second))
                })
                .collect()
        })
        .transpose()?;
    let orientation_word = if let Some(record) = prepared.get("phase_reentry_clock") {
        let clock = RuntimeClock::from_record(record)?;
        let read_states = |key: &str| -> Result<Vec<u8>, String> {
            prepared
                .get(key)
                .and_then(serde_json::Value::as_array)
                .ok_or_else(|| format!("missing {key}"))?
                .iter()
                .map(|state| {
                    state
                        .as_u64()
                        .filter(|state| *state < 16)
                        .map(|state| state as u8)
                        .ok_or_else(|| format!("invalid state in {key}"))
                })
                .collect()
        };
        let source_states = read_states("phase_source_states")?;
        let clock_states = read_states("phase_clock_states")?;
        if source_states.is_empty() || source_states.len() != clock_states.len() {
            return Err("phase source and re-entry clock words differ in length".into());
        }
        let mut reentered = Vec::with_capacity(source_states.len() * 4 + 3);
        for (&source_state, &clock_state) in source_states.iter().zip(&clock_states) {
            if clock.step(source_state) != clock_state {
                return Err("phase coordinate differs from its retained re-entry step".into());
            }
            let pair = (clock_state & 2 != 0, clock_state & 4 != 0);
            reentered.extend(core::iter::repeat(pair).take(4));
        }
        let prefix = reentered[..3].to_vec();
        reentered.extend(prefix);
        if prepared_orientation_word.as_ref() != Some(&reentered) {
            return Err(
                "phase orientation word differs from its re-entered motif coordinates".into(),
            );
        }
        let y_forward = y_fixed_point & 2 != 0;
        let y_return = y_fixed_point & 4 != 0;
        Some(
            reentered
                .into_iter()
                .map(|(forward, inverse)| (forward ^ y_forward, inverse ^ y_return))
                .collect(),
        )
    } else {
        prepared_orientation_word
    };
    if orientation_word
        .as_ref()
        .map(|word| word.len() != phase_digit_count)
        .unwrap_or(false)
    {
        return Err("phase orientation word differs from the prepared phase resolution".into());
    }
    let orientation_word = orientation_word.unwrap_or_default();
    let mut executor = WholeRegisterPhaseExecutor::prepare_with_reentry_and_work(
        n.clone(),
        base.clone(),
        Some(phase_width),
        seed,
        &orientation_word,
        work,
    )?;
    let mut phase_samples = Vec::new();
    loop {
        shot += 1u8;
        let readout = executor.measure_next_with_readout(|phase| {
            if open_readout {
                let word = |v: &num_bigint::BigUint| {
                    encode_cell_binary(&Nat::from_bits_le(
                        (0..v.bits()).map(|bit| v.bit(bit)).collect(),
                    ))
                };
                let event = serde_json::json!({
                    "event": "whole_register_phase_readout",
                    "source_word": word(&phase.source),
                    "base_word": word(&phase.base),
                    "numerator_word": word(&phase.numerator),
                    "denominator_word": word(&phase.denominator),
                    "width_word": word(&num_bigint::BigUint::from(phase.width)),
                    "shot_word": word(&shot),
                });
                let mut output = std::io::stdout().lock();
                writeln!(output, "{event}").map_err(|error| error.to_string())?;
                output.flush().map_err(|error| error.to_string())?;
            }
            Ok(())
        })?;
        phase_samples.push((
            readout.phase.numerator.clone(),
            readout.phase.denominator.clone(),
        ));
        if let Some(closure) = readout.closure {
            let (p_word, q_word) = closure.nested_radix_words(radix_word)?;
            let source_decimal = support::numeral(closure.source_word())?.to_str_radix(10);
            let p_decimal = support::numeral(&p_word)?.to_str_radix(10);
            let q_decimal = support::numeral(&q_word)?.to_str_radix(10);
            let mut lean_report = Vec::new();
            let lean_status = lean_kernel::run_to(
                &["product".into(), source_decimal, p_decimal, q_decimal],
                |bytes| lean_report.extend_from_slice(bytes),
            );
            if lean_status != 0
                || !String::from_utf8_lossy(&lean_report).lines().any(|line| {
                    line.starts_with("product-certificate=true ")
                        && line.ends_with("evidence-certified=true")
                })
            {
                return Err(format!(
                    "Lean rejected source-bound factor product: {}",
                    String::from_utf8_lossy(&lean_report)
                ));
            }
            let word = |v: &num_bigint::BigUint| {
                encode_cell_binary(&Nat::from_bits_le(
                    (0..v.bits()).map(|bit| v.bit(bit)).collect(),
                ))
            };
            let phase_denominator = readout.phase.denominator.clone();
            let samples: Vec<_> = phase_samples.iter().map(|(numerator, denominator)|
                serde_json::json!({"numerator_word": word(numerator), "denominator_word": word(denominator)})).collect();
            let samples = serde_json::to_string(&samples).map_err(|e| e.to_string())?;
            let sic_samples = "[]";
            let nesting_word = format!("{}|{}|{}", closure.source_word(), p_word, q_word);
            let report = format!(
                "completed ququart factor extraction\nsource_word={}\nbase_word={}\nshots_word={}\nphase_numerator_word={}\nphase_denominator_word={}\norder_word={}\np_word={}\nq_word={}\ngodel_product_verified=true\nclosure_word={}\nfourier_computational_word={}\nfourier_leakage_word={}\nfourier_return_word={}\nfourier_exchanges_word={}\nphase_samples={samples}\n",
                closure.source_word(), prepared["base_word"].as_str().ok_or("missing baked base word")?,
                word(&shot), word(&readout.phase.numerator),
                word(&phase_denominator), word(closure.order()), p_word, q_word, nesting_word,
                word(&num_bigint::BigUint::from(metrics.computational.to_bits())),
                word(&num_bigint::BigUint::from(metrics.leakage.to_bits())),
                word(&num_bigint::BigUint::from(metrics.closure.to_bits())),
                word(&num_bigint::BigUint::from(metrics.exchanges)));
            let phase_arm = word(&num_bigint::BigUint::from(1u8));
            let tick = num_bigint::BigUint::from(1_000_000_000_000_000_001u64);
            let step = closure.residue_step(&tick);
            let residue_report = format!(
                "residue_base_word={}\nresidue_tick_word={}\nresidue_period_word={}\nresidue_index_word={}\nresidue_value_word={}\n",
                word(closure.residue_base()), word(&step.tick), word(&step.period),
                word(&step.residue), word(&step.value));
            return Ok(format!("{report}{residue_report}producing_arm_word={phase_arm}\nradix_word={radix_word}\nnested_factor_word={nesting_word}\nsic_frame_samples={sic_samples}\n"));
        }
    }
}

fn main() {
    #[cfg(feature = "hosted")]
    vox_core::ordering::cli::intercept();
    let args: Vec<_> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [] => execute(false),
        [flag] if flag == "--open-readout" => execute(true),
        _ => Err("prepared membrane accepts only the --open-readout instrument flag".into()),
    };
    let (report, code) = match result {
        Ok(report) => (report, 0),
        Err(error) => (
            format!("factor extraction failed\nerror_detail={error}\n"),
            1,
        ),
    };
    // The final report follows extraction. Open readout additionally emits the
    // measured whole-register value at the phase boundary, before extraction.
    if std::io::stdout()
        .lock()
        .write_all(report.as_bytes())
        .is_err()
    {
        std::process::exit(1);
    }
    if code != 0 {
        std::process::exit(code);
    }
}
