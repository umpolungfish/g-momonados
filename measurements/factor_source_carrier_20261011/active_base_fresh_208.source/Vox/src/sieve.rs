//! sieve.rs — a Dixon / quadratic-sieve core over the folded numeral tapes.
//!
//! The sub-exponential arm for the HARD shape: a factor that is large, far from
//! the root, and not smooth, where trial, rho, the frontier and p±1 all fail.
//! It collects relations a^2 == q (mod N) whose q factors over a small-prime
//! base, finds a subset of relations whose exponents are all even (a linear
//! dependency over GF(2)), and from the resulting X^2 == Y^2 (mod N) takes
//! gcd(X - Y, N). Value-sized arithmetic (a^2 mod N, the gcd) runs on the shared
//! folded kernel; the base primes and the GF(2) matrix are machine words.

use crate::morphism_factor::{
    add, cmp, divmod, gcd, isqrt, modulo, mul, mul_mod, mul_mod_add, one, sub, tape_u64, trim, zero,
};
use crate::vox::EVALF;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

type Tape = Vec<char>;

// Silent gauges for Vox's native debugger. Only the tracer reads these fields.
// bits, base width, relation target, polynomials, scanned positions, candidates,
// accepted relations, surviving matrix rows, surviving matrix columns,
// non-unit cofactors <= base bound squared, <= its fourth power, and larger;
// polynomial A bits, target A bits, the latest candidate magnitude bits,
// smooth, square and paired closures, and unmatched residual count.
#[unsafe(no_mangle)]
pub static VOX_SIEVE_COUNTERS: [core::sync::atomic::AtomicU64; 19] =
    [const { core::sync::atomic::AtomicU64::new(0) }; 19];
fn sieve_gauge(index: usize, value: usize) {
    VOX_SIEVE_COUNTERS[index].store(value as u64, core::sync::atomic::Ordering::Relaxed);
}


type PartialRelations = alloc::collections::BTreeMap<Tape, (Tape, Vec<(usize, u32)>)>;

fn polynomial_multiplier(n: &Tape) -> u64 {
    let primes = small_primes(1000);
    let residues: Vec<u64> = primes.iter().map(|&p| n_mod_u64(n,p)).collect();
    let mut best = (i64::MIN, 1u64);
    for multiplier in (1u64..=97).step_by(2) {
        if (3u64..=9).any(|p| multiplier % (p*p) == 0) { continue; }
        let bits = 63-multiplier.leading_zeros();
        let leading = 1u64 << bits;
        let log = i64::from(bits)*1024 + ((multiplier-leading)*1024/leading) as i64;
        let mut score = -log/2;
        score += match (n_mod_u64(n,8)*multiplier)%8 { 1=>2048, 5=>1024, _=>512 };
        for (&prime,&residue) in primes.iter().zip(&residues) {
            if prime == 2 { continue; }
            let r = (residue*multiplier)%prime;
            let weight = i64::from(63-prime.leading_zeros())*1024;
            if r == 0 { score += weight/prime as i64; }
            else if legendre(r,prime) == 1 { score += 2*weight/(prime-1) as i64; }
        }
        if score > best.0 { best = (score,multiplier); }
    }
    best.1
}

// Lift small-prime roots in window coordinates. Each power contributes one
// additional prime-log mark, matching repeated division in relation extraction.
#[cfg(test)]
fn prime_power_marks(a: u128, b: i128, c: &Tape, window: usize,
    primes: &[u64]) -> Vec<(usize, usize, i32)> {
    let mut marks = Vec::new();
    for &prime in primes.iter().filter(|&&p| p >= 2 && p <= 31) {
        if prime != 2 && a % prime as u128 == 0 { continue; }
        let p = prime as usize;
        let weight = (63 - prime.leading_zeros()) as i32;
        let mut modulus = p;
        let mut roots: Vec<usize> = (0..p).collect();
        loop {
            let cm = n_mod_u64(c, modulus as u64) as u128;
            let mm = modulus as u128;
            let am = a % mm;
            let bm = b.rem_euclid(modulus as i128) as u128;
            roots.retain(|&offset| {
                let x = (offset + modulus - window % modulus) % modulus;
                let x = x as u128;
                (am*x*x + 2*bm*x + mm - cm) % mm == 0
            });
            if roots.is_empty() { break; }
            if modulus > p || p == 2 {
                marks.extend(roots.iter().map(|&root| (modulus, root, weight)));
            }
            if modulus > 65_536 / p { break; }
            let previous = modulus;
            modulus *= p;
            roots = roots.iter().flat_map(|&root|
                (0..p).map(move |k| root + k*previous)).collect();
        }
    }
    marks
}

// The score scale is A*M². Its logarithm must not inherit a machine-word
// wrap when a larger source places the polynomial above that boundary.
fn polynomial_score_log(a: u128, window: u128) -> i32 {
    if let Some(magnitude) = a.checked_mul(window).and_then(|v| v.checked_mul(window)) {
        return (127 - magnitude.max(2).leading_zeros()) as i32;
    }
    let a = u128_to_tape(a);
    let window = u128_to_tape(window);
    let magnitude = mul(&a, &mul(&window,&window));
    (magnitude.len()-1) as i32
}

/// Two relations carrying the same residual contribute its exact square.
/// The first partial is retained as an anchor for every later matching word.
fn fold_residual_relation(n: &Tape, axb: Tape, mut exponents: Vec<u32>, residual: Tape,
    partials: &mut PartialRelations) -> Option<(Vec<u32>, Tape, Tape)> {
    if zero(&residual) { return None; }
    if residual == one() {
        VOX_SIEVE_COUNTERS[15].fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        return Some((exponents, axb, residual));
    }
    // A residual that is already a square needs no matching partial. Keep its
    // exact root as the square contribution consumed by matrix reconstruction.
    let low = residual.iter().take(4).enumerate().fold(0u8, |v, (i, &bit)|
        v | if bit == EVALF { 1 << i } else { 0 });
    if matches!(low, 0 | 1 | 4 | 9) {
        let root = if let Some(value) = tape_to_u128(&residual) {
            let bits = 128 - value.leading_zeros();
            let mut root = 1u128 << ((bits + 1) / 2);
            loop {
                let next = (root + value / root) / 2;
                if next >= root { break; }
                root = next;
            }
            u128_to_tape(root)
        } else { isqrt(&residual) };
        if mul(&root, &root) == residual {
            VOX_SIEVE_COUNTERS[16].fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            return Some((exponents, axb, root));
        }
    }
    if let Some((prior_x, prior_exponents)) = partials.get(&residual) {
        VOX_SIEVE_COUNTERS[17].fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        for &(column, exponent) in prior_exponents { exponents[column] += exponent; }
        Some((exponents, mul_mod(prior_x, &axb, n), residual))
    } else {
        let sparse = exponents.into_iter().enumerate()
            .filter(|&(_, exponent)| exponent != 0).collect();
        partials.insert(residual, (axb, sparse));
        sieve_gauge(18, partials.len());
        None
    }
}

fn residual_bucket(residual: &Tape, bound2: &Tape, bound4: &Tape) -> usize {
    if cmp(residual, bound2) != core::cmp::Ordering::Greater { 9 }
    else if cmp(residual, bound4) != core::cmp::Ordering::Greater { 10 }
    else { 11 }
}


