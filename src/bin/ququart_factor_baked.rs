//! Baked ququart phase measurements and Gödel factor-product extraction.
//! All work amplitudes and phase digits remain resident until completion.
#[path = "ququart_support/mod.rs"]
mod support;
#[path = "../../../p4rakernel/bridge/client.rs"]
#[allow(dead_code)]
mod lean_kernel;
use g_momonados::{godel_calculus::{encode_cell_binary, Nat}, ququart_factor::QuquartFactorExecutor,
    ququart_folded_work::QuquartFoldedWorkDevice};
use std::io::Write;
use num_traits::ToPrimitive;
include!(concat!(env!("OUT_DIR"), "/ququart_prepared.rs"));

// Vox reads this progress marker at native samples. Codes: 0 startup, 1 Lean
// admitted, 2 input parsed, 3 Fourier checked, 4 base checked, 5 device
// prepared, 6 powers decoded, 7 schedule validated, 8 phase executor.
#[no_mangle]
pub static VOX_QUQUART_PIPELINE_STAGE: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);
#[no_mangle]
pub static VOX_QUQUART_POWER_WORDS_DECODED: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);
fn pipeline_stage(stage: u64) {
    VOX_QUQUART_PIPELINE_STAGE.store(stage, std::sync::atomic::Ordering::Relaxed);
}

