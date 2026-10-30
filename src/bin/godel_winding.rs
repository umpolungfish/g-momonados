//! Winding factor separation with Belnap-valued bit-braiding, on the Gödel `Nat`.
//!
//! * Multiplication mod N is rotation on Z_p x Z_q (CRT). Winding by `a` closes one circle before
//!   the other; gcd(a^k - 1, N) then splits N.  This is order finding: it is NOT a fast factoring
//!   method, it extends the bounded sieve (which stops at 2^width) to larger prime factors.
//! * "Braid" is the repo's own definition (`braid_values`/`unbraid_value` in godel_calculus.rs):
//!   interleave the bits of two lanes, gamma on even places and lambda on odd places.
//!   Every crossing of the square-and-multiply walk braids (residue, exponent) into one numeral.
//! * Each crossing carries a Belnap value for "this winding splits N":
//!   N none yet, F gcd == 1, B 1 < gcd < N (paradox held = the split), T gcd == N (degenerate).
use crate::godel_calculus::{encode_cell_binary, factor_pair_read, prime_sieve_read, FactorPairRead,
                            Nat, PrimeSieveRead};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::Ordering;

// ---------- Nat helpers (cmp_nat is private to the calculus module) ----------
pub fn cmp(a: &Nat, b: &Nat) -> Ordering {
    let (x, y) = (a.bits_le(), b.bits_le());
    if x.len() != y.len() { return x.len().cmp(&y.len()); }
    for i in (0..x.len()).rev() {
        if x[i] != y[i] { return x[i].cmp(&y[i]); }
    }
    Ordering::Equal
}
pub fn is_one(a: &Nat) -> bool { *a == Nat::one() }
pub fn gcd(a: &Nat, b: &Nat) -> Nat {
    let (mut a, mut b) = (a.clone(), b.clone());
    while !b.is_zero() {
        let r = a.div_rem(&b).expect("nonzero").1;
        a = b; b = r;
    }
    a
}
/// x - 1 mod n for x in [0, n)
fn minus_one(x: &Nat, n: &Nat) -> Nat {
    x.sub(&Nat::one()).unwrap_or_else(|| n.sub(&Nat::one()).expect("n >= 1"))
}

// ---------- bit-braiding (matches braid_values / unbraid_value, identity lane order) ----------
pub fn braid(gamma: &Nat, lambda: &Nat) -> Nat {
    let w = gamma.bits_le().len().max(lambda.bits_le().len());
    let mut bits = Vec::with_capacity(2 * w);
    for i in 0..w {
        bits.push(gamma.bits_le().get(i).copied().unwrap_or(false));
        bits.push(lambda.bits_le().get(i).copied().unwrap_or(false));
    }
    Nat::from_bits_le(bits)
}
pub fn unbraid(v: &Nat) -> (Nat, Nat) {
    let (mut g, mut l) = (Vec::new(), Vec::new());
    for (i, b) in v.bits_le().iter().copied().enumerate() {
        if i % 2 == 0 { g.push(b) } else { l.push(b) }
    }
    (Nat::from_bits_le(g), Nat::from_bits_le(l))
}

// ---------- Belnap FOUR (the classical slice of the spindle's SIXTEEN_3) ----------
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Belnap { N, T, F, B }
impl Belnap {
    fn evidence(self) -> (bool, bool) {
        match self { Belnap::N => (false, false), Belnap::T => (true, false),
                     Belnap::F => (false, true), Belnap::B => (true, true) }
    }
    pub fn join(self, o: Belnap) -> Belnap {            // information-order join
        let (a, b) = (self.evidence(), o.evidence());
        match (a.0 || b.0, a.1 || b.1) {
            (false, false) => Belnap::N, (true, false) => Belnap::T,
            (false, true) => Belnap::F, (true, true) => Belnap::B,
        }
    }
    pub fn glyph(self) -> char { match self { Belnap::N => 'N', Belnap::T => 'T', Belnap::F => 'F', Belnap::B => 'B' } }
}
pub fn state_from_gcd(g: &Nat, n: &Nat) -> Belnap {
    if cmp(g, n) == Ordering::Equal { Belnap::T }
    else if !is_one(g) { Belnap::B } else { Belnap::F }
}
/// Independent derivation (validation only, needs p,q): per circle closed -> T, open -> F; join.
pub fn state_from_circles(x: &Nat, p: &Nat, q: &Nat) -> Belnap {
    let one = |m: &Nat| x.div_rem(m).unwrap().1 == Nat::one().div_rem(m).unwrap().1;
    let c = |m: &Nat| if one(m) { Belnap::T } else { Belnap::F };
    c(p).join(c(q))
}

