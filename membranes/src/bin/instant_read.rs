// ⊣⊣⊙∈≻⊤≺⊥⊞⋈∋⊡ — BOTH arms. ∈ bifurcates the group state into a forward leaper
// (≻⊤) and a reverse/adjoint leaper (≺⊥); ⊞ holds both live until they collide;
// ∋ fuses the collision into the order (difference of accumulated exponents);
// ⊡ fixes it. Random large jumps a^{s_i} give a birthday collision in O(sqrt r),
// O(1) memory — not the O(r) single-arm walk. Factor by gcd(a^{r/2}±1, N).
use membranes::{Big, from_dec, to_dec, mul, sub, add, divmod, from_u32, cmp, is_zero};
use core::cmp::Ordering::*;
fn one() -> Big { from_u32(1) }
fn mulmod(a: &Big, b: &Big, n: &Big) -> Big { divmod(&mul(a, b), n).1 }
fn gcd(a: &Big, b: &Big) -> Big {
    let (mut x, mut y) = (a.clone(), b.clone());
    while !is_zero(&y) { let r = divmod(&x, &y).1; x = y; y = r; }
    x
}
fn powmod(a: &Big, e: &Big, n: &Big) -> Big {
    let mut r = one();
    let mut base = divmod(a, n).1;
    let mut ee = e.clone();
    let two = from_u32(2);
    while !is_zero(&ee) {
        let (q, rem) = divmod(&ee, &two);
        if !is_zero(&rem) { r = mulmod(&r, &base, n); }
        base = mulmod(&base, &base, n);
        ee = q;
    }
    r
}
fn low(b: &Big) -> u64 { b.iter().rev().take(2).fold(0u64, |acc, &l| (acc << 32) | l as u64) }
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let n = from_dec(&args[0]);
    let a = from_u32(2);
    const B: usize = 64;
    let mut seed: u64 = 0x2545F4914F6CDD1D;
    let mut jexp = vec![0u64; B];
    let mut jump = vec![one(); B];
    for i in 0..B {
        seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17;
        let s = (seed >> 1) | 1;
        jexp[i] = s;
        jump[i] = powmod(&a, &from_dec(&s.to_string()), &n);
    }
    let bucket = |x: &Big| -> usize { (low(x) % (B as u64)) as usize };
    let (mut tx, mut te) = (a.clone(), 1u128);
    let (mut hx, mut he) = (a.clone(), 1u128);
    let mut ticks: u64 = 0;
    let mut order: Option<u128> = None;
    loop {
        ticks += 1;
        let b = bucket(&tx); tx = mulmod(&tx, &jump[b], &n); te += jexp[b] as u128;
        for _ in 0..2 { let b = bucket(&hx); hx = mulmod(&hx, &jump[b], &n); he += jexp[b] as u128; }
        if cmp(&tx, &hx) == Equal {
            let d = if te > he { te - he } else { he - te };
            if d > 0 { order = Some(d); }
            break;
        }
        if ticks > (1u64 << 34) { break; }
    }
    println!("instant_read ⊣⊣⊙∈≻⊤≺⊥⊞⋈∋⊡  N={}", to_dec(&n));
    match order {
        Some(r) => {
            println!("  collision at {ticks} leaps -> r*k = {r}  (a^(r*k) = 1 mod N)");
            let rb = from_dec(&r.to_string());
            let mut divs = vec![rb.clone()];
            let two = from_u32(2);
            let mut t = rb.clone();
            loop {
                let (q, rem) = divmod(&t, &two);
                if is_zero(&rem) { t = q; divs.push(t.clone()); } else { break; }
            }
            for e in &divs {
                let h = powmod(&a, e, &n);
                let cand = [ if cmp(&h, &one()) != Less { sub(&h, &one()) } else { one() }, add(&h, &one()) ];
                for pm in cand {
                    let g = gcd(&pm, &n);
                    if cmp(&g, &one()) == Greater && cmp(&g, &n) == Less {
                        let (co, _) = divmod(&n, &g);
                        println!("  {} = {} x {}", to_dec(&n), to_dec(&g), to_dec(&co));
                        return;
                    }
                }
            }
            println!("  order multiple found but gcd trivial for base 2 -- needs another base");
        }
        None => println!("  no collision within leap budget ({ticks} leaps)"),
    }
}
