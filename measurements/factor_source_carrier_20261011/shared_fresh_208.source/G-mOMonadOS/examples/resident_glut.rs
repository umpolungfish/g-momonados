//! Baked N-only instrument for one coherent factor/product/carry glut.
use g_momonados::resident_glut::ResidentGlut;
use num_bigint::BigUint;
use num_traits::Signed;
use std::time::Instant;

fn main() -> Result<(), String> {
    let n = BigUint::parse_bytes(include_str!(
        "../../measurements/factor_synthesis_20261011/n.txt").trim().as_bytes(), 10)
        .ok_or("invalid baked source")?;
    eprintln!("source {n}");
    let preparation = Instant::now();
    let mut resident = ResidentGlut::new(n.clone())?;
    let sic = resident.sic_witness()?;
    eprintln!("SIC initial residual={} without_uniform_correction_bits={} tolerance={} signed_residuals={:?}",
        sic.reconstruction.maximum_residual, sic.uncorrected_maximum_residual.to_biguint().unwrap().bits(),
        sic.reconstruction.tolerance, sic.signed_residuals);
    let fractional = sic.rational_signed_residuals.iter().flat_map(|(re, im)| [re.abs(), im.abs()])
        .max().unwrap();
    eprintln!("SIC rational residual_numerator_bits={} denominator_bits={}",
        fractional.to_biguint().unwrap().bits(), sic.rational_denominator.to_biguint().unwrap().bits());
    eprintln!("SIC rational denominator={} signed_residual_numerators={:?}",
        sic.rational_denominator, sic.rational_signed_residuals);
    let frame = g_momonados::anyon_ququart::FixedQuquartSic::new(
        &g_momonados::phase_unbraid::FixedPointFormat::for_modulus(&n)?)?;
    let mut control = sic.gram.clone();
    for col in 1..4 {
        let (re, im) = &sic.gram[col];
        control[col] = (-im, re.clone());
        control[4*col] = (im.clone(), -re);
    }
    let control_masses = frame.gram_mass_numerators(&control)?;
    frame.certify_gram_frame_unrounded_masses(&control, &control_masses)?;
    let mismatched = frame.certify_gram_frame_unrounded_masses(&control, &sic.mass_numerators).is_err();
    if !mismatched { return Err("SIC coherence control accepted mismatched masses".into()) }
    eprintln!("SIC coherence-control recovered=true mismatched-masses-rejected=true");
    if option_env!("RESIDENT_SHARED_CONSTRAINTS") == Some("product-prefix") {
        let columns = (n.bits() as usize).min(8);
        let before = resident.support_mass();
        let gates = resident.constrain_product_prefix(columns, |_, _| {})?;
        let coherent = resident.support_mass();
        let device_pairs = resident.constrain_gpu_product_prefix(columns, 0)?;
        let device = resident.support_mass();
        if coherent != device {
            return Err("GPU product relation changed coherent-prefix support".into());
        }
        let constrained_sic = resident.sic_witness()?;
        eprintln!("shared-product-prefix columns={columns} coherent_gates={gates} gpu_pairs={device_pairs} initial_mass={before} selected_mass={device} SIC-residual={} SIC-tolerance={}",
            constrained_sic.reconstruction.maximum_residual, constrained_sic.reconstruction.tolerance);
        eprintln!("shared-product-prefix SIC-rational denominator={} signed_residual_numerators={:?}",
            constrained_sic.rational_denominator, constrained_sic.rational_signed_residuals);
    }
    eprintln!("prepared bits={} seconds={:.6}", n.bits(), preparation.elapsed().as_secs_f64());
    let started = Instant::now();
    let mut last_report = Instant::now();
    let result = if option_env!("RESIDENT_SOURCE_PORT") == Some("gpu-word-ecm") {
        eprintln!("source-port gpu-word-ecm-source B1=1000 B2=20000 device=0 sigma=6 curves=256 seconds={:.6}", started.elapsed().as_secs_f64());
        resident.readout_gpu_word()?
    } else if option_env!("RESIDENT_SOURCE_PORT") == Some("gpu-limb-ecm") {
        eprintln!("source-port gpu-limb-ecm-source B1=1000 B2=20000 devices=available curve_schedule=existing-source-derived-width seconds={:.6}", started.elapsed().as_secs_f64());
        resident.readout_gpu_limbs()?
    } else if matches!(option_env!("RESIDENT_SOURCE_PORT"), Some("nested-carrier" | "source-carrier")) {
        let source_word = vox_core::morphism_factor::source_carrier_word();
        let word = if option_env!("RESIDENT_SOURCE_PORT") == Some("source-carrier") {
            source_word.as_str()
        } else { vox_core::morphism_factor::NINE_ARM };
        let marks: Vec<char> = word.chars().collect();
        let tower = vox_core::morphism_factor::construct_carrier(word)?;
        eprintln!("carrier word={word} vox-control-flow-verdict={} arms={:?}",
            vox_core::vox::verdict(&marks), tower.iter().map(|arm|
                vox_core::morphism_factor::morphism_name(arm)).collect::<Vec<_>>());
        eprintln!("carrier-banked {}", imasm_core::lattice_flow::banked_report(word).replace('\n', " | "));
        #[cfg(feature = "carrier-observation")]
        std::thread::spawn(|| loop {
            std::thread::sleep(std::time::Duration::from_secs(5));
            let gauges: Vec<_> = vox_core::morphism_factor::VOX_FACTOR_COUNTERS.iter()
                .map(|g| g.load(std::sync::atomic::Ordering::Relaxed)).collect();
            eprintln!("carrier counters={gauges:?}");
            let sieve: Vec<_> = vox_core::sieve::VOX_SIEVE_COUNTERS.iter()
                .map(|g| g.load(std::sync::atomic::Ordering::Relaxed)).collect();
            eprintln!("carrier-sieve counters={sieve:?}");
            let admission: Vec<_> = vox_core::sieve::VOX_SIEVE_ADMISSION.iter()
                .map(|g| g.load(std::sync::atomic::Ordering::Relaxed)).collect();
            eprintln!("carrier-sieve admission_threshold_max_blocks={admission:?}");
        });
        if option_env!("RESIDENT_SOURCE_PORT") == Some("source-carrier") {
            resident.readout_source_carrier()?
        } else { resident.readout_carrier()? }
    } else { resident.readout_with_ports(|gates, nodes| {
        if last_report.elapsed().as_secs() >= 5 {
            eprintln!("resident seconds={:.3} gates={gates} retained_nodes={nodes}", started.elapsed().as_secs_f64());
            last_report = Instant::now();
        }
    }, |port| {
        let gauges: Vec<_> = vox_core::sieve::VOX_SIEVE_COUNTERS.iter()
            .map(|g| g.load(std::sync::atomic::Ordering::Relaxed)).collect();
        eprintln!("source-port {port} seconds={:.6} sieve_counters={gauges:?}", started.elapsed().as_secs_f64());
    })? };
    if let Some(result) = result {
        let readout_seconds = started.elapsed().as_secs_f64();
        assert_eq!(&result.divisor * &result.quotient, n);
        assert_eq!(&n % &result.divisor, BigUint::from(0u8));
        println!("trigger_to_readout_seconds {readout_seconds:.9}");
        println!("trigger_to_verified_divisor_seconds {:.9}", started.elapsed().as_secs_f64());
        println!("factor {}\nquotient {}", result.divisor, result.quotient);
        println!("producer {}", result.producer);
        println!("boundary gamma_delta_closed={} product_closed={} word={}",
            result.gamma_delta_closed, result.product_closed, result.boundary_word);
        println!("initial_mass {} selected_mass {} gates {} seconds {:.6}",
            result.initial_mass, result.selected_mass, result.gates, started.elapsed().as_secs_f64());
        println!("SIC return residual={} tolerance={} signed_residuals={:?}",
            result.sic.reconstruction.maximum_residual, result.sic.reconstruction.tolerance,
            result.sic.signed_residuals);
        println!("SIC return rational denominator={} signed_residual_numerators={:?}",
            result.sic.rational_denominator, result.sic.rational_signed_residuals);
    } else { println!("source port returned no divisor"); }
    Ok(())
}
