//! phase_unbraid_membrane.rs — the phase unbraider instantiated AS a
//! membrane: the value to be factored is BAKED IN, the register is
//! self-contained, and the run is the witness.
//!
//! A membrane is not a program that takes an argument. N is mounted at
//! compile time (N_VALUE below); the binary has no factor-seeking input at
//! all. What it does on run is fixed structure: prepare the collapsed comb,
//! apply the exact QFT, measure the winding k/M, continued-fraction it into
//! the period, close with one gcd, and emit the factors — verified
//! word-natively before the report exists. μ∘δ=id is the exit condition:
//! the run aborts non-zero unless the emitted pair re-multiplies to the
//! mounted value through the word ops.
//!
//! Run: cargo run --release --example phase_unbraid_membrane
//! (artifact at target/release/examples/phase_unbraid_membrane)

use g_momonados::native_numeral::{self, encode, multiply_via_word, syzygy_preserves};
use g_momonados::phase_unbraid::run_phase_unbraid;
use num_bigint::BigUint;

/// The baked-in value. The membrane IS this number's factoring diagram.
const N_VALUE: u64 = 8051;
/// The coprime base the phase register winds around. Baked in with N.
const A_BASE: u64 = 2;
/// Bounded re-measurement budget of the same prepared state.
const MAX_SHOTS: u32 = 12;

fn main() {
    let n = N_VALUE;
    println!("membrane mount: N = {}  word: {}", n, encode(&n.to_string()));
    println!("register: phase readout — comb -> exact QFT -> one Born shot -> winding -> CF -> gcd");
    let res = run_phase_unbraid(n, A_BASE, MAX_SHOTS)
        .unwrap_or_else(|e| { eprintln!("membrane rupture: {}", e); std::process::exit(1); });
    print!("{}", res.trace);
    let (p, q) = match res.factors {
        Some(pair) => pair,
        None => {
            eprintln!("membrane did not close in {} shot(s) — reported as measured, no guess emitted", res.total_shots);
            std::process::exit(2);
        }
    };
    let nb = BigUint::from(n);
    let pb = &p;
    let qb = &q;
    let mult_ok = multiply_via_word(pb, qb) == nb;
    let syz_ok = syzygy_preserves(&nb, pb, qb);
    if !mult_ok || !syz_ok {
        eprintln!("FROBENIUS FAILURE: p={} q={} mult_ok={} syzygy_ok={} — membrane emitted nothing", p, q, mult_ok, syz_ok);
        std::process::exit(3);
    }
    println!("FACTORS: p = {}   q = {}", p, q);
    println!("p × q = N: {}  [multiply_via_word]", mult_ok);
    println!("syzygy preserves [encode; Γ; Λ; μ]: {}", syz_ok);
    println!("{}", native_numeral::factor_words_line(pb, qb));
    println!("μ∘δ = id — membrane closed.");
}
