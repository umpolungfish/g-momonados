//! Standalone ququart phase factor extraction binary.
//! Usage: ququart_factor <semiprime> [prepared Fourier operator JSON]
#[path = "ququart_support/mod.rs"]
#[allow(dead_code)]
mod support;
use g_momonados::{arbitrary_factor::nat_to_biguint_pub, godel_calculus::{decode, encode_cell_binary, Family, Nat}, ququart_factor::QuquartFactorExecutor, ququart_folded_work::QuquartFoldedWorkDevice, phase_unbraid::FixedPointFormat, anyon_pair::{PairMatrix, COMPUTATIONAL_CHANNELS, LEAKAGE_CHANNEL}, phase_unbraid::FixedComplex};
use num_bigint::{BigUint, BigInt};
use num_traits::Zero;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if !(2..=3).contains(&args.len()) {
        eprintln!("Usage: ququart_factor <semiprime> [prepared Fourier operator JSON]");
        std::process::exit(1);
    }
    let input = &args[1];
    
    // Parse input (decimal or IMASM word)
    let (source, _source_word) = if input.starts_with('⊢') {
        let reading = decode(input).expect("invalid IMASM word");
        assert!(matches!(reading.family, Family::CellBinary));
        let canonical = encode_cell_binary(&reading.value);
        assert_eq!(canonical.as_str(), input, "non-canonical IMASM word");
        (nat_to_biguint_pub(&reading.value), canonical)
    } else {
        let value = BigUint::parse_bytes(input.as_bytes(), 10).expect("not a natural number");
        let nat = Nat::from_decimal(input).expect("not a natural number");
        (value, encode_cell_binary(&nat))
    };
    
    if source.bits() < 200 {
        eprintln!("RSA-style factor checks require a modulus of at least 200 bits");
        std::process::exit(2);
    }

    // Retain the ideal matrix as a reference mode. Physical runs decode the
    // source-bound contraction of an emitted braid and check its actual map.
    let (matrix, radix_word) = if let Some(path) = args.get(2) {
        let prepared: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(path).expect("cannot read prepared Fourier operator"))
            .expect("invalid prepared Fourier JSON");
        assert!(prepared.get("prepared_operator").is_some(), "physical run requires a contracted operator");
        let (bound_source, matrix, metrics) = support::contract(&prepared)
            .expect("prepared Fourier operator failed resident entry checks");
        assert_eq!(bound_source, source, "prepared Fourier operator belongs to a different source");
        eprintln!("fourier_mode=contracted_physical exchanges={} computational={:.8e} leakage={:.8e} return={:.8e}",
            metrics.exchanges, metrics.computational, metrics.leakage, metrics.closure);
        (matrix, prepared.get("radix_word").map(|value|
            value.as_str().expect("prepared work radix must be a word").to_owned()))
    } else {
    let format = FixedPointFormat::for_modulus(&source).expect("format error");
    let mut matrix = PairMatrix(core::array::from_fn(|_| FixedComplex {
        re: BigInt::zero(),
        im: BigInt::zero(),
    }));
    for (row, &r) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
        for (col, &c) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
            let mut z = FixedComplex::winding_twiddle(
                &BigInt::from(row * col),
                &BigUint::from(4u8),
                &format,
            ).unwrap();
            z.re /= 2u8;
            z.im /= 2u8;
            matrix.0[5 * r + c] = z;
        }
    }
    matrix.0[5 * LEAKAGE_CHANNEL + LEAKAGE_CHANNEL].re = format.scale();
    eprintln!("fourier_mode=ideal_reference");
    (matrix, None)
    };
    let device = if let Some(radix_word) = radix_word {
        QuquartFoldedWorkDevice::new_interleaved_radix(source.clone(), matrix, 1729, &radix_word)
    } else {
        QuquartFoldedWorkDevice::new_interleaved(source.clone(), matrix, 1729)
    }.expect("device init error");
    if std::env::var_os("QUQUART_TRACE").is_some() {
        std::thread::spawn(|| loop {
            std::thread::sleep(std::time::Duration::from_secs(2));
            let counters: Vec<u64> = g_momonados::ququart_folded_work::VOX_QUQUART_COUNTERS
                .iter().map(|counter| counter.load(std::sync::atomic::Ordering::Relaxed)).collect();
            eprintln!("ququart_counters={:?}", counters);
            let modular: Vec<u64> = g_momonados::ququart_decision::VOX_MODULAR_COUNTERS
                .iter().map(|counter| counter.load(std::sync::atomic::Ordering::Relaxed)).collect();
            eprintln!("modular_counters={:?}", modular);
        });
    }
    let mut executor = QuquartFactorExecutor::new(device);
    
    // Retain phase evidence across shots. The phase register denominator is
    // a power of four, not the measured modular order; only a closed resident
    // readout may provide factors.
    let base = BigUint::from(2u32);
    loop {
        let shot = executor.shot(&source, &base).expect("ququart shot error");
        if let Some(closure) = shot.closure {
            let (p, q) = closure.factors();
            assert!(*p > BigUint::from(1u8) && *q > BigUint::from(1u8));
            assert_eq!(p * q, source, "resident factor pair must reconstruct N");
            println!("{} = {} x {}", source, p, q);
            println!("order={}", closure.order());
            return;
        }
    }
}
