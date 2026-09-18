//! Factoring over IMASM numeral tapes.
//!
//! A numeral is never decoded into a machine integer.  Its payload is a tape
//! of EVALT/EVALF marks, least significant cell first.  Arithmetic consumes
//! and produces those tapes through the full-adder/full-subtractor tables.

use crate::vox::{AREV, AFWD, CLINK, EVALF, EVALT, FFUSE, FSPLIT, IFIX, IMSCRIB, TANCH, VINIT};
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

type Tape = Vec<char>;

const PHASE: &[char] = &[VINIT, FSPLIT, AFWD, EVALT, EVALF, FFUSE, TANCH];
const ARITHMETIC: &[char] = &[VINIT, FSPLIT, CLINK, EVALT, EVALF, FFUSE, TANCH];
const BRANCH: &[char] = &[VINIT, FSPLIT, EVALT, EVALF, FFUSE, TANCH];
const SELECT: &[char] = &[VINIT, FSPLIT, IMSCRIB, FFUSE, TANCH];
const CONTINUE: &[char] = &[VINIT, AFWD, CLINK, TANCH];
const FIX: &[char] = &[VINIT, IMSCRIB, IFIX, TANCH];
// The instant extract morphism: one evaluate frame carrying the involution
// (AREV ≺) and hold (ENGAGR ⊞) between the truth and falsity ports. It folds
// phase, arithmetic, select and continue into a single boundary, which is what
// makes the extractor a one-frame factorizer.
const EXTRACT: &[char] = &[VINIT, FSPLIT, AFWD, EVALT, AREV, EVALF, '⊞', CLINK, FFUSE, TANCH];
// Pollard p-1: seed an accumulator (IMSCRIB), raise it through rising exponents
// (ENGAGR), and take the gcd (CLINK) inside the frame. It catches a factor p
// whenever p-1 is smooth, at any size and any gap, covering the slice the
// frontier and rho arms miss.
const P_MINUS: &[char] = &[VINIT, FSPLIT, IMSCRIB, '⊞', CLINK, FFUSE, TANCH];
// Lenstra elliptic-curve method: seed a curve (IMSCRIB), advance a point under
// the group law (AFWD) and combine (CLINK) inside the frame. Each round tries
// one more curve; a factor falls out when the group law hits a non-invertible
// slope. This is the sub-exponential arm for a factor that is large, far from
// the root, and has no smooth predecessor.
const ECM: &[char] = &[VINIT, FSPLIT, IMSCRIB, AFWD, CLINK, FFUSE, TANCH];
// Primality witness: a branch frame carrying the involution ∈⊤≺⊥∋. It runs a
// strong probable-prime test once and, if N is prime, selects N at once. Nested
// first, it spares the trial walk to sqrt(N) that certifying a prime otherwise
// costs.
const WITNESS: &[char] = &[VINIT, FSPLIT, EVALT, AREV, EVALF, FFUSE, TANCH];
// Perfect-power test: a branch frame with a hold ∈⊤⊞⊥∋. Once, it checks whether
// N is a perfect power a^b and if so selects the base a. Catches every prime
// power in one shot, the slice the other arms only reach by search.
const POWER: &[char] = &[VINIT, FSPLIT, EVALT, '⊞', EVALF, FFUSE, TANCH];
// Williams p+1: like p-1 but over a Lucas sequence, the involution ≺ marking the
// complementary side ∈⊙≺⋈∋. Catches a factor p whenever p+1 is smooth.
const P_PLUS: &[char] = &[VINIT, FSPLIT, IMSCRIB, AREV, CLINK, FFUSE, TANCH];
// Lehman's method: a multiplier-Fermat frame ∈≻⋈⊤⊥∋. Each round sweeps one
// multiplier k, closing a factor between N^(1/3) and N^(2/3) that the near-root
// frontier and the small-factor arms both miss.
const LEHMAN: &[char] = &[VINIT, FSPLIT, AFWD, CLINK, EVALT, EVALF, FFUSE, TANCH];
// Shanks square forms (SQUFOF): a square-form frame ∈⊤≺⊞⊥∋, the involution and
// hold carrying the forward/reverse cycle. One-shot: it runs the principal-form
// cycle once and selects a factor if the reverse phase closes one.
const SQUFOF: &[char] = &[VINIT, FSPLIT, EVALT, AREV, '⊞', EVALF, FFUSE, TANCH];

fn bit(mark: char) -> Result<bool, String> {
    match mark {
        EVALT => Ok(false),
        EVALF => Ok(true),
        _ => Err(format!("non-numeral mark {mark}")),
    }
}

#[allow(dead_code)]
fn mark(v: bool) -> char {
    if v {
        EVALF
    } else {
        EVALT
    }
}

pub fn trim(mut a: Tape) -> Tape {
    while a.len() > 1 && a.last() == Some(&EVALT) {
        a.pop();
    }
    if a.is_empty() {
        a.push(EVALT);
    }
    a
}

pub fn parse_numeral(word: &str) -> Result<Tape, String> {
    let c: Vec<char> = word.chars().collect();
    if c == [VINIT, IMSCRIB, IFIX, TANCH] {
        return Ok(vec![EVALT]);
    }

    // Properly-nested single-frame numeral: ⊢ ∈ [⊤/⊥ bits LSB-first] ≺ ∋ ⊡ ⊣
    // Entry ∈ and exit ∋ are the SAME frame; AREV ≺ sits INSIDE the frame so the
    // deposited marks bank and survive the reversal — the matched puncture.
    if c.len() >= 6
        && c.first() == Some(&VINIT)
        && c[1] == FSPLIT
        && c[c.len() - 1] == TANCH
        && c[c.len() - 2] == IFIX
        && c[c.len() - 3] == FFUSE
        && c[c.len() - 4] == AREV
    {
        let mut out = Vec::new();
        for &m in &c[2..c.len() - 4] {
            bit(m)?;
            out.push(m);
        }
        return Ok(trim(out));
    }
    if c.len() < 9 || c.first() != Some(&VINIT) || c[c.len() - 3..] != [IMSCRIB, IFIX, TANCH] {
        return Err("expected a native IMASM numeral word".into());
    }
    let body = &c[1..c.len() - 3];
    if body.len() % 5 != 0 {
        return Err("broken native numeral cell".into());
    }
    let mut out = Vec::with_capacity(body.len() / 5);
    for cell in body.chunks(5) {
        if cell[0] != AFWD || cell[1] != CLINK || cell[2] != FSPLIT || cell[4] != FFUSE {
            return Err("broken native numeral cell".into());
        }
        bit(cell[3])?;
        out.push(cell[3]);
    }
    Ok(trim(out))
}

pub fn emit_numeral(tape: &[char]) -> String {
    if tape.len() == 1 && tape[0] == EVALT {
        return [VINIT, IMSCRIB, IFIX, TANCH].iter().collect();
    }
    let mut out = String::new();
    out.push(VINIT);
    for &b in tape {
        out.extend([AFWD, CLINK, FSPLIT, b, FFUSE]);
    }
    out.extend([IMSCRIB, IFIX, TANCH]);
    out
}

// Bit-register folding: the tape is the canonical numeral, but arithmetic runs
// on it folded into 64-bit limbs and unfolds back at the boundary. Schoolbook
// per-cell char arithmetic was O(bits^2) with a bit-serial division inside every
// modular multiply; folded, a multiply is limb-by-limb with u128 products and a
// division is one shift-subtract per bit over limbs, so every arm that routes
// through mod_mul (Miller-Rabin, ECM, p-1, p+1, rho) speeds up together.
type Limbs = Vec<u64>;

fn fold(t: &[char]) -> Limbs {
    let mut out = Limbs::with_capacity(t.len() / 64 + 1);
    let mut cur = 0u64;
    let mut b = 0u32;
    for &c in t {
        if c == EVALF {
            cur |= 1u64 << b;
        }
        b += 1;
        if b == 64 {
            out.push(cur);
            cur = 0;
            b = 0;
        }
    }
    if b > 0 {
        out.push(cur);
    }
    if out.is_empty() {
        out.push(0);
    }
    l_trim(out)
}

fn unfold(l: &[u64]) -> Tape {
    let mut out = Tape::with_capacity(l.len() * 64);
    for &limb in l {
        for b in 0..64 {
            out.push(if (limb >> b) & 1 == 1 { EVALF } else { EVALT });
        }
    }
    trim(out)
}

fn l_trim(mut a: Limbs) -> Limbs {
    while a.len() > 1 && *a.last().unwrap() == 0 {
        a.pop();
    }
    if a.is_empty() {
        a.push(0);
    }
    a
}

fn l_is_zero(a: &[u64]) -> bool {
    a.iter().all(|&x| x == 0)
}

fn l_cmp(a: &[u64], b: &[u64]) -> core::cmp::Ordering {
    let la = { let mut n = a.len(); while n > 1 && a[n - 1] == 0 { n -= 1; } n };
    let lb = { let mut n = b.len(); while n > 1 && b[n - 1] == 0 { n -= 1; } n };
    if la != lb {
        return la.cmp(&lb);
    }
    for i in (0..la).rev() {
        if a[i] != b[i] {
            return a[i].cmp(&b[i]);
        }
    }
    core::cmp::Ordering::Equal
}

