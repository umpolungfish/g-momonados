//! factor_operator.rs — the CL9NK moat resolver over IMASM numeral tapes.
//!
//! Ported from G-mOMonadOS `factor_operator.rs`. The primitive is not "search
//! for a factor" but "construct the factor pair as the fixed point of a
//! 2-adic winding". p and q are built one bit at a time from the low end; at
//! each level the only branches kept are those where p*q agrees with N on the
//! low k+1 bits (the exact winding constraint), pruned past sqrt(N) as the
//! magnitude bound, and a completion that closes on p*q = N is the fixation.
//! The count of branch nodes walked is the width of the moat, the free factor
//! bits held as paradox, read off the run rather than asserted.
//!
//! Arithmetic is the shared bit-register-folded numeral kernel in
//! morphism_factor, so the moat walk runs on folded tapes.

use crate::morphism_factor::{cmp, isqrt, miller_rabin, modulo, mul, parse_numeral, emit_numeral, trim};
use crate::vox::{EVALF, EVALT};
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

type Tape = Vec<char>;

fn one() -> Tape { vec![EVALF] }
fn two() -> Tape { vec![EVALT, EVALF] }
fn is_zero(t: &[char]) -> bool { trim(t.to_vec()) == [EVALT] }
fn gt_one(t: &[char]) -> bool { cmp(t, &one()) == core::cmp::Ordering::Greater }

/// Return `base` with bit k set to v (LSB-first cells, padded with EVALT).
fn with_bit(base: &[char], k: usize, v: bool) -> Tape {
    let mut t = base.to_vec();
    while t.len() <= k {
        t.push(EVALT);
    }
    t[k] = if v { EVALF } else { EVALT };
    trim(t)
}

/// Do prod and n agree on bits 0..=upto?
fn low_bits_match(prod: &[char], n: &[char], upto: usize) -> bool {
    for i in 0..=upto {
        let a = prod.get(i).copied().unwrap_or(EVALT);
        let b = n.get(i).copied().unwrap_or(EVALT);
        if a != b {
            return false;
        }
    }
    true
}

/// Resolve the moat: fork each free bit into its two completions, keep only the
/// branch the winding constraint admits (p*q agrees with N on the low k+1
/// bits), bound by p,q <= sqrt(N), and lift until a completion closes on
/// p*q = N. Returns the factor pair, the branch-node count, and whether the
/// node budget capped the walk.
pub fn resolve_moat(n: &[char], max_nodes: u64) -> (Option<(Tape, Tape)>, u64, bool) {
    let root = isqrt(n);
    let bits = trim(n.to_vec()).len();
    let mut nodes: u64 = 0;
    let mut found: Option<(Tape, Tape)> = None;
    // DFS stack of (level k, p_low, q_low).
    let mut stack: Vec<(usize, Tape, Tape)> = vec![(0, vec![EVALT], vec![EVALT])];
    while let Some((k, plo, qlo)) = stack.pop() {
        if found.is_some() || nodes >= max_nodes {
            break;
        }
        if k >= bits + 1 {
            continue;
        }
        // Only p branches; q's bit k is wound, not searched. With p odd its low
        // bit is 1, so bit k of p*q flips exactly with q_k, and the congruence
        // p*q == N mod 2^{k+1} forces q_k = N_k XOR (p*q_low)_k. An even p leaves
        // q_k unable to fix bit k, so its branch fails the match and is pruned.
        for pk in [false, true] {
            let p = with_bit(&plo, k, pk);
            nodes += 1;
            let prod0 = mul(&p, &qlo);
            let nk = n.get(k).copied().unwrap_or(EVALT) == EVALF;
            let p0k = prod0.get(k).copied().unwrap_or(EVALT) == EVALF;
            let qk = nk ^ p0k;
            let q = with_bit(&qlo, k, qk);
            let prod = mul(&p, &q);
            if !low_bits_match(&prod, n, k) {
                continue;
            }
            if cmp(&prod, n) == core::cmp::Ordering::Equal && gt_one(&p) && gt_one(&q) {
                found = Some(if cmp(&p, &q) != core::cmp::Ordering::Greater {
                    (p.clone(), q.clone())
                } else {
                    (q.clone(), p.clone())
                });
            }
            // magnitude bound: p is the smaller factor, it cannot exceed sqrt(N).
            if cmp(&p, &root) == core::cmp::Ordering::Greater {
                continue;
            }
            stack.push((k + 1, p, q));
        }
    }
    let capped = nodes >= max_nodes;
    (found, nodes, capped)
}

