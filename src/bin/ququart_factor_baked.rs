//! Baked ququart phase measurements and Gödel factor-product extraction.
//! All work amplitudes and phase digits remain resident until completion.
#[path = "ququart_support/mod.rs"]
mod support;
use g_momonados::{godel_calculus::{encode_cell_binary, Nat}, ququart_factor::QuquartFactorExecutor,
    ququart_folded_work::QuquartFoldedWorkDevice};
use std::io::Write;
use num_traits::ToPrimitive;
include!(concat!(env!("OUT_DIR"), "/ququart_prepared.rs"));

fn execute() -> Result<String, String> {
    let prepared: serde_json::Value = serde_json::from_str(PREPARED).map_err(|e| e.to_string())?;
    if prepared.get("prepared_operator").is_none() {
        return Err("baked membrane requires a prepared operator in IMASM words".into());
    }
    let (n, fourier, metrics) = support::contract(&prepared)?;
    support::validate_base_scaling(&prepared)?;
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
    let powers = prepared["prepared_operator"]["controlled_power_words"].as_array()
        .ok_or("missing baked controlled power words")?.iter()
        .map(|value| support::numeral(value.as_str().ok_or("controlled power must be an IMASM word")?))
        .collect::<Result<Vec<_>, String>>()?;
    let digits = support::numeral(prepared["prepared_operator"]["phase_digits_word"].as_str()
        .ok_or("missing baked phase resolution word")?)?;
    if digits != num_bigint::BigUint::from(powers.len()) { return Err("baked phase resolution differs from schedule".into()); }
    let schedule = g_momonados::ququart_factor::QuquartPowerSchedule::from_prepared(&n, &base, powers)?;
    let mut executor = QuquartFactorExecutor::with_prepared_schedule(device, schedule);
    let mut shot = num_bigint::BigUint::from(0u8);
    loop {
        shot += 1u8;
        let readout = executor.shot(&n, &base)?;
        if let Some(closure) = readout.closure {
            let (p_word, q_word) = closure.nested_radix_words(radix_word)?;
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
            return Ok(format!("{report}producing_arm_word={phase_arm}\nradix_word={radix_word}\nnested_factor_word={nesting_word}\nsic_frame_samples={sic_samples}\n"));
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