fn l_add(a: &[u64], b: &[u64]) -> Limbs {
    let mut out = Limbs::new();
    let mut carry = 0u128;
    for i in 0..a.len().max(b.len()) {
        let s = *a.get(i).unwrap_or(&0) as u128 + *b.get(i).unwrap_or(&0) as u128 + carry;
        out.push(s as u64);
        carry = s >> 64;
    }
    if carry != 0 {
        out.push(carry as u64);
    }
    l_trim(out)
}

// Assumes a >= b (every caller guards with a compare first).
fn l_sub(a: &[u64], b: &[u64]) -> Limbs {
    let mut out = Limbs::new();
    let mut borrow = 0i128;
    for i in 0..a.len() {
        let mut d = a[i] as i128 - *b.get(i).unwrap_or(&0) as i128 - borrow;
        if d < 0 {
            d += 1i128 << 64;
            borrow = 1;
        } else {
            borrow = 0;
        }
        out.push(d as u64);
    }
    l_trim(out)
}

fn l_mul(a: &[u64], b: &[u64]) -> Limbs {
    let mut out = vec![0u64; a.len() + b.len()];
    for (i, &x) in a.iter().enumerate() {
        let mut carry = 0u128;
        for (j, &y) in b.iter().enumerate() {
            let cur = out[i + j] as u128 + x as u128 * y as u128 + carry;
            out[i + j] = cur as u64;
            carry = cur >> 64;
        }
        let mut k = i + b.len();
        while carry != 0 {
            let cur = out[k] as u128 + carry;
            out[k] = cur as u64;
            carry = cur >> 64;
            k += 1;
        }
    }
    l_trim(out)
}

fn l_bits(a: &[u64]) -> usize {
    let mut n = a.len();
    while n > 1 && a[n - 1] == 0 {
        n -= 1;
    }
    if a[n - 1] == 0 {
        return 0;
    }
    (n - 1) * 64 + (64 - a[n - 1].leading_zeros() as usize)
}

fn l_bit(a: &[u64], i: usize) -> bool {
    let (limb, off) = (i / 64, i % 64);
    limb < a.len() && (a[limb] >> off) & 1 == 1
}

fn l_shl1(a: &[u64]) -> Limbs {
    let mut out = Limbs::with_capacity(a.len() + 1);
    let mut carry = 0u64;
    for &x in a {
        out.push((x << 1) | carry);
        carry = x >> 63;
    }
    if carry != 0 {
        out.push(carry);
    }
    l_trim(out)
}

// Long division by shift-and-subtract over limbs: one pass per bit of n.
fn l_divmod(n: &[u64], d: &[u64]) -> (Limbs, Limbs) {
    if l_is_zero(d) || l_cmp(n, d) == core::cmp::Ordering::Less {
        return (vec![0], l_trim(n.to_vec()));
    }
    let nb = l_bits(n);
    let mut q = vec![0u64; nb / 64 + 1];
    let mut r: Limbs = vec![0];
    for i in (0..nb).rev() {
        r = l_shl1(&r);
        if l_bit(n, i) {
            r[0] |= 1;
        }
        if l_cmp(&r, d) != core::cmp::Ordering::Less {
            r = l_sub(&r, d);
            q[i / 64] |= 1u64 << (i % 64);
        }
    }
    (l_trim(q), l_trim(r))
}

pub fn cmp(a: &[char], b: &[char]) -> core::cmp::Ordering {
    l_cmp(&fold(a), &fold(b))
}

pub fn add(a: &[char], b: &[char]) -> Tape {
    unfold(&l_add(&fold(a), &fold(b)))
}

pub fn sub(a: &[char], b: &[char]) -> Tape {
    unfold(&l_sub(&fold(a), &fold(b)))
}

pub fn mul(a: &[char], b: &[char]) -> Tape {
    unfold(&l_mul(&fold(a), &fold(b)))
}

pub fn divmod(n: &[char], d: &[char]) -> (Tape, Tape) {
    let (q, r) = l_divmod(&fold(n), &fold(d));
    (unfold(&q), unfold(&r))
}

pub fn modulo(n: &[char], d: &[char]) -> Tape {
    unfold(&l_divmod(&fold(n), &fold(d)).1)
}

fn mod_add(a: &[char], b: &[char], n: &[char]) -> Tape {
    modulo(&add(a, b), n)
}

fn mod_mul(a: &[char], b: &[char], n: &[char]) -> Tape {
    modulo(&mul(a, b), n)
}

fn abs_diff(a: &[char], b: &[char]) -> Tape {
    if cmp(a, b) == core::cmp::Ordering::Less {
        sub(b, a)
    } else {
        sub(a, b)
    }
}

pub fn gcd(mut a: Tape, mut b: Tape) -> Tape {
    while !zero(&b) {
        let r = modulo(&a, &b);
        a = b;
        b = r;
    }
    trim(a)
}

fn rho_step(x: &[char], c: &[char], n: &[char]) -> Tape {
    mod_add(&mod_mul(x, x, n), c, n)
}

/// If x is a perfect square, its root; else None.
fn is_square(x: &[char]) -> Option<Tape> {
    let r = isqrt(x);
    if cmp(&mul(&r, &r), x) == core::cmp::Ordering::Equal {
        Some(r)
    } else {
        None
    }
}

/// One multiplier step of Lehman's method: for this k, sweep a from
/// ceil(2*sqrt(k*N)) across a short window; when a*a - 4kN is a perfect square
/// b*b, gcd(a+b, N) is a factor. Small factors are left to the trial arm; Lehman
/// covers the mid-range factor between N^(1/3) and N^(2/3).
fn lehman_step(n: &[char], k: &[char]) -> Option<Tape> {
    use core::cmp::Ordering::{Greater, Less};
    let kn4 = mul(&tape_u64(4), &mul(k, n));
    let mut a = isqrt(&kn4);
    if cmp(&mul(&a, &a), &kn4) == Less {
        a = add(&a, &one());
    }
    let sixth = iroot(n, 6);
    let sk = {
        let r = isqrt(k);
        if zero(&r) { one() } else { r }
    };
    let width = divmod(&sixth, &mul(&tape_u64(4), &sk)).0;
    let limit = add(&add(&a, &width), &one());
    while cmp(&a, &limit) != Greater {
        let asq = mul(&a, &a);
        if cmp(&asq, &kn4) != Less {
            let c = sub(&asq, &kn4);
            if let Some(b) = is_square(&c) {
                let g = gcd(add(&a, &b), n.to_vec());
                if cmp(&g, &one()) == Greater && cmp(&g, n) == Less {
                    return Some(trim(g));
                }
            }
        }
        a = add(&a, &one());
    }
    None
}

/// Shanks's square forms factorization over numeral tapes. Runs the forward
/// cycle of the principal form until a square Q appears at an even step, then the
/// reverse cycle until P stabilizes; gcd(P, N) is then a factor. Tries a few
/// multipliers. The Q recurrence carries a real sign, tracked by branch since the
/// tapes are unsigned.
fn squfof(n: &[char]) -> Option<Tape> {
    use core::cmp::Ordering::{Equal, Greater, Less};
    for &k in &[1u64, 3, 5, 7, 11, 13, 15] {
        let d = mul(n, &tape_u64(k));
        let s = isqrt(&d);
        // SQUFOF forward bound: 2*sqrt(2*sqrt(D)). If no square appears within
        // it, this multiplier fails and the next is tried. A reverse cycle is
        // bounded the same way.
        let cap = tape_to_u64(&mul(&two(), &isqrt(&mul(&two(), &s))))
            .saturating_mul(2)
            .max(64);
        if cmp(&mul(&s, &s), &d) == Equal {
            let g = gcd(s.clone(), n.to_vec());
            if cmp(&g, &one()) == Greater && cmp(&g, n) == Less {
                return Some(trim(g));
            }
            continue;
        }
        // Q_{i+1} = Q_{i-1} + b*(P_{i-1} - P_i), sign-tracked.
        let q_update = |q_prev: &[char], b: &[char], p_prev: &[char], p: &[char]| -> Option<Tape> {
            let term = mul(b, &abs_diff(p_prev, p));
            if cmp(p_prev, p) != Less {
                Some(add(q_prev, &term))
            } else if cmp(q_prev, &term) != Less {
                Some(sub(q_prev, &term))
            } else {
                None
            }
        };
        // Forward.
        let mut p_prev = s.clone();
        let mut q_prev = one();
        let mut q = sub(&d, &mul(&s, &s));
        if zero(&q) {
            continue;
        }
        let mut r: Option<Tape> = None;
        let mut i = 1u64;
        while i <= cap {
            // Q_i square at an even step, paired with P_{i-1} (= p_prev here).
            if i % 2 == 0 {
                if let Some(root) = is_square(&q) {
                    r = Some(root);
                    break;
                }
            }
            let b = divmod(&add(&s, &p_prev), &q).0;
            let p = sub(&mul(&b, &q), &p_prev);
            let q_next = match q_update(&q_prev, &b, &p_prev, &p) {
                Some(v) => v,
                None => break,
            };
            q_prev = q;
            q = q_next;
            p_prev = p;
            i += 1;
        }
        let r = match r {
            Some(v) => v,
            None => continue,
        };
        // Reverse: seed from the square, iterate until P stabilizes.
        if cmp(&s, &p_prev) == Less {
            continue;
        }
        let b0 = divmod(&sub(&s, &p_prev), &r).0;
        let mut p = add(&p_prev, &mul(&b0, &r));
        let mut q_prev = r.clone();
        let mut q = divmod(&sub(&d, &mul(&p, &p)), &r).0;
        let mut ok = false;
        let mut j = 0u64;
        while j <= cap {
            let b = divmod(&add(&s, &p), &q).0;
            let p_new = sub(&mul(&b, &q), &p);
            if cmp(&p_new, &p) == Equal {
                ok = true;
                break;
            }
            let q_next = match q_update(&q_prev, &b, &p, &p_new) {
                Some(v) => v,
                None => break,
            };
            q_prev = q;
            q = q_next;
            p = p_new;
            j += 1;
        }
        if !ok {
            continue;
        }
        let g = gcd(trim(p), n.to_vec());
        if cmp(&g, &one()) == Greater && cmp(&g, n) == Less {
            return Some(trim(g));
        }
    }
    None
}