#[cfg(test)]
mod threshold_tests {
    #[test]
    fn polynomial_scaling_retains_the_source_modular_identity() {
        for value in [91u64,8051,1000036000099] {
            let n = tape_u64(value);
            let k = polynomial_multiplier(&n);
            assert!(k >= 1 && k <= 97 && k % 2 == 1);
            assert!(!(2u64..=9).any(|p| k%(p*p) == 0));
            assert_eq!(polynomial_multiplier(&n), k);
            assert!(zero(&modulo(&mul(&n,&tape_u64(k)),&n)));
        }
    }
    #[test]
    fn lifted_prime_power_scores_match_repeated_divisibility() {
        let window = 128usize;
        let primes = [2u64, 3, 7];
        let marks = prime_power_marks(5, 2, &tape_u64(7), window, &primes);
        let mut period = Vec::new();
        prepare_score_period_with_powers(&[1], &[-1], &[-1], &[0],
            65536, &mut period, &marks);
        let mut folded = vec![0; 2*window+1];
        seed_score_period(&mut folded, &period, 0);
        for &(modulus, root, weight) in &marks {
            if period.len() % modulus == 0 { continue; }
            for position in (root..folded.len()).step_by(modulus) { folded[position] += weight; }
        }
        for offset in 0..=2*window {
            let x = offset as i128 - window as i128;
            let value = (5*x*x + 4*x - 7).unsigned_abs();
            let actual: i32 = marks.iter().filter(|&&(modulus, root, _)|
                offset % modulus == root).map(|&(_,_,weight)|weight).sum();
            let mut expected = 0;
            for prime in primes {
                let mut power = prime as u128;
                while power <= 65_536 {
                    if (power > prime as u128 || prime == 2) && value % power == 0 {
                        expected += (63-prime.leading_zeros()) as i32;
                    }
                    power *= prime as u128;
                }
            }
            assert_eq!(actual, expected, "offset {offset}");
            assert_eq!(folded[offset], expected, "folded offset {offset}");
        }
    }
    #[test]
    fn square_residual_closes_without_waiting_for_a_duplicate() {
        let n = tape_u64(91);
        let mut pool = PartialRelations::new();
        assert!(fold_residual_relation(&n, tape_u64(3), vec![0], tape_u64(0), &mut pool).is_none());
        let (exponents, x, root) = fold_residual_relation(
            &n, tape_u64(11), vec![0], tape_u64(121), &mut pool).unwrap();
        assert!(pool.is_empty());
        assert_eq!(exponents, vec![0]);
        assert_eq!(root, tape_u64(11));
        assert_eq!(mul_mod(&x, &x, &n), mul_mod(&root, &root, &n));
        for residual in [17, 20, 25, 81, 88] {
            let result = fold_residual_relation(
                &n, tape_u64(3), vec![0], tape_u64(residual), &mut pool);
            assert_eq!(result.is_some(), matches!(residual, 25 | 81));
        }
    }
    use super::*;
    #[test]
    fn polynomial_score_log_preserves_the_full_product_above_machine_width() {
        for a in [0,1,3,1u128<<80,1u128<<96,u128::MAX] {
            for window in [0,1,3,1u128<<24,u128::MAX] {
                let product = mul(&u128_to_tape(a),
                    &mul(&u128_to_tape(window),&u128_to_tape(window)));
                let expected = product.len().saturating_sub(1).max(1) as i32;
                assert_eq!(polynomial_score_log(a,window),expected);
            }
        }
        assert_eq!(polynomial_score_log(1u128<<80,1u128<<24),128);
        assert_eq!(polynomial_score_log(1u128<<96,1u128<<24),144);
        assert!((1u128<<80).checked_mul(1u128<<24)
            .and_then(|v| v.checked_mul(1u128<<24)).is_none());
    }
    #[test]
    fn matching_residuals_close_only_with_their_square_contribution() {
        let n = tape_u64(91);
        let base = [1, 2, 3];
        let mut pool = PartialRelations::new();
        assert!(fold_residual_relation(&n, tape_u64(19), vec![0, 3, 0], tape_u64(11), &mut pool).is_none());
        let (exponents, x, root) = fold_residual_relation(&n, tape_u64(33), vec![0, 3, 0], tape_u64(11), &mut pool).unwrap();
        assert_eq!(exponents, vec![0, 6, 0]);
        assert_eq!(mul_mod(&x, &x, &n), mul_mod(&mul(&root, &root), &tape_u64(64), &n));
        let rows = [x, tape_u64(8)];
        let exponents = [exponents, vec![0, 6, 0]];
        assert!(combine(&n, &rows, &exponents, &base).is_none());
        let roots = [root, one()];
        let factor = combine_with_square_factors(&n, &rows, &exponents, &base, Some(&roots)).unwrap();
        assert_eq!(factor, tape_u64(7));
        assert_eq!(mul(&factor, &divmod(&n, &factor).0), n);
        // A third matching word also uses the first partial as its anchor.
        assert!(fold_residual_relation(&n, tape_u64(58), vec![0, 3, 0], tape_u64(11), &mut pool).is_some());
    }
    #[test]
    fn shared_residual_rows_preserve_signed_parity() {
        let n = tape_u64(91);
        let base = [1, 2, 5];
        let mut pool = PartialRelations::new();
        assert!(fold_residual_relation(&n, tape_u64(6), vec![1, 0, 1], tape_u64(11), &mut pool).is_none());
        let first = fold_residual_relation(&n, tape_u64(19), vec![0, 3, 0], tape_u64(11), &mut pool).unwrap();
        let second = fold_residual_relation(&n, tape_u64(33), vec![0, 3, 0], tape_u64(11), &mut pool).unwrap();
        let rows = [first.1, second.1];
        let exponents = [first.0, second.0];
        let roots = [first.2, second.2];
        assert_eq!(exponents[0], vec![1, 3, 1]);
        assert_eq!(mul_mod(&rows[0], &rows[0], &n), tape_u64(74));
        assert_eq!(mul_mod(&rows[1], &rows[1], &n), tape_u64(74));
        assert!(combine(&n, &rows, &exponents, &base).is_none());
        let factor = combine_with_square_factors(&n, &rows, &exponents, &base, Some(&roots)).unwrap();
        assert_eq!(factor, tape_u64(13));
        assert_eq!(mul(&factor, &divmod(&n, &factor).0), n);
    }
    #[test]
    fn a_composite_shared_residual_also_contributes_an_exact_square() {
        let n = tape_u64(91);
        let mut pool = PartialRelations::new();
        assert!(fold_residual_relation(&n, tape_u64(19), vec![0], tape_u64(88), &mut pool).is_none());
        let paired = fold_residual_relation(&n, tape_u64(33), vec![0], tape_u64(88), &mut pool).unwrap();
        let rows = [paired.1, one()];
        let exponents = [paired.0, vec![0]];
        let roots = [paired.2, one()];
        assert!(combine(&n, &rows, &exponents, &[1]).is_none());
        assert_eq!(combine_with_square_factors(&n, &rows, &exponents, &[1], Some(&roots)), Some(tape_u64(7)));
    }
    #[test]
    fn residual_buckets_include_exact_boundaries_and_wide_values() {
        let b2 = tape_u64(49);
        let b4 = mul(&b2, &b2);
        for (value, bucket) in [(48, 9), (49, 9), (50, 10), (2401, 10), (2402, 11)] {
            assert_eq!(residual_bucket(&tape_u64(value), &b2, &b4), bucket);
        }
        let wide = mul(&u128_to_tape(u128::MAX), &u128_to_tape(u128::MAX));
        assert_eq!(residual_bucket(&wide, &b2, &b4), 11);
        assert_eq!(residual_bucket(&wide, &wide, &wide), 9);
    }
    #[test]
    fn matrix_closes_with_two_rows_and_many_unused_columns() {
        let base = [1, 2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47];
        let n = tape_u64(91);
        let rows = [tape_u64(10), tape_u64(17)];
        let mut exponents = vec![vec![0; base.len()]; rows.len()];
        exponents[0][2] = 2;
        exponents[1][1] = 4;
        // 10^2 == 3^2 and 17^2 == 2^4 modulo 91.
        assert_eq!(mul_mod(&rows[0], &rows[0], &n), tape_u64(9));
        assert_eq!(mul_mod(&rows[1], &rows[1], &n), tape_u64(16));
        let factor = combine(&n, &rows, &exponents, &base).unwrap();
        assert_eq!(factor, tape_u64(7));
        let (quotient, remainder) = divmod(&n, &factor);
        assert!(zero(&remainder));
        assert_eq!(mul(&factor, &quotient), n);
    }
    #[test]
    fn sliced_pivot_xor_matches_full_rows_at_word_boundaries() {
        for width in [1, 63, 64, 65, 127, 128, 129, 255, 1025] {
            let words = width / 64 + 1;
            for column in 0..width {
                let first = column / 64;
                let mut rows = vec![vec![0; words]; 3];
                for word in first..words {
                    rows[0][word] = (word as u64 + 17).wrapping_mul(0x9e3779b97f4a7c15);
                    rows[2][word] = !(word as u64 + 31);
                }
                let source = rows[0].clone();
                let expected: Vec<u64> = rows[2].iter().zip(&source)
                    .map(|(&left, &right)| left ^ right).collect();
                xor_pivot_words(&mut rows, 2, 0, first, words);
                assert_eq!(rows[2], expected);
                assert_eq!(rows[0], source);
            }
        }
        for current in [1, 63, 64, 65, 127, 128, 129] {
            let words = current / 64 + 2;
            let mut rows = vec![vec![0; words]; current + 1];
            rows[0][0] = 1;
            rows[current][current / 64] = 1 << (current % 64);
            let expected: Vec<u64> = rows[current].iter().zip(&rows[0])
                .map(|(&left, &right)| left ^ right).collect();
            xor_pivot_words(&mut rows, current, 0, 0, current / 64 + 1);
            assert_eq!(rows[current], expected);
        }
    }
    #[test]
    fn word_pivot_matches_bit_scan_with_padding_and_zero_words() {
        for width in [0, 1, 63, 64, 65, 127, 128, 129, 255, 256, 512, 1025] {
            let mut row = vec![0u64; width / 64 + 2];
            assert_eq!(first_set_column(&row, width), None);
            for column in 0..width + 8 {
                row.fill(0);
                row[column / 64] |= 1 << (column % 64);
                let other = (column + 73) % (width + 8);
                row[other / 64] |= 1 << (other % 64);
                let expected = (0..width).find(|&c| (row[c / 64] >> (c % 64)) & 1 != 0);
                assert_eq!(first_set_column(&row, width), expected);
            }
        }
    }
    #[test]
    fn folded_period_matches_prime_root_marks_across_block_boundaries() {
        let base = [1, 2, 3, 5, 7, 11, 13, 17];
        let roots1 = [-1, -1, 0, 2, 6, -1, 3, 15];
        let roots2 = [-1, -1, 1, 2, 0, -1, 8, 0];
        let logs = [0, 1, 1, 2, 2, 3, 3, 4];
        let mut period = Vec::new();
        for capacity in [1, 4, 16, 104, 105, 256, 1024, 2048, 65536] {
            let end = prepare_score_period(&base, &roots1, &roots2, &logs,
                capacity, &mut period);
            assert!(period.len() <= capacity);
            for start in [0, 1, 104, 105, 106, 317, 1023, 23204] {
                for length in [0, 1, 13, 64, 256] {
                    let mut block = vec![-1; length];
                    seed_score_period(&mut block, &period, start);
                    let expected: Vec<i32> = (start..start + length).map(|position| {
                        (1..end).filter(|&j| roots1[j] >= 0).map(|j| {
                            let residue = (position % base[j] as usize) as i64;
                            logs[j] * (i32::from(residue == roots1[j])
                                + i32::from(residue == roots2[j]))
                        }).sum()
                    }).collect();
                    assert_eq!(block, expected);
                }
            }
        }
    }
    #[test]
    fn score_period_doubling_preserves_a_large_rotated_partial_tail() {
        let period: Vec<i32> = (0..23205).map(|i| i % 17).collect();
        let mut block = vec![-1; 70000];
        seed_score_period(&mut block, &period, 22000);
        for (index, score) in block.iter().enumerate() {
            assert_eq!(*score, period[(22000 + index) % period.len()]);
        }
    }
    #[test]
    fn vector_threshold_matches_signed_scalar_at_every_tail() {
        let scores = [i32::MIN, -10, -1, 0, 1, 4, 15, 16, 17, 100, i32::MAX];
        let mut positions = vec![usize::MAX];
        for length in 0..=scores.len() {
            for threshold in [i32::MIN, -10, -1, 0, 1, 15, 16, 17, i32::MAX] {
                threshold_positions(&scores[..length], threshold, &mut positions);
                let expected: Vec<_> = scores[..length].iter().enumerate()
                    .filter_map(|(i, &score)| (score >= threshold).then_some(i)).collect();
                assert_eq!(positions, expected);
            }
        }
    }
}

