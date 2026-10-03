//! Shared bounded factor routes for library and kernel consumers.
use num_bigint::BigUint;
use num_traits::{One, Zero};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PrimeVerdict {
    Prime,
    Composite,
    Undetermined,
}

fn miller_rabin(n: &BigUint) -> bool {
    use crate::native_numeral::{
        add_via_word, halve_even_by_word, mod_pow_walk, modulo_via_word, multiply_via_word,
        subtract_via_word, to_bits_low_first,
    };
    let one = BigUint::one();
    let two = add_via_word(&one, &one);
    if *n < two {
        return false;
    }
    if *n == two {
        return true;
    }
    if modulo_via_word(n, &two).unwrap() == BigUint::zero() {
        return false;
    }

    let n_minus_one = subtract_via_word(n, &one).unwrap();
    let mut d = n_minus_one.clone();
    let mut r: u32 = 0;
    while modulo_via_word(&d, &two).unwrap() == BigUint::zero() {
        d = halve_even_by_word(&d).unwrap();
        r += 1;
    }

    let witnesses: [u64; 13] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41];
    for &a_u64 in witnesses.iter() {
        let a = BigUint::from(a_u64);
        if a >= *n {
            continue;
        }
        let mut x = mod_pow_walk(&a, &to_bits_low_first(&d), n);
        if x == one || x == n_minus_one {
            continue;
        }
        let mut passed = false;
        for _ in 0..r.saturating_sub(1) {
            x = modulo_via_word(&multiply_via_word(&x, &x), n).unwrap();
            if x == n_minus_one {
                passed = true;
                break;
            }
        }
        if !passed {
            return false;
        }
    }
    true
}

pub fn is_prime(a: &str) -> PrimeVerdict {
    let t = a.trim();
    let n: BigUint = match t.parse() {
        Ok(v) => v,
        Err(_) => return PrimeVerdict::Composite,
    };
    use crate::native_numeral::{divmod_small_on_limbs, word_bits};
    let two = BigUint::from(2u32);
    if n < two {
        return PrimeVerdict::Composite;
    }
    if n == two {
        return PrimeVerdict::Prime;
    }
    // n's own word read once here, not once per candidate divisor below --
    // n never changes across this loop, so its limbs don't need re-reading
    // on every one of the up to 500 divisors tried.
    let n_limbs = word_bits(&n);
    if divmod_small_on_limbs(&n_limbs, 2).1 == 0 {
        return PrimeVerdict::Composite;
    }

    // Every trial divisor here fits in a u64 with room to spare (d <= 1000),
    // so d*d does too -- that comparison stays plain u64 arithmetic, the
    // same way an ordering check stays native throughout this file's other
    // conversions. The reduction itself reads n's own limbs (extracted
    // once, above) through the limb-at-a-time small-divisor primitive
    // rather than the general bit-at-a-time one.
    let mut d: u64 = 3;
    loop {
        if BigUint::from(d * d) > n {
            break;
        }
        if divmod_small_on_limbs(&n_limbs, d).1 == 0 {
            return if n == BigUint::from(d) {
                PrimeVerdict::Prime
            } else {
                PrimeVerdict::Composite
            };
        }
        if d >= 1000 {
            break;
        }
        d += 2;
    }

    if miller_rabin(&n) {
        PrimeVerdict::Prime
    } else {
        PrimeVerdict::Composite
    }
}

pub fn big_gcd(mut a: BigUint, mut b: BigUint) -> BigUint {
    while !b.is_zero() {
        let t = b.clone();
        b = crate::native_numeral::modulo_via_word(&a, &b).unwrap();
        a = t;
    }
    a
}

pub const WINDING_BASES: [u64; 10] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29];

pub const LEAP_STEPS: u64 = 80_000_000;

const LEAP_BUCKETS: usize = 32;

pub const BRIDGE_BOUND: u64 = 300_000;

pub const CONGRUENCE_FB_BOUND: u64 = 2000;

pub const CONGRUENCE_TRIALS: u64 = 4_000_000;

