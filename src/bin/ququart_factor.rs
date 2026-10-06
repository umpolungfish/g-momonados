//! Standalone ququart phase factor extraction binary.
//! Usage: ququart_factor <semiprime>
use g_momonados::{arbitrary_factor::nat_to_biguint_pub, godel_calculus::{decode, encode_cell_binary, Family, Nat}, ququart_factor::{extract_certified_winding, QuquartFactorExecutor}, ququart_folded_work::QuquartFoldedWorkDevice, phase_unbraid::FixedPointFormat, anyon_pair::{PairMatrix, COMPUTATIONAL_CHANNELS, LEAKAGE_CHANNEL}, phase_unbraid::FixedComplex};
use num_bigint::{BigUint, BigInt};
use num_traits::Zero;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: ququart_factor <semiprime>");
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
    
    // Compute λ = lcm(p-1, q-1) = (p-1)(q-1)/gcd(p-1,q-1) via phase order finding
    // We know 2^λ ≡ 1 (mod N) for the Carmichael function
    // The ququart phase measurement gives us the order denominator
    
    // Build Fourier braid matrix and run phase estimation
    
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
    
    let device = QuquartFoldedWorkDevice::new(source.clone(), matrix, 1729).expect("device init error");
    let mut executor = QuquartFactorExecutor::new(device);
    
    // Execute shot with base 2 to measure the order
    let base = BigUint::from(2u32);
    let _shot = executor.shot(&source, &base).expect("ququart shot error");
    
    // Extract the measured order and use multi-base winding extraction
    if let Some((_numerator, denominator)) = executor.measured_phases().first() {
        let closure = extract_certified_winding(&source, denominator).expect("winding extraction failed");
        let (p, q): (&BigUint, &BigUint) = closure.factors();
        println!("{}", p);
        println!("{}", q);
        return;
    }
    
    eprintln!("Extraction did not close a factor pair");
    std::process::exit(1);
}