// ---- machine-word number theory for the base and the roots ----

fn mulmod(a: u64, b: u64, m: u64) -> u64 {
    ((a as u128 * b as u128) % m as u128) as u64
}

/// Exact remainder for a 32-bit candidate and a fixed 32-bit sieve prime.
/// The reciprocal quotient is at most one below the exact quotient.
fn remainder_u32(n: u32, modulus: u32, reciprocal: u64) -> u32 {
    let quotient = (u64::from(n) * reciprocal) >> 32;
    let mut remainder = u64::from(n) - quotient * u64::from(modulus);
    if remainder >= u64::from(modulus) {
        remainder -= u64::from(modulus);
    }
    remainder as u32
}

/// Collect exactly the positions whose sieve score meets the threshold.
/// SSE2 compares four scores per instruction on x86-64; the tail stays scalar.
fn threshold_positions(scores: &[i32], threshold: i32, positions: &mut Vec<usize>) {
    positions.clear();
    let mut offset = 0;
    #[cfg(target_arch = "x86_64")]
    unsafe {
        use core::arch::x86_64::*;
        let boundary = _mm_set1_epi32(threshold);
        while offset + 4 <= scores.len() {
            let values = _mm_loadu_si128(scores.as_ptr().add(offset).cast());
            let below = _mm_cmpgt_epi32(boundary, values);
            let mut mask = (!_mm_movemask_epi8(below) as u32) & 0x1111;
            while mask != 0 {
                let lane = (mask.trailing_zeros() / 4) as usize;
                positions.push(offset + lane);
                mask &= mask - 1;
            }
            offset += 4;
        }
    }
    for (index, &score) in scores[offset..].iter().enumerate() {
        if score >= threshold {
            positions.push(offset + index);
        }
    }
}

fn score_block_capacity(width: usize) -> usize {
    #[cfg(target_arch = "x86_64")]
    {
        use core::arch::x86_64::__cpuid_count;

        if unsafe { core::arch::x86_64::__cpuid(0) }.eax >= 4 {
            for subleaf in 0..32 {
                let cache = unsafe { __cpuid_count(4, subleaf) };
                let cache_type = cache.eax & 0x1f;
                if cache_type == 0 {
                    break;
                }
                let level = (cache.eax >> 5) & 0x7;
                if level == 2 && matches!(cache_type, 1 | 3) {
                    let line_size = (cache.ebx & 0xfff) + 1;
                    let partitions = ((cache.ebx >> 12) & 0x3ff) + 1;
                    let ways = ((cache.ebx >> 22) & 0x3ff) + 1;
                    let sets = cache.ecx + 1;
                    return (line_size as usize)
                        .saturating_mul(partitions as usize)
                        .saturating_mul(ways as usize)
                        .saturating_mul(sets as usize)
                        / core::mem::size_of::<i32>();
                }
            }
        }
    }

    width
        .checked_next_power_of_two()
        .unwrap_or(width)
        .saturating_mul(core::mem::size_of::<i32>())
        .max(1)
}

/// Fold the first active prime lanes into one common score period. Its length
/// is chosen from the live primes and must fit the existing block storage.
fn prepare_score_period(base: &[u64], roots1: &[i64], roots2: &[i64], logs: &[i32],
    capacity: usize, scores: &mut Vec<i32>) -> usize {
    prepare_score_period_with_powers(base, roots1, roots2, logs, capacity, scores, &[])
}

fn prepare_score_period_with_powers(base: &[u64], roots1: &[i64], roots2: &[i64], logs: &[i32],
    capacity: usize, scores: &mut Vec<i32>, powers: &[(usize, usize, i32)]) -> usize {
    let mut period = 1usize;
    for &(modulus, _, _) in powers.iter().filter(|&&(modulus, _, _)| modulus <= 16) {
        let mut a = period;
        let mut b = modulus;
        while b != 0 { let r = a % b; a = b; b = r; }
        if let Some(next) = (period/a).checked_mul(modulus).filter(|&n| n <= capacity) {
            period = next;
        }
    }
    let mut end = 1;
    for j in 1..base.len() {
        if roots1[j] < 0 { continue; }
        let p = base[j] as usize;
        let multiplier = if period % p == 0 { 1 } else { p };
        let Some(next) = period.checked_mul(multiplier).filter(|&n| n <= capacity)
            else { break; };
        period = next;
        end = j + 1;
    }
    scores.resize(period, 0);
    scores.fill(0);
    for &(modulus, root, weight) in powers {
        if period % modulus == 0 {
            for position in (root..period).step_by(modulus) { scores[position] += weight; }
        }
    }
    for j in 1..end {
        if roots1[j] < 0 { continue; }
        for root in [roots1[j], roots2[j]] {
            for position in (root as usize..period).step_by(base[j] as usize) {
                scores[position] += logs[j];
            }
        }
    }
    end
}

/// Seed a block at its absolute phase, then double complete periods by copying.
fn seed_score_period(block: &mut [i32], period: &[i32], start: usize) {
    if period.len() == 1 { block.fill(period[0]); return; }
    let phase = start % period.len();
    let mut filled = (period.len() - phase).min(block.len());
    block[..filled].copy_from_slice(&period[phase..phase + filled]);
    if filled < block.len() && phase != 0 {
        let amount = phase.min(block.len() - filled);
        block[filled..filled + amount].copy_from_slice(&period[..amount]);
        filled += amount;
    }
    while filled < block.len() {
        let amount = filled.min(block.len() - filled);
        block.copy_within(..amount, filled);
        filled += amount;
    }
}

/// Locate the same lowest pivot column while skipping an entire zero word.
fn first_set_column(row: &[u64], width: usize) -> Option<usize> {
    for (word, &bits) in row.iter().enumerate() {
        if bits != 0 {
            let column = word * 64 + bits.trailing_zeros() as usize;
            return (column < width).then_some(column);
        }
    }
    None
}

/// Earlier pivot rows and the current row occupy disjoint storage. Slice the
/// rows once so the word loop carries no repeated outer-vector indexing.
fn xor_pivot_words(rows: &mut [Vec<u64>], current: usize, pivot: usize,
    start: usize, end: usize) {
    debug_assert!(pivot < current);
    let (earlier, remaining) = rows.split_at_mut(current);
    let source = &earlier[pivot][start..end];
    let target = &mut remaining[0][start..end];
    for (left, &right) in target.iter_mut().zip(source) {
        *left ^= right;
    }
}


fn powmod(mut a: u64, mut e: u64, m: u64) -> u64 {
    let mut r = 1u64 % m;
    a %= m;
    while e > 0 {
        if e & 1 == 1 {
            r = mulmod(r, a, m);
        }
        a = mulmod(a, a, m);
        e >>= 1;
    }
    r
}
fn legendre(a: u64, p: u64) -> i64 {
    let r = powmod(a % p, (p - 1) / 2, p);
    if r == 0 {
        0
    } else if r == 1 {
        1
    } else {
        -1
    }
}
/// Tonelli-Shanks: a square root of n mod p (odd prime, n a QR), or None.
fn tonelli(n: u64, p: u64) -> Option<u64> {
    if p == 2 {
        return Some(n % 2);
    }
    if legendre(n, p) != 1 {
        return None;
    }
    if p % 4 == 3 {
        return Some(powmod(n, (p + 1) / 4, p));
    }
    let mut q = p - 1;
    let mut s = 0u32;
    while q % 2 == 0 {
        q /= 2;
        s += 1;
    }
    let mut z = 2u64;
    while legendre(z, p) != -1 {
        z += 1;
    }
    let mut m = s;
    let mut c = powmod(z, q, p);
    let mut t = powmod(n, q, p);
    let mut r = powmod(n, (q + 1) / 2, p);
    while t != 1 {
        let mut i = 0u32;
        let mut t2 = t;
        while t2 != 1 {
            t2 = mulmod(t2, t2, p);
            i += 1;
            if i == m {
                return None;
            }
        }
        let b = powmod(c, 1u64 << (m - i - 1), p);
        m = i;
        c = mulmod(b, b, p);
        t = mulmod(t, c, p);
        r = mulmod(r, b, p);
    }
    Some(r)
}
/// N mod p (p small) by folding the tape modulo through the kernel.
fn n_mod_u64(n: &Tape, p: u64) -> u64 {
    let r = modulo(n, &tape_u64(p));
    let mut v = 0u64;
    for (i, &c) in trim(r).iter().enumerate() {
        if c == EVALF && i < 64 {
            v |= 1u64 << i;
        }
    }
    v
}

/// Small odd primes up to bound b (plus 2), by a byte sieve of Eratosthenes.
fn small_primes(b: usize) -> Vec<u64> {
    let mut is_c = vec![false; b + 1];
    let mut ps = Vec::new();
    let mut i = 2usize;
    while i <= b {
        if !is_c[i] {
            ps.push(i as u64);
            let mut j = i * i;
            while j <= b {
                is_c[j] = true;
                j += i;
            }
        }
        i += 1;
    }
    ps
}

/// Trial-factor the tape v over the base; return the exponent per base prime if
/// v is fully smooth (reduced to 1), else None.
fn smooth_over(v: &Tape, base: &[u64]) -> Option<Vec<u32>> {
    let mut cur = trim(v.clone());
    let mut exps = vec![0u32; base.len()];
    for (i, &p) in base.iter().enumerate() {
        let pt = tape_u64(p);
        loop {
            let (q, r) = divmod(&cur, &pt);
            if zero(&r) {
                exps[i] += 1;
                cur = q;
            } else {
                break;
            }
        }
    }
    if cur == vec![EVALF] {
        Some(exps)
    } else {
        None
    }
}