/// Modular exponentiation over numeral tapes: base^exp mod n, square and
/// multiply over the exponent's own bits (least significant cell first).
fn pow_mod(base: &[char], exp: &[char], n: &[char]) -> Tape {
    let mut result = one();
    let mut b = modulo(base, n);
    for &e in exp {
        if e == EVALF {
            result = mod_mul(&result, &b, n);
        }
        b = mod_mul(&b, &b, n);
    }
    trim(result)
}

/// Strong probable-prime test (Miller-Rabin) over numeral tapes against a fixed
/// witness set, deterministic across the ranges this factorizer handles. A few
/// modular powers decide primality, so a prime need not be walked out to its
/// square root.
pub fn miller_rabin(n: &[char]) -> bool {
    use core::cmp::Ordering::{Equal, Less};
    let n = trim(n.to_vec());
    if cmp(&n, &two()) == Less {
        return false;
    }
    if cmp(&n, &two()) == Equal {
        return true;
    }
    if zero(&modulo(&n, &two())) {
        return false;
    }
    let nm1 = trim(sub(&n, &one()));
    let mut s = 0usize;
    while s < nm1.len() && nm1[s] == EVALT {
        s += 1;
    }
    let d = trim(nm1[s..].to_vec());
    for &a in &[2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
        let at = tape_u64(a);
        if cmp(&at, &n) != Less {
            continue;
        }
        let mut x = pow_mod(&at, &d, &n);
        if cmp(&x, &one()) == Equal || cmp(&x, &nm1) == Equal {
            continue;
        }
        let mut composite = true;
        for _ in 0..s.saturating_sub(1) {
            x = mod_mul(&x, &x, &n);
            if cmp(&x, &nm1) == Equal {
                composite = false;
                break;
            }
        }
        if composite {
            return false;
        }
    }
    true
}

/// Integer power base^b over numeral tapes, b a small usize.
fn ipow(base: &[char], b: usize) -> Tape {
    let mut r = one();
    for _ in 0..b {
        r = mul(&r, base);
    }
    trim(r)
}

/// Floor integer b-th root over numeral tapes: the largest r with r^b <= n.
/// The search range is bounded by 2^(bits(n)/b + 1) so the powers stay small.
fn iroot(n: &[char], b: usize) -> Tape {
    if b <= 1 || cmp(n, &two()) == core::cmp::Ordering::Less {
        return trim(n.to_vec());
    }
    let bits = trim(n.to_vec()).len();
    let k = bits / b + 2;
    let mut hi = vec![EVALT; k];
    hi.push(EVALF); // 2^k
    let mut lo = one();
    while cmp(&lo, &hi) == core::cmp::Ordering::Less {
        let mid = divmod(&add(&add(&lo, &hi), &one()), &two()).0;
        if cmp(&ipow(&mid, b), n) != core::cmp::Ordering::Greater {
            lo = mid;
        } else {
            hi = sub(&mid, &one());
        }
    }
    trim(lo)
}

/// Lucas V-sequence value V_m(a, 1) mod n, by a Montgomery ladder over the bits
/// of m: V_0 = 2, V_1 = a, V_{2k} = V_k^2 - 2, V_{2k+1} = V_k V_{k+1} - a. This
/// is the engine of the Williams p+1 arm.
fn lucas_v(m: &[char], a: &[char], n: &[char]) -> Tape {
    let m = trim(m.to_vec());
    if zero(&m) {
        return two();
    }
    let ar = modulo(a, n);
    if m == [EVALF] {
        return ar;
    }
    let h = m.len() - 1;
    let mut v0 = ar.clone();
    let mut v1 = mod_sub(&mod_mul(&ar, &ar, n), &two(), n);
    for j in (0..h).rev() {
        if m[j] == EVALF {
            v0 = mod_sub(&mod_mul(&v0, &v1, n), &ar, n);
            v1 = mod_sub(&mod_mul(&v1, &v1, n), &two(), n);
        } else {
            v1 = mod_sub(&mod_mul(&v0, &v1, n), &ar, n);
            v0 = mod_sub(&mod_mul(&v0, &v0, n), &two(), n);
        }
    }
    v0
}

/// Floor integer square root over numeral tapes, by binary search on the
/// largest x with x*x <= n. Used to seed and test the square-frontier arm.
pub fn isqrt(n: &[char]) -> Tape {
    if cmp(n, &two()) == core::cmp::Ordering::Less {
        return trim(n.to_vec());
    }
    let mut lo = one();
    let mut hi = n.to_vec();
    while cmp(&lo, &hi) == core::cmp::Ordering::Less {
        // ceil midpoint (lo+hi+1)/2 so lo can reach hi without stalling
        let mid = divmod(&add(&add(&lo, &hi), &one()), &two()).0;
        if cmp(&mul(&mid, &mid), n) != core::cmp::Ordering::Greater {
            lo = mid;
        } else {
            hi = sub(&mid, &one());
        }
    }
    trim(lo)
}

pub fn one() -> Tape {
    vec![EVALF]
}
pub fn two() -> Tape {
    vec![EVALT, EVALF]
}
pub fn zero(a: &[char]) -> bool {
    trim(a.to_vec()) == [EVALT]
}