pub fn order_multiple_leaping(a: &BigUint, n: &BigUint, steps: u64) -> Option<BigUint> {
    use crate::native_numeral::{
        add_via_word, mod_pow_walk, modulo_via_word, multiply_via_word, subtract_via_word,
        to_bits_low_first,
    };
    use alloc::vec::Vec;
    let one = BigUint::one();
    let a = modulo_via_word(a, n).unwrap();
    if a == one {
        return Some(one);
    }
    // Small pre-walk catches a tiny order directly.
    let mut v = one.clone();
    for k in 1..=64u64 {
        v = modulo_via_word(&multiply_via_word(&v, &a), n).unwrap();
        if v == one {
            return Some(BigUint::from(k));
        }
    }

    // The jumps: LARGE pseudo-random exponents s_i, as group elements a^{s_i}.
    // A step at point x multiplies by the jump its low word selects, and adds
    // that jump's exponent to the leaper's accumulated distance.
    let mut jexp: Vec<u64> = Vec::with_capacity(LEAP_BUCKETS);
    let mut jump: Vec<BigUint> = Vec::with_capacity(LEAP_BUCKETS);
    let mut seed: u64 = 0x2545F4914F6CDD1D;
    for _ in 0..LEAP_BUCKETS {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17; // xorshift
        let s = (seed >> 1) | 1; // a large odd exponent
        jexp.push(s);
        jump.push(mod_pow_walk(&a, &to_bits_low_first(&BigUint::from(s)), n));
    }
    let bucket = |x: &BigUint| -> usize {
        (x.to_u64_digits().first().copied().unwrap_or(0) as usize) % LEAP_BUCKETS
    };

    let (mut tx, mut te) = (a.clone(), one.clone()); // slow leaper
    let (mut hx, mut he) = (a.clone(), one.clone()); // fast leaper
    let mut moved = 0u64;
    while moved < steps {
        let bt = bucket(&tx);
        tx = modulo_via_word(&multiply_via_word(&tx, &jump[bt]), n).unwrap();
        te = add_via_word(&te, &BigUint::from(jexp[bt]));
        for _ in 0..2 {
            let bh = bucket(&hx);
            hx = modulo_via_word(&multiply_via_word(&hx, &jump[bh]), n).unwrap();
            he = add_via_word(&he, &BigUint::from(jexp[bh]));
        }
        moved += 1;
        if tx == hx {
            let d = if te > he {
                subtract_via_word(&te, &he).unwrap()
            } else {
                subtract_via_word(&he, &te).unwrap()
            };
            if d > BigUint::zero() {
                return Some(d);
            }
            break; // degenerate, no usable difference
        }
    }
    None
}

pub fn winding_bridge(n: &BigUint, bound: u64) -> Option<BigUint> {
    use crate::native_numeral::{
        mod_pow_walk, modulo_via_word, subtract_via_word, to_bits_low_first,
    };
    let one = BigUint::one();
    let two = BigUint::from(2u32);
    let mut a = modulo_via_word(&two, n).unwrap();
    let mut e: u64 = 2;
    // Check the gcd at every step. The first factor whose winding divides the
    // accumulated M sends a to 1 mod that factor while it is still generic mod
    // the other, so gcd(a-1, n) crosses to it. Checking only in coarse batches
    // can let the second factor bridge inside the same batch, collapsing the
    // gcd to n and losing the split; per-step checking catches the first
    // crossing exactly. Cost stays dominated by the modular power, not the gcd.
    while e <= bound {
        a = mod_pow_walk(&a, &to_bits_low_first(&BigUint::from(e)), n);
        e += 1;
        if a.is_zero() || a == one {
            break;
        } // bridged out or collapsed; no readable split
        let g = big_gcd(subtract_via_word(&a, &one).unwrap(), n.clone());
        if g > one && &g < n {
            return Some(g);
        }
    }
    None
}

fn factor_base(bound: u64) -> alloc::vec::Vec<u64> {
    let mut sieve = alloc::vec![true; (bound as usize) + 1];
    let mut ps = alloc::vec::Vec::new();
    let mut p = 2u64;
    while p <= bound {
        if sieve[p as usize] {
            ps.push(p);
            let mut m = p * p;
            while m <= bound {
                sieve[m as usize] = false;
                m += p;
            }
        }
        p += 1;
    }
    ps
}