/// Dixon over the folded tapes. Returns a nontrivial factor of n, or None if no
/// dependency inside the relation/candidate budget yielded one.
pub fn dixon(n: &Tape, base_bound: usize, extra: usize, max_candidates: u64) -> Option<Tape> {
    let base = small_primes(base_bound);
    let width = base.len();
    if width == 0 {
        return None;
    }
    let mut a_of: Vec<Tape> = Vec::new();
    let mut exp_of: Vec<Vec<u32>> = Vec::new();
    let mut a = isqrt(n);
    if cmp(&mul(&a, &a), n) != core::cmp::Ordering::Greater {
        a = add(&a, &one());
    }
    let mut tried = 0u64;
    let need = width + extra;
    while a_of.len() < need && tried < max_candidates {
        tried += 1;
        let q = mul_mod(&a, &a, n); // a^2 mod N
        if let Some(exps) = smooth_over(&q, &base) {
            a_of.push(a.clone());
            exp_of.push(exps);
        }
        a = add(&a, &one());
    }
    combine(n, &a_of, &exp_of, &base)
}

/// GF(2) solve over the relation exponent-parity rows plus reconstruct: find
/// dependencies (relation subsets with all-even exponent sum), and for each,
/// X = prod a_i mod N, Y = prod p^(e/2) mod N, then gcd(X - Y, N). Returns the
/// first nontrivial factor. Shared by Dixon and the quadratic sieve.
fn combine(n: &Tape, a_of: &[Tape], exp_of: &[Vec<u32>], base: &[u64]) -> Option<Tape> {
    combine_with_square_factors(n, a_of, exp_of, base, None)
}

/// The same GF(2) row transformation, retained as relations arrive. Histories
/// refer to the source-bound relation bank and survive every polynomial return.
struct OnlineRelations {
    pivots: Vec<Option<(Vec<u64>, Vec<u64>)>>,
}

impl OnlineRelations {
    fn new(width: usize) -> Self {
        Self { pivots: vec![None; width] }
    }

    fn insert(&mut self, exponents: &[u32], index: usize) -> Option<Vec<usize>> {
        let width = self.pivots.len();
        let mut row = vec![0u64; width / 64 + 1];
        for (column, exponent) in exponents.iter().enumerate() {
            if exponent & 1 == 1 { row[column / 64] |= 1 << (column % 64); }
        }
        let mut history = vec![0u64; index / 64 + 1];
        history[index / 64] = 1 << (index % 64);
        while let Some(column) = first_set_column(&row, width) {
            if let Some((pivot, ancestry)) = &self.pivots[column] {
                for (left, right) in row[column / 64..].iter_mut().zip(&pivot[column / 64..]) {
                    *left ^= right;
                }
                for (left, right) in history.iter_mut().zip(ancestry) { *left ^= right; }
            } else {
                self.pivots[column] = Some((row, history));
                return None;
            }
        }
        Some((0..=index).filter(|&i| (history[i / 64] >> (i % 64)) & 1 == 1).collect())
    }
}

fn dependency_divisor(n: &Tape, selected: &[usize], a_of: &[Tape],
    exp_of: &[Vec<u32>], base: &[u64], square_factors: Option<&[Tape]>) -> Option<Tape> {
    if selected.is_empty() { return None; }
    let mut x = one();
    let mut total = vec![0u32; base.len()];
    let mut y = one();
    for &index in selected {
        x = mul_mod(&x, &a_of[index], n);
        for (sum, exponent) in total.iter_mut().zip(&exp_of[index]) { *sum += exponent; }
        if let Some(squares) = square_factors {
            y = mul_mod(&y, &squares[index], n);
        }
    }
    if total.iter().any(|exponent| exponent & 1 == 1) { return None; }
    for (&exponent, &prime) in total.iter().zip(base) {
        for _ in 0..exponent / 2 { y = mul_mod(&y, &tape_u64(prime), n); }
    }
    let difference = if cmp(&x, &y).is_lt() { sub(&y, &x) } else { sub(&x, &y) };
    let divisor = gcd(trim(difference), n.clone());
    (cmp(&divisor, &one()).is_gt() && cmp(&divisor, n).is_lt()).then_some(trim(divisor))
}

fn combine_with_square_factors(n: &Tape, a_of: &[Tape], exp_of: &[Vec<u32>], base: &[u64],
    square_factors: Option<&[Tape]>) -> Option<Tape> {
    debug_assert!(square_factors.is_none_or(|factors| factors.len() == a_of.len()));
    let width = base.len();
    let rel = a_of.len();
    if rel < 2 || width == 0 {
        return None;
    }
    // Structured Gaussian elimination first: a relation that owns a prime no other
    // relation carries (a column of weight one) cannot sit in any dependency, so
    // drop it; that lowers other columns' weights and cascades. What survives keeps
    // every dependency the full set held, over only the heavy columns (weight >= 2).
    // The surviving row and column counts depend on the collected relations;
    // the dense solve runs on that measured residual.
    let par: Vec<Vec<usize>> = exp_of
        .iter()
        .map(|e| (0..width).filter(|&c| e[c] & 1 == 1).collect())
        .collect();
    let mut alive = vec![true; rel];
    let mut colcount = vec![0u32; width];
    for row in &par {
        for &c in row {
            colcount[c] += 1;
        }
    }
    loop {
        let mut changed = false;
        for r in 0..rel {
            if alive[r] && par[r].iter().any(|&c| colcount[c] == 1) {
                alive[r] = false;
                changed = true;
                for &c in &par[r] {
                    colcount[c] -= 1;
                }
            }
        }
        if !changed {
            break;
        }
    }
    let keep_rows: Vec<usize> = (0..rel).filter(|&r| alive[r]).collect();
    let keep_cols: Vec<usize> = (0..width).filter(|&c| colcount[c] >= 2).collect();
    let rrel = keep_rows.len();
    sieve_gauge(7, rrel);
    sieve_gauge(8, keep_cols.len());
    if rrel < 2 {
        return None;
    }
    // compact column index for the surviving heavy columns
    let mut col_idx = vec![usize::MAX; width];
    for (i, &c) in keep_cols.iter().enumerate() {
        col_idx[c] = i;
    }
    let rwidth = keep_cols.len();
    let words = rwidth / 64 + 1;
    let hwords = rrel / 64 + 1;
    let mut mat: Vec<Vec<u64>> = keep_rows
        .iter()
        .map(|&r| {
            let mut m = vec![0u64; words];
            for &c in &par[r] {
                let ci = col_idx[c];
                if ci != usize::MAX {
                    m[ci / 64] |= 1u64 << (ci % 64);
                }
            }
            m
        })
        .collect();
    let mut hist: Vec<Vec<u64>> = (0..rrel)
        .map(|i| {
            let mut h = vec![0u64; hwords];
            h[i / 64] |= 1u64 << (i % 64);
            h
        })
        .collect();
    let mut pivot_row = vec![usize::MAX; rwidth];
    #[cfg(feature = "mpqs_debug")]
    let (mut _deps, mut _trivial) = (0usize, 0usize);
    for r in 0..rrel {
        loop {
            let col = first_set_column(&mat[r], rwidth);
            match col {
                None => break,
                Some(c) => {
                    if pivot_row[c] == usize::MAX {
                        pivot_row[c] = r;
                        break;
                    } else {
                        let pr = pivot_row[c];
                        // Both rows have zero bits before their lowest pivot.
                        xor_pivot_words(&mut mat, r, pr, c / 64, words);
                        // Histories contain only already-visited row indices.
                        xor_pivot_words(&mut hist, r, pr, 0, r / 64 + 1);
                    }
                }
            }
        }
        if mat[r].iter().all(|&w| w == 0) {
            let sel: Vec<usize> = (0..rrel)
                .filter(|&i| (hist[r][i / 64] >> (i % 64)) & 1 == 1)
                .map(|i| keep_rows[i])
                .collect();
            if sel.is_empty() {
                continue;
            }
            let mut x = one();
            for &i in &sel {
                x = mul_mod(&x, &a_of[i], n);
            }
            let mut total = vec![0u32; width];
            for &i in &sel {
                for c in 0..width {
                    total[c] += exp_of[i][c];
                }
            }
            let mut y = one();
            if let Some(square_factors) = square_factors {
                for &i in &sel {
                    if square_factors[i] != one() {
                        y = mul_mod(&y, &square_factors[i], n);
                    }
                }
            }
            for c in 0..width {
                for _ in 0..total[c] / 2 {
                    y = mul_mod(&y, &tape_u64(base[c]), n);
                }
            }
            let diff = if cmp(&x, &y) != core::cmp::Ordering::Less {
                trim(sub(&x, &y))
            } else {
                trim(sub(&y, &x))
            };
            if zero(&diff) {
                continue;
            }
            let g = gcd(diff, n.clone());
            #[cfg(feature = "mpqs_debug")]
            {
                extern crate std;
                _deps += 1;
                if cmp(&g, &one()) == core::cmp::Ordering::Equal || cmp(&g, n) == core::cmp::Ordering::Equal {
                    _trivial += 1;
                }
            }
            if cmp(&g, &one()) == core::cmp::Ordering::Greater && cmp(&g, n) == core::cmp::Ordering::Less {
                return Some(trim(g));
            }
        }
    }
    #[cfg(feature = "mpqs_debug")]
    {
        extern crate std;
        std::eprintln!("[combine] rel={} width={} deps_tried={} trivial={}", rel, width, _deps, _trivial);
    }
    None
}

