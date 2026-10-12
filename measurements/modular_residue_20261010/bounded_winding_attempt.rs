use g_momonados::factor_routes::{certified_modular_residue_step, order_multiple_leaping, big_gcd};
use num_bigint::BigUint;
use num_traits::{One, Zero};
use std::env;
fn main() {
    let args: Vec<String> = env::args().collect();
    let n: BigUint = args[1].parse().expect("decimal source");
    let a: BigUint = args[2].parse().expect("decimal base");
    let steps: u64 = args[3].parse().expect("bounded leap budget");
    let start = std::time::Instant::now();
    let d = order_multiple_leaping(&a, &n, steps);
    let mut factor = None;
    if let Some(mut period) = d.clone() {
        while (&period % BigUint::from(2u8)).is_zero() {
            let half = &period / BigUint::from(2u8);
            let step = certified_modular_residue_step(&half, &a, &n, &period).expect("certified winding half-step");
            if step.value == BigUint::one() { period = half; continue; }
            if step.value == &n - BigUint::one() { break; }
            let p = big_gcd(&step.value - BigUint::one(), n.clone());
            let q = &n / &p;
            if p > BigUint::one() && q > BigUint::one() && &p * &q == n {
                factor = Some((p, q, period, half, step.residue, step.value));
            }
            break;
        }
    }
    println!("elapsed_ms={}", start.elapsed().as_millis());
    println!("source_bits={}", n.bits());
    println!("base={a}");
    println!("leap_budget={steps}");
    match d { Some(ref value) => println!("winding_multiple={value}"), None => println!("winding_multiple=none_within_budget") }
    if let Some((p,q,period,half,residue,value)) = factor {
        println!("factor_p={p}\nfactor_q={q}\nproduct_verified={}\nperiod={period}\nhalf_tick={half}\nresidue={residue}\nresidue_value={value}", &p*&q==n);
    } else { println!("factor_pair=none"); }
}