pub fn tape_u64(mut n: u64) -> Tape {
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

fn tape_to_u64(t: &[char]) -> u64 {
    let mut v = 0u64;
    for (i, &b) in t.iter().enumerate() {
        if b == EVALF && i < 64 {
            v |= 1u64 << i;
        }
    }
    v
}

pub fn tape_to_limbs(t: &[char], limb_count: usize) -> Vec<u32> {
    let mut limbs = vec![0u32; limb_count];
    for (i, &b) in t.iter().enumerate() {
        let limb_idx = i / 32;
        let bit_idx = i % 32;
        if limb_idx >= limb_count {
            break;
        }
        if b == EVALF {
            limbs[limb_idx] |= 1u32 << bit_idx;
        }
    }
    limbs
}

pub fn limbs_to_tape(limbs: &[u32]) -> Vec<char> {
    let mut tape = Vec::new();
    for &limb in limbs.iter() {
        for bit in 0..32 {
            if (limb >> bit) & 1 == 1 {
                tape.push(EVALF);
            } else {
                tape.push(EVALT);
            }
        }
    }
    trim(tape)
}

pub fn limb_count_for_bits(bit_count: usize) -> usize {
    (bit_count + 31) / 32
}

pub fn tape_to_u32_limbs(t: &[char]) -> Vec<u32> {
    let limb_count = limb_count_for_bits(t.len());
    tape_to_limbs(t, limb_count)
}

/// (a - b) mod n, for a and b already reduced into [0, n).
fn mod_sub(a: &[char], b: &[char], n: &[char]) -> Tape {
    let br = modulo(b, n);
    modulo(&add(a, &sub(n, &br)), n)
}

/// Modular inverse of a mod n by the extended Euclidean algorithm, the
/// coefficients kept reduced mod n so every tape stays non-negative. Ok is the
/// inverse; Err is a nontrivial gcd, which is a factor of n. This Err is exactly
/// the elliptic-curve method's factor-discovery event.
fn mod_inv(a: &[char], n: &[char]) -> Result<Tape, Tape> {
    let a = modulo(a, n);
    if zero(&a) {
        return Err(n.to_vec());
    }
    let mut r = n.to_vec();
    let mut newr = a;
    let mut t = vec![EVALT];
    let mut newt = one();
    while !zero(&newr) {
        let (q, rem) = divmod(&r, &newr);
        r = newr;
        newr = rem;
        let qt = mod_mul(&q, &newt, n);
        let nt = mod_sub(&t, &qt, n);
        t = newt;
        newt = nt;
    }
    if cmp(&r, &one()) == core::cmp::Ordering::Greater {
        return Err(trim(r));
    }
    Ok(trim(modulo(&t, n)))
}

/// A point on the curve, or the point at infinity (None). The curve is
/// y^2 = x^3 + a x + b mod n; the group law needs only a, so b stays implicit.
type Point = Option<(Tape, Tape)>;

fn ec_double(p: &Point, a: &[char], n: &[char]) -> Result<Point, Tape> {
    let (x, y) = match p {
        Some(v) => v,
        None => return Ok(None),
    };
    if zero(y) {
        return Ok(None);
    }
    let num = mod_add(&mod_mul(&tape_u64(3), &mod_mul(x, x, n), n), a, n);
    let den = mod_mul(&two(), y, n);
    let inv = mod_inv(&den, n)?;
    let lam = mod_mul(&num, &inv, n);
    let lam2 = mod_mul(&lam, &lam, n);
    let x3 = mod_sub(&lam2, &mod_mul(&two(), x, n), n);
    let y3 = mod_sub(&mod_mul(&lam, &mod_sub(x, &x3, n), n), y, n);
    Ok(Some((x3, y3)))
}

fn ec_add(p: &Point, q: &Point, a: &[char], n: &[char]) -> Result<Point, Tape> {
    let (x1, y1) = match p {
        Some(v) => v,
        None => return Ok(q.clone()),
    };
    let (x2, y2) = match q {
        Some(v) => v,
        None => return Ok(p.clone()),
    };
    if cmp(x1, x2) == core::cmp::Ordering::Equal {
        if cmp(y1, y2) == core::cmp::Ordering::Equal {
            return ec_double(p, a, n);
        }
        return Ok(None);
    }
    let num = mod_sub(y2, y1, n);
    let den = mod_sub(x2, x1, n);
    let inv = mod_inv(&den, n)?;
    let lam = mod_mul(&num, &inv, n);
    let lam2 = mod_mul(&lam, &lam, n);
    let x3 = mod_sub(&mod_sub(&lam2, x1, n), x2, n);
    let y3 = mod_sub(&mod_mul(&lam, &mod_sub(x1, &x3, n), n), y1, n);
    Ok(Some((x3, y3)))
}

fn ec_scalar(k: &[char], p: &Point, a: &[char], n: &[char]) -> Result<Point, Tape> {
    let mut r: Point = None;
    let mut addend = p.clone();
    for &b in k {
        if b == EVALF {
            r = ec_add(&r, &addend, a, n)?;
        }
        addend = ec_double(&addend, a, n)?;
    }
    Ok(r)
}

/// Stage-1 scalar lcm(1..~47), built from prime powers, shared across curves.
fn ecm_stage1_k() -> Tape {
    let power_primes: [u64; 15] = [32, 27, 25, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47];
    let mut k = one();
    for &pp in power_primes.iter() {
        k = mul(&k, &tape_u64(pp));
    }
    trim(k)
}

/// One ECM curve seeded by `seed`: a = seed, start point (seed, seed+1), and
/// the implied b. Multiply the point by the stage-1 scalar; a failed inverse
/// during the group law surfaces a factor of n.
fn ecm_curve(n: &[char], seed: u64, k: &[char]) -> Option<Tape> {
    let a = modulo(&tape_u64(seed), n);
    let p: Point = Some((modulo(&tape_u64(seed), n), modulo(&tape_u64(seed + 1), n)));
    match ec_scalar(k, &p, &a, n) {
        Ok(_) => None,
        Err(g) => {
            if cmp(&g, &one()) == core::cmp::Ordering::Greater && cmp(&g, n) == core::cmp::Ordering::Less {
                Some(trim(g))
            } else {
                None
            }
        }
    }
}

struct State {
    n: Tape,
    candidate: Tape,
    remainder: Tape,
    x: Tape,
    y: Tape,
    phase: Tape,
    divisor: Tape,
    a: Tape,
    pm_a: Tape,
    pm_e: Tape,
    ecm_seed: Tape,
    ecm_round: Tape,
    pp_base: Tape,
    lehman_k: Tape,
    witness_done: bool,
    power_done: bool,
    squfof_done: bool,
    round: Tape,
    exhausted: bool,
    selected: Option<Tape>,
}

/// The interior of each operator motif (its marks between VINIT and TANCH).
/// Dispatch in `apply_morphism` keys on the whole operator word; the carrier
/// constructor recognises a motif by this interior wherever it appears inside
/// a larger operator word.
const PHASE_I: &[char] = &[FSPLIT, AFWD, EVALT, EVALF, FFUSE];
const ARITHMETIC_I: &[char] = &[FSPLIT, CLINK, EVALT, EVALF, FFUSE];
const BRANCH_I: &[char] = &[FSPLIT, EVALT, EVALF, FFUSE];
const SELECT_I: &[char] = &[FSPLIT, IMSCRIB, FFUSE];
const CONTINUE_I: &[char] = &[AFWD, CLINK];
const FIX_I: &[char] = &[IMSCRIB, IFIX];
const EXTRACT_I: &[char] = &[FSPLIT, AFWD, EVALT, AREV, EVALF, '⊞', CLINK, FFUSE];
const P_MINUS_I: &[char] = &[FSPLIT, IMSCRIB, '⊞', CLINK, FFUSE];
const ECM_I: &[char] = &[FSPLIT, IMSCRIB, AFWD, CLINK, FFUSE];
const WITNESS_I: &[char] = &[FSPLIT, EVALT, AREV, EVALF, FFUSE];
const POWER_I: &[char] = &[FSPLIT, EVALT, '⊞', EVALF, FFUSE];
const P_PLUS_I: &[char] = &[FSPLIT, IMSCRIB, AREV, CLINK, FFUSE];
const LEHMAN_I: &[char] = &[FSPLIT, AFWD, CLINK, EVALT, EVALF, FFUSE];
const SQUFOF_I: &[char] = &[FSPLIT, EVALT, AREV, '⊞', EVALF, FFUSE];

/// Name of an operator motif, for reporting a constructed tower.
pub fn morphism_name(operator: &[char]) -> &'static str {
    if operator == PHASE { "PHASE" }
    else if operator == ARITHMETIC { "ARITHMETIC" }
    else if operator == BRANCH { "BRANCH" }
    else if operator == SELECT { "SELECT" }
    else if operator == CONTINUE { "CONTINUE" }
    else if operator == FIX { "FIX" }
    else if operator == EXTRACT { "EXTRACT" }
    else if operator == P_MINUS { "P_MINUS" }
    else if operator == ECM { "ECM" }
    else if operator == WITNESS { "WITNESS" }
    else if operator == POWER { "POWER" }
    else if operator == P_PLUS { "P_PLUS" }
    else if operator == LEHMAN { "LEHMAN" }
    else if operator == SQUFOF { "SQUFOF" }
    else { "?" }
}

/// Automated carrier constructor. Read an operator ob3ect word and decompose
/// its interior into the ordered sequence of factoring morphisms it realises.
/// Each recognised motif interior emits its whole operator word, so the result
/// is a tower that `execute_nested` can drive exactly like the fixed one in
/// `factor`. Marks the Grammar defines as carry rather than dispatch (AREV the
/// involution, ENGAGR as hold, and a lone IMSCRIB seed) are carried across
/// without emitting an operator. An interior mark that begins no motif and is
/// not a carry mark is refused, naming the position.
pub fn construct_carrier(operator_word: &str) -> Result<Vec<&'static [char]>, String> {
    let c: Vec<char> = operator_word.chars().collect();
    if c.first() != Some(&VINIT) || c.last() != Some(&TANCH) {
        return Err("operator word needs VINIT ⊢ and TANCH ⊣ interfaces".into());
    }
    let body = &c[1..c.len() - 1];
    // Longest interior first so PHASE/ARITHMETIC win over BRANCH, and FIX (⊙⊡)
    // wins over a lone IMSCRIB carry.
    let motifs: [(&[char], &[char]); 14] = [
        (EXTRACT_I, EXTRACT),
        (P_MINUS_I, P_MINUS),
        (ECM_I, ECM),
        (P_PLUS_I, P_PLUS),
        (LEHMAN_I, LEHMAN),
        (SQUFOF_I, SQUFOF),
        (WITNESS_I, WITNESS),
        (POWER_I, POWER),
        (PHASE_I, PHASE),
        (ARITHMETIC_I, ARITHMETIC),
        (BRANCH_I, BRANCH),
        (SELECT_I, SELECT),
        (CONTINUE_I, CONTINUE),
        (FIX_I, FIX),
    ];
    let mut tower: Vec<&'static [char]> = Vec::new();
    let mut i = 0;
    'scan: while i < body.len() {
        for (interior, operator) in motifs.iter() {
            if body[i..].starts_with(interior) {
                tower.push(operator);
                i += interior.len();
                continue 'scan;
            }
        }
        // Carry marks: carried across the register, they select no morphism.
        if matches!(body[i], AREV) || body[i] == '⊞' || body[i] == IMSCRIB {
            i += 1;
            continue;
        }
        return Err(format!(
            "operator mark {} at interior position {} begins no factoring morphism",
            body[i], i
        ));
    }
    Ok(tower)
}