/// Quadratic sieve. The factor base is only the primes where N is a quadratic
/// residue (the only ones that can divide a^2 - N), each with its two roots by
/// Tonelli-Shanks. A log-sieve over a window above sqrt(N) marks where each
/// prime divides, so only positions whose log-sum approaches log2(a^2 - N) are
/// trial-factored exactly. Then the shared GF(2) combine closes it.
pub fn qs(n: &Tape, b_bound: usize, m_interval: usize, extra: usize) -> Option<Tape> {
    let primes = small_primes(b_bound);
    let mut base: Vec<u64> = Vec::new();
    let mut roots: Vec<(u64, u64)> = Vec::new();
    for &p in &primes {
        let np = n_mod_u64(n, p);
        if np == 0 {
            return Some(tape_u64(p)); // p actually divides N
        }
        if p == 2 {
            base.push(2);
            roots.push((1, 1));
        } else if legendre(np, p) == 1 {
            if let Some(r) = tonelli(np, p) {
                base.push(p);
                roots.push((r, p - r));
            }
        }
    }
    let width = base.len();
    if width == 0 {
        return None;
    }
    let mut root = isqrt(n);
    if cmp(&mul(&root, &root), n) == core::cmp::Ordering::Less {
        root = add(&root, &one());
    }
    // Offsets: position i has p | (a^2 - N) iff i ≡ off (mod p) for off in offs[k].
    let flog2 = |x: u64| -> u32 { if x < 2 { 0 } else { 63 - x.leading_zeros() } };
    let mut offs: Vec<(u64, u64)> = Vec::with_capacity(width);
    for (k, &p) in base.iter().enumerate() {
        let rootmod = n_mod_u64(&root, p) % p;
        let (r1, r2) = roots[k];
        let o1 = (r1 + p - rootmod) % p;
        let o2 = if p == 2 { o1 } else { (r2 + p - rootmod) % p };
        offs.push((o1, o2));
    }
    // Integer log-sieve (bit-length weights; no_std has no float log).
    let mut logs = vec![0u32; m_interval];
    for (k, &p) in base.iter().enumerate() {
        let lp = flog2(p);
        let (o1, o2) = offs[k];
        let os = if p == 2 || o1 == o2 { vec![o1] } else { vec![o1, o2] };
        for o in os {
            let mut i = o as usize;
            while i < m_interval {
                logs[i] += lp;
                i += p as usize;
            }
        }
    }
    let bits_n = trim(n.clone()).len() as u32;
    let slack = 2 * (flog2(b_bound as u64) + 1) + 4;
    // Resieve: for a candidate at i, divide the value only by the base primes
    // whose root position hits i, as tape divmods, instead of trial-dividing the
    // whole base. Smooth iff the residue reduces to 1.
    let factor_at = |v: &Tape, i: usize| -> Option<Vec<u32>> {
        let mut cur = trim(v.clone());
        let mut exps = vec![0u32; width];
        for (k, &p) in base.iter().enumerate() {
            let (o1, o2) = offs[k];
            let im = (i as u64) % p;
            if im == o1 || im == o2 {
                let pt = tape_u64(p);
                loop {
                    let (q, r) = divmod(&cur, &pt);
                    if zero(&r) {
                        exps[k] += 1;
                        cur = q;
                    } else {
                        break;
                    }
                }
            }
        }
        if cur == vec![EVALF] {
            Some(exps)
        } else {
            None
        }
    };
    let mut a_of: Vec<Tape> = Vec::new();
    let mut exp_of: Vec<Vec<u32>> = Vec::new();
    let need = width + extra;
    for i in 0..m_interval {
        if a_of.len() >= need {
            break;
        }
        let target = bits_n / 2 + flog2(i as u64 + 1) + 1;
        if logs[i] + slack < target {
            continue;
        }
        let a = add(&root, &tape_u64(i as u64));
        let v = trim(sub(&mul(&a, &a), n));
        if let Some(exps) = factor_at(&v, i) {
            a_of.push(a);
            exp_of.push(exps);
        }
    }
    combine(n, &a_of, &exp_of, &base)
}

/// Extended Euclid modular inverse of a mod m (m prime, a not 0 mod m).
fn modinv(a: u64, m: u64) -> u64 {
    // a^(m-2) mod m by Fermat, m prime.
    powmod(a % m, m - 2, m)
}

/// Read a tape as a u128 little-endian (cell EVALF at position i is bit i), or
/// None when it does not fit in 127 bits.
fn tape_to_u128(n: &Tape) -> Option<u128> {
    let t = trim(n.clone());
    if t.len() > 127 {
        return None;
    }
    let mut v = 0u128;
    for (i, &c) in t.iter().enumerate() {
        if c == EVALF {
            v |= 1u128 << i;
        }
    }
    Some(v)
}

/// Write a u128 as a little-endian tape (cell EVALF at each set bit).
fn u128_to_tape(mut v: u128) -> Tape {
    if v == 0 {
        return vec![crate::vox::EVALT];
    }
    let mut t = Tape::new();
    while v > 0 {
        t.push(if v & 1 == 1 { EVALF } else { crate::vox::EVALT });
        v >>= 1;
    }
    trim(t)
}

/// Step a k-combination of {0..n} to the next in lex order; false when exhausted.
fn next_combination(c: &mut [usize], n: usize) -> bool {
    let k = c.len();
    let mut i = k;
    while i > 0 {
        i -= 1;
        if c[i] < n - (k - i) {
            c[i] += 1;
            for j in i + 1..k {
                c[j] = c[j - 1] + 1;
            }
            return true;
        }
    }
    false
}

/// Signed tape addition. Each operand is (is_negative, magnitude); returns the sum
/// the same way. This lets g(x) and A x + B be carried on the tapes with a sign, so
/// the value arithmetic has no bit ceiling.
fn sadd(a: (bool, Tape), b: (bool, Tape)) -> (bool, Tape) {
    if a.0 == b.0 {
        (a.0, trim(add(&a.1, &b.1)))
    } else {
        match cmp(&a.1, &b.1) {
            core::cmp::Ordering::Less => (b.0, trim(sub(&b.1, &a.1))),
            _ => {
                let m = trim(sub(&a.1, &b.1));
                (if zero(&m) { false } else { a.0 }, m)
            }
        }
    }
}

/// Integer k-th root of v (largest r with r^k <= v), for the A-prime sizing.
fn iroot(v: u128, k: u32) -> u128 {
    if k == 0 || v < 2 {
        return v.max(1);
    }
    let mut lo = 1u128;
    let mut hi = v;
    while lo < hi {
        let candidate = lo + (hi - lo) / 2 + 1;
        let mut p = 1u128;
        let mut over = false;
        for _ in 0..k {
            match p.checked_mul(candidate) {
                Some(np) if np <= v => p = np,
                None => {
                    over = true;
                    break;
                }
                Some(_) => { over = true; break; }
            }
        }
        if over { hi = candidate - 1; } else { lo = candidate; }
    }
    lo
}

fn coefficient_prime_size(target: &Tape, bound: usize) -> (usize, u128) {
    let limit = (bound * 3 / 5).max(2) as u128;
    let root = |degree: usize| -> u128 {
        if let Some(value) = tape_to_u128(target) {
            iroot(value, degree as u32)
        } else {
            tape_to_u128(&crate::morphism_factor::iroot(target, degree)).unwrap_or(u128::MAX)
        }
    };
    let size_bits = (127 - limit.leading_zeros()).max(1) as usize;
    let mut degree = target.len().div_ceil(size_bits).max(3);
    while degree > 3 && root(degree - 1) <= limit { degree -= 1; }
    let mut size = root(degree);
    while size > limit { degree += 1; size = root(degree); }
    (degree, size)
}

fn next_sibling(signs: &mut [bool]) -> bool {
    for sign in signs {
        *sign = !*sign;
        if *sign { return true; }
    }
    false
}

fn wide_crt_terms(a: &Tape, indices: &[usize], base: &[u64], roots: &[u64]) -> Vec<Tape> {
    indices.iter().map(|&index| {
        let prime = base[index];
        let quotient = divmod(a, &tape_u64(prime)).0;
        let inverse = modinv(n_mod_u64(&quotient, prime), prime);
        let weight = (roots[index] as u128 * inverse as u128 % prime as u128) as u64;
        modulo(&mul(&quotient, &tape_u64(weight)), a)
    }).collect()
}

/// Multiple-polynomial quadratic sieve over folded numeral tapes. Machine-word
/// arithmetic is used where the exact intermediates fit. Each polynomial is
/// g(x) = A x^2 + 2B x + C with A a product of
/// base primes and B^2 == N (mod A), so (A x + B)^2 == A g(x) (mod N) and the A
/// factors sit in the base. A fresh polynomial keeps its values small near its own
/// root, so relations come thick without the single-polynomial window growing with
/// N. The -1 sign of g rides a phantom base column so the GF(2) combine, shared
/// with Dixon and single-poly QS, needs no change. Returns a factor or None.
pub fn mpqs(n: &Tape, base_bound: usize, m_half: usize, extra: usize) -> Option<Tape> {
    mpqs_with_source_port(n, base_bound, m_half, extra, || None).map(|(divisor, _)| divisor)
}

