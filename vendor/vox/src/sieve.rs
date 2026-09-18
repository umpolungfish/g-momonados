//! sieve.rs — a Dixon / quadratic-sieve core over the folded numeral tapes.
//!
//! The sub-exponential arm for the HARD shape: a factor that is large, far from
//! the root, and not smooth, where trial, rho, the frontier and p±1 all fail.
//! It collects relations a^2 == q (mod N) whose q factors over a small-prime
//! base, finds a subset of relations whose exponents are all even (a linear
//! dependency over GF(2)), and from the resulting X^2 == Y^2 (mod N) takes
//! gcd(X - Y, N). Value-sized arithmetic (a^2 mod N, the gcd) runs on the shared
//! folded kernel; the base primes and the GF(2) matrix are machine words.

use crate::morphism_factor::{add, cmp, divmod, gcd, isqrt, modulo, mul, one, sub, tape_u64, trim, zero};
use crate::vox::EVALF;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

type Tape = Vec<char>;

// ---- machine-word number theory for the base and the roots ----

fn mulmod(a: u64, b: u64, m: u64) -> u64 {
    ((a as u128 * b as u128) % m as u128) as u64
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
        let q = modulo(&mul(&a, &a), n); // a^2 mod N
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
    let width = base.len();
    let rel = a_of.len();
    if rel < 2 || width == 0 {
        return None;
    }
    // Structured Gaussian elimination first: a relation that owns a prime no other
    // relation carries (a column of weight one) cannot sit in any dependency, so
    // drop it; that lowers other columns' weights and cascades. What survives keeps
    // every dependency the full set held, over only the heavy columns (weight >= 2).
    // This collapses the plane, width ~9000 down to a few hundred, before the dense
    // solve, which then runs on the small residual.
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
            let col = (0..rwidth).find(|&c| (mat[r][c / 64] >> (c % 64)) & 1 == 1);
            match col {
                None => break,
                Some(c) => {
                    if pivot_row[c] == usize::MAX {
                        pivot_row[c] = r;
                        break;
                    } else {
                        let pr = pivot_row[c];
                        for w in 0..words {
                            mat[r][w] ^= mat[pr][w];
                        }
                        for w in 0..hwords {
                            hist[r][w] ^= hist[pr][w];
                        }
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
                x = modulo(&mul(&x, &a_of[i]), n);
            }
            let mut total = vec![0u32; width];
            for &i in &sel {
                for c in 0..width {
                    total[c] += exp_of[i][c];
                }
            }
            let mut y = one();
            for c in 0..width {
                for _ in 0..total[c] / 2 {
                    y = modulo(&mul(&y, &tape_u64(base[c])), n);
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
    let mut r = 1u128;
    while {
        let mut p = 1u128;
        let mut over = false;
        for _ in 0..k {
            match p.checked_mul(r + 1) {
                Some(np) => p = np,
                None => {
                    over = true;
                    break;
                }
            }
        }
        !over && p <= v
    } {
        r += 1;
    }
    r
}

/// Multiple-polynomial quadratic sieve over machine integers, exact for N up to
/// about 120 bits. Each polynomial is g(x) = A x^2 + 2B x + C with A a product of
/// base primes and B^2 == N (mod A), so (A x + B)^2 == A g(x) (mod N) and the A
/// factors sit in the base. A fresh polynomial keeps its values small near its own
/// root, so relations come thick without the single-polynomial window growing with
/// N. The -1 sign of g rides a phantom base column so the GF(2) combine, shared
/// with Dixon and single-poly QS, needs no change. Returns a factor or None.
pub fn mpqs(n: &Tape, base_bound: usize, m_half: usize, extra: usize) -> Option<Tape> {
    let n = trim(n.clone());
    let bits = n.len();
    // Wider targets amortize polynomial setup across a larger window.
    let m_half = m_half.max(match bits {
        0..=120 => 32_768,
        121..=150 => 131_072,
        _ => 524_288,
    });
    // N stays on the tape. The polynomial coefficients A and B fit a machine word
    // (they are near sqrt(N)); C, g(x) and A x + B are carried on the tapes, so the
    // value arithmetic has no bit ceiling. No cap on how large N may be.
    let sqrt2n = isqrt(&mul(&tape_u64(2), &n));
    let sqrt2n_u = tape_to_u128(&sqrt2n).unwrap_or(u128::MAX);
    // Free lunch, no cap: run the per-x value in a machine word while it fits
    // (g ~ M*sqrt(2N)), and only fall to the tapes when it would overflow. Fast
    // below the boundary, uncapped above it.
    let wide = 128 - sqrt2n_u.leading_zeros() as usize + (usize::BITS - m_half.leading_zeros()) as usize + 4 >= 126;
    let flog2 = |x: u128| -> u32 {
        if x < 2 {
            0
        } else {
            127 - x.leading_zeros()
        }
    };
    // Target A ~ sqrt(2N)/M.
    let a_target = (sqrt2n_u / (m_half as u128).max(1)).max(8);
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
    let primes = small_primes(eff_bound);
    // QR base: primes where N is a residue, each with a root of N. Index 0 is the
    // phantom -1 (sign), value 1 so it contributes nothing to the reconstruction.
    let mut base: Vec<u64> = vec![1];
    let mut sqrt_n: Vec<u64> = vec![0];
    for &p in &primes {
        let np = n_mod_u64(&n, p);
        if np == 0 {
            return Some(tape_u64(p));
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
    let need = width + extra;
    // A is a product of k distinct QR primes each near a_target^(1/k), so their
    // product lands close to the optimal A that keeps the polynomial values small.
    // k grows with N so the per-prime size stays inside the factor base, which is
    // what lifts the arm past the width where three big primes would need an
    // unreachable base. Choosing near the k-th root, not the largest primes, is
    // what keeps the values minimal and the smooth hits frequent.
    let mut k = 3usize;
    let mut s = iroot(a_target, k as u32);
    while s as usize > base_bound * 3 / 5 && k < 16 {
        k += 1;
        s = iroot(a_target, k as u32);
    }
    // A-prime band around the per-prime size s. A wide band gives many distinct
    // k-subsets, which is what keeps each polynomial's A fresh so relations do not
    // repeat before a dependency forms.
    let lo = (s * 2 / 5).max(3);
    let hi = (s * 3).max(8);
    let a_pool: Vec<usize> = (1..width)
        .filter(|&i| base[i] > 2 && base[i] as u128 >= lo && base[i] as u128 <= hi)
        .collect();
    if a_pool.len() < k {
        return None;
    }
    let npool = a_pool.len();
    let m = m_half as i128;
    let span = (2 * m_half + 1) as usize;
    let mut a_of: Vec<Tape> = Vec::new();
    let mut exp_of: Vec<Vec<u32>> = Vec::new();
    let mut seen: alloc::collections::BTreeSet<Tape> = alloc::collections::BTreeSet::new();
    let n_tape = n.clone();
    let lp: Vec<i32> = base.iter().map(|&p| flog2(p as u128) as i32).collect();
    let thresh_slack = (2 * (flog2(base_bound as u128) + 1) + 6) as i32;

    // A owns the inverses and sieve storage. Its B siblings prepare window
    // offsets, which their candidates consume without repeating that setup.
    // Distinct A sets come from lexicographic combinations of the prime pool.
    let mut combo: Vec<usize> = (0..k).collect();
    let mut _poly = 0usize;
    let max_a = 100_000usize;
    let mut a_count = 0usize;
    // Fork the rho arm into the sieve's own loop: a batch of rho advances each A
    // iteration, and whichever arm closes first returns. rho wins the unbalanced
    // shape (a small factor found in few steps), the sieve wins the balanced shape;
    // fused here, the membrane's time is the minimum of the two, by the wiring.
    let two = tape_u64(2);
    let (mut rx, mut ry, mut rc, mut rprod) = (two.clone(), two.clone(), one(), one());
    let rho_close = |g: &Tape| cmp(g, &one()) == core::cmp::Ordering::Greater && cmp(g, &n) == core::cmp::Ordering::Less;
    'outer: while a_of.len() < need && a_count < max_a {
        // rho arm: 2048 steps with a batched gcd, fused first-close with the sieve
        for _ in 0..2048 {
            rx = modulo(&add(&mul(&rx, &rx), &rc), &n);
            let y1 = modulo(&add(&mul(&ry, &ry), &rc), &n);
            ry = modulo(&add(&mul(&y1, &y1), &rc), &n);
            let d = if cmp(&rx, &ry) != core::cmp::Ordering::Less { sub(&rx, &ry) } else { sub(&ry, &rx) };
            let dt = trim(d);
            if !zero(&dt) {
                rprod = modulo(&mul(&rprod, &dt), &n);
            }
        }
        let g = gcd(trim(rprod.clone()), n.clone());
        if rho_close(&g) {
            return Some(g);
        }
        if cmp(&g, &n) == core::cmp::Ordering::Equal {
            rc = add(&rc, &one());
            rx = two.clone();
            ry = two.clone();
        }
        rprod = one();
        let ks: Vec<usize> = combo.iter().map(|&c| a_pool[c]).collect();
        let mut a_val: u128 = 1;
        for &kk in &ks {
            a_val *= base[kk] as u128;
        }
        a_count += 1;
        if !next_combination(&mut combo, npool) {
            combo = (0..k).collect();
        }
        let kk = ks.len();
        if kk < 3 {
            continue;
        }

        // per-A CRT terms: B_l ≡ sqrt_n mod its own prime, 0 mod the other A-primes
        let mut bl = vec![0u128; kk];
        for (l, &idx) in ks.iter().enumerate() {
            let q = base[idx] as u128;
            let mj = a_val / q;
            let inv = modinv((mj % q) as u64, base[idx]) as u128;
            let rj = sqrt_n[idx] as u128 % q;
            bl[l] = ((rj * (mj % a_val)) % a_val) * inv % a_val;
        }
        // per-A setup, computed ONCE: A^{-1} mod p, the expensive modular inverse.
        // skip[j] marks a prime left out of the sieve (2, or a divisor of A),
        // handled in the factor step instead.
        let mut ainv = vec![0i64; width];
        let mut skip = vec![true; width];
        for j in 1..width {
            let p = base[j];
            if p == 2 || a_val % p as u128 == 0 {
                continue;
            }
            ainv[j] = modinv((a_val % p as u128) as u64, p) as i64;
            skip[j] = false;
        }
        let thresh =
            flog2((a_val * (m as u128) * (m as u128)).max(2)) as i32 - thresh_slack;

        // inner: each of the 2^(k-1) sign patterns is a B sibling that reuses the
        // per-A inverse; its two roots per prime are recomputed directly from the
        // cached inverse (one multiply each, the same cost an incremental update
        // would be, and correct without any carry bookkeeping).
        let nb = 1usize << (kk - 1);
        let mut soln1 = vec![0i64; width];
        let mut soln2 = vec![0i64; width];
        // The enclosing A frame owns storage reused by every B sibling. The sieve
        // runs in cache-resident blocks: `blk` is one block's log column, and
        // next1/next2 carry each prime's running mark position across blocks so no
        // hit is recomputed. Blocking keeps the working set in cache, which is what
        // the bandwidth-bound span sieve was thrashing.
        const BLOCK: usize = 1 << 15;
        let mut blk = vec![0i32; BLOCK];
        let mut next1 = vec![0i64; width];
        let mut next2 = vec![0i64; width];
        for pat in 0..nb {
            if a_of.len() >= need {
                break 'outer;
            }
            _poly += 1;
            // B = bl[0] + sum_{l>=1} (±bl[l]); pattern bit picks the sign
            let mut b_cur = bl[0] % a_val;
            for l in 1..kk {
                let blm = bl[l] % a_val;
                if (pat >> (l - 1)) & 1 == 1 {
                    b_cur = (b_cur + a_val - blm) % a_val;
                } else {
                    b_cur = (b_cur + blm) % a_val;
                }
            }
            let a_i = a_val as i128;
            // Center the representative consistently for roots and coefficients.
            // Subtracting A translates the polynomial by one x position.
            let b_i = if b_cur > a_val / 2 {
                b_cur as i128 - a_i
            } else {
                b_cur as i128
            };
            for j in 1..width {
                if skip[j] {
                    soln1[j] = -1;
                    continue;
                }
                let p = base[j];
                let pi = p as i64;
                let t = sqrt_n[j] as i64;
                let bmod = b_i.rem_euclid(pi as i128) as i64;
                soln1[j] = (((ainv[j] * (((t - bmod) % pi) + pi)) % pi) + pi) % pi;
                soln2[j] = (((ainv[j] * ((((pi - t) - bmod) % pi) + pi)) % pi) + pi) % pi;
                // Store window coordinates once per sibling. Both sieving and
                // candidate division consume these same prepared offsets.
                soln1[j] = (soln1[j] + m as i64).rem_euclid(pi);
                soln2[j] = (soln2[j] + m as i64).rem_euclid(pi);
            }
            // C = (B^2 - N)/A on the tapes; C < 0 since B^2 < N. Kept as magnitude.
            let b_abs = b_i.unsigned_abs();
            let b2 = mul(&u128_to_tape(b_abs), &u128_to_tape(b_abs));
            let (c_mag, _r) = divmod(&sub(&n_tape, &b2), &u128_to_tape(a_val));
            let a_t = u128_to_tape(a_val);
            let b_t = u128_to_tape(b_abs);
            let b_neg = b_i < 0;
            // machine-word C, valid only on the fast path (values fit i128)
            let cc_i: i128 = if wide { 0 } else { -(tape_to_u128(&c_mag).unwrap_or(0) as i128) };

            // running mark positions start at the roots and advance across blocks
            next1.copy_from_slice(&soln1);
            next2.copy_from_slice(&soln2);
            let mut bstart = 0usize;
            while bstart < span {
                let bend = (bstart + BLOCK).min(span);
                let blen = bend - bstart;
                for e in blk[..blen].iter_mut() {
                    *e = 0;
                }
                for j in 1..width {
                    if soln1[j] < 0 {
                        continue;
                    }
                    let pi = base[j] as usize;
                    let l = lp[j];
                    let mut idx = next1[j] as usize;
                    while idx < bend {
                        blk[idx - bstart] += l;
                        idx += pi;
                    }
                    next1[j] = idx as i64;
                    let mut idx2 = next2[j] as usize;
                    while idx2 < bend {
                        blk[idx2 - bstart] += l;
                        idx2 += pi;
                    }
                    next2[j] = idx2 as i64;
                }
                for off in 0..blen {
                    if a_of.len() >= need {
                        break 'outer;
                    }
                    if blk[off] < thresh {
                        continue;
                    }
                    let xi = bstart + off;
                    let xi64 = xi as i64;
                    // hit test: which base primes land on this position
                    let hit = |j: usize| -> bool {
                        if soln1[j] < 0 {
                            skip[j]
                        } else {
                            let xr = xi64 % base[j] as i64;
                            xr == soln1[j] || xr == soln2[j]
                        }
                    };
                    // Produce the smooth relation (g_neg, exps, |Ax+B|) or skip.
                    // Machine word while g fits it (fast), tapes when it would not.
                    let relation: Option<(bool, Vec<u32>, Tape)> = if !wide {
                        let x = xi as i128 - m;
                        let g = a_i * x * x + 2 * b_i * x + cc_i;
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
                                None
                            } else {
                                for &idx in &ks {
                                    exps[idx] += 1;
                                }
                                let axb = a_i * x + b_i;
                                Some((g < 0, exps, u128_to_tape(axb.unsigned_abs())))
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
                                None
                            } else {
                                for &idx in &ks {
                                    exps[idx] += 1;
                                }
                                let (_s, axb) = sadd((x_neg, mul(&a_t, &x_t)), (b_neg, b_t.clone()));
                                Some((g_neg, exps, trim(axb)))
                            }
                        }
                    };
                    let (_g_neg, exps, axb_t) = match relation {
                        Some(r) => r,
                        None => continue,
                    };
                    if !seen.insert(axb_t.clone()) {
                        continue;
                    }
                    #[cfg(feature = "mpqs_debug")]
                    if a_of.is_empty() {
                        extern crate std;
                        let lhs = mul(&axb_t, &axb_t);
                        // A*|g| reconstructed from the exponents (A's primes plus g's)
                        let mut ag = one();
                        for c in 0..width {
                            for _ in 0..exps[c] {
                                ag = mul(&ag, &u128_to_tape(base[c] as u128));
                            }
                        }
                        let rhs = if _g_neg { sub(&n_tape, &ag) } else { add(&ag, &n_tape) };
                        std::eprintln!(
                            "[mpqs] identity (Ax+B)^2==A*g+N : {}",
                            if trim(lhs) == trim(rhs) { "PASS" } else { "FAIL" }
                        );
                    }
                    a_of.push(axb_t);
                    exp_of.push(exps);
                }
                bstart += BLOCK;
            }
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
        let r = combine(&n, &a_of, &exp_of, &base);
        std::eprintln!("[mpqs] combine took {:?}", t.elapsed());
        return r;
    }
    #[cfg(not(feature = "mpqs_debug"))]
    combine(&n, &a_of, &exp_of, &base)
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
    fn qs_factors_through_the_full_pipeline() {
        // QR base + Tonelli roots + log-sieve + GF(2) solve, end to end.
        for &n in &[8051u64, 100160063, 2027651281, 191873633311] {
            let g = qs(&tape(n), 500, 200_000, 12).expect("qs no factor");
            let gv = val(&g);
            assert!(gv > 1 && gv < n && n % gv == 0, "qs({n}) = {gv}");
        }
    }
}