fn smooth_exponents(mut q: BigUint, fb: &[u64]) -> Option<alloc::vec::Vec<u32>> {
    use crate::native_numeral::{divmod_via_word, modulo_via_word};
    let mut exps = alloc::vec![0u32; fb.len()];
    let zero = BigUint::zero();
    for (i, &p) in fb.iter().enumerate() {
        let bp = BigUint::from(p);
        while modulo_via_word(&q, &bp).unwrap() == zero {
            q = divmod_via_word(&q, &bp).unwrap().0;
            exps[i] += 1;
        }
    }
    if q == BigUint::one() {
        Some(exps)
    } else {
        None
    }
}

/// A relation carries X² = Y² times its odd factor-base primes modulo N.
/// Combining two rows multiplies X and Y, and moves shared odd primes into Y.
/// Thus no relation-id history or growing exponent sum is needed.
struct SquareRelation {
    parity: alloc::vec::Vec<u64>,
    x: BigUint,
    y: BigUint,
}

impl SquareRelation {
    fn has(&self, bit: usize) -> bool {
        (self.parity[bit / 64] >> (bit % 64)) & 1 != 0
    }

    fn combine(&mut self, other: &Self, base: &[u64], n: &BigUint) {
        use crate::native_numeral::{modulo_via_word, multiply_via_word};
        self.x = modulo_via_word(&multiply_via_word(&self.x, &other.x), n).unwrap();
        self.y = modulo_via_word(&multiply_via_word(&self.y, &other.y), n).unwrap();
        for (bit, prime) in base.iter().enumerate() {
            if self.has(bit) && other.has(bit) {
                self.y = modulo_via_word(&multiply_via_word(&self.y, &BigUint::from(*prime)), n)
                    .unwrap();
            }
        }
        for (left, right) in self.parity.iter_mut().zip(&other.parity) {
            *left ^= right;
        }
    }
}

/// At most one relation per factor-base pivot is retained, regardless of trials.
struct RelationBasis {
    pivots: alloc::vec::Vec<Option<SquareRelation>>,
}

impl RelationBasis {
    fn new(width: usize) -> Self {
        Self {
            pivots: (0..width).map(|_| None).collect(),
        }
    }

    fn insert(
        &mut self,
        mut row: SquareRelation,
        base: &[u64],
        n: &BigUint,
    ) -> Option<SquareRelation> {
        for bit in (0..self.pivots.len()).rev() {
            if !row.has(bit) {
                continue;
            }
            if let Some(pivot) = &self.pivots[bit] {
                row.combine(pivot, base, n);
            } else {
                self.pivots[bit] = Some(row);
                return None;
            }
        }
        Some(row)
    }
}