/// Retain the sieve relations while returning through another N-bound port.
/// The enclosing producer owns that port's state and checks its provenance.
pub(crate) fn mpqs_with_source_port(n: &Tape, base_bound: usize, m_half: usize,
    extra: usize, mut source_port: impl FnMut() -> Option<Tape>) -> Option<(Tape, &'static str)> {
    for counter in &VOX_SIEVE_COUNTERS {
        counter.store(0, core::sync::atomic::Ordering::Relaxed);
    }
    let n = trim(n.clone());
    let bits = n.len();
    // Wider targets amortize polynomial setup across a larger window.
    let m_half = m_half.max(match bits {
        0..=120 => 32_768,
        121..=150 => 131_072,
        _ => 1_048_576,
    });
    // N and the coefficient target stay on the tape. A and B use the same full
    // tape path when their CRT intermediates exceed a machine word. C, g(x),
    // and Ax+B also retain their complete values.
    let polynomial_n = mul(&n, &tape_u64(polynomial_multiplier(&n)));
    let sqrt2n = isqrt(&mul(&tape_u64(2), &polynomial_n));
    // Free lunch, no cap: run the per-x value in a machine word while it fits
    // (g ~ M*sqrt(2N)), and only fall to the tapes when it would overflow. Fast
    // below the boundary, uncapped above it.
    let wide = sqrt2n.len() + (usize::BITS - m_half.leading_zeros()) as usize + 4 >= 126;
    let flog2 = |x: u128| -> u32 {
        if x < 2 {
            0
        } else {
            127 - x.leading_zeros()
        }
    };
    // Target A ~ sqrt(2N)/M.
    let mut a_target = divmod(&sqrt2n, &u128_to_tape(m_half.max(1) as u128)).0;
    if cmp(&a_target, &tape_u64(8)).is_lt() { a_target = tape_u64(8); }
    sieve_gauge(13, a_target.len());
    // Factor-base bound near the sieve optimum exp(0.5*sqrt(ln N ln ln N)), which
    // grows slowly with N. Too small a base makes smooth values too rare to
    // collect; this table tracks the optimum by width (no float in no_std).
    let opt_bound = match bits {
        0..=70 => base_bound,
        71..=90 => 6_000,
        91..=110 => 9_000,
        111..=125 => 15_000,
        126..=140 => 30_000,
        141..=155 => 55_000,
        156..=170 => 90_000,
        171..=185 => 150_000,
        _ => 250_000,
    };
    let eff_bound = base_bound.max(opt_bound);
    let residual_bound2_u = (eff_bound as u128) * (eff_bound as u128);
    let residual_bound4_u = residual_bound2_u.checked_mul(residual_bound2_u).unwrap_or(u128::MAX);
    let residual_bound2 = u128_to_tape(residual_bound2_u);
    let residual_bound4 = mul(&residual_bound2, &residual_bound2);
    let primes = small_primes(eff_bound);
    // QR base: primes where N is a residue, each with a root of N. Index 0 is the
    // phantom -1 (sign), value 1 so it contributes nothing to the reconstruction.
    let mut base: Vec<u64> = vec![1];
    let mut sqrt_n: Vec<u64> = vec![0];
    for &p in &primes {
        let np = n_mod_u64(&polynomial_n, p);
        if np == 0 {
            if n_mod_u64(&n,p) == 0 { return Some((tape_u64(p), "mpqs-base-source")); }
            base.push(p);
            sqrt_n.push(0);
            continue;
        }
        if p == 2 {
            base.push(2);
            sqrt_n.push(1);
        } else if legendre(np, p) == 1 {
            if let Some(r) = tonelli(np, p) {
                base.push(p);
                sqrt_n.push(r);
            }
        }
    }
    let width = base.len();
    if width < 4 {
        return None;
    }
    let reciprocal32: Vec<u64> = base
        .iter()
        .map(|&prime| (1u64 << 32) / prime)
        .collect();
    let need = width + extra;
    sieve_gauge(0, bits);
    sieve_gauge(1, width);
    sieve_gauge(2, need);
    // A is a product of k distinct QR primes each near a_target^(1/k), so their
    // product lands close to the optimal A that keeps the polynomial values small.
    // k grows with N so the per-prime size stays inside the factor base, which is
    // what lifts the arm past the width where three big primes would need an
    // unreachable base. Choosing near the k-th root, not the largest primes, is
    // what keeps the values minimal and the smooth hits frequent.
    let (k, s) = coefficient_prime_size(&a_target, base_bound);
    // A-prime band around the per-prime size s. A wide band gives many distinct
    // k-subsets, which is what keeps each polynomial's A fresh so relations do not
    // repeat before a dependency forms.
    let lo = (s * 2 / 5).max(3);
    let hi = (s * 3).max(8);
    let mut a_pool: Vec<usize> = (1..width)
        .filter(|&i| base[i] > 2 && sqrt_n[i] != 0 && base[i] as u128 >= lo && base[i] as u128 <= hi)
        .collect();
    if a_pool.len() < k {
        return None;
    }
    // Visit the same prime combinations in order of distance from the live
    // per-prime target. Starting at the band's low edge makes A far too small,
    // which enlarges N/A and makes the polynomial values harder to close.
    a_pool.sort_by_key(|&index| ((base[index] as u128).abs_diff(s), base[index]));
    let npool = a_pool.len();
    let m = m_half as i128;
    let span = (2 * m_half + 1) as usize;
    let mut a_of: Vec<Tape> = Vec::new();
    let mut exp_of: Vec<Vec<u32>> = Vec::new();
    let mut square_factors = Vec::new();
    let mut online_relations = OnlineRelations::new(width);
    let mut partials = PartialRelations::new();
    let mut seen: alloc::collections::BTreeSet<Tape> = alloc::collections::BTreeSet::new();
    let n_tape = polynomial_n;
    let lp: Vec<i32> = base.iter().map(|&p| flog2(p as u128) as i32).collect();
    let thresh_slack = (flog2(eff_bound as u128) + flog2(width as u128)) as i32;
    let score_block = span.min(score_block_capacity(width));

    // A owns the inverses and sieve storage. Its B siblings prepare window
    // offsets, which their candidates consume without repeating that setup.
    // Distinct A sets come from lexicographic combinations of the prime pool.
    let mut combo: Vec<usize> = (0..k).collect();
    let mut _poly = 0usize;
    let mut a_count = 0usize;
    // Check the rho arm at amortized checkpoints while the sieve advances. A
    // complete rho batch per polynomial repeats orbit work on balanced inputs.
    let two = tape_u64(2);
    let (mut rx, mut ry, mut rc, mut rprod) = (two.clone(), two.clone(), one(), one());
    let rho_close = |g: &Tape| cmp(g, &one()) == core::cmp::Ordering::Greater && cmp(g, &n) == core::cmp::Ordering::Less;
    'outer: while a_of.len() < need {
        if let Some(divisor) = source_port() {
            if rho_close(&divisor) && zero(&divmod(&n, &divisor).1) {
                return Some((divisor, "retained-source-port"));
            }
        }
        // The current factor-base width sets both the orbit batch and its
        // checkpoint spacing, so rho's work scales with the live sieve shape.
        if a_count % width == 0 {
            for _ in 0..width {
                rx = mul_mod_add(&rx, &rx, &rc, &n);
                let y1 = mul_mod_add(&ry, &ry, &rc, &n);
                ry = mul_mod_add(&y1, &y1, &rc, &n);
                let d = if cmp(&rx, &ry) != core::cmp::Ordering::Less { sub(&rx, &ry) } else { sub(&ry, &rx) };
                let dt = trim(d);
                if !zero(&dt) {
                    rprod = mul_mod(&rprod, &dt, &n);
                }
            }
            let g = gcd(trim(rprod.clone()), n.clone());
            if rho_close(&g) {
                return Some((g, "mpqs-rho-source"));
            }
            if cmp(&g, &n) == core::cmp::Ordering::Equal {
                rc = add(&rc, &one());
                rx = two.clone();
                ry = two.clone();
            }
            rprod = one();
        }
        let ks: Vec<usize> = combo.iter().map(|&c| a_pool[c]).collect();
        let a_t = ks.iter().fold(one(), |a, &index| mul(&a, &tape_u64(base[index])));
        let a_val = tape_to_u128(&a_t).unwrap_or(0);
        let coefficient_wide = a_t.len() + ks.iter().map(|&i|
            (64 - base[i].leading_zeros()) as usize).max().unwrap_or(0) >= 128;
        a_count += 1;
        let combinations_exhausted = !next_combination(&mut combo, npool);
        let kk = ks.len();
        if kk < 3 {
            if combinations_exhausted {
                break 'outer;
            }
            continue;
        }

        // per-A CRT terms: B_l ≡ sqrt_n mod its own prime, 0 mod the other A-primes
        let mut bl = vec![0u128; kk];
        let bl_t = if coefficient_wide { wide_crt_terms(&a_t, &ks, &base, &sqrt_n) } else { Vec::new() };
        for (l, &idx) in ks.iter().enumerate() {
            let q = base[idx] as u128;
            if !coefficient_wide {
                let mj = a_val / q;
                let inv = modinv((mj % q) as u64, base[idx]) as u128;
                let rj = sqrt_n[idx] as u128 % q;
                bl[l] = ((rj * (mj % a_val)) % a_val) * inv % a_val;
            }
        }
        // per-A setup, computed ONCE: A^{-1} mod p, the expensive modular inverse.
        // skip[j] marks a prime left out of the sieve (2, or a divisor of A),
        // handled in the factor step instead.
        let mut ainv = vec![0i64; width];
            let mut skip = vec![true; width];
            for j in 1..width {
                let p = base[j];
            let amod = if coefficient_wide { n_mod_u64(&a_t, p) } else { (a_val % p as u128) as u64 };
            if p == 2 || sqrt_n[j] == 0 || amod == 0 {
                continue;
            }
            ainv[j] = modinv(amod, p) as i64;
            skip[j] = false;
        }
        let thresh = if coefficient_wide {
            (mul(&a_t, &u128_to_tape((m_half as u128).pow(2))).len() - 1) as i32
        } else { polynomial_score_log(a_val,m as u128) } - thresh_slack;

        // inner: each of the 2^(k-1) sign patterns is a B sibling that reuses the
        // per-A inverse; its two roots per prime are recomputed directly from the
        // cached inverse (one multiply each, the same cost an incremental update
        // would be, and correct without any carry bookkeeping).
        let mut signs = vec![false; kk - 1];
        let mut soln1 = vec![0i64; width];
        let mut soln2 = vec![0i64; width];
        // The enclosing A frame owns storage reused by every B sibling. The sieve
        // runs in cache-resident blocks: `blk` is one block's log column, and
        // next1/next2 carry each prime's running mark position across blocks so no
        // hit is recomputed. Blocking keeps the working set in cache, which is what
        // the bandwidth-bound span sieve was thrashing.
        let mut blk = vec![0i32; score_block];
        let mut candidates = Vec::new();
        let mut folded_scores = Vec::new();
        let mut next1 = vec![0i64; width];
        let mut next2 = vec![0i64; width];
        loop {
            if a_of.len() >= need {
                break 'outer;
            }
            if let Some(divisor) = source_port() {
                if rho_close(&divisor) && zero(&divmod(&n, &divisor).1) {
                    return Some((divisor, "retained-source-port"));
                }
            }
            _poly += 1;
            sieve_gauge(3, _poly);
            // B = bl[0] + sum_{l>=1} (±bl[l]); pattern bit picks the sign
            let mut b_cur = if coefficient_wide { 0 } else { bl[0] % a_val };
            let mut b_cur_t = if coefficient_wide { bl_t[0].clone() } else { one() };
            for l in 1..kk {
                if coefficient_wide {
                    b_cur_t = if signs[l - 1] {
                        modulo(&sub(&add(&b_cur_t, &a_t), &bl_t[l]), &a_t)
                    } else { modulo(&add(&b_cur_t, &bl_t[l]), &a_t) };
                    continue;
                }
                let blm = bl[l] % a_val;
                if signs[l - 1] {
                    b_cur = (b_cur + a_val - blm) % a_val;
                } else {
                    b_cur = (b_cur + blm) % a_val;
                }
            }
            sieve_gauge(12, a_t.len());
            let a_i = a_val as i128;
            // Center the representative consistently for roots and coefficients.
            // Subtracting A translates the polynomial by one x position.
            let b_i = if !coefficient_wide && b_cur > a_val / 2 {
                b_cur as i128 - a_i
            } else {
                b_cur as i128
            };
            let (b_neg, b_t) = if coefficient_wide {
                if cmp(&mul(&b_cur_t, &two), &a_t).is_gt() {
                    (true, sub(&a_t, &b_cur_t))
                } else { (false, b_cur_t) }
            } else { (b_i < 0, u128_to_tape(b_i.unsigned_abs())) };
            for j in 1..width {
                if skip[j] {
                    soln1[j] = -1;
                    continue;
                }
                let p = base[j];
                let pi = p as i64;
            let t = sqrt_n[j] as i64;
                let bmod = if coefficient_wide {
                    let residue = n_mod_u64(&b_t, p);
                    (if b_neg && residue != 0 { p - residue } else { residue }) as i64
                } else { b_i.rem_euclid(pi as i128) as i64 };
                soln1[j] = (ainv[j] * (t - bmod)).rem_euclid(pi);
                soln2[j] = (ainv[j] * ((pi - t) - bmod)).rem_euclid(pi);
                // Store window coordinates once per sibling. Both sieving and
                // candidate division consume these same prepared offsets.
                soln1[j] = (soln1[j] + m as i64) % pi;
                soln2[j] = (soln2[j] + m as i64) % pi;
            }
            // C = (B^2 - N)/A on the tapes; C < 0 since B^2 < N. Kept as magnitude.
            let b2 = mul(&b_t, &b_t);
            if cmp(&b2, &n_tape).is_gt() { return None; }
            let (c_mag, remainder) = divmod(&sub(&n_tape, &b2), &a_t);
            if !zero(&remainder) { return None; }
            // machine-word C, valid only on the fast path (values fit i128)
            let cc_i: i128 = if wide { 0 } else { -(tape_to_u128(&c_mag).unwrap_or(0) as i128) };

            // running mark positions start at the roots and advance across blocks
            next1.copy_from_slice(&soln1);
            next2.copy_from_slice(&soln2);
            let unfolded_start = prepare_score_period(&base, &soln1, &soln2, &lp,
                score_block, &mut folded_scores);
            let mut bstart = 0usize;
            while bstart < span {
                let bend = (bstart + score_block).min(span);
                let blen = bend - bstart;
                seed_score_period(&mut blk[..blen], &folded_scores, bstart);
                for j in unfolded_start..width {
                    if soln1[j] < 0 {
                        continue;
                    }
                    let pi = base[j] as usize;
                    let l = lp[j];
                    let mut idx = next1[j] as usize;
                    while idx < bend {
                        // Running positions are >= bstart. The loop guard puts
                        // idx-bstart below blen, and blen is at most blk.len().
                        unsafe { *blk.get_unchecked_mut(idx - bstart) += l; }
                        idx += pi;
                    }
                    next1[j] = idx as i64;
                    let mut idx2 = next2[j] as usize;
                    while idx2 < bend {
                        // The same carried-position invariant applies to lane 2.
                        unsafe { *blk.get_unchecked_mut(idx2 - bstart) += l; }
                        idx2 += pi;
                    }
                    next2[j] = idx2 as i64;
                }
                threshold_positions(&blk[..blen], thresh, &mut candidates);
                VOX_SIEVE_COUNTERS[4].fetch_add(blen as u64, core::sync::atomic::Ordering::Relaxed);
                VOX_SIEVE_COUNTERS[5].fetch_add(candidates.len() as u64, core::sync::atomic::Ordering::Relaxed);
                for &off in &candidates {
                    let xi = bstart + off;
                    // hit test: which base primes land on this position
                    let hit = |j: usize| -> bool {
                        if soln1[j] < 0 {
                            skip[j]
                        } else {
                            let xr = remainder_u32(xi as u32, base[j] as u32, reciprocal32[j]);
                            i64::from(xr) == soln1[j] || i64::from(xr) == soln2[j]
                        }
                    };
                    // Produce the factor-base exponents, |Ax+B| and residual word.
                    // Machine word while g fits it (fast), tapes when it would not.
                    let relation: Option<(Vec<u32>, Tape, Tape)> = if !wide {
                        let x = xi as i128 - m;
                        let g = a_i * x * x + 2 * b_i * x + cc_i;
                        sieve_gauge(14, (128 - g.unsigned_abs().leading_zeros()) as usize);
                        if g == 0 {
                            None
                        } else {
                            let mut val = g.unsigned_abs();
                            let mut exps = vec![0u32; width];
                            if g < 0 {
                                exps[0] = 1;
                            }
                            for j in 1..width {
                                if val == 1 {
                                    break;
                                }
                                if !hit(j) {
                                    continue;
                                }
                                let pu = base[j] as u128;
                                while val % pu == 0 {
                                    exps[j] += 1;
                                    val /= pu;
                                }
                            }
                            if val != 1 {
                                let bucket = if val <= residual_bound2_u { 9 }
                                    else if val <= residual_bound4_u { 10 } else { 11 };
                                VOX_SIEVE_COUNTERS[bucket].fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                            }
                            if val > residual_bound2_u { None } else {
                                for &idx in &ks { exps[idx] += 1; }
                                let axb = a_i * x + b_i;
                                Some((exps, u128_to_tape(axb.unsigned_abs()), u128_to_tape(val)))
                            }
                        }
                    } else {
                        let x = xi as i128 - m;
                        let x_neg = x < 0;
                        let x_abs = x.unsigned_abs();
                        let x_t = u128_to_tape(x_abs);
                        let x2_t = u128_to_tape(x_abs.wrapping_mul(x_abs));
                        let term1 = (false, mul(&a_t, &x2_t));
                        let term2 = (b_neg ^ x_neg, mul(&mul(&tape_u64(2), &b_t), &x_t));
                        let (g_neg, mut val) = sadd(sadd(term1, term2), (true, c_mag.clone()));
                        sieve_gauge(14, val.len());
                        let one_t = vec![EVALF];
                        if zero(&val) {
                            None
                        } else {
                            let mut exps = vec![0u32; width];
                            if g_neg {
                                exps[0] = 1;
                            }
                            for j in 1..width {
                                if val == one_t {
                                    break;
                                }
                                if !hit(j) {
                                    continue;
                                }
                                let pt = tape_u64(base[j]);
                                loop {
                                    let (q, r) = divmod(&val, &pt);
                                    if zero(&r) {
                                        exps[j] += 1;
                                        val = q;
                                    } else {
                                        break;
                                    }
                                }
                            }
                            if val != one_t {
                                let bucket = residual_bucket(&val, &residual_bound2, &residual_bound4);
                                VOX_SIEVE_COUNTERS[bucket].fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                            }
                            if cmp(&val, &residual_bound2) == core::cmp::Ordering::Greater { None } else {
                                for &idx in &ks { exps[idx] += 1; }
                                let (_s, axb) = sadd((x_neg, mul(&a_t, &x_t)), (b_neg, b_t.clone()));
                                Some((exps, trim(axb), val))
                            }
                        }
                    };
                    let (exps, axb_t, residual) = match relation {
                        Some(r) => r,
                        None => continue,
                    };
                    if !seen.insert(axb_t.clone()) {
                        continue;
                    }
                    let Some((exps, axb_t, square_factor)) = fold_residual_relation(
                        &n, axb_t, exps, residual, &mut partials) else { continue; };
                    #[cfg(feature = "mpqs_debug")]
                    if a_of.is_empty() {
                        extern crate std;
                        let lhs = mul_mod(&axb_t, &axb_t, &n_tape);
                        let mut ag = mul_mod(&square_factor, &square_factor, &n_tape);
                        for c in 1..width {
                            for _ in 0..exps[c] {
                                ag = mul_mod(&ag, &u128_to_tape(base[c] as u128), &n_tape);
                            }
                        }
                        let rhs = if exps[0] & 1 == 1 && !zero(&ag) { sub(&n_tape, &ag) } else { ag };
                        std::eprintln!(
                            "[mpqs] relation square closes modulo N : {}",
                            if trim(lhs) == trim(rhs) { "PASS" } else { "FAIL" }
                        );
                    }
                    a_of.push(axb_t);
                    exp_of.push(exps);
                    square_factors.push(square_factor);
                    sieve_gauge(6, a_of.len());
                    if let Some(selected) = online_relations.insert(exp_of.last().unwrap(), a_of.len() - 1) {
                        if let Some(divisor) = dependency_divisor(&n, &selected, &a_of,
                            &exp_of, &base, Some(&square_factors)) {
                            return Some((divisor, "mpqs-relation-source"));
                        }
                    }
                    if a_of.len() >= need {
                        break 'outer;
                    }
                }
                bstart += score_block;
            }
            if !next_sibling(&mut signs) { break; }
        }
        if combinations_exhausted {
            break 'outer;
        }
    }
    #[cfg(feature = "mpqs_debug")]
    {
        extern crate std;
        std::eprintln!(
            "[mpqs] bits={} k={} s={} eff_bound={} pool={} width={} need={} relations={} polys={}",
            bits, k, s, eff_bound, a_pool.len(), width, need, a_of.len(), _poly
        );
    }
    #[cfg(feature = "mpqs_debug")]
    {
        extern crate std;
        let t = std::time::Instant::now();
        let r = combine_with_square_factors(&n, &a_of, &exp_of, &base, Some(&square_factors));
        std::eprintln!("[mpqs] combine took {:?}", t.elapsed());
        return r.map(|divisor| (divisor, "mpqs-relation-source"));
    }
    #[cfg(not(feature = "mpqs_debug"))]
    combine_with_square_factors(&n, &a_of, &exp_of, &base, Some(&square_factors))
        .map(|divisor| (divisor, "mpqs-relation-source"))
}

