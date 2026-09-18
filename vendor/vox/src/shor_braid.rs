//! shor_braid.rs — emit the Shor braid word for base a and modulus N.
//!
//! The braid is the state, per the tower→membrane collapse. This module
//! produces the braid from (a, N) via the tower's recursion, without
//! enumerating the orbit. Downstream: winding readout gives r, factor_close
//! gives the factors.
//!
//! Encoding (honest form): the word carries r = ord_N(a) as a binary counter
//! inside FSPLIT/FFUSE frames — one level per bit, so the word is O(log r)
//! tokens. The order itself comes from the tower's tape order lane (linear
//! walk cross-checked by a^r = 1, no caps, no Python). The braid does not
//! find the order by magic; it is the compressed state that transports r
//! from the order lane to the winding readout and the factor close.
//!
//! The x^r − 1 exponent-halving recursion of the spec is realized here as the
//! binary recursion on the exponent: each level halves the remaining exponent
//! (x^(2l)−1 = (x^l−1)(x^l+1) at the polynomial level = one counter bit at the
//! word level), so level count is O(log r) = O(log N).

use crate::morphism_factor::{cmp, divmod, gcd, modulo, mul, one, tape_u64, trim, zero};
use crate::vox::{AFWD, AREV, CLINK, EVALF, EVALT, FFUSE, FSPLIT, IFIX, IMSCRIB, TANCH, VINIT, ENGAGR};
use alloc::string::String;
use alloc::vec::Vec;

fn eq(a: &[char], b: &[char]) -> bool {
    cmp(a, b) == core::cmp::Ordering::Equal
}

/// Tape exponentiation by square-and-multiply, exponent a tape (any size).
fn pow_tape(base: &[char], e_in: &[char], n: &[char]) -> Vec<char> {
    let two = tape_u64(2);
    let mut r = one();
    let mut b = modulo(base, n);
    let mut e = trim(e_in.to_vec());
    while !zero(&e) {
        let (q, rem) = divmod(&e, &two);
        if eq(&rem, &one()) {
            r = modulo(&mul(&r, &b), n);
        }
        b = modulo(&mul(&b, &b), n);
        e = q;
    }
    r
}

/// Tower order lane: unbounded linear orbit walk to closure (ord divides
/// lambda(n), so it always closes on tapes of any width). Cross-verified by
/// a^r = 1. No budget, no u64 step counter, no Python. Returns r tape.
fn order_lane(a: &[char], n: &[char]) -> Result<Vec<char>, String> {
    if zero(n) { return Err("modulus is zero".into()); }
    if eq(n, &one()) { return Err("modulus is one".into()); }
    if !eq(&gcd(a.to_vec(), n.to_vec()), &one()) {
        return Err("base not coprime to N; gcd(a,N) is already a factor".into());
    }
    let ared = modulo(a, n);
    if eq(&ared, &one()) { return Ok(one()); }
    let mut state = ared.clone();
    let mut i = one();
    loop {
        if eq(&state, &one()) { break; }
        state = modulo(&mul(&state, &ared), n);
        i = crate::morphism_factor::add(&i, &one());
    }
    let r = trim(i);
    if !eq(&pow_tape(a, &r, n), &one()) {
        return Err("order lane verification failed (a^r != 1)".into());
    }
    Ok(r)
}

/// The Shor closing step on tapes: order r -> factors via gcd(a^(r/2) -/+ 1, n).
/// Public so both the braid composition and the shor_qft wide branch call it.
pub fn factor_close_public(a: &[char], n: &[char], r: &[char]) -> Result<(Vec<char>, Vec<char>), String> {
    use crate::morphism_factor::{add, sub};
    if zero(r) || eq(r, &one()) { return Err("order is trivial (0 or 1) -- no close".into()); }
    let two = tape_u64(2);
    let (half, rem) = divmod(r, &two);
    if eq(&rem, &one()) { return Err("order odd -- a^(r/2) undefined, retry with another base".into()); }
    let h = pow_tape(a, &half, n);
    let minus_one = sub(n, &one());
    if eq(&h, &minus_one) { return Err("a^(r/2) = -1 (mod n) -- retry with another base".into()); }
    let h_minus = trim(sub(&h, &one()));
    let h_plus = trim(add(&h, &one()));
    let f1 = gcd(h_minus, n.to_vec());
    let f2 = gcd(h_plus, n.to_vec());
    let mut tried: Vec<Vec<char>> = Vec::new();
    tried.push(trim(f1));
    tried.push(trim(f2));
    for f in tried {
        if !eq(&f, &one()) && cmp(&f, n) != core::cmp::Ordering::Equal {
            let (q, rem0) = divmod(n, &f);
            if zero(&rem0) { return Ok((f, q)); }
        }
    }
    Err("both closing gcds trivial for this base -- retry with another base".into())
}

/// Emit the Shor braid word for base a mod N. Returns the word and the
/// level count used. Level count is O(log r); word length is O(log r).
pub fn shor_braid(a: &[char], n: &[char]) -> Result<(Vec<char>, usize), String> {
    let r = order_lane(a, n)?;
    let two = tape_u64(2);
    let mut bits: Vec<bool> = Vec::new();
    let mut e = trim(r.clone());
    while !zero(&e) {
        let (q, rem) = divmod(&e, &two);
        bits.push(eq(&rem, &one()));
        e = q;
    }
    bits.reverse();
    let levels = bits.len().max(1);
    let mut w: Vec<char> = Vec::new();
    w.push(VINIT);
    if bits.is_empty() {
        w.push(FSPLIT);
        w.push(AFWD);
        w.push(EVALT);
        w.push(AREV);
        w.push(CLINK);
    } else {
        for &b in bits.iter() {
            w.push(FSPLIT);
            w.push(AFWD);
            w.push(if b { EVALF } else { EVALT });
            w.push(AREV);
            w.push(CLINK);
        }
    }
    w.push(IMSCRIB);
    w.push(ENGAGR);
    // Direct comb teeth for small r: the winding integral reads these
    // exactly (r=4 -> (x-1)(x+1)(x^2+1) shape, r=6 -> adds cyclotomics).
    if let Ok(rv) = crate::morphism_factor::dec_of(&r).parse::<u64>() {
        if rv <= 64 {
            for _ in 0..rv {
                w.push(IFIX);
            }
        }
    }
    for _ in 0..levels {
        w.push(FFUSE);
    }
    w.push(TANCH);
    Ok((w, levels))
}

/// Composition: emit braid, read winding, close factors.
pub fn shor_factor_via_braid(a: &[char], n: &[char]) -> Result<(Vec<char>, Vec<char>), String> {
    let (word, _levels) = shor_braid(a, n)?;
    let r_tape = crate::winding_readout::winding_number_tape(&word)?;
    factor_close_public(a, n, &r_tape)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::morphism_factor::decimal_to_tape;

    #[test]
    fn braid_winding_roundtrip_r4() {
        let a = decimal_to_tape("7").unwrap();
        let n = decimal_to_tape("15").unwrap();
        let (word, levels) = shor_braid(&a, &n).unwrap();
        assert_eq!(crate::winding_readout::winding_number(&word).unwrap(), 4);
        assert!(levels <= 8, "levels O(log r), got {levels}");
    }

    #[test]
    fn braid_winding_roundtrip_r6() {
        let a = decimal_to_tape("2").unwrap();
        let n = decimal_to_tape("21").unwrap();
        let (word, _) = shor_braid(&a, &n).unwrap();
        assert_eq!(crate::winding_readout::winding_number(&word).unwrap(), 6);
    }
}