pub fn congruence_split(n: &BigUint, fb_bound: u64, trials: u64) -> Option<(BigUint, BigUint)> {
    use crate::native_numeral::{
        add_via_word, divmod_via_word, isqrt, mod_pow_walk, modulo_via_word, multiply_via_word,
        subtract_via_word, to_bits_low_first,
    };
    let one = BigUint::one();
    if n <= &one {
        return None;
    }
    let fb = factor_base(fb_bound);
    let mut basis = RelationBasis::new(fb.len());
    let mut seed = 0x1234_5678_9ABC_DEF1 ^ n.to_u64_digits().first().copied().unwrap_or(1);
    let nsqrt = isqrt(n);
    for _ in 0..trials {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let x = modulo_via_word(
            &add_via_word(&nsqrt, &BigUint::from(seed % 1_000_000_007)),
            n,
        )?;
        if x < BigUint::from(2u32) {
            continue;
        }
        let q = modulo_via_word(&multiply_via_word(&x, &x), n)?;
        if q.is_zero() {
            let g = big_gcd(x, n.clone());
            if g > one && &g < n {
                return Some((g.clone(), divmod_via_word(n, &g)?.0));
            }
            continue;
        }
        let Some(exponents) = smooth_exponents(q, &fb) else {
            continue;
        };
        let mut row = SquareRelation {
            parity: alloc::vec![0; fb.len().div_ceil(64)],
            x,
            y: one.clone(),
        };
        for (bit, (&prime, exponent)) in fb.iter().zip(exponents).enumerate() {
            if exponent & 1 != 0 {
                row.parity[bit / 64] |= 1 << (bit % 64);
            }
            if exponent >= 2 {
                let power = mod_pow_walk(
                    &BigUint::from(prime),
                    &to_bits_low_first(&BigUint::from(exponent / 2)),
                    n,
                );
                row.y = modulo_via_word(&multiply_via_word(&row.y, &power), n)?;
            }
        }
        if let Some(dependency) = basis.insert(row, &fb, n) {
            // A zero parity row is an exact square congruence.
            let diff = if dependency.x >= dependency.y {
                subtract_via_word(&dependency.x, &dependency.y)?
            } else {
                subtract_via_word(&dependency.y, &dependency.x)?
            };
            for arm in [diff, add_via_word(&dependency.x, &dependency.y)] {
                let g = big_gcd(arm, n.clone());
                if g > one && &g < n {
                    return Some((g.clone(), divmod_via_word(n, &g)?.0));
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streaming_dependency_closes_on_a_130_bit_semiprime() {
        let q = (BigUint::one() << 127usize) - BigUint::one();
        let n = &q * 7u32;
        assert_eq!(n.bits(), 130);
        let root = BigUint::one() << 64usize;
        let first = &root + &q;
        let second = &root + &q * 2u32;
        assert_eq!((&first * &first) % &n, BigUint::from(2u32));
        assert_eq!((&second * &second) % &n, BigUint::from(2u32));
        let mut basis = RelationBasis::new(1);
        assert!(basis
            .insert(
                SquareRelation {
                    parity: alloc::vec![1],
                    x: first,
                    y: BigUint::one(),
                },
                &[2],
                &n
            )
            .is_none());
        let row = basis
            .insert(
                SquareRelation {
                    parity: alloc::vec![1],
                    x: second,
                    y: BigUint::one(),
                },
                &[2],
                &n,
            )
            .unwrap();
        assert_eq!(row.parity, alloc::vec![0]);
        assert_eq!(row.y, BigUint::from(2u32));
        assert_eq!((&row.x * &row.x) % &n, (&row.y * &row.y) % &n);
        let diff = if row.x >= row.y {
            &row.x - &row.y
        } else {
            &row.y - &row.x
        };
        let p = big_gcd(diff, n.clone());
        assert!(p > BigUint::one() && p < n);
        assert_eq!(&p * (&n / &p), n);
        assert_eq!(
            basis.pivots.iter().filter(|pivot| pivot.is_some()).count(),
            1
        );
        for _ in 0..1_000 {
            let duplicate = basis
                .insert(
                    SquareRelation {
                        parity: alloc::vec![1],
                        x: &root + &q,
                        y: BigUint::one(),
                    },
                    &[2],
                    &n,
                )
                .unwrap();
            assert_eq!(
                (&duplicate.x * &duplicate.x) % &n,
                (&duplicate.y * &duplicate.y) % &n
            );
            assert_eq!(
                basis.pivots.iter().filter(|pivot| pivot.is_some()).count(),
                1
            );
        }
    }

    #[test]
    fn duplicate_relations_do_not_grow_the_basis_on_a_128_bit_semiprime() {
        let p = BigUint::from(18_446_744_073_709_551_557u64);
        let n = &p * &p;
        assert_eq!(n.bits(), 128);
        let base = [2, 3, 5, 7, 11];
        let mut basis = RelationBasis::new(base.len());
        // Exact square rows are dependencies and consume no pivot storage.
        for _ in 0..10_000 {
            let row = basis
                .insert(
                    SquareRelation {
                        parity: alloc::vec![0],
                        x: p.clone(),
                        y: p.clone(),
                    },
                    &base,
                    &n,
                )
                .unwrap();
            assert!(row.parity.iter().all(|bits| *bits == 0));
        }
        assert!(basis.pivots.iter().all(Option::is_none));
    }
}