/// Base bound and window sized from the width of N: B grows about like the
/// square of the digit count, the window a few hundred thousand.
pub fn sieve_params(n: &Tape) -> (usize, usize) {
    let bits = trim(n.clone()).len();
    let bound = ((bits * bits) / 3 + 300).min(60_000);
    // Single-polynomial window: a^2-N grows across the interval, so the count of
    // smooth values is thin and the window must widen with N to collect ~B
    // relations. Below 64 bits the narrow window already suffices; above it the
    // window grows with the extra width.
    // Single-poly QS is now the fallback behind MPQS, so its window no longer
    // needs to chase the width without limit; a few million positions is enough
    // for the narrow N that reach it.
    let m = if bits <= 64 {
        1_500_000usize
    } else {
        (1_500_000usize + (bits - 64) * 750_000usize).min(6_000_000usize)
    };
    (bound, m)
}

pub fn sieve_factor(n: &Tape) -> Option<Tape> {
    let (bound, m) = sieve_params(n);
    qs(n, bound, m, 16).or_else(|| dixon(n, bound, 8, 2_000_000))
}

pub fn repl_sieve(n: &Tape) -> String {
    let (bound, _m) = sieve_params(n);
    match sieve_factor(n) {
        Some(g) => {
            let q = divmod(n, &g).0;
            format!("{} = {} x {}  [quadratic sieve, base<= {}]", dec(n), dec(&g), dec(&q), bound)
        }
        None => format!("{}  [sieve found no dependency within budget]", dec(n)),
    }
}

