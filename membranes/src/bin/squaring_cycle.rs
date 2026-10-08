// The phase-kick chain a^(2^k) mod N (one squaring per k, O(log) work each).
// Floyd-detect the cycle of the squaring map x -> x^2 mod N started at a.
// A collision a^(2^i) == a^(2^j) means 2^i ≡ 2^j (mod r), so r | 2^j·(2^(i-j) - 1).
// The tail gives the 2-adic part, the loop gives the odd part. Reconstruct a
// multiple R of the order from (i, j) and factor by gcd(a^(R/2)±1, N).
use membranes::{Big, from_word, to_word, mul, sub, add, divmod, from_u32, cmp, is_zero};
use core::cmp::Ordering::*;
fn one() -> Big { from_u32(1) }
fn mulmod(a: &Big, b: &Big, n: &Big) -> Big { divmod(&mul(a, b), n).1 }
fn sq(x: &Big, n: &Big) -> Big { mulmod(x, x, n) }
fn gcd(a: &Big, b: &Big) -> Big { let (mut x, mut y) = (a.clone(), b.clone()); while !is_zero(&y) { let r = divmod(&x, &y).1; x = y; y = r; } x }
fn powmod(a: &Big, e: &Big, n: &Big) -> Big {
    let mut r = one(); let mut base = divmod(a, n).1; let mut ee = e.clone(); let two = from_u32(2);
    while !is_zero(&ee) { let (q, rem) = divmod(&ee, &two); if !is_zero(&rem) { r = mulmod(&r, &base, n); } base = mulmod(&base, &base, n); ee = q; }
    r
}
fn try_factor(a: &Big, r_mult: &Big, n: &Big) -> Option<(Big, Big)> {
    // strip 2s from R, at each even exponent try the gcd
    let two = from_u32(2);
    let mut e = r_mult.clone();
    let mut guard = 0;
    loop {
        let h = powmod(a, &e, n);
        if cmp(&h, &one()) == Equal || cmp(&h, &sub(n, &one())) == Equal { /* trivial sqrt, keep stripping */ }
        else {
            for pm in [ if cmp(&h,&one())!=Less {sub(&h,&one())} else {one()}, add(&h,&one()) ] {
                let g = gcd(&pm, n);
                if cmp(&g,&one())==Greater && cmp(&g,n)==Less { let (co,_) = divmod(n, &g); return Some((g, co)); }
            }
        }
        let (q, rem) = divmod(&e, &two);
        if !is_zero(&rem) || is_zero(&q) { break; }
        e = q; guard += 1; if guard > 4096 { break; }
    }
    None
}
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args.len() > 2 {
        eprintln!("usage: membrane_squaring_cycle <canonical-IMASM-N> [squaring-cap]");
        std::process::exit(2);
    }
    let n = from_word(&args[0]).unwrap_or_else(|| { eprintln!("expected canonical IMASM numeral word"); std::process::exit(2) });
    let cap = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(200_000u64);
    let a = from_u32(2);
    // Floyd on the squaring map
    let mut t = sq(&a, &n);          // a^(2^1)
    let mut h = sq(&t, &n);          // a^(2^2)
    let mut steps: u64 = 1;
    let mut met = false;
    while steps < cap { t = sq(&t, &n); h = sq(&sq(&h, &n), &n); steps += 1; if cmp(&t,&h)==Equal { met = true; break; } }
    println!("squaring_cycle input bits={}", n.bit_len());
    if !met { println!("  no squaring-cycle collision within {cap} steps"); return; }
    println!("  squaring map cycled after {steps} squarings (tortoise 2^{steps} == hare 2^{})", 2*steps);
    // collision at exponents 2^steps and 2^(2*steps): r | (2^(2steps) - 2^steps) = 2^steps*(2^steps - 1)
    // build R = 2^steps * (2^steps - 1) as a Big and factor
    let two = from_u32(2);
    let mut pow2 = one(); for _ in 0..steps { pow2 = mul(&pow2, &two); }   // 2^steps
    let odd = sub(&pow2, &one());                                          // 2^steps - 1
    let r_mult = mul(&pow2, &odd);                                         // multiple of r
    match try_factor(&a, &r_mult, &n) {
        Some((p,q)) => println!("  {} = {} x {}", to_word(&n), to_word(&p), to_word(&q)),
        None => println!("  R = 2^{steps}·(2^{steps}-1) did not yield a nontrivial gcd (order not built from this collision)"),
    }
}
