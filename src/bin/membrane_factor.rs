//! Direct membrane factorization using the GLUT 2-adic running-product membrane.
//! This preserves the proper nested states through the factorization, unlike
//! the pass-through arbitrary_factor approach which loses the membrane structure.

use vox_core::morphism_factor::decimal_to_tape;
use vox_core::glut_system::glut_correlation_execution;
use num_bigint::BigUint;
use std::env;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some(input) = args.first() else {
        eprintln!("Usage: membrane_factor <decimal-number>");
        std::process::exit(1);
    };

    let n = BigUint::parse_bytes(input.as_bytes(), 10)
        .ok_or_else(|| format!("Invalid decimal: {}", input))
        .unwrap();

    if n.bit(0) == false {
        eprintln!("N must be odd");
        std::process::exit(1);
    }

    println!("N = {}", n);
    println!("Bit length = {}", n.bits());

    // Convert to IMASM tape (LSB-first, ⊥=1, ⊤=0)
    let tape = decimal_to_tape(input).unwrap();
    println!("IMASM tape length = {}", tape.len());

    // Execute the GLUT correlation membrane directly
    // This preserves the proper nested states through the running-product congruence
    let execution = glut_correlation_execution(&tape);

    match execution {
        Some(exec) => {
            // Verify the execution
            exec.verify(&tape).expect("Execution verification failed");

            // Convert factor tapes back to BigUint
            let p = tape_to_biguint(&exec.p);
            let q = tape_to_biguint(&exec.q);

            println!("\n=== FACTORIZATION COMPLETE ===");
            println!("p = {}", p);
            println!("q = {}", q);
            println!("p * q = {}", &p * &q);
            println!("Verified: {}", &p * &q == n);
            println!("Checkpoint count: {}", exec.checkpoints.len());

            // Print the nested state trace
            println!("\n=== NESTED STATE TRACE (proper membrane states preserved) ===");
            for (i, checkpoint) in exec.checkpoints.iter().enumerate() {
                let p_val = tape_to_biguint(&checkpoint.p_prefix);
                let q_val = tape_to_biguint(&checkpoint.q_prefix);
                let prod_val = &p_val * &q_val;
                let n_mod = &n % (BigUint::from(1u32) << (checkpoint.position + 1));
                println!("Frame {}: pos={}, p={}, q={}, p*q≡N(mod 2^{})? {}",
                    i, checkpoint.position, p_val, q_val, checkpoint.position + 1,
                    prod_val % (BigUint::from(1u32) << (checkpoint.position + 1)) == n_mod);
            }
        }
        None => {
            eprintln!("Membrane execution failed to find factor pair");
            std::process::exit(1);
        }
    }
}

fn tape_to_biguint(tape: &[char]) -> BigUint {
    let mut bytes = vec![0u8; (tape.len() + 7) / 8];
    for (index, mark) in tape.iter().enumerate() {
        if *mark == '⊥' {
            bytes[index / 8] |= 1 << (index % 8);
        }
    }
    BigUint::from_bytes_le(&bytes)
}