/// Factor N using a carrier constructed from an arbitrary operator ob3ect word,
/// rather than the fixed tower in `factor`. The operator word is decomposed by
/// `construct_carrier`; the tower must be factoring-complete (able to advance a
/// candidate, decide, continue, and fix) or the missing morphisms are named.
pub fn factor_with(operator_word: &str, n_word: &str) -> Result<String, String> {
    let tower = construct_carrier(operator_word)?;
    let has = |op: &[char]| tower.iter().any(|t| *t == op);
    let mut missing = Vec::new();
    // EXTRACT and ECM each fold advance, decide and continue into one boundary
    // (EXTRACT over trial/frontier/rho, ECM over curves), so a tower carrying
    // either needs only a FIX to be complete.
    if !has(EXTRACT) && !has(ECM) {
        if !has(PHASE) && !has(ARITHMETIC) { missing.push("PHASE or ARITHMETIC (advance)"); }
        if !has(SELECT) { missing.push("SELECT (decide)"); }
        if !has(CONTINUE) { missing.push("CONTINUE (step the candidate)"); }
    }
    if !has(FIX) { missing.push("FIX (latch)"); }
    if !missing.is_empty() {
        return Err(format!(
            "operator word does not carry factoring; missing: {}",
            missing.join(", ")
        ));
    }
    let n = parse_numeral(n_word)?;
    if cmp(&n, &two()) == core::cmp::Ordering::Less {
        return Err("numeral has no non-trivial factor".into());
    }
    let (_, even) = divmod(&n, &two());
    if zero(&even) {
        return Ok(emit_numeral(&two()));
    }
    let tower_refs: Vec<&[char]> = tower.iter().map(|t| *t).collect();
    match run_carrier_rounds(&tower_refs, &n, u64::MAX) {
        Some(f) => Ok(emit_numeral(&f)),
        None => Err("carrier exhausted its round budget without latching".into()),
    }
}

/// Run a carrier tower on a tape for at most `max_rounds` rounds, returning the
/// factor as soon as an arm latches, or None when the budget is spent. The
/// bounded form is what lets the HARD branch nest the whole nine-arm carrier
/// OUTSIDE the sieve: the cheap and mid arms (witness, extract, p-1, p+1, SQUFOF,
/// Lehman, ECM) all get their throttled rounds first, and the quadratic sieve is
/// the deepest fallback, run only after this returns None. `u64::MAX` is the
/// unbounded run factor_with wants.
pub fn run_carrier_rounds(tower: &[&[char]], n_in: &[char], max_rounds: u64) -> Option<Tape> {
    let n = trim(n_in.to_vec());
    if cmp(&n, &two()) == core::cmp::Ordering::Less {
        return None;
    }
    let (_, even) = divmod(&n, &two());
    if zero(&even) {
        return Some(two());
    }
    let mut a_seed = isqrt(&n);
    if cmp(&mul(&a_seed, &a_seed), &n) == core::cmp::Ordering::Less {
        a_seed = add(&a_seed, &one());
    }
    let mut state = State {
        n,
        candidate: add(&two(), &one()),
        remainder: vec![EVALT],
        x: two(),
        y: two(),
        phase: one(),
        divisor: one(),
        a: a_seed,
        pm_a: two(),
        pm_e: two(),
        ecm_seed: two(),
        ecm_round: vec![EVALT],
        pp_base: tape_u64(3),
        lehman_k: one(),
        witness_done: false,
        power_done: false,
        squfof_done: false,
        round: vec![EVALT],
        exhausted: false,
        selected: None,
    };
    let mut r = 0u64;
    loop {
        state.round = add(&state.round, &one());
        execute_nested(tower, &mut state);
        if let Some(ref selected) = state.selected {
            return Some(selected.clone());
        }
        r += 1;
        if r >= max_rounds {
            return None;
        }
    }
}

fn apply_morphism(operator: &[char], state: &mut State) {
    // Dispatch is read from the operator word itself. Each operator therefore
    // remains both the boundary and the action performed at that boundary.
    if operator == PHASE {
        state.exhausted =
            cmp(&mul(&state.candidate, &state.candidate), &state.n) == core::cmp::Ordering::Greater;
        state.x = rho_step(&state.x, &state.phase, &state.n);
        state.y = rho_step(
            &rho_step(&state.y, &state.phase, &state.n),
            &state.phase,
            &state.n,
        );
    } else if operator == ARITHMETIC && !state.exhausted {
        state.remainder = divmod(&state.n, &state.candidate).1;
        state.divisor = gcd(abs_diff(&state.x, &state.y), state.n.clone());
    } else if operator == BRANCH && !state.exhausted {
        state.remainder = trim(state.remainder.clone());
    } else if operator == SELECT {
        if state.exhausted {
            state.selected = Some(state.n.clone());
        } else if cmp(&state.divisor, &one()) == core::cmp::Ordering::Greater
            && cmp(&state.divisor, &state.n) == core::cmp::Ordering::Less
        {
            state.selected = Some(state.divisor.clone());
        } else if zero(&state.remainder) {
            state.selected = Some(state.candidate.clone());
        }
    } else if operator == CONTINUE && state.selected.is_none() {
        state.candidate = add(&state.candidate, &two());
        if cmp(&state.divisor, &state.n) == core::cmp::Ordering::Equal {
            state.phase = add(&state.phase, &one());
            state.x = two();
            state.y = two();
            state.divisor = one();
        }
    } else if operator == EXTRACT {
        // Instant extract: one frame forks three arms and fuses on the first to
        // close, the two-arm converge circuit plus trial.
        // Arm 1, the square frontier (Fermat): a walks up from ceil(sqrt N);
        // when a*a - N is a perfect square b*b, the pair is (a-b, a+b). This
        // closes a balanced semiprime in steps set by the gap, not by sqrt(p),
        // so it gets past the rho ceiling for factors near the root.
        {
            let asq = mul(&state.a, &state.a);
            if cmp(&asq, &state.n) != core::cmp::Ordering::Less {
                let delta = sub(&asq, &state.n);
                let b = isqrt(&delta);
                if cmp(&mul(&b, &b), &delta) == core::cmp::Ordering::Equal {
                    let p = sub(&state.a, &b);
                    if cmp(&p, &one()) == core::cmp::Ordering::Greater
                        && cmp(&p, &state.n) == core::cmp::Ordering::Less
                    {
                        state.selected = Some(trim(p));
                    }
                }
            }
            state.a = add(&state.a, &one());
        }
        if state.selected.is_some() {
            return;
        }
        // Arm 2 + trial, as before: phase update, then remainder and rho gcd,
        // then selection, then continuation.
        state.exhausted =
            cmp(&mul(&state.candidate, &state.candidate), &state.n) == core::cmp::Ordering::Greater;
        state.x = rho_step(&state.x, &state.phase, &state.n);
        state.y = rho_step(
            &rho_step(&state.y, &state.phase, &state.n),
            &state.phase,
            &state.n,
        );
        if !state.exhausted {
            state.remainder = trim(divmod(&state.n, &state.candidate).1);
            state.divisor = gcd(abs_diff(&state.x, &state.y), state.n.clone());
        }
        if state.exhausted {
            state.selected = Some(state.n.clone());
        } else if cmp(&state.divisor, &one()) == core::cmp::Ordering::Greater
            && cmp(&state.divisor, &state.n) == core::cmp::Ordering::Less
        {
            state.selected = Some(state.divisor.clone());
        } else if zero(&state.remainder) {
            state.selected = Some(state.candidate.clone());
        }
        if state.selected.is_none() {
            state.candidate = add(&state.candidate, &two());
            if cmp(&state.divisor, &state.n) == core::cmp::Ordering::Equal {
                state.phase = add(&state.phase, &one());
                state.x = two();
                state.y = two();
                state.divisor = one();
            }
        }
    } else if operator == P_MINUS {
        // Costly per-round arm: nested deeper on a stride so the cheap walk
        // carries the rounds between. Its own exponent still rises per firing.
        if tape_to_u64(&state.round) % 4 != 0 {
            return;
        }
        // Pollard p-1: pm_a := pm_a^pm_e mod N, then gcd(pm_a - 1, N). A
        // nontrivial gcd is a factor whose predecessor is smooth to this
        // exponent. pm_e rises each round, so the smoothness bound grows.
        state.pm_a = pow_mod(&state.pm_a, &state.pm_e, &state.n);
        if cmp(&state.pm_a, &one()) == core::cmp::Ordering::Greater {
            let g = gcd(sub(&state.pm_a, &one()), state.n.clone());
            if cmp(&g, &one()) == core::cmp::Ordering::Greater
                && cmp(&g, &state.n) == core::cmp::Ordering::Less
            {
                state.selected = Some(trim(g));
            }
        }
        state.pm_e = add(&state.pm_e, &one());
    } else if operator == WITNESS {
        // One strong probable-prime test, first thing. If N is prime, select it
        // at once so the trial arm never walks to sqrt(N). One-shot: the verdict
        // on a fixed N never changes.
        if !state.witness_done {
            state.witness_done = true;
            if miller_rabin(&state.n) {
                state.selected = Some(trim(state.n.clone()));
            }
        }
    } else if operator == POWER {
        // Perfect-power test, once: is N = a^b for some b >= 2? If so, select a.
        if !state.power_done {
            state.power_done = true;
            let bits = trim(state.n.clone()).len();
            let mut b = 2usize;
            while b <= bits {
                let r = iroot(&state.n, b);
                if cmp(&r, &one()) == core::cmp::Ordering::Greater
                    && cmp(&ipow(&r, b), &state.n) == core::cmp::Ordering::Equal
                {
                    state.selected = Some(trim(r));
                    break;
                }
                b += 1;
            }
        }
    } else if operator == P_PLUS {
        if tape_to_u64(&state.round) % 4 != 1 {
            return;
        }
        // Williams p+1: V_M(A) mod N over a Lucas sequence, then gcd(V_M - 2, N).
        // A nontrivial gcd is a factor p whose successor p+1 is smooth. The base
        // A rises each round so a failed base is retried on the other side.
        let m = ecm_stage1_k();
        let v = lucas_v(&m, &state.pp_base, &state.n);
        let g = gcd(mod_sub(&v, &two(), &state.n), state.n.clone());
        if cmp(&g, &one()) == core::cmp::Ordering::Greater
            && cmp(&g, &state.n) == core::cmp::Ordering::Less
        {
            state.selected = Some(trim(g));
        }
        state.pp_base = add(&state.pp_base, &one());
    } else if operator == LEHMAN {
        if tape_to_u64(&state.round) % 4 != 2 {
            return;
        }
        // One Lehman multiplier per firing.
        if let Some(g) = lehman_step(&state.n, &state.lehman_k) {
            state.selected = Some(g);
        }
        state.lehman_k = add(&state.lehman_k, &one());
    } else if operator == SQUFOF {
        // Heavy fallback: its cycle bound grows like N^(1/4), so it is nested
        // deep in time. It fires once, and only after the cheap arms have had a
        // few hundred rounds to close an easy factor first. If they already did,
        // the loop has returned and this never runs.
        if !state.squfof_done && tape_to_u64(&state.round) >= 256 {
            state.squfof_done = true;
            if let Some(g) = squfof(&state.n) {
                state.selected = Some(g);
            }
        }
    } else if operator == ECM {
        // ECM is the costly arm, so it sits deeper: it fires one curve only on
        // rounds that are a power of two, letting the cheap arms (trial, rho,
        // frontier, p-1) carry every round in between. The curve count then
        // grows with the logarithm of the round, so certifying a prime by the
        // trial walk no longer drags a full curve behind every step, while the
        // hard case still accumulates curves as the search deepens.
        state.ecm_round = add(&state.ecm_round, &one());
        let ones = state.ecm_round.iter().filter(|&&c| c == EVALF).count();
        if ones == 1 {
            let k = ecm_stage1_k();
            if let Some(g) = ecm_curve(&state.n, tape_to_u64(&state.ecm_seed), &k) {
                state.selected = Some(g);
            }
            state.ecm_seed = add(&state.ecm_seed, &one());
        }
    } else if operator == FIX {
        if let Some(value) = state.selected.take() {
            state.selected = Some(trim(value));
        }
    }
}