fn dec(t: &[char]) -> String {
    let mut b = trim(t.to_vec());
    if zero(&b) {
        return "0".into();
    }
    let ten = tape_u64(10);
    let mut ds = Vec::new();
    while !zero(&b) {
        let (q, r) = divmod(&b, &ten);
        let mut dv = 0u8;
        for (i, &c) in trim(r).iter().enumerate() {
            if c == EVALF {
                dv |= 1 << i;
            }
        }
        ds.push(b'0' + dv);
        b = q;
    }
    ds.reverse();
    String::from_utf8(ds).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vox::EVALT;
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
    fn val(t: &Tape) -> u64 {
        let mut a = 0u64;
        for &c in trim(t.clone()).iter().rev() {
            a = (a << 1) | if c == EVALF { 1 } else { 0 };
        }
        a
    }
    #[test]
    fn dixon_factors() {
        for &n in &[8051u64, 100160063, 16843009, 2027651281] {
            let g = dixon(&tape(n), 500, 8, 2_000_000).expect("no factor");
            let gv = val(&g);
            assert!(gv > 1 && gv < n && n % gv == 0, "dixon({n}) = {gv}");
        }
    }

    #[test]
    fn mpqs_siblings_return_exact_divisors() {
        for decimal in ["588836796098867516121023", "3050585191915710906097942786821407"] {
            let n = crate::morphism_factor::decimal_to_tape(decimal).unwrap();
            let (bound, _) = sieve_params(&n);
            let g = mpqs(&n, bound, 32_768, 32).expect("mpqs no factor");
            let (q, r) = divmod(&n, &g);
            assert!(zero(&r));
            assert!(cmp(&g, &one()).is_gt());
            assert!(cmp(&q, &one()).is_gt());
            assert_eq!(trim(mul(&g, &q)), n);
        }
    }

    #[test]
    fn source_895_coefficient_target_and_crt_stay_on_full_tapes() {
        let n = crate::morphism_factor::decimal_to_tape(include_str!(
            "../../measurements/factor_synthesis_20261011/n_895.txt").trim()).unwrap();
        let target = divmod(&isqrt(&mul(&tape_u64(2), &n)), &tape_u64(1_048_576)).0;
        assert_eq!(target.len(), 428);
        let (degree, size) = coefficient_prime_size(&target, 60_000);
        assert!(degree > 16);
        assert!(size <= 36_000);
        let mut base = Vec::new();
        let mut roots = Vec::new();
        for prime in small_primes(1000).into_iter().filter(|&prime| prime > 2) {
            if let Some(root) = tonelli(n_mod_u64(&n, prime), prime) {
                base.push(prime); roots.push(root);
            }
        }
        let indices: Vec<usize> = (0..40).collect();
        let a = indices.iter().fold(one(), |a, &i| mul(&a, &tape_u64(base[i])));
        assert!(a.len() > 127);
        let terms = wide_crt_terms(&a, &indices, &base, &roots);
        for sign_word in [false, true] {
            let mut b = terms[0].clone();
            for (index, term) in terms.iter().enumerate().skip(1) {
                b = if sign_word && index % 2 == 1 {
                    modulo(&sub(&add(&b, &a), term), &a)
                } else { modulo(&add(&b, term), &a) };
            }
            assert_eq!(modulo(&mul(&b, &b), &a), modulo(&n, &a));
            let b2 = mul(&b, &b);
            assert!(cmp(&b2, &n).is_lt());
            let (c, remainder) = divmod(&sub(&n, &b2), &a);
            assert!(zero(&remainder));
            let x = tape_u64(1_048_576);
            let axb = add(&mul(&a, &x), &b);
            let (negative, g) = sadd((false, add(&mul(&a, &mul(&x, &x)),
                &mul(&mul(&tape_u64(2), &b), &x))), (true, c));
            let ag = mul_mod(&a, &g, &n);
            let signed = if negative && !zero(&ag) { sub(&n, &ag) } else { ag };
            assert_eq!(mul_mod(&axb, &axb, &n), signed);
        }
        let mut signs = vec![true; 130]; signs[129] = false;
        assert!(next_sibling(&mut signs));
        assert!(signs[129] && signs[..129].iter().all(|sign| !sign));
    }

    #[test]
    fn online_relation_histories_reconstruct_the_exact_even_exponents() {
        let width = 129;
        let mut bank = OnlineRelations::new(width);
        let rows: Vec<Vec<u32>> = (0..300).map(|index| {
            (0..width).map(|column| u32::from(
                (column == index % 65) || (column == (index * 7 + 3) % 65))).collect()
        }).collect();
        let mut dependencies = 0;
        for (index, row) in rows.iter().enumerate() {
            if let Some(selected) = bank.insert(row, index) {
                assert!(!selected.is_empty());
                assert!(selected.contains(&index));
                for column in 0..width {
                    let total: u32 = selected.iter().map(|&i| rows[i][column]).sum();
                    assert_eq!(total % 2, 0);
                }
                dependencies += 1;
            }
        }
        assert!(dependencies > 200);
    }

    #[test]
    fn coefficient_integer_root_closes_at_machine_boundary() {
        for value in [2u128, 8, 26, 27, 28, (1u128 << 108) - 1, u128::MAX] {
            for degree in 1..=16 {
                let root = iroot(value, degree);
                assert!(root.checked_pow(degree).is_some_and(|power| power <= value));
                assert!(root.checked_add(1).and_then(|next| next.checked_pow(degree))
                    .is_none_or(|power| power > value));
            }
        }
    }

    #[test]
    fn mpqs_source_return_rejects_improper_divisors_and_keeps_sieve_work() {
        let n = crate::morphism_factor::decimal_to_tape("3050585191915710906097942786821407").unwrap();
        let (bound, _) = sieve_params(&n);
        let mut returns = 0;
        let (g, producer) = mpqs_with_source_port(&n, bound, 32_768, 32, || {
            returns += 1;
            match returns {
                1 => Some(one()),
                2 => Some(n.clone()),
                3 => Some(tape_u64(4)),
                _ => None,
            }
        }).expect("retained sieve failed after invalid source returns");
        assert!(returns > 3);
        assert_eq!(producer, "mpqs-relation-source");
        let (q, remainder) = divmod(&n, &g);
        assert!(zero(&remainder));
        assert!(cmp(&g, &one()).is_gt() && cmp(&g, &n).is_lt());
        assert_eq!(trim(mul(&g, &q)), n);
    }

    #[test]
    fn qs_factors_through_the_full_pipeline() {
        // QR base + Tonelli roots + log-sieve + GF(2) solve, end to end.
        for &n in &[8051u64, 100160063, 2027651281, 191873633311] {
            let g = qs(&tape(n), 500, 200_000, 12).expect("qs no factor");
            let gv = val(&g);
            assert!(gv > 1 && gv < n && n % gv == 0, "qs({n}) = {gv}");
        }
    }
}