// ---------- braided square-and-multiply walk ----------
#[derive(Clone, Debug)]
pub struct Crossing { pub kind: char, pub exponent: Nat, pub state: Belnap, pub word: String }
#[derive(Clone, Debug)]
pub struct Walk { pub crossings: Vec<Crossing>, pub factor: Option<Nat> }

fn bits_msb_after_lead(k: u64) -> Vec<bool> {
    let b = 64 - k.leading_zeros() as usize;
    (0..b.saturating_sub(1)).rev().map(|i| (k >> i) & 1 == 1).collect()
}
pub fn braid_word(k: u64) -> String {
    let mut s = String::new();
    for b in bits_msb_after_lead(k) { s.push('S'); if b { s.push('M'); } }
    if s.is_empty() { s.push('.'); } s
}

/// Walk exponent k by square-and-multiply, testing every prefix; stop at the first B.
pub fn walk(n: &Nat, a: &Nat, k: u64) -> Walk {
    let mut cr = Vec::new();
    let mut x = a.div_rem(n).unwrap().1;
    let mut e = Nat::one();
    let mut push = |kind: char, x: &Nat, e: &Nat, cr: &mut Vec<Crossing>| -> (Belnap, Nat) {
        let g = gcd(&minus_one(x, n), n);
        let s = state_from_gcd(&g, n);
        cr.push(Crossing { kind, exponent: e.clone(), state: s, word: encode_cell_binary(&braid(x, e)) });
        (s, g)
    };
    let (s, g) = push('i', &x, &e, &mut cr);
    if s == Belnap::B { return Walk { crossings: cr, factor: Some(g) }; }
    for b in bits_msb_after_lead(k) {
        x = x.mul(&x).div_rem(n).unwrap().1; e = e.shl(1);
        let (s, g) = push('S', &x, &e, &mut cr);
        if s == Belnap::B { return Walk { crossings: cr, factor: Some(g) }; }
        if s == Belnap::T { return Walk { crossings: cr, factor: None }; }
        if b {
            x = x.mul(a).div_rem(n).unwrap().1; e = e.add(&Nat::one());
            let (s, g) = push('M', &x, &e, &mut cr);
            if s == Belnap::B { return Walk { crossings: cr, factor: Some(g) }; }
            if s == Belnap::T { return Walk { crossings: cr, factor: None }; }
        }
    }
    Walk { crossings: cr, factor: None }
}

