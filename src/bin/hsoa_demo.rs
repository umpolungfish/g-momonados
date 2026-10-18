//! Reproduces the p3 hsoa_shor_state readout for the three known cases.

use num_bigint::BigUint;
use vox::hsoa_winding;

fn main() {
    let cases: [(u64, u64, usize); 3] = [
        (7, 15, 64),
        (2, 21, 64),
        (2, 35, 256),
    ];
    for (a, n, m) in cases {
        let comb = hsoa_winding::build_comb(
            &BigUint::from(a),
            &BigUint::from(n),
            m,
        );
        println!("\n=========================================================================");
        println!("HSOA on Shor state (periodic comb)");
        println!("  a={}  N={}  register={} qubits (M={})", a, n, (m as f64).log2() as u32, m);
        println!("=========================================================================\n");
        match hsoa_winding::read_period(&comb) {
            Some(r) => {
                println!("  [autocorr]  period r = {}", r.period);
                println!("  [comb]      non-zero amplitudes: {}", r.nonzero_count);
                println!("  [Frobenius] ||ψ||² = {:.6}  (μ∘δ = id iff norm matches)",
                    r.norm_sq);
                println!("  [EP]        min discriminant: {:.6e}", r.min_discriminant);
                println!("  [EP]        center: {:.4}{:+.4}j",
                    r.ep_center.re, r.ep_center.im);
                println!("  [EP]        order: {}", r.ep_order);
                println!("  [winding]   W = {}   (W == r : {})", r.winding, r.validated);
                println!("  [MBL]       localization ξ = {:.4}", r.localization_length);
                println!("  [time]      {} µs", r.elapsed_us);
            }
            None => println!("  no period detected"),
        }
    }
}