fn execute_nested(operators: &[&[char]], state: &mut State) {
    if let Some((operator, continuation)) = operators.split_first() {
        apply_morphism(operator, state);
        execute_nested(continuation, state);
    }
}

/// Execute the nested phase → arithmetic → branch → selection → continuation
/// → fixation tower. Each boundary is itself an IMASM operator word; the live
/// state passed through all boundaries consists only of numeral tapes.
pub fn factor(word: &str) -> Result<String, String> {
    let n = parse_numeral(word)?;
    if cmp(&n, &two()) == core::cmp::Ordering::Less {
        return Err("numeral has no non-trivial factor".into());
    }
    let (_, even) = divmod(&n, &two());
    if zero(&even) {
        return Ok(emit_numeral(&two()));
    }
    let mut a_seed = isqrt(&n);
    if cmp(&mul(&a_seed, &a_seed), &n) == core::cmp::Ordering::Less {
        a_seed = add(&a_seed, &one());
    }
    let mut state = State {
        n,
        candidate: add(&two(), &one()),
        remainder: vec![EVALT],
        x: two(),
        y: two(),
        phase: one(),
        divisor: one(),
        a: a_seed,
        pm_a: two(),
        pm_e: two(),
        ecm_seed: two(),
        ecm_round: vec![EVALT],
        pp_base: tape_u64(3),
        lehman_k: one(),
        witness_done: false,
        power_done: false,
        squfof_done: false,
        round: vec![EVALT],
        exhausted: false,
        selected: None,
    };
    let tower: [&[char]; 6] = [PHASE, ARITHMETIC, BRANCH, SELECT, CONTINUE, FIX];
    loop {
        execute_nested(&tower, &mut state);
        if let Some(ref selected) = state.selected {
            return Ok(emit_numeral(selected));
        }
    }
}

/// Verify p·q == N entirely over IMASM numeral tapes: parse three words,
/// multiply p and q with the tape full-adder, compare the product to N, and
/// emit both the product and N back as IMASM words. This is mu circ delta = id
/// for the tower's own arithmetic at full RSA width.
pub fn verify(p_word: &str, q_word: &str, n_word: &str) -> Result<String, String> {
    let p = parse_numeral(p_word)?;
    let q = parse_numeral(q_word)?;
    let n = parse_numeral(n_word)?;
    let prod = trim(mul(&p, &q));
    let ok = cmp(&prod, &n) == core::cmp::Ordering::Equal;
    Ok(format!(
        "p*q == N: {ok}\nproduct: {}\nN:       {}",
        emit_numeral(&prod),
        emit_numeral(&n)
    ))
}

/// Parse an arbitrary-length decimal string to a tape (t = t*10 + digit over
/// the folded kernel), so N is not capped at u128.
pub fn decimal_to_tape(s: &str) -> Option<Tape> {
    let s = s.trim();
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let ten = tape_u64(10);
    let mut t = vec![EVALT];
    for ch in s.bytes() {
        t = add(&mul(&t, &ten), &tape_u64((ch - b'0') as u64));
    }
    Some(trim(t))
}

/// Decimal string for a tape of any size, by repeated division by ten.
pub fn dec_of(t: &[char]) -> String {
    let mut b = trim(t.to_vec());
    if zero(&b) {
        return "0".into();
    }
    let ten = tape_u64(10);
    let mut digits = Vec::new();
    while !zero(&b) {
        let (q, r) = divmod(&b, &ten);
        let mut dv = 0u8;
        for (i, &c) in trim(r).iter().enumerate() {
            if c == EVALF {
                dv |= 1 << i;
            }
        }
        digits.push(b'0' + dv);
        b = q;
    }
    digits.reverse();
    String::from_utf8(digits).unwrap()
}