/// Full factorization: strip the 2-part and small odd primes by the divisor
/// leg, then cross the moat for the balanced core. Returns the sorted factor
/// multiset, the total moat-node cost, and whether the budget held.
pub fn full_resolve(n0: &[char], small_bound: u64, max_nodes: u64) -> (Vec<Tape>, u64, bool) {
    let mut factors: Vec<Tape> = Vec::new();
    let mut moat_nodes: u64 = 0;
    let mut capped = false;
    let mut n = trim(n0.to_vec());
    // 2-part.
    while is_zero(&modulo(&n, &two())) {
        factors.push(two());
        n = crate::morphism_factor::divmod(&n, &two()).0;
    }
    // small odd primes to the cheap bound.
    let mut d = vec![EVALF, EVALF]; // 3
    let sb = crate::morphism_factor::tape_u64(small_bound);
    while cmp(&d, &sb) != core::cmp::Ordering::Greater
        && cmp(&mul(&d, &d), &n) != core::cmp::Ordering::Greater
    {
        while is_zero(&modulo(&n, &d)) {
            factors.push(d.clone());
            n = crate::morphism_factor::divmod(&n, &d).0;
        }
        d = crate::morphism_factor::add(&d, &two());
    }
    let sb2 = mul(&sb, &sb);
    let mut stack = vec![n];
    while let Some(c) = stack.pop() {
        if cmp(&c, &one()) != core::cmp::Ordering::Greater {
            continue;
        }
        if cmp(&c, &sb2) == core::cmp::Ordering::Less {
            factors.push(c);
            continue;
        }
        // A cheap primality test is nested before the moat: a prime core has no
        // pair, so crossing its whole moat just to conclude prime is the costly
        // walk in the wrong place. Recognize it first and never moat it.
        if miller_rabin(&c) {
            factors.push(c);
            continue;
        }
        let (res, nodes, cap) = resolve_moat(&c, max_nodes);
        moat_nodes += nodes;
        if cap {
            capped = true;
            factors.push(c);
            continue;
        }
        match res {
            Some((p, q)) => {
                stack.push(p);
                stack.push(q);
            }
            None => factors.push(c),
        }
    }
    factors.sort_by(|a, b| cmp(a, b));
    (factors, moat_nodes, capped)
}

fn to_dec(t: &[char]) -> String {
    // Small helper for the report: decode the tape to decimal via emit/parse
    // round-trip is not needed; render bits directly.
    let mut v = alloc::string::String::new();
    // Build decimal by repeated division by ten over tapes would be heavy; the
    // tape is already the answer, so print its decimal through a u128 when it
    // fits, else fall back to the numeral word.
    let bits = trim(t.to_vec());
    if bits.len() <= 127 {
        let mut acc: u128 = 0;
        for &c in bits.iter().rev() {
            acc = (acc << 1) | if c == EVALF { 1 } else { 0 };
        }
        v.push_str(&format!("{acc}"));
    } else {
        v.push_str(&emit_numeral(&bits));
    }
    v
}

