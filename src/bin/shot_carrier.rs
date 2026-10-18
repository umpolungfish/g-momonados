//! shot_carrier.rs — BigUint N, arbitrary q, bitfolded Shor with
//! topological period readout via HSOA exceptional-point winding.
#![deny(warnings)]
use std::env;
use std::process;
use num_bigint::BigUint;
use num_traits::{One, Zero};
use vox::hsoa_winding;

fn mod_pow(mut a: BigUint, mut e: BigUint, n: &BigUint) -> BigUint {
    a %= n;
    let mut r = BigUint::one();
    while !e.is_zero() {
        if e.bit(0) { r = (&r * &a) % n; }
        a = (&a * &a) % n;
        e >>= 1u64;
    }
    r
}

fn gcd(mut a: BigUint, mut b: BigUint) -> BigUint {
    while !b.is_zero() { let t = &a % &b; a = b; b = t; }
    a
}

fn factor_from_order(a: &BigUint, n: &BigUint, r: u64) -> Option<(BigUint, BigUint)> {
    if r == 0 || r % 2 != 0 { return None; }
    let half = mod_pow(a.clone(), BigUint::from(r / 2), n);
    let one = BigUint::one();
    let n_minus_1 = n - &one;
    if half == n_minus_1 { return None; }
    let g1 = gcd((&half + &n_minus_1) % n, n.clone());
    if g1 > one && g1 < *n { return Some((g1.clone(), n.clone() / &g1)); }
    let g2 = gcd((&half + &one) % n, n.clone());
    if g2 > one && g2 < *n { return Some((g2.clone(), n.clone() / &g2)); }
    None
}

fn small_factor(n: &BigUint, bound: u64) -> Option<BigUint> {
    if !n.bit(0) { return Some(BigUint::from(2u64)); }
    let mut p = 3u64;
    while p <= bound {
        let pb = BigUint::from(p);
        if &pb * &pb > *n { break; }
        if (n % &pb).is_zero() { return Some(pb); }
        p += 2;
    }
    None
}

fn try_topological(a: &BigUint, n: &BigUint, m: usize)
    -> Option<(BigUint, BigUint, u64, u64)>
{
    let g = gcd(a.clone(), n.clone());
    if g > BigUint::one() && g < *n {
        return Some((g.clone(), n.clone() / &g, 0, 0));
    }
    let comb = hsoa_winding::build_comb(a, n, m);
    let readout = hsoa_winding::read_period(&comb)?;
    if !readout.validated { return None; }
    let (p, f) = factor_from_order(a, n, readout.period)?;
    Some((p, f, readout.period, readout.winding))
}

fn main() {
    let raw: Vec<String> = env::args().skip(1).collect();
    if raw.len() < 2 {
        eprintln!("usage: shot_carrier <N> <q>   (q < 63)");
        process::exit(2);
    }
    let n = match BigUint::parse_bytes(raw[0].as_bytes(), 10) {
        Some(v) if !v.is_zero() && !v.is_one() => v,
        _ => { eprintln!("bad N: {:?}", raw[0]); process::exit(2); }
    };
    let q: usize = match raw[1].parse() {
        Ok(v) if v < 63 => v,
        _ => { eprintln!("bad q (0 < q < 63): {:?}", raw[1]); process::exit(2); }
    };
    let t0 = std::time::Instant::now();
    let m = 1usize << q;

    println!("N = {}", n);
    println!("q = {}, m = {}, carrier = {} B",
        q, m, (m as u128) * 16);

    print!("\n[1] trial division < 10^7 ... ");
    if let Some(p) = small_factor(&n, 10_000_000) {
        let qq = &n / &p;
        println!("FOUND  N = {} × {}", p, qq);
        println!("Time: {:?}", t0.elapsed());
        return;
    }
    println!("none");

    println!("[2] topological readout  (m={})", m);
    let bases: [u64; 8] = [2, 3, 5, 7, 11, 13, 17, 19];
    for &a in &bases {
        let ab = BigUint::from(a);
        print!("  a={:>2}: ", a);
        std::io::Write::flush(&mut std::io::stdout()).ok();
        match try_topological(&ab, &n, m) {
            Some((p, f, r, w)) => {
                println!("r={} W={}  => {} × {}", r, w, p, f);
                println!("\nFOUND: N = {} × {}", p, f);
                println!("Time: {:?}", t0.elapsed());
                return;
            }
            None => println!("—"),
        }
    }

    println!("\n[FAIL] no factor with m={}. Register too small for r; increase q.", m);
    println!("Time: {:?}", t0.elapsed());
}