fn execute() -> Result<String, String> {
    // Admit the source-bound carrier through the same Lean kernel bridge used
    // by the hosted evaluator. This checks SIXTEEN_3, kernel re-entry, and Y.
    lean_kernel::ensure_default().map_err(|error| format!("Lean admission: {error}"))?;
    pipeline_stage(1);
    let prepared: serde_json::Value = serde_json::from_str(PREPARED).map_err(|e| e.to_string())?;
    if prepared.get("prepared_operator").is_none() {
        return Err("baked membrane requires a prepared operator in IMASM words".into());
    }
    pipeline_stage(2);
    let (n, fourier, metrics) = support::contract(&prepared)?;
    pipeline_stage(3);
    support::validate_base_scaling(&prepared)?;
    pipeline_stage(4);
    let base = support::numeral(prepared["base_word"].as_str().ok_or("missing baked base word")?)?;
    let seed = support::numeral(prepared["seed_word"].as_str().ok_or("missing baked seed word")?)?
        .to_u64().ok_or("invalid seed word")?;
    let radix_word = prepared["radix_word"].as_str().ok_or("missing baked nested radix word")?;
    g_momonados::ququart_factor::power_of_two_radix_word(radix_word)?;
    let source_word = prepared["source_word"].as_str().ok_or("missing source word")?;
    let native_enabled = support::numeral(prepared["native_arm_word"].as_str().ok_or("missing native arm selector")?)?
        == num_bigint::BigUint::from(1u8);
    // A disabled arm must not call the deliberately removed native producer.
    let native_pair = if native_enabled {
        g_momonados::arbitrary_factor::native_factor_word_pair(source_word)?
    } else {
        None
    };
    if let Some((p,q)) = native_pair {
        let (p,q) = g_momonados::ququart_factor::nested_radix_factor_words(source_word,&p,&q,radix_word)?;
        let closure = format!("{source_word}|{p}|{q}");
        let word = |value:u64| encode_cell_binary(&Nat::from_bits_le(
            (0..64).map(|bit| value & (1u64 << bit) != 0).collect()));
        return Ok(format!(concat!("completed ququart factor extraction\n",
            "source_word={}\nbase_word={}\nproducing_arm_word={}\n",
            "p_word={}\nq_word={}\ngodel_product_verified=true\nclosure_word={}\n",
            "radix_word={}\nnested_factor_word={}\nshots_word={}\nphase_samples=[]\nsic_frame_samples=[]\n",
            "fourier_computational_word={}\nfourier_leakage_word={}\nfourier_return_word={}\nfourier_exchanges_word={}\n"),
            source_word, prepared["base_word"].as_str().ok_or("missing base word")?, word(0),p,q,closure,
            radix_word,closure,word(0),word(metrics.computational.to_bits()),word(metrics.leakage.to_bits()),
            word(metrics.closure.to_bits()),word(metrics.exchanges as u64)));
    }
    let device = QuquartFoldedWorkDevice::new_interleaved_radix(n.clone(), fourier, seed, radix_word)?;
    let device = if prepared.get("prepared_work").is_some() {
        device.with_prepared_work(support::work::decode(&prepared)?)
    } else {
        device
    };
    pipeline_stage(5);
    let power_words = prepared["prepared_operator"]["controlled_power_words"].as_array()
        .ok_or("missing baked controlled power words")?.iter()
        .collect::<Vec<_>>();
    VOX_QUQUART_POWER_WORDS_DECODED.store(0, std::sync::atomic::Ordering::Relaxed);
    let mut powers = Vec::with_capacity(power_words.len());
    for value in power_words {
        powers.push(support::numeral(value.as_str().ok_or("controlled power must be an IMASM word")?)?);
        VOX_QUQUART_POWER_WORDS_DECODED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    let digits = support::numeral(prepared["prepared_operator"]["phase_digits_word"].as_str()
        .ok_or("missing baked phase resolution word")?)?;
    if digits != num_bigint::BigUint::from(powers.len()) { return Err("baked phase resolution differs from schedule".into()); }
    pipeline_stage(6);
    let schedule = g_momonados::ququart_factor::QuquartPowerSchedule::from_prepared(&n, &base, powers)?;
    pipeline_stage(7);
    let mut executor = QuquartFactorExecutor::with_prepared_schedule(device, schedule);
    pipeline_stage(8);
    if std::env::var_os("QUQUART_TRACE").is_some() {
        std::thread::spawn(|| loop {
            std::thread::sleep(std::time::Duration::from_secs(2));
            let nested: Vec<u64> = g_momonados::ququart_folded_work::VOX_NESTED_OPERATION_COUNTERS
                .iter().map(|counter| counter.load(std::sync::atomic::Ordering::Relaxed)).collect();
            eprintln!("nested_operation_counters={:?}", nested);
            let counters: Vec<u64> = g_momonados::ququart_folded_work::VOX_QUQUART_COUNTERS
                .iter().map(|counter| counter.load(std::sync::atomic::Ordering::Relaxed)).collect();
            eprintln!("ququart_counters={:?}", counters);
        });
    }
    let mut shot = num_bigint::BigUint::from(0u8);
    loop {
        shot += 1u8;
        let readout = executor.shot(&n, &base)?;
        if let Some(closure) = readout.closure {
            let (p_word, q_word) = closure.nested_radix_words(radix_word)?;
            let source_decimal = support::numeral(closure.source_word())?.to_str_radix(10);
            let p_decimal = support::numeral(&p_word)?.to_str_radix(10);
            let q_decimal = support::numeral(&q_word)?.to_str_radix(10);
            let mut lean_report = Vec::new();
            let lean_status = lean_kernel::run_to(&[
                "product".into(), source_decimal, p_decimal, q_decimal,
            ], |bytes| lean_report.extend_from_slice(bytes));
            if lean_status != 0 || !String::from_utf8_lossy(&lean_report)
                .lines().any(|line| line.starts_with("product-certificate=true ")
                    && line.ends_with("evidence-certified=true")) {
                return Err(format!("Lean rejected source-bound factor product: {}",
                    String::from_utf8_lossy(&lean_report)));
            }
            let word = |v: &num_bigint::BigUint| encode_cell_binary(&Nat::from_bits_le(
                (0..v.bits()).map(|bit| v.bit(bit)).collect()));
            let phase_denominator = readout.phase.denominator()?;
            let samples: Vec<_> = executor.measured_phases().iter().map(|(numerator, denominator)|
                serde_json::json!({"numerator_word": word(numerator), "denominator_word": word(denominator)})).collect();
            let samples = serde_json::to_string(&samples).map_err(|e| e.to_string())?;
            let signed = |value: &num_bigint::BigInt| format!("{}{}",
                if value.sign() == num_bigint::Sign::Minus { "≺" } else { "≻" },word(value.magnitude()));
            let sic_samples: Vec<_> = executor.device().sic_witnesses().iter().map(|witness|
                serde_json::json!({
                    "gram":witness.gram.iter().map(|(re,im)| serde_json::json!({"re_word":signed(re),"im_word":signed(im)})).collect::<Vec<_>>(),
                    "mass_words":witness.masses.iter().map(&word).collect::<Vec<_>>(),
                    "recovered_gram":witness.reconstruction.recovered.iter().map(|(re,im)| serde_json::json!({"re_word":signed(re),"im_word":signed(im)})).collect::<Vec<_>>(),
                    "reconstruction_residual_word":word(witness.reconstruction.maximum_residual.magnitude()),
                    "reconstruction_tolerance_word":word(witness.reconstruction.tolerance.magnitude()),
                    "digit_word":word(&num_bigint::BigUint::from(witness.digit as u8))
                })).collect();
            let sic_samples = serde_json::to_string(&sic_samples).map_err(|e|e.to_string())?;
            let nesting_word = format!("{}|{}|{}", closure.source_word(), p_word, q_word);
            let report = format!(
                "completed ququart factor extraction\nsource_word={}\nbase_word={}\nshots_word={}\nphase_numerator_word={}\nphase_denominator_word={}\norder_word={}\np_word={}\nq_word={}\ngodel_product_verified=true\nclosure_word={}\nfourier_computational_word={}\nfourier_leakage_word={}\nfourier_return_word={}\nfourier_exchanges_word={}\nphase_samples={samples}\n",
                closure.source_word(), prepared["base_word"].as_str().ok_or("missing baked base word")?,
                word(&shot), word(readout.phase.numerator()),
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
    let result = if std::env::args_os().len() == 1 {
        execute()
    } else {
        Err("prepared membrane accepts no runtime inputs".into())
    };
    let (report, code) = match result {
        Ok(report) => (report, 0),
        Err(error) => (format!("factor extraction failed\nerror_detail={error}\n"), 1),
    };
    // This is the sole output operation and follows measurement and extraction.
    if std::io::stdout().lock().write_all(report.as_bytes()).is_err() { std::process::exit(1); }
    if code != 0 { std::process::exit(code); }
}