pub fn repl_factor_operator(args: &[&str]) -> String {
    if args.is_empty() {
        return String::from(
            "factor_operator — the CL9NK moat resolver over folded tapes\n\
             resolve <N-word|decimal> [max_nodes]   cross the moat for the balanced core\n\
             full <N-word|decimal> [small_bound] [max_nodes]   strip then resolve, full multiset",
        );
    }
    let value = |s: &str| -> Option<Tape> {
        if s.starts_with('⊢') {
            parse_numeral(s).ok()
        } else {
            s.parse::<u128>().ok().map(|n| {
                if n == 0 {
                    vec![EVALT]
                } else {
                    let mut t = Vec::new();
                    let mut m = n;
                    while m != 0 {
                        t.push(if m & 1 == 1 { EVALF } else { EVALT });
                        m >>= 1;
                    }
                    t
                }
            })
        }
    };
    match args[0] {
        "resolve" => {
            let n = match args.get(1).and_then(|s| value(s)) {
                Some(v) => v,
                None => return String::from("bad N"),
            };
            let max_nodes = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(u64::MAX);
            if gt_one(&n) && miller_rabin(&n) {
                return format!("N={}\n  prime (witness, before the moat)", to_dec(&n));
            }
            let (res, nodes, capped) = resolve_moat(&n, max_nodes);
            match res {
                Some((p, q)) => format!(
                    "N={}\n  {} = {} x {}\n  moat nodes walked: {}",
                    to_dec(&n), to_dec(&n), to_dec(&p), to_dec(&q), nodes
                ),
                None if capped => format!("N={}\n  moat budget {} exhausted; core survives", to_dec(&n), max_nodes),
                None => format!("N={}\n  moat closed with no pair: prime\n  moat nodes walked: {}", to_dec(&n), nodes),
            }
        }
        "full" => {
            let n = match args.get(1).and_then(|s| value(s)) {
                Some(v) => v,
                None => return String::from("bad N"),
            };
            let sb = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1000u64);
            let max_nodes = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(u64::MAX);
            let (factors, nodes, capped) = full_resolve(&n, sb, max_nodes);
            let fs: Vec<String> = factors.iter().map(|f| to_dec(f)).collect();
            format!(
                "N={}\n  factors: {}\n  moat nodes walked: {}{}",
                to_dec(&n), fs.join(" x "), nodes,
                if capped { "  (budget capped; a core is left composite)" } else { "" }
            )
        }
        _ => String::from("unknown subcommand (resolve | full)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tape(mut n: u64) -> Tape {
        if n == 0 {
            return vec![EVALT];
        }
        let mut t = Vec::new();
        while n != 0 {
            t.push(if n & 1 == 1 { EVALF } else { EVALT });
            n >>= 1;
        }
        t
    }
    #[test]
    fn moat_resolves_semiprimes() {
        for &(n, _) in &[(91u64, 0), (8051, 0), (143, 0), (100160063, 0)] {
            let (res, _, _) = resolve_moat(&tape(n), 20_000_000);
            let (p, q) = res.expect("moat found nothing");
            let pv = { let mut a = 0u64; for &c in trim(p.clone()).iter().rev() { a = (a << 1) | if c == EVALF { 1 } else { 0 }; } a };
            let qv = { let mut a = 0u64; for &c in trim(q.clone()).iter().rev() { a = (a << 1) | if c == EVALF { 1 } else { 0 }; } a };
            assert_eq!(pv * qv, n, "moat({n}) = {pv} x {qv}");
            assert!(pv > 1 && qv > 1);
        }
    }
    #[test]
    fn moat_reports_prime() {
        let (res, _, capped) = resolve_moat(&tape(9973), 20_000_000);
        assert!(res.is_none() && !capped);
    }
    #[test]
    fn full_resolve_gives_multiset() {
        let (factors, _, capped) = full_resolve(&tape(360), 1000, 20_000_000);
        assert!(!capped);
        let prod = factors.iter().fold(1u64, |acc, f| {
            let mut a = 0u64; for &c in trim(f.clone()).iter().rev() { a = (a << 1) | if c == EVALF { 1 } else { 0 }; } acc * a
        });
        assert_eq!(prod, 360);
    }
}
