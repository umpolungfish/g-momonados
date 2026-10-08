// Quantum phase membrane using Floyd cycle detection on squaring map.
// Same efficient pattern as shor_order.rs, found to work for N=143.

use membranes::{Big, from_word, to_word, mul, sub, add, divmod, from_u32, cmp, is_zero};
use core::cmp::Ordering::*;

fn one() -> Big { from_u32(1) }
fn mulmod(a: &Big, b: &Big, n: &Big) -> Big { divmod(&mul(a, b), n).1 }
fn sq(x: &Big, n: &Big) -> Big { mulmod(x, x, n) }
fn gcd(a: &Big, b: &Big) -> Big { 
    let (mut x, mut y) = (a.clone(), b.clone()); 
    while !is_zero(&y) { let r = divmod(&x, &y).1; x = y; y = r; } 
    x 
}
fn powmod(a: &Big, e: &Big, n: &Big) -> Big {
    let mut r = one(); let mut base = divmod(a, n).1; let mut ee = e.clone(); let two = from_u32(2);
    while !is_zero(&ee) { 
        let (q, rem) = divmod(&ee, &two); 
        if !is_zero(&rem) { r = mulmod(&r, &base, n); } 
        base = mulmod(&base, &base, n); ee = q; 
    }
    r
}

fn try_factor(a: &Big, r_mult: &Big, n: &Big) -> Option<(Big, Big)> {
    let two = from_u32(2);
    let mut e = r_mult.clone();
    loop {
        let h = powmod(a, &e, n);
        if cmp(&h, &one()) == Equal || cmp(&h, &sub(n, &one())) == Equal { }
        else {
            for pm in [if cmp(&h, &one()) != Less { sub(&h, &one()) } else { one() }, add(&h, &one())] {
                let g = gcd(&pm, n);
                if cmp(&g, &one()) == Greater && cmp(&g, n) == Less { 
                    let (co, _) = divmod(n, &g); 
                    return Some((g, co)); 
                }
            }
        }
        let (q, rem) = divmod(&e, &two);
        if !is_zero(&rem) || is_zero(&q) { break; }
        e = q;
    }
    None
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args.len() > 2 {
        eprintln!("usage: membrane_quantum_phase <canonical-IMASM-N> [squaring-cap]");
        std::process::exit(2);
    }
    let n = from_word(&args[0]).unwrap_or_else(|| { eprintln!("expected canonical IMASM numeral word"); std::process::exit(2) });
    let cap = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(500_000u64);
    let a = from_u32(2);
    
    println!("quantum_phase input bits={}", n.bit_len());
    
    // Floyd cycle detection on squaring map x -> x^2 mod N
    let mut t = sq(&a, &n);  // a^(2^1)
    let mut h = sq(&t, &n);  // a^(2^2)
    let mut steps: u64 = 1;
    let mut met = false;
    
    while steps < cap {
        t = sq(&t, &n);
        h = sq(&sq(&h, &n), &n);
        steps += 1;
        if cmp(&t, &h) == Equal { met = true; break; }
    }
    
    if !met { 
        println!("  no cycle within {cap} squarings"); 
        return; 
    }
    
    println!("  cycle at step {steps}: a^(2^{steps}) == a^(2^{})", 2*steps);
    
    // Build R = 2^steps * (2^steps - 1) as multiple of order
    let two = from_u32(2);
    let mut pow2 = one(); 
    for _ in 0..steps { pow2 = mul(&pow2, &two); }
    let odd = sub(&pow2, &one());
    let r_mult = mul(&pow2, &odd);
    
    match try_factor(&a, &r_mult, &n) {
        Some((p, q)) => println!("  {} = {} x {}  (quantum phase membrane)", to_word(&n), to_word(&p), to_word(&q)),
        None => println!("  R=2^{steps}·(2^{steps}-1) failed to yield nontrivial gcd"),
    }
}