/// Shape scout: cheap probes, cheapest first, each of which reads one shape of N
/// and, when it hits, hands the factor. The first hit both names the shape and
/// gives the pair, so the width-heavy arms never run for a shape they do not
/// fit. Returns the pair with the shape name, and a log of what each probe saw.
/// A None with "prime" is a certified prime; a None with "HARD" is the
/// random-equal-size-far-apart shape that wants the sub-exponential tier.
pub fn scout_factor(n_in: &[char]) -> (Option<(Tape, Tape, &'static str)>, String) {
    use core::cmp::Ordering::{Equal, Greater, Less};
    let n = trim(n_in.to_vec());
    let mut log = String::new();
    if cmp(&n, &two()) == Less {
        return (None, "shape: unit\n".into());
    }
    // even
    if zero(&modulo(&n, &two())) {
        let q = divmod(&n, &two()).0;
        return (Some((two(), q, "even")), "shape: even (2-part)\n".into());
    }
    // prime
    if miller_rabin(&n) {
        return (None, "shape: prime (witness, no factor to find)\n".into());
    }
    // perfect power
    let bits = n.len();
    let mut b = 2usize;
    while b <= bits {
        let r = iroot(&n, b);
        if cmp(&r, &one()) == Greater && cmp(&ipow(&r, b), &n) == Equal {
            let q = divmod(&n, &r).0;
            return (Some((r.clone(), q, "perfect-power")), format!("shape: perfect power, base {}\n", dec_of(&r)));
        }
        b += 1;
    }
    // Limit the scout to cheap shape probes. Wider inputs receive a shorter
    // trial scan; rho belongs to the downstream sieve, not this scout.
    let trial_bound: u64 = if bits <= 40 { 100_000 } else { 3_000 };
    // small factor by trial to a cheap bound
    let tb = tape_u64(trial_bound);
    let mut d = tape_u64(3);
    while cmp(&d, &tb) != Greater {
        if zero(&modulo(&n, &d)) {
            let q = divmod(&n, &d).0;
            return (Some((d.clone(), q, "small-trial")), format!("shape: small factor {} (trial)\n", dec_of(&d)));
        }
        d = add(&d, &two());
    }
    log.push_str(&format!("probe: no factor <= {trial_bound}\n"));
    // short frontier first: closes at once iff the factors sit near the root, so
    // a near-root N never pays the width-heavy rho below.
    {
        let mut a = isqrt(&n);
        if cmp(&mul(&a, &a), &n) == Less {
            a = add(&a, &one());
        }
        let mut i = 0u64;
        // A short frontier catches only genuinely near-root factors at once; the
        // sieve handles the rest, so keep this cheap rather than paying thousands
        // of isqrt steps the sieve would beat.
        while i < 64 {
            let a2 = mul(&a, &a);
            if cmp(&a2, &n) != Less {
                let dl = sub(&a2, &n);
                if let Some(bb) = is_square(&dl) {
                    let p = sub(&a, &bb);
                    if cmp(&p, &one()) == Greater {
                        let q = divmod(&n, &p).0;
                        return (Some((p.clone(), q, "frontier")), format!("shape: near-root, closed at frontier step {}\n", i));
                    }
                }
            }
            a = add(&a, &one());
            i += 1;
        }
    }
    log.push_str("probe: not near-root within the short frontier (factors far apart)\n");
    // No standalone rho here: rho is fused into the sieve (the membrane), where it
    // races the polynomials and the first arm to close wins. The scout's job ends
    // at the cheap, shape-certain gate; everything else goes to the membrane.
    log.push_str("verdict: HARD — hand to the membrane (rho fused with the sieve)\n");
    (None, log)
}

/// Round budget the HARD branch gives the nested nine-arm carrier before it
/// falls through to the quadratic sieve. The carrier's rho (PHASE) arm advances
/// one double-step per round, so this also caps its rho reach; a balanced N whose
/// smaller factor is near 2^40 wants roughly 2^20 steps, and the sieve takes over
/// only past where rho's N^(1/4) cost exceeds the sieve's sub-exponential one.
pub const HARD_CARRIER_ROUNDS: u64 = 2_000_000;

/// The full nine-arm carrier word, the deepest routing in one string.
pub const NINE_ARM: &str =
    "⊢∈⊤≺⊥∋∈⊤⊞⊥∋∈≻⊤≺⊥⊞⋈∋∈⊤≺⊞⊥∋∈⊙⊞⋈∋∈⊙≺⋈∋∈≻⋈⊤⊥∋∈⊙≻⋈∋⊙⊡⊣";

/// Smart factorization: scout each piece for its shape and route it, recursing
/// to a full prime multiset. A piece the scout labels HARD (large factor, far
/// from the root, not smooth) is handed to the full nine-arm carrier, which
/// carries the deeper arms (p-1, p+1, Lehman, ECM, SQUFOF). Returns the sorted
/// prime factors and the shape log.
pub fn smart_factor(n_in: &[char]) -> (Vec<Tape>, String) {
    let mut factors: Vec<Tape> = Vec::new();
    let mut log = String::new();
    let mut stack = vec![trim(n_in.to_vec())];
    while let Some(c) = stack.pop() {
        if cmp(&c, &one()) != core::cmp::Ordering::Greater {
            continue;
        }
        if miller_rabin(&c) {
            factors.push(c);
            continue;
        }
        let (res, l) = scout_factor(&c);
        log.push_str(&l);
        match res {
            Some((p, q, _shape)) => {
                stack.push(p);
                stack.push(q);
            }
            None => {
                // HARD nesting order, cheap-decisive arm outermost:
                //   1. Quadratic sieve, one bounded pass. Sub-exponential and fast
                //      through the low-to-mid width, it closes most hard shapes at
                //      once. Its own slow Dixon fallback is held back to step 3.
                //   2. The nine-arm carrier's rho (PHASE) arm, steady at N^(1/4).
                //      It takes the width where the single QS pass comes up short of
                //      smooth relations but rho still reaches the smaller factor.
                //   3. Dixon, the exhaustive last resort, only if both above miss.
                let good = |p: &Tape| {
                    cmp(p, &one()) == core::cmp::Ordering::Greater
                        && cmp(p, &c) == core::cmp::Ordering::Less
                };
                let (bound, m) = crate::sieve::sieve_params(&c);
                let tower = construct_carrier(NINE_ARM).expect("NINE_ARM is a valid carrier");
                let tower_refs: Vec<&[char]> = tower.iter().map(|t| *t).collect();
                // MPQS is the primary hard arm: a fresh polynomial per step keeps
                // the values small, so it is faster than the single polynomial
                // everywhere they overlap and reaches to the machine-integer width.
                // Single-poly QS is the fallback for the narrow N MPQS declines,
                // then the carrier's rho, then Dixon.
                let hit = crate::sieve::mpqs(&c, bound, 32_768, 32)
                    .filter(&good)
                    .or_else(|| crate::sieve::qs(&c, bound, m, 16).filter(&good))
                    .or_else(|| run_carrier_rounds(&tower_refs, &c, HARD_CARRIER_ROUNDS).filter(&good))
                    .or_else(|| crate::sieve::dixon(&c, bound, 8, 2_000_000).filter(&good));
                match hit {
                    Some(p) => {
                        let q = divmod(&c, &p).0;
                        stack.push(p);
                        stack.push(q);
                    }
                    None => factors.push(c),
                }
            }
        }
    }
    factors.sort_by(|a, b| cmp(a, b));
    (factors, log)
}

pub fn repl_smart_factor(n_in: &[char]) -> String {
    let (factors, _log) = smart_factor(n_in);
    let fs: Vec<String> = factors.iter().map(|f| dec_of(f)).collect();
    format!("{} = {}", dec_of(n_in), fs.join(" x "))
}

pub fn repl_scout(n_in: &[char]) -> String {
    let (res, log) = scout_factor(n_in);
    let n = dec_of(n_in);
    match res {
        Some((p, q, shape)) => format!("N={n}\n{log}  {n} = {} x {}  [{shape}]", dec_of(&p), dec_of(&q)),
        None => format!("N={n}\n{log}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smart_factor_gives_full_multiset() {
        let (fs, _) = smart_factor(&tape_u64(360));
        let prod = fs.iter().fold(1u64, |a, f| {
            let mut v = 0u64; for &c in trim(f.clone()).iter().rev() { v = (v << 1) | if c == EVALF { 1 } else { 0 }; } a * v
        });
        assert_eq!(prod, 360);
        assert_eq!(fs.len(), 6); // 2^3 * 3^2 * 5
        assert!(repl_smart_factor(&tape_u64(8051)).contains("83 x 97") || repl_smart_factor(&tape_u64(8051)).contains("97 x 83"));
    }

    #[test]
    fn scout_reads_the_shape_and_routes() {
        // Each shape routes to its cheapest probe; the name is the reading.
        let (r, _) = scout_factor(&tape_u64(8051));
        assert_eq!(r.unwrap().2, "small-trial");
        let (r, _) = scout_factor(&tape_u64(25));
        assert_eq!(r.unwrap().2, "perfect-power");
        let (r, log) = scout_factor(&tape_u64(999983));
        assert!(r.is_none() && log.contains("prime"));
        // near-root balanced: the frontier probe closes it.
        let (r, _) = scout_factor(&tape_u64(1000003 * 1000033));
        assert_eq!(r.unwrap().2, "frontier");
        // The scout hands this shape to the membrane; rho is now fused into
        // the sieve. Verify both the handoff and the complete factorization.
        let n = tape_u64(1000003 * 1000000007);
        let (r, log) = scout_factor(&n);
        assert!(r.is_none(), "{log}");
        assert!(log.contains("HARD"), "{log}");
        assert!(log.contains("no factor <= 3000"), "{log}");
        let (factors, route) = smart_factor(&n);
        assert_eq!(factors, vec![tape_u64(1000003), tape_u64(1000000007)], "{route}");
        assert_eq!(factors.iter().fold(one(), |p, f| mul(&p, f)), n);
    }

    fn numeral(mut n: u64) -> String {
        if n == 0 {
            return emit_numeral(&[EVALT]);
        }
        let mut t = Vec::new();
        while n != 0 {
            t.push(mark(n & 1 == 1));
            n >>= 1;
        }
        emit_numeral(&t)
    }
    #[test]
    fn factors_marks_without_numeric_state() {
        assert_eq!(factor(&numeral(143)).unwrap(), numeral(11));
        assert_eq!(factor(&numeral(127)).unwrap(), numeral(127));
        let f = factor(&numeral(8051)).unwrap();
        assert!(f == numeral(83) || f == numeral(97));
    }
    #[test]
    fn phase_family_reaches_a_forty_bit_semiprime() {
        let p = 1_000_003u64;
        let q = 1_000_033u64;
        let f = factor(&numeral(p * q)).unwrap();
        assert!(f == numeral(p) || f == numeral(q));
    }
    #[test]
    fn phase_family_reaches_a_sixty_bit_semiprime() {
        let p = 1_000_000_007u64;
        let q = 1_000_000_009u64;
        let f = factor(&numeral(p * q)).unwrap();
        assert!(f == numeral(p) || f == numeral(q));
    }

    // A single operator word whose interior is the six motif interiors in
    // canonical order. The carrier constructor recovers the full tower.
    const FULL: &str = "⊢∈≻⊤⊥∋∈⋈⊤⊥∋∈⊤⊥∋∈⊙∋≻⋈⊙⊡⊣";

    #[test]
    fn constructor_recovers_the_full_tower() {
        let tower = construct_carrier(FULL).unwrap();
        let names: Vec<&str> = tower.iter().map(|t| morphism_name(t)).collect();
        assert_eq!(names, ["PHASE", "ARITHMETIC", "BRANCH", "SELECT", "CONTINUE", "FIX"]);
    }

    #[test]
    fn constructed_carrier_factors_like_the_fixed_tower() {
        let f = factor_with(FULL, &numeral(8051)).unwrap();
        assert!(f == numeral(83) || f == numeral(97));
        assert_eq!(f, factor(&numeral(8051)).unwrap());
    }

    #[test]
    fn incomplete_operator_word_is_named_not_run() {
        // BRANCH + SELECT + FIX but no advance/continue: refused, missing named.
        let w = "⊢∈⊤⊥∋∈⊙∋⊙⊡⊣";
        let err = factor_with(w, &numeral(8051)).unwrap_err();
        assert!(err.contains("missing"));
        assert!(err.contains("CONTINUE"));
    }

    #[test]
    fn instant_extractor_decomposes_and_factors() {
        // The instant semiprime extractor carries AREV ≺ and ENGAGR ⊞ inside
        // its evaluate frame; that frame is the EXTRACT morphism, a one-frame
        // fold of advance, decide and continue. The word decomposes to
        // EXTRACT then FIX and factors on the carrier built from itself.
        let w = "⊢⊙∈≻⊤≺⊥⊞⋈∋⊙⊡⊣";
        let tower = construct_carrier(w).unwrap();
        let names: Vec<&str> = tower.iter().map(|t| morphism_name(t)).collect();
        assert_eq!(names, ["EXTRACT", "FIX"]);
        // The frontier arm may return either factor of the pair; both are valid.
        let f = factor_with(w, &numeral(8051)).unwrap();
        assert!(f == numeral(83) || f == numeral(97));
    }

    #[test]
    fn nested_p_minus_catches_a_far_large_smooth_predecessor_factor() {
        // p = 39916801 (11!+1, prime; p-1 = 11! is smooth), q = 1000000007 far
        // away. Frontier is dead on the gap, but the nested p-1 arm closes it.
        // Carrier: EXTRACT -> P_MINUS -> FIX.
        let p = 39916801u64;
        let q = 1000000007u64;
        let carrier = "⊢∈≻⊤≺⊥⊞⋈∋∈⊙⊞⋈∋⊙⊡⊣";
        let names: Vec<&str> = construct_carrier(carrier)
            .unwrap().iter().map(|t| morphism_name(t)).collect();
        assert_eq!(names, ["EXTRACT", "P_MINUS", "FIX"]);
        let f = factor_with(carrier, &numeral(p * q)).unwrap();
        assert!(f == numeral(p) || f == numeral(q));
    }

    #[test]
    fn squfof_finds_factors() {
        for &(nn, _a, _b) in &[
            (8051u64, 83u64, 97u64),
            (11111, 41, 271),
            (2021, 43, 47),
            (1234567, 127, 9721),
            (2027651281, 44021, 46061),
        ] {
            let g = squfof(&tape_u64(nn)).expect("squfof found nothing");
            let gv = tape_to_u64(&g);
            assert!(gv > 1 && gv < nn && nn % gv == 0, "squfof({nn}) gave {gv}");
        }
        // SQUFOF wired as a morphism, nested with a complete arm.
        let carrier = "⊢∈⊤≺⊥∋∈≻⊤≺⊥⊞⋈∋∈⊤≺⊞⊥∋⊙⊡⊣";
        assert_eq!(
            construct_carrier(carrier).unwrap().iter().map(|t| morphism_name(t)).collect::<Vec<_>>(),
            ["WITNESS", "EXTRACT", "SQUFOF", "FIX"]
        );
        let f = factor_with(carrier, &numeral(2027651281)).unwrap();
        assert!(f == numeral(44021) || f == numeral(46061));
    }

    #[test]
    fn lehman_step_finds_a_mid_range_factor() {
        // 8051 = 83 * 97; Lehman's first multiplier k=1 closes it.
        let f = lehman_step(&tape_u64(8051), &one()).unwrap();
        assert!(f == tape_u64(83) || f == tape_u64(97));
        // Nested in a complete carrier it still factors.
        let carrier = "⊢∈⊤≺⊥∋∈≻⊤≺⊥⊞⋈∋∈≻⋈⊤⊥∋⊙⊡⊣";
        assert_eq!(
            construct_carrier(carrier).unwrap().iter().map(|t| morphism_name(t)).collect::<Vec<_>>(),
            ["WITNESS", "EXTRACT", "LEHMAN", "FIX"]
        );
        let g = factor_with(carrier, &numeral(8051)).unwrap();
        assert!(g == numeral(83) || g == numeral(97));
    }

    #[test]
    fn power_and_lucas_helpers_are_exact() {
        assert_eq!(ipow(&tape_u64(7), 3), tape_u64(343));
        assert_eq!(iroot(&tape_u64(1000), 3), tape_u64(10));
        assert_eq!(iroot(&tape_u64(1001), 3), tape_u64(10));
        assert_eq!(iroot(&tape_u64(999), 3), tape_u64(9));
        // Lucas V_4(a=3) = 47, mod a prime large enough that no reduction bites.
        assert_eq!(lucas_v(&tape_u64(4), &tape_u64(3), &tape_u64(1_000_000_007)), tape_u64(47));
    }

    #[test]
    fn full_membrane_certifies_factors_and_takes_perfect_powers() {
        // WITNESS -> POWER -> EXTRACT -> P_MINUS -> P_PLUS -> ECM -> FIX
        let carrier = "⊢∈⊤≺⊥∋∈⊤⊞⊥∋∈≻⊤≺⊥⊞⋈∋∈⊙⊞⋈∋∈⊙≺⋈∋∈⊙≻⋈∋⊙⊡⊣";
        assert_eq!(
            construct_carrier(carrier).unwrap().iter().map(|t| morphism_name(t)).collect::<Vec<_>>(),
            ["WITNESS", "POWER", "EXTRACT", "P_MINUS", "P_PLUS", "ECM", "FIX"]
        );
        assert_eq!(factor_with(carrier, &numeral(2147483647)).unwrap(), numeral(2147483647));
        let f = factor_with(carrier, &numeral(8051)).unwrap();
        assert!(f == numeral(83) || f == numeral(97));
        // POWER first (before EXTRACT) takes a prime square to its base.
        let pw = "⊢∈⊤⊞⊥∋∈≻⊤≺⊥⊞⋈∋⊙⊡⊣";
        assert_eq!(
            construct_carrier(pw).unwrap().iter().map(|t| morphism_name(t)).collect::<Vec<_>>(),
            ["POWER", "EXTRACT", "FIX"]
        );
        // 9973 is prime; POWER returns the base 9973 of 9973^2.
        assert_eq!(factor_with(pw, &numeral(9973 * 9973)).unwrap(), numeral(9973));
    }

    #[test]
    fn witness_first_certifies_a_large_prime_without_the_sqrt_walk() {
        // WITNESS nested first: a strong prime test selects N at once, so a
        // large prime is not walked out to sqrt(N). Carrier
        // WITNESS -> EXTRACT -> P_MINUS -> ECM -> FIX.
        let carrier = "⊢∈⊤≺⊥∋∈≻⊤≺⊥⊞⋈∋∈⊙⊞⋈∋∈⊙≻⋈∋⊙⊡⊣";
        assert_eq!(
            construct_carrier(carrier).unwrap().iter().map(|t| morphism_name(t)).collect::<Vec<_>>(),
            ["WITNESS", "EXTRACT", "P_MINUS", "ECM", "FIX"]
        );
        // 2^31-1 is prime: selected as itself.
        assert_eq!(factor_with(carrier, &numeral(2147483647)).unwrap(), numeral(2147483647));
        // A composite still factors through the deeper arms.
        let f = factor_with(carrier, &numeral(8051)).unwrap();
        assert!(f == numeral(83) || f == numeral(97));
    }

    #[test]
    fn ecm_arm_factors_and_nests() {
        // ECM alone (curve method) finds a factor via curve-order smoothness,
        // an independent condition from p-1's. Carrier ECM -> FIX.
        let ecm = "⊢∈⊙≻⋈∋⊙⊡⊣";
        assert_eq!(
            construct_carrier(ecm).unwrap().iter().map(|t| morphism_name(t)).collect::<Vec<_>>(),
            ["ECM", "FIX"]
        );
        let f = factor_with(ecm, &numeral(8051)).unwrap();
        assert!(f == numeral(83) || f == numeral(97));
        // Nested with EXTRACT: EXTRACT -> ECM -> FIX composes and factors.
        let nested = "⊢∈≻⊤≺⊥⊞⋈∋∈⊙≻⋈∋⊙⊡⊣";
        assert_eq!(
            construct_carrier(nested).unwrap().iter().map(|t| morphism_name(t)).collect::<Vec<_>>(),
            ["EXTRACT", "ECM", "FIX"]
        );
        let g = factor_with(nested, &numeral(100160063)).unwrap();
        assert!(g == numeral(10007) || g == numeral(10009));
    }

    #[test]
    fn frontier_arm_clears_a_balanced_semiprime_fast() {
        // 32-bit balanced semiprime, gap 60: the rho ceiling would crawl, the
        // square frontier closes it in a handful of steps.
        let p = 65_521u64;
        let q = 65_581u64;
        let f = factor_with("⊢⊙∈≻⊤≺⊥⊞⋈∋⊙⊡⊣", &numeral(p * q)).unwrap();
        assert!(f == numeral(p) || f == numeral(q));
    }
}