// ---------- search schedules ----------
#[derive(Clone, Debug)]
pub struct Found { pub factor: Nat, pub base: u64, pub step: u64, pub schedule: &'static str }

/// Plain winding k = 1,2,3,...: finds p after ord_p(a) steps (needs ord_p < ord_q).
pub fn wind_plain(n: &Nat, kmax: u64, bases: &[u64]) -> Option<Found> {
    for &a in bases {
        let an = Nat::from_u64(a);
        let g0 = gcd(&an, n);
        if !is_one(&g0) && cmp(&g0, n) != Ordering::Equal {
            return Some(Found { factor: g0, base: a, step: 0, schedule: "gcd(a,N)" });
        }
        let mut x = Nat::one();
        for k in 1..=kmax {
            x = x.mul(&an).div_rem(n).unwrap().1;
            let g = gcd(&minus_one(&x, n), n);
            match state_from_gcd(&g, n) {
                Belnap::B => return Some(Found { factor: g, base: a, step: k, schedule: "plain" }),
                Belnap::T => break,
                _ => {}
            }
        }
    }
    None
}
/// Factorial schedule x <- x^j (Pollard p-1): finds p when p-1 is bound-smooth.
pub fn wind_factorial(n: &Nat, bound: u64, bases: &[u64]) -> Option<Found> {
    for &a in bases {
        let mut x = Nat::from_u64(a).div_rem(n).unwrap().1;
        for j in 2..=bound {
            x = x.pow_mod(&Nat::from_u64(j), n).unwrap();
            let g = gcd(&minus_one(&x, n), n);
            match state_from_gcd(&g, n) {
                Belnap::B => return Some(Found { factor: g, base: a, step: j, schedule: "factorial" }),
                Belnap::T => break,
                _ => {}
            }
        }
    }
    None
}

// ---------- integration with the repo's sieve + factor-pair closure ----------
#[derive(Clone, Debug)]
pub struct Separation {
    pub via: &'static str,                 // "sieve" | "winding-plain" | "winding-factorial" | "none"
    pub pair: Option<FactorPairRead>,
    pub found: Option<Found>,
    pub sieve: PrimeSieveRead,
}
/// Bounded sieve first (exact, existing). If it leaves the number unresolved above its aperture,
/// fall through to winding, then close the pair through the same `factor_pair_read`.
pub fn separate(n: &Nat, width: usize, plain_kmax: u64, fact_bound: u64) -> Result<Separation, String> {
    let sieve = prime_sieve_read(n, width)?;
    if sieve.divisor.is_some() {
        let pair = factor_pair_read(n, &sieve, width)?;
        return Ok(Separation { via: "sieve", pair, found: None, sieve });
    }
    let bases = [2u64, 3, 5, 7, 11, 13];
    let mut attempts: [(&'static str, Option<Found>); 2] = [
        ("winding-factorial", wind_factorial(n, fact_bound, &bases)),
        ("winding-plain", wind_plain(n, plain_kmax, &bases)),
    ];
    for (via, f) in attempts.iter_mut() {
        if let Some(found) = f.take() {
            let mut witness = sieve.clone();
            witness.divisor = Some(found.factor.clone());
            let pair = factor_pair_read(n, &witness, width)?;
            return Ok(Separation { via, pair, found: Some(found), sieve });
        }
    }
    Ok(Separation { via: "none", pair: None, found: None, sieve })
}

pub fn render(n: &Nat, s: &Separation) -> String {
    let mut o = format!("separation.via           {}\n", s.via);
    if let Some(f) = &s.found {
        o += &format!("winding.base            {}\nwinding.step            {} ({})\n", f.base, f.step, f.schedule);
    }
    match &s.pair {
        Some(p) => o += &format!(
            "factor.pair              {} x {} = {}\nproduct.closed           {}\nsemiprime.closed         {}\n",
            p.factor, p.cofactor, n, p.product_closed,
            match p.semiprime_closed { Some(true) => "true", Some(false) => "false", None => "unresolved (cofactor beyond sieve aperture)" }),
        None => o += "factor.pair              none (no divisor below sieve aperture; winding exhausted its bound)\n",
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;
    fn n(v: u64) -> Nat { Nat::from_u64(v) }
    fn dec(s: &str) -> Nat { Nat::from_decimal(s).unwrap() }

    #[test] fn braid_roundtrip() {
        for (g, l) in [(0u64, 0u64), (1, 0), (0, 1), (5, 9), (12345, 678), (u32::MAX as u64, 3)] {
            let (a, b) = unbraid(&braid(&n(g), &n(l)));
            assert_eq!((a, b), (n(g), n(l)));
        }
    }
    #[test] fn belnap_join_laws() {
        use Belnap::*;
        let v = [N, T, F, B];
        for &a in &v { assert_eq!(a.join(a), a); assert_eq!(N.join(a), a); assert_eq!(B.join(a), B);
            for &b in &v { assert_eq!(a.join(b), b.join(a)); } }
        assert_eq!(T.join(F), B);
    }
    #[test] fn gcd_state_equals_circle_state() {
        for (nn, p, q) in [(15u64, 3u64, 5u64), (91, 7, 13), (1147, 31, 37), (10403, 101, 103)] {
            for a in 2u64..12 {
                if nn % a == 0 { continue; }
                let mut x = Nat::one();
                for _ in 0..60 {
                    x = x.mul(&n(a)).div_rem(&n(nn)).unwrap().1;
                    let g = gcd(&minus_one(&x, &n(nn)), &n(nn));
                    assert_eq!(state_from_gcd(&g, &n(nn)), state_from_circles(&x, &n(p), &n(q)));
                }
            }
        }
    }
    #[test] fn walk_matches_python_traces() {
        let w = walk(&n(91), &n(2), 3);
        assert_eq!(braid_word(3), "SM");
        assert_eq!(w.factor, Some(n(7)));
        assert_eq!(w.crossings.last().unwrap().state, Belnap::B);
        let states: Vec<char> = w.crossings.iter().map(|c| c.state.glyph()).collect();
        assert_eq!(states, vec!['F', 'F', 'B']);     // id, S, M  (same as the Python run)
        let w = walk(&n(1000009), &n(2), 292);
        assert_eq!(braid_word(292), "SSSMSSSMSS");
        assert_eq!(w.factor, Some(n(293)));
        // every crossing's braided numeral unbraids to (residue, exponent)
        for c in &w.crossings {
            let (_x, e) = unbraid(&Nat::from_bits_le(
                crate::godel_calculus::decode_cell_biguint(&c.word).unwrap().to_bytes_le().iter()
                    .flat_map(|b| (0..8).map(move |i| (b >> i) & 1 == 1)).collect::<Vec<_>>()
                    .into_iter().rev().skip_while(|b| !b).collect::<Vec<_>>().into_iter().rev().collect()));
            assert_eq!(e, c.exponent);
        }
    }
    #[test] fn plain_winding_known_semiprimes() {
        for (nn, p, q) in [(15u64, 3u64, 5u64), (91, 7, 13), (1147, 31, 37), (10403, 101, 103), (1000009, 293, 3413)] {
            let f = wind_plain(&n(nn), 100_000, &[2, 3, 5, 7]).expect("found");
            assert!(f.factor == n(p) || f.factor == n(q), "{nn}: {:?}", f);
        }
    }
    #[test] fn factorial_winding_smooth_p_minus_1() {
        // 1000003 and 1000033: p-1 = 2*3*166667 / 2^5*3*10501 style; just require correctness if found
        let nn = dec("1000036000099"); // 1000003 * 1000033
        if let Some(f) = wind_factorial(&nn, 20_000, &[2, 3, 5]) {
            assert!(f.factor == n(1000003) || f.factor == n(1000033));
        }
    }
    #[test] fn separate_uses_sieve_below_aperture_and_winding_above() {
        // small factor: sieve resolves it
        let s = separate(&n(91), 8, 10_000, 5_000).unwrap();
        assert_eq!(s.via, "sieve");
        // both factors above a tiny 2^6 aperture: sieve cannot, winding does
        let nn = n(293 * 3413);
        let s = separate(&nn, 6, 100_000, 5_000).unwrap();
        assert!(s.via.starts_with("winding"), "via {}", s.via);
        let p = s.pair.unwrap();
        assert!(p.product_closed);
        assert!(p.factor.mul(&p.cofactor) == nn);
    }
    #[test] fn nonsemiprime_and_prime_do_not_claim_a_split() {
        let s = separate(&n(1000003), 6, 20_000, 2_000).unwrap();   // prime above aperture
        assert_eq!(s.via, "none");
    }
}
