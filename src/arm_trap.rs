//! Two-arm trap producer.
//!
//! The source is split into mu and delta arms with
//!     mu^2 - delta^2 = 4 k N,
//! one arm per braided multiplier k. mu is trapped at the winding ceiling
//! ceil(2 sqrt(kN)); if the trapped mu closes against an integral delta arm,
//! one proper factor falls out of gcd(mu - delta, N) and the cofactor is the
//! derived arm N / d. The 4kN family is the Lehman-Riesel winding of the
//! plain Fermat trap (k = 1), carried here as the cheapest-decisive arm ahead
//! of the relation-source producers. A trapped mu that misses its delta arm
//! inside the bounded window is a run observation, not a feasibility veto.

use num_bigint::BigUint;
use num_traits::{One, Zero};

/// Bounded arm trap. Returns one proper factor of `n` when the braided mu/delta
/// trapping closes inside `k_budget` multipliers and `window_steps` probe
/// slots per arm, otherwise `None` with the plain fall-through semantics.
pub fn trap_arms(n: &BigUint, k_budget: u64, window_steps: u64) -> Option<BigUint> {
    if n.bits() < 3 || !n.bit(0) {
        return None; // even sources are outside the odd semiprime trap
    }
    let two = BigUint::from(2u8);
    let four = BigUint::from(4u8);
    for k in 1u64..=k_budget {
        if k % 4 == 0 {
            continue; // redundant winding of a smaller squarefree multiplier
        }
        let target = &four * BigUint::from(k) * n;
        let root = isqrt(&target);
        let mut mu = if &(&root * &root) == &target { root } else { root + BigUint::one() };
        let mut mu_squared = &mu * &mu;
        for _step in 0..window_steps {
            let delta_sq = &mu_squared - &target;
            if !delta_sq.bit(0) {
                if let Some(delta) = isqrt_exact(&delta_sq) {
                    let lower = if &mu >= &delta { mu.clone() - &delta } else { delta.clone() - &mu };
                    let higher = if &mu >= &delta { mu.clone() + delta } else { delta.clone() + &mu };
                    let divisor = gcd(&lower, n);
                    if divisor > BigUint::one() && &divisor < n {
                        return Some(divisor);
                    }
                    let divisor = gcd(&higher, n);
                    if divisor > BigUint::one() && &divisor < n {
                        return Some(divisor);
                    }
                }
            }
            mu += BigUint::one();
            mu_squared = &mu_squared + &two * &mu - BigUint::one();
        }
    }
    None
}

fn isqrt(value: &BigUint) -> BigUint {
    if value.is_zero() {
        return BigUint::zero();
    }
    let bits = value.bits();
    let mut x = BigUint::one() << ((bits + 1) / 2);
    loop {
        let y = (&x + value / &x) >> 1;
        if y >= x {
            return x;
        }
        x = y;
    }
}

fn isqrt_exact(value: &BigUint) -> Option<BigUint> {
    let root = isqrt(value);
    if &(&root * &root) == value { Some(root) } else { None }
}

fn gcd(a: &BigUint, b: &BigUint) -> BigUint {
    let (mut a, mut b) = (a.clone(), b.clone());
    while !b.is_zero() {
        let r = &a % &b;
        a = b;
        b = r;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trap_closes_the_control_and_the_208_bit_case_structure() {
        let n = BigUint::from(91u8);
        assert_eq!(trap_arms(&n, 8, 16), Some(7u64.into()));
        let semiprime = BigUint::from(101u64) * BigUint::from(103u64);
        let factor = trap_arms(&semiprime, 8, 16).unwrap();
        assert!(factor == BigUint::from(101u64) || factor == BigUint::from(103u64));
        // A balanced trap the window cannot reach must fall through cleanly.
        let balanced = BigUint::parse_bytes(
            b"249880503243289860256926205840602687030138007067151469448678503", 10,
        ).unwrap();
        assert_eq!(trap_arms(&balanced, 64, 8), None);
    }

    #[test]
    fn trap_respects_its_budget_and_bedrock_boundaries() {
        let n = BigUint::from(281u64) * BigUint::from(283u64);
        assert_eq!(trap_arms(&n, 1, 0), None, "a zero window cannot trap any arm");
        let even = BigUint::from(90u8);
        assert_eq!(trap_arms(&even, 8, 16), None);
    }
}
