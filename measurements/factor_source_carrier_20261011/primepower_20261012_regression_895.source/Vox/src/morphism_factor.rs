//! Factoring over IMASM numeral tapes.
//!
//! A numeral is never decoded into a machine integer.  Its payload is a tape
//! of EVALT/EVALF marks, least significant cell first.  Arithmetic consumes
//! and produces those tapes through the full-adder/full-subtractor tables.

use crate::vox::{AFWD, AREV, CLINK, EVALF, EVALT, FFUSE, FSPLIT, IFIX, IMSCRIB, TANCH, VINIT};
use crate::morphism_factor as arithmetic;
use crate::godel_calculus::encode_cell_binary;
use crate::semiprime_descent::{
    cyclic_split, difference, event, natural, square_lift, verify_pair, word_parts, Four, Limits,
    Pair, Report,
};
use core::cmp::Ordering::{Greater, Less};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

type Tape = Vec<char>;

/// Observational counters for Vox's native sampler; never used for dispatch.
#[cfg(feature = "carrier-profile")]
#[used]
#[no_mangle]
pub static VOX_FACTOR_COUNTERS: [core::sync::atomic::AtomicU64; 9] =
    [const { core::sync::atomic::AtomicU64::new(0) }; 9];

const PHASE: &[char] = &[VINIT, FSPLIT, AFWD, EVALT, EVALF, FFUSE, TANCH];
const ARITHMETIC: &[char] = &[VINIT, FSPLIT, CLINK, EVALT, EVALF, FFUSE, TANCH];
const BRANCH: &[char] = &[VINIT, FSPLIT, EVALT, EVALF, FFUSE, TANCH];
const SELECT: &[char] = &[VINIT, FSPLIT, IMSCRIB, FFUSE, TANCH];
const CONTINUE: &[char] = &[VINIT, AFWD, CLINK, TANCH];
const FIX: &[char] = &[VINIT, IMSCRIB, IFIX, TANCH];
const SIEVE_SOURCE: &[char] = &[VINIT, FSPLIT, AFWD, '⊞', CLINK, FFUSE, TANCH];
const FIX_BANKED: &[char] = &[VINIT, IFIX, CLINK, IMSCRIB, TANCH];
// The repaired extract morphism banks its T/F/⊞ deposits and AREV within one
// frame; the adjacent ⊡⋈⊙ tail is the paired fixed-point latch.
const EXTRACT: &[char] = &[
    VINIT, FSPLIT, AFWD, EVALT, AREV, EVALF, '⊞', CLINK, FFUSE, TANCH,
];
const EXTRACT_BANKED: &[char] = &[
    VINIT, FSPLIT, AFWD, EVALT, CLINK, IMSCRIB, AREV, EVALF, '⊞', FFUSE, TANCH,
];
// EML evaluation-frame transport from the kernel-grounded EML ob3ect.  Its
// exp/log pair opens the evaluation frame, evaluates both supports, holds the
// winding boundary, then fuses and fixes the returned frame.
const EML_FRAME: &[char] = &[
    VINIT, AFWD, AREV, FSPLIT, EVALT, EVALF, '⊞', AREV, IMSCRIB, FFUSE, IFIX, TANCH,
];
const EML_EXP_MINUS_LOG: &[char] = &[
    VINIT, IMSCRIB, FSPLIT, AFWD, EVALT, AREV, EVALF, FFUSE, '⊞', CLINK, IFIX, TANCH,
];
const EML_EXP_DIV_LOG: &[char] = &[
    VINIT, IMSCRIB, FSPLIT, AFWD, EVALT, AREV, EVALF, '⊞', FFUSE, CLINK, IFIX, TANCH,
];
const EML_REVERSE_NEGATIVE: &[char] = &[
    VINIT, FSPLIT, AFWD, EVALT, AREV, EVALF, FFUSE, '⊞', CLINK, IMSCRIB, IFIX, TANCH,
];
// PHASE owns the EML evaluation frame: the phase frame opens first, the EML
// support frame is evaluated inside it, and the phase frame fuses last.
const PHASE_EML: &[char] = &[
    VINIT, FSPLIT, AFWD, EVALT, EVALF, AFWD, AREV, FSPLIT, EVALT, EVALF, '⊞', AREV, IMSCRIB, FFUSE,
    IFIX, FFUSE, TANCH,
];
// Generic structural bridge: the repaired kernel word closes to N and
// transports a banked negative phase read into the next phase-base frame.
const STRUCTURAL_BRIDGE: &[char] = &[
    VINIT, FFUSE, FSPLIT, EVALF, IMSCRIB, AFWD, CLINK, '⊞', EVALT, AREV, IFIX, TANCH,
];
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
// The bit-carry unfolding: fork a candidate low bit for each factor (FSPLIT),
// seed the pair's running tapes (IMSCRIB), advance one position (AFWD), test
// the low-order match (EVALT), retreat a dead branch (AREV), test the retreat
// outcome (EVALF), combine the surviving frame (CLINK), close (FFUSE). Reads
// p and q straight off N's own bits via a bottom-up 2-adic lift, the same
// carry/convolution recurrence SEMIPRIMALRAGE.md derives, rather than walking
// an order or a factor gap. At the swap-symmetric split (p_bits==q_bits) the
// odd-parity fork's two branches are related by exchanging p and q for every
// remaining bit, so only one is walked; the even-parity fork (both bits equal)
// has no such symmetry and both sides are walked.
const UNBRAID: &[char] = &[
    VINIT, FSPLIT, IMSCRIB, AFWD, EVALT, AREV, EVALF, CLINK, FFUSE, TANCH,
];

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
    let la = {
        let mut n = a.len();
        while n > 1 && a[n - 1] == 0 {
            n -= 1;
        }
        n
    };
    let lb = {
        let mut n = b.len();
        while n > 1 && b[n - 1] == 0 {
            n -= 1;
        }
        n
    };
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

// Normalized limb division. The quotient is selected one whole limb at a time,
// so modular powering does not pay a bit-serial scan for every remainder.
fn l_divmod(n: &[u64], d: &[u64]) -> (Limbs, Limbs) {
    if l_is_zero(d) || l_cmp(n, d) == core::cmp::Ordering::Less {
        return (vec![0], l_trim(n.to_vec()));
    }
    let n = l_trim(n.to_vec());
    let d = l_trim(d.to_vec());
    if d.len() == 1 {
        let divisor = d[0] as u128;
        let mut q = vec![0u64; n.len()];
        let mut rem = 0u128;
        for i in (0..n.len()).rev() {
            let dividend = (rem << 64) | n[i] as u128;
            q[i] = (dividend / divisor) as u64;
            rem = dividend % divisor;
        }
        return (l_trim(q), vec![rem as u64]);
    }

    let shift = d[d.len() - 1].leading_zeros();
    let mut v = vec![0u64; d.len()];
    let mut carry = 0u64;
    for (i, &limb) in d.iter().enumerate() {
        v[i] = (limb << shift) | carry;
        carry = if shift == 0 { 0 } else { limb >> (64 - shift) };
    }

    let mut u = vec![0u64; n.len() + 1];
    carry = 0;
    for (i, &limb) in n.iter().enumerate() {
        u[i] = (limb << shift) | carry;
        carry = if shift == 0 { 0 } else { limb >> (64 - shift) };
    }
    u[n.len()] = carry;

    let divisor_len = v.len();
    let top = v[divisor_len - 1] as u128;
    let m = n.len() - divisor_len;
    let mut q = vec![0u64; m + 1];
    for j in (0..=m).rev() {
        let numerator = ((u[j + divisor_len] as u128) << 64) | u[j + divisor_len - 1] as u128;
        let mut qhat = numerator / top;
        let mut rhat = numerator % top;
        if qhat >= (1u128 << 64) {
            qhat = (1u128 << 64) - 1;
            rhat = numerator - qhat * top;
        }
        while rhat < (1u128 << 64)
            && qhat * v[divisor_len - 2] as u128 > (rhat << 64) + u[j + divisor_len - 2] as u128
        {
            qhat -= 1;
            rhat += top;
        }

        let mut borrow = 0u128;
        for i in 0..divisor_len {
            let product = qhat * v[i] as u128 + borrow;
            let low = product as u64;
            borrow = product >> 64;
            let old = u[j + i];
            u[j + i] = old.wrapping_sub(low);
            if old < low {
                borrow += 1;
            }
        }
        let old_high = u[j + divisor_len];
        let underflow = (old_high as u128) < borrow;
        u[j + divisor_len] = old_high.wrapping_sub(borrow as u64);
        if underflow {
            qhat -= 1;
            let mut add_carry = 0u128;
            for i in 0..divisor_len {
                let sum = u[j + i] as u128 + v[i] as u128 + add_carry;
                u[j + i] = sum as u64;
                add_carry = sum >> 64;
            }
            u[j + divisor_len] = u[j + divisor_len].wrapping_add(add_carry as u64);
        }
        q[j] = qhat as u64;
    }

    let mut r = vec![0u64; divisor_len];
    if shift == 0 {
        r.copy_from_slice(&u[..divisor_len]);
    } else {
        for i in 0..divisor_len {
            r[i] = (u[i] >> shift) | (u[i + 1] << (64 - shift));
        }
    }
    (l_trim(q), l_trim(r))
}

/// Retain folded limbs while deriving modular views of an unrestricted tape.
pub(crate) struct FoldedTape(Limbs);
impl FoldedTape {
    pub(crate) fn new(tape: &[char]) -> Self {
        Self(fold(tape))
    }
    pub(crate) fn into_tape(self) -> Tape {
        unfold(&self.0)
    }
    pub(crate) fn divide_word_exact(&mut self, divisor: u64) -> bool {
        assert!(divisor > 1);
        if self.remainder_word(divisor) != 0 {
            return false;
        }
        let mut carry = 0u128;
        for limb in self.0.iter_mut().rev() {
            let wide = (carry << 64) | u128::from(*limb);
            *limb = (wide / u128::from(divisor)) as u64;
            carry = wide % u128::from(divisor);
        }
        while self.0.len() > 1 && self.0.last() == Some(&0) {
            self.0.pop();
        }
        true
    }
    pub(crate) fn remainder_word(&self, modulus: u64) -> u64 {
        assert_ne!(modulus, 0);
        let mut remainder = 0u128;
        for &limb in self.0.iter().rev() {
            remainder = ((remainder << 64) | u128::from(limb)) % u128::from(modulus);
        }
        remainder as u64
    }
}

pub(crate) fn inverse_residue_word(value: u64, modulus: u64) -> Option<u64> {
    if modulus < 2 {
        return None;
    }
    let (mut r0, mut r1) = (modulus, value % modulus);
    let (mut t0, mut t1) = (0u64, 1u64);
    while r1 != 0 {
        let quotient = r0 / r1;
        (r0, r1) = (r1, r0 % r1);
        let product = (u128::from(quotient) * u128::from(t1)) % u128::from(modulus);
        let next = (u128::from(t0) + u128::from(modulus) - product) % u128::from(modulus);
        (t0, t1) = (t1, next as u64);
    }
    (r0 == 1).then_some(t0)
}

pub fn cmp(a: &[char], b: &[char]) -> core::cmp::Ordering {
    let extent = |tape: &[char]| tape.iter().rposition(|&mark| mark == EVALF).map_or(0, |i| i + 1);
    let left = extent(a);
    let right = extent(b);
    match left.cmp(&right) {
        core::cmp::Ordering::Equal => {
            for i in (0..left).rev() {
                let ordering = (a[i] == EVALF).cmp(&(b[i] == EVALF));
                if ordering != core::cmp::Ordering::Equal { return ordering; }
            }
            core::cmp::Ordering::Equal
        }
        ordering => ordering,
    }
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

/// Multiply and add before one modular reduction, keeping every external
/// numeral on its IMASM tape while avoiding repeated fold/unfold boundaries.
pub fn mul_mod_add(a: &[char], b: &[char], c: &[char], n: &[char]) -> Tape {
    let left = fold(a);
    let right = fold(b);
    let addend = fold(c);
    let modulus = fold(n);
    if left.len() == 1
        && right.len() == 1
        && addend.len() == 1
        && modulus.len() == 1
        && modulus[0] != 0
    {
        let value = left[0] as u128 * right[0] as u128 + addend[0] as u128;
        return unfold(&[(value % modulus[0] as u128) as u64]);
    }
    let product = l_mul(&left, &right);
    let sum = l_add(&product, &addend);
    unfold(&l_divmod(&sum, &modulus).1)
}

/// Modular product with a single dynamic-limb fold and unfold per operand.
pub fn mul_mod(a: &[char], b: &[char], n: &[char]) -> Tape {
    let left = fold(a);
    let right = fold(b);
    let modulus = fold(n);
    if left.len() == 1 && right.len() == 1 && modulus.len() == 1 && modulus[0] != 0 {
        return unfold(&[((left[0] as u128 * right[0] as u128) % modulus[0] as u128) as u64]);
    }
    unfold(&l_divmod(&l_mul(&left, &right), &modulus).1)
}

/// Evaluate an LSB-first binary support tape as a polynomial through one
/// evaluation frame. The one-limb path keeps residues in the arithmetic
/// registers between coefficient groups; wider values use the same dynamic
/// limb/tape operations without a width ceiling.
pub fn eval_binary_support_frame(bits: &[char], x: &[char], n: &[char], width: usize) -> Tape {
    if width == 0 || bits.is_empty() {
        return vec![EVALT];
    }
    let modulus = fold(n);
    let phase = fold(x);
    if width <= 8 && modulus.len() == 1 && modulus[0] != 0 && phase.len() == 1 {
        let modulus = modulus[0] as u128;
        let phase = phase[0] as u128 % modulus;
        let mut powers = vec![1u128; width + 1];
        for index in 1..=width {
            powers[index] = powers[index - 1] * phase % modulus;
        }
        let frame_base = powers[width];
        let mut value = 0u128;
        for group in bits.chunks(width).rev() {
            let mut symbol = 0u128;
            for (position, mark) in group.iter().enumerate() {
                if *mark == EVALF {
                    symbol += powers[position];
                }
            }
            symbol %= modulus;
            value = ((value * frame_base) % modulus + symbol) % modulus;
        }
        return unfold(&[value as u64]);
    }

    let mut powers = vec![vec![1u64]; width + 1];
    for index in 1..=width {
        let product = l_mul(&powers[index - 1], &phase);
        powers[index] = l_divmod(&product, &modulus).1;
    }
    let frame_base = &powers[width];
    let mut value = vec![0u64];
    for group in bits.chunks(width).rev() {
        let mut symbol = vec![0u64];
        for (position, mark) in group.iter().enumerate() {
            if *mark == EVALF {
                symbol = l_add(&symbol, &powers[position]);
            }
        }
        let product = l_mul(&value, frame_base);
        let sum = l_add(&product, &symbol);
        value = l_divmod(&sum, &modulus).1;
    }
    unfold(&value)
}

fn mod_add(a: &[char], b: &[char], n: &[char]) -> Tape {
    let left = fold(a);
    let right = fold(b);
    let modulus = fold(n);
    if left.len() == 1 && right.len() == 1 && modulus.len() == 1 && modulus[0] != 0 {
        return unfold(&[((u128::from(left[0]) + u128::from(right[0]))
            % u128::from(modulus[0])) as u64]);
    }
    let sum = l_add(&left, &right);
    unfold(&l_divmod(&sum, &modulus).1)
}

fn mod_mul(a: &[char], b: &[char], n: &[char]) -> Tape {
    mul_mod(a, b, n)
}

fn abs_diff(a: &[char], b: &[char]) -> Tape {
    if cmp(a, b) == core::cmp::Ordering::Less {
        sub(b, a)
    } else {
        sub(a, b)
    }
}

pub fn gcd(mut a: Tape, mut b: Tape) -> Tape {
    let left = fold(&a);
    let right = fold(&b);
    if left.len() == 1 && right.len() == 1 {
        let (mut x, mut y) = (left[0], right[0]);
        while y != 0 {
            (x, y) = (y, x % y);
        }
        return unfold(&[x]);
    }
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
struct LehmanSweep {
    kn4: Tape,
    a: Tape,
    limit: Tape,
}

fn lehman_sweep(n: &[char], k: &[char]) -> LehmanSweep {
    use core::cmp::Ordering::Less;
    let kn4 = mul(&tape_u64(4), &mul(k, n));
    let mut a = isqrt(&kn4);
    if cmp(&mul(&a, &a), &kn4) == Less {
        a = add(&a, &one());
    }
    let sixth = iroot(n, 6);
    let sk = {
        let r = isqrt(k);
        if zero(&r) {
            one()
        } else {
            r
        }
    };
    let width = divmod(&sixth, &mul(&tape_u64(4), &sk)).0;
    let limit = add(&add(&a, &width), &one());
    LehmanSweep { kn4, a, limit }
}

fn lehman_resume(n: &[char], sweep: &mut LehmanSweep, budget: usize) -> Option<Tape> {
    use core::cmp::Ordering::{Greater, Less};
    for _ in 0..budget {
        if cmp(&sweep.a, &sweep.limit) == Greater {
            break;
        }
        let asq = mul(&sweep.a, &sweep.a);
        if cmp(&asq, &sweep.kn4) != Less {
            let c = sub(&asq, &sweep.kn4);
            if let Some(b) = is_square(&c) {
                let g = gcd(add(&sweep.a, &b), n.to_vec());
                if cmp(&g, &one()) == Greater && cmp(&g, n) == Less {
                    return Some(trim(g));
                }
            }
        }
        sweep.a = add(&sweep.a, &one());
    }
    None
}

#[cfg(test)]
fn lehman_step(n: &[char], k: &[char]) -> Option<Tape> {
    let mut sweep = lehman_sweep(n, k);
    while cmp(&sweep.a, &sweep.limit) != core::cmp::Ordering::Greater {
        if let Some(g) = lehman_resume(n, &mut sweep, 16) {
            return Some(g);
        }
    }
    None
}

/// Shanks's square forms factorization over numeral tapes. Runs the forward
/// cycle of the principal form until a square Q appears at an even step, then the
/// reverse cycle until P stabilizes; gcd(P, N) is then a factor. Tries a few
/// multipliers. The Q recurrence carries a real sign, tracked by branch since the
/// tapes are unsigned.
#[cfg(test)]
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

const SQUFOF_MULTIPLIERS: [u64; 7] = [1, 3, 5, 7, 11, 13, 15];

#[derive(Default)]
struct SqufofCycle {
    multiplier: usize,
    frame: Option<SqufofFrame>,
}

struct SqufofFrame {
    d: Tape,
    s: Tape,
    p: Tape,
    q_prev: Tape,
    q: Tape,
    cap: Tape,
    steps: Tape,
    reverse: bool,
}

impl SqufofCycle {
    fn advance(&mut self) {
        self.frame = None;
        self.multiplier += 1;
    }

    fn done(&self) -> bool {
        self.multiplier >= SQUFOF_MULTIPLIERS.len()
    }
}

fn squfof_resume(n: &[char], cycle: &mut SqufofCycle, budget: usize) -> Option<Tape> {
    use core::cmp::Ordering::{Equal, Greater, Less};
    for _ in 0..budget {
        if cycle.done() {
            break;
        }
        if cycle.frame.is_none() {
            let d = mul(n, &tape_u64(SQUFOF_MULTIPLIERS[cycle.multiplier]));
            let s = isqrt(&d);
            let square = mul(&s, &s);
            if cmp(&square, &d) == Equal {
                let g = gcd(s, n.to_vec());
                cycle.advance();
                if cmp(&g, &one()) == Greater && cmp(&g, n) == Less {
                    return Some(g);
                }
                continue;
            }
            let bound = mul(&tape_u64(4), &isqrt(&mul(&two(), &s)));
            let cap = if cmp(&bound, &tape_u64(64)) == Less { tape_u64(64) } else { bound };
            cycle.frame = Some(SqufofFrame {
                q: sub(&d, &square), p: s.clone(), d, s,
                q_prev: one(), cap, steps: one(), reverse: false,
            });
        }
        let frame = cycle.frame.as_mut().unwrap();
        if cmp(&frame.steps, &frame.cap) == Greater || zero(&frame.q) {
            cycle.advance();
            continue;
        }
        if !frame.reverse && frame.steps[0] == EVALT {
            if let Some(root) = is_square(&frame.q) {
                if cmp(&frame.s, &frame.p) == Less {
                    cycle.advance();
                    continue;
                }
                let b = divmod(&sub(&frame.s, &frame.p), &root).0;
                let p = add(&frame.p, &mul(&b, &root));
                let square = mul(&p, &p);
                if cmp(&square, &frame.d) == Greater {
                    cycle.advance();
                    continue;
                }
                frame.q = divmod(&sub(&frame.d, &square), &root).0;
                frame.q_prev = root;
                frame.p = p;
                frame.steps = vec![EVALT];
                frame.reverse = true;
                continue;
            }
        }
        let b = divmod(&add(&frame.s, &frame.p), &frame.q).0;
        let product = mul(&b, &frame.q);
        if cmp(&product, &frame.p) == Less {
            cycle.advance();
            continue;
        }
        let p = sub(&product, &frame.p);
        if frame.reverse && cmp(&p, &frame.p) == Equal {
            let g = gcd(p, n.to_vec());
            cycle.advance();
            if cmp(&g, &one()) == Greater && cmp(&g, n) == Less {
                return Some(g);
            }
            continue;
        }
        let term = mul(&b, &abs_diff(&frame.p, &p));
        let q = if cmp(&frame.p, &p) != Less {
            add(&frame.q_prev, &term)
        } else if cmp(&frame.q_prev, &term) != Less {
            sub(&frame.q_prev, &term)
        } else {
            cycle.advance();
            continue;
        };
        frame.q_prev = core::mem::replace(&mut frame.q, q);
        frame.p = p;
        frame.steps = add(&frame.steps, &one());
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

/// Structural congruence support for the prime lane.
///
/// For every prime p > 3, 12 divides p^2 - 1. This is a support read rather
/// than a primality decision: composite values such as 25 can satisfy the
/// congruence and remain in the falsity lane for the Miller-Rabin witness.
pub fn prime12_support(n: &[char]) -> bool {
    let n = trim(n.to_vec());
    if cmp(&n, &tape_u64(3)) != core::cmp::Ordering::Greater {
        return false;
    }
    let square = mul(&n, &n);
    let predecessor = sub(&square, &one());
    zero(&modulo(&predecessor, &tape_u64(12)))
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
pub(crate) fn iroot(n: &[char], b: usize) -> Tape {
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

/// Floor integer square root over numeral tapes.
///
/// The old binary search performed one full tape division for every bit of
/// the answer.  That made the square-frontier arm re-scan the same nested
/// membrane for large operands.  Newton's decreasing iteration keeps the
/// resident value folded and reaches the floor in O(log bits) divisions.
pub fn isqrt(n: &[char]) -> Tape {
    let source = fold(n);
    if source.len() == 1 && source[0] < 2 {
        return unfold(&source);
    }
    let bits = (source.len() - 1) * 64
        + (64 - source.last().unwrap().leading_zeros() as usize);
    let exponent = (bits + 1) / 2;
    let mut x = vec![0; exponent / 64 + 1];
    x[exponent / 64] = 1 << (exponent % 64);
    loop {
        let q = l_divmod(&source, &x).0;
        let next = l_divmod(&l_add(&x, &q), &[2]).0;
        if l_cmp(&next, &x) != core::cmp::Ordering::Less {
            break;
        }
        x = next;
    }
    while l_cmp(&l_mul(&x, &x), &source) == core::cmp::Ordering::Greater {
        x = l_sub(&x, &[1]);
    }
    unfold(&x)
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

/// (a - b) mod n, for a and b already reduced into [0, n).
fn mod_sub(a: &[char], b: &[char], n: &[char]) -> Tape {
    let left = l_divmod(&fold(a), &fold(n)).1;
    let right = l_divmod(&fold(b), &fold(n)).1;
    let modulus = fold(n);
    if modulus.len() == 1 {
        let m = u128::from(modulus[0]);
        let lhs = u128::from(left[0]);
        let rhs = u128::from(right[0]);
        return unfold(&[if lhs >= rhs {
            (lhs - rhs) as u64
        } else {
            (m + lhs - rhs) as u64
        }]);
    }
    if l_cmp(&left, &right) != core::cmp::Ordering::Less {
        unfold(&l_sub(&left, &right))
    } else {
        unfold(&l_sub(&modulus, &l_sub(&right, &left)))
    }
}

/// Modular inverse of a mod n by the extended Euclidean algorithm, the
/// coefficients kept reduced mod n so every tape stays non-negative. Ok is the
/// inverse; Err is a nontrivial gcd, which is a factor of n. This Err is exactly
/// the elliptic-curve method's factor-discovery event.
pub(crate) fn mod_inv(a: &[char], n: &[char]) -> Result<Tape, Tape> {
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
            if cmp(&g, &one()) == core::cmp::Ordering::Greater
                && cmp(&g, n) == core::cmp::Ordering::Less
            {
                Some(trim(g))
            } else {
                None
            }
        }
    }
}

pub const SEMIPRIME_DESCENT: &[char] = &[FSPLIT, EVALT, EVALF, '⊞', FFUSE];

struct State {
    n: Tape,
    phase_base: Tape,
    candidate: Tape,
    remainder: Tape,
    x: Tape,
    y: Tape,
    phase: Tape,
    eml_partners: Option<crate::phase_partners::Partners>,
    eml_phase_done: bool,
    // Negative support is banked at frame apertures instead of being reduced
    // to the old phase-done boolean. Each entry is (phase index, residue).
    negative_support: Vec<(Tape, Tape)>,
    descent_evidence: Four,
    descent_request: Option<(Tape, Tape, String, Limits, Four)>,
    descent_report: Option<Result<Report, String>>,
    divisor: Tape,
    a: Tape,
    pm_a: Tape,
    pm_e: Tape,
    ecm_seed: Tape,
    ecm_round: Tape,
    pp_base: Tape,
    lehman_k: Tape,
    lehman_sweep: Option<LehmanSweep>,
    witness_done: bool,
    prime12_support: Option<bool>,
    power_done: bool,
    squfof_done: bool,
    squfof_cycle: SqufofCycle,
    round: Tape,
    exhausted: bool,
    selected: Option<Tape>,
    selected_by: Option<&'static str>,
    // Each frame carries the two LSB-first lanes, the exact IMASM carry, and
    // the next product position to close.
    unbraid_stack: Vec<(Tape, Tape, Tape, usize)>,
    unbraid_pair: Option<(Tape, Tape)>,
    unbraid_p_bits: usize,
    unbraid_q_bits: usize,
    unbraid_p_bound: Tape,
    unbraid_q_bound: Tape,
    unbraid_started: bool,
    unbraid_done: bool,
    carrier_closed: bool,
    bridge_count: usize,
}

fn advance_eml_phase(state: &mut State) {
    if state.eml_phase_done || state.selected.is_some() {
        return;
    }
    if state.eml_partners.is_none() {
        state.eml_partners =
            crate::phase_partners::Partners::new(state.phase_base.clone(), state.n.clone()).ok();
        state.eml_phase_done = state.eml_partners.is_none();
    }
    let candidate = if let Some(partners) = state.eml_partners.as_mut() {
        let support_probe = partners.support_probe();
        let negative_snapshot = if support_probe {
            Some(partners.negative_support_snapshot())
        } else {
            None
        };
        if let Some(target) = partners.support_target() {
            state.eml_phase_done = true;
            Some((target.p, target.q))
        } else {
            match partners.observe() {
                Ok(Some(relation)) => {
                    state.eml_phase_done = true;
                    relation
                        .half_residues
                        .as_ref()
                        .and_then(|(current, earlier)| {
                            crate::shor_braid::phase_factor_register_seeds(
                                &state.n, current, earlier,
                            )
                            .ok()
                        })
                        .or_else(|| {
                            crate::shor_braid::factor_close_public(
                                &state.phase_base,
                                &state.n,
                                &relation.return_exponent,
                            )
                            .ok()
                        })
                }
                Ok(None) => {
                    if let Some(snapshot) = negative_snapshot {
                        state.negative_support.push(snapshot);
                    }
                    None
                }
                Err(_) => {
                    if let Some(snapshot) = negative_snapshot {
                        state.negative_support.push(snapshot);
                    }
                    state.eml_phase_done = true;
                    None
                }
            }
        }
    } else {
        None
    };
    if let Some((p, q)) = candidate {
        if let Some(fixed) = crate::factor_2adic::meet_factor_nestings(&state.n, &p, &q, &two()) {
            if let Some(pair) =
                crate::factor_2adic::terminal_pair_given_semiprime_promise(&state.n, fixed)
            {
                state.selected = Some(pair.p);
            }
        }
    }
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
const FIX_BANKED_I: &[char] = &[IFIX, CLINK, IMSCRIB];
const EXTRACT_I: &[char] = &[FSPLIT, AFWD, EVALT, AREV, EVALF, '⊞', CLINK, FFUSE];
const EXTRACT_BANKED_I: &[char] = &[FSPLIT, AFWD, EVALT, CLINK, IMSCRIB, AREV, EVALF, '⊞', FFUSE];
const EML_FRAME_I: &[char] = &[
    AFWD, AREV, FSPLIT, EVALT, EVALF, '⊞', AREV, IMSCRIB, FFUSE, IFIX,
];
const PHASE_EML_I: &[char] = &[
    FSPLIT, AFWD, EVALT, EVALF, AFWD, AREV, FSPLIT, EVALT, EVALF, '⊞', AREV, IMSCRIB, FFUSE, IFIX,
    FFUSE,
];
const EML_EXP_MINUS_LOG_I: &[char] = &[
    IMSCRIB, FSPLIT, AFWD, EVALT, AREV, EVALF, FFUSE, '⊞', CLINK, IFIX,
];
const EML_EXP_DIV_LOG_I: &[char] = &[
    IMSCRIB, FSPLIT, AFWD, EVALT, AREV, EVALF, '⊞', FFUSE, CLINK, IFIX,
];
const EML_REVERSE_NEGATIVE_I: &[char] = &[
    FSPLIT, AFWD, EVALT, AREV, EVALF, FFUSE, '⊞', CLINK, IMSCRIB, IFIX,
];
const STRUCTURAL_BRIDGE_I: &[char] = &[
    FFUSE, FSPLIT, EVALF, IMSCRIB, AFWD, CLINK, '⊞', EVALT, AREV, IFIX,
];
const P_MINUS_I: &[char] = &[FSPLIT, IMSCRIB, '⊞', CLINK, FFUSE];
const ECM_I: &[char] = &[FSPLIT, IMSCRIB, AFWD, CLINK, FFUSE];
const WITNESS_I: &[char] = &[FSPLIT, EVALT, AREV, EVALF, FFUSE];
const POWER_I: &[char] = &[FSPLIT, EVALT, '⊞', EVALF, FFUSE];
const P_PLUS_I: &[char] = &[FSPLIT, IMSCRIB, AREV, CLINK, FFUSE];
const LEHMAN_I: &[char] = &[FSPLIT, AFWD, CLINK, EVALT, EVALF, FFUSE];
const SQUFOF_I: &[char] = &[FSPLIT, EVALT, AREV, '⊞', EVALF, FFUSE];
const UNBRAID_I: &[char] = &[FSPLIT, IMSCRIB, AFWD, EVALT, AREV, EVALF, CLINK, FFUSE];

/// Name of an operator motif, for reporting a constructed tower.
pub fn morphism_name(operator: &[char]) -> &'static str {
    if operator == SIEVE_SOURCE {
        "SIEVE_SOURCE"
    } else if operator == SEMIPRIME_DESCENT {
        "SEMIPRIME_DESCENT"
    } else if operator == &[AFWD] {
        "FORWARD_PAIR"
    } else if operator == &[CLINK] {
        "LINK_PRODUCT"
    } else if operator == &[AREV] {
        "RETURN_SOURCE"
    } else if operator == &[IFIX] {
        "LATCH_PAIR"
    } else if operator == &[TANCH] {
        "RELEASE_PAIR"
    } else if operator == PHASE {
        "PHASE"
    } else if operator == ARITHMETIC {
        "ARITHMETIC"
    } else if operator == BRANCH {
        "BRANCH"
    } else if operator == SELECT {
        "SELECT"
    } else if operator == CONTINUE {
        "CONTINUE"
    } else if operator == FIX || operator == FIX_BANKED {
        "FIX"
    } else if operator == EXTRACT || operator == EXTRACT_BANKED {
        "EXTRACT"
    } else if operator == EML_FRAME {
        "EML_FRAME"
    } else if operator == EML_EXP_MINUS_LOG {
        "EML_EXP_MINUS_LOG"
    } else if operator == EML_EXP_DIV_LOG {
        "EML_EXP_DIV_LOG"
    } else if operator == EML_REVERSE_NEGATIVE {
        "EML_REVERSE_NEGATIVE"
    } else if operator == PHASE_EML {
        "PHASE_EML"
    } else if operator == STRUCTURAL_BRIDGE {
        "STRUCTURAL_BRIDGE"
    } else if operator == P_MINUS {
        "P_MINUS"
    } else if operator == ECM {
        "ECM"
    } else if operator == WITNESS {
        "WITNESS"
    } else if operator == POWER {
        "POWER"
    } else if operator == P_PLUS {
        "P_PLUS"
    } else if operator == LEHMAN {
        "LEHMAN"
    } else if operator == SQUFOF {
        "SQUFOF"
    } else if operator == UNBRAID {
        "UNBRAID"
    } else {
        "?"
    }
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
    if operator_word == source_carrier_word() {
        return Ok(vec![WITNESS, POWER, EXTRACT, SQUFOF, P_MINUS, P_PLUS,
            LEHMAN, ECM, SIEVE_SOURCE, FIX]);
    }
    if operator_word == NINE_ARM {
        return Ok(vec![
            WITNESS, POWER, EXTRACT, SQUFOF, P_MINUS, P_PLUS, LEHMAN, ECM, FIX,
        ]);
    }
    // This register carries its own five-stage frame. External interfaces are
    // optional, and sealing marks carry verified output without extracting it.
    if let Ok((glyphs, start)) = word_parts(operator_word) {
        let mut tower = vec![SEMIPRIME_DESCENT];
        for glyph in &glyphs[start + SEMIPRIME_DESCENT.len()..] {
            tower.push(match *glyph {
                AFWD => &[AFWD],
                CLINK => &[CLINK],
                AREV => &[AREV],
                IFIX => &[IFIX],
                TANCH => &[TANCH],
                _ => unreachable!(),
            });
        }
        return Ok(tower);
    }
    let c: Vec<char> = operator_word.chars().collect();
    if operator_word == "⊢⊙∈≻⊤≺⊥∋⊞⋈⊡⊣" {
        return Ok(vec![EML_EXP_MINUS_LOG]);
    }
    if operator_word == "⊢⊙∈≻⊤≺⊥⊞∋⋈⊡⊣" {
        return Ok(vec![EML_EXP_DIV_LOG]);
    }
    if operator_word == "⊢∈≻⊤≺⊥∋⊞⋈⊙⊡⊣" {
        return Ok(vec![EML_REVERSE_NEGATIVE]);
    }
    // The source is independently bound by the caller. Optional boundary
    // glyphs do not determine whether the interior has executable morphisms.
    let start = usize::from(c.first() == Some(&VINIT));
    let end = c.len() - usize::from(c.last() == Some(&TANCH));
    let body = &c[start..end.max(start)];
    // Longest interior first so complete phase motifs win over BRANCH, and the
    // two FIX spellings win over a lone IMSCRIB carry.
    let motifs: [(&[char], &[char]); 24] = [
        (UNBRAID_I, UNBRAID),
        (PHASE_EML_I, PHASE_EML),
        (STRUCTURAL_BRIDGE_I, STRUCTURAL_BRIDGE),
        (EML_EXP_MINUS_LOG_I, EML_EXP_MINUS_LOG),
        (EML_EXP_DIV_LOG_I, EML_EXP_DIV_LOG),
        (EML_REVERSE_NEGATIVE_I, EML_REVERSE_NEGATIVE),
        (EML_FRAME_I, EML_FRAME),
        (EXTRACT_BANKED_I, EXTRACT_BANKED),
        (EXTRACT_I, EXTRACT),
        (SEMIPRIME_DESCENT, SEMIPRIME_DESCENT),
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
        (FIX_BANKED_I, FIX_BANKED),
        (FIX_I, FIX),
    ];
    let mut tower: Vec<&'static [char]> = Vec::new();
    let mut i = 0;
    'scan: while i < body.len() {
        if body[i] == TANCH && body[i+1..].iter().all(|&glyph| glyph == IMSCRIB) {
            i += 1;
            continue;
        }
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

/// Frozen VOX semantics: an executable resident object has two projections.
/// `walk` is the word the resident machine executes; `type_word` is the word
/// whose structure is audited.  They are related by one defect-class
/// reconciliation morphism, never silently identified.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reconciliation {
    /// WORK-ENCLOSING: the walk already is its closing structural type.
    Identity,
    /// ADJACENT-DYAD: `∈∋τ` is reconciled to `∈τ∋`.
    FrameWork,
    /// UNCLOSED: one dangling `∈` is closed before the outer `⊣` boundary.
    CloseFrame,
}

#[derive(Clone, Copy, Debug)]
pub struct FactorObject<'a> {
    pub walk: &'a str,
    pub type_word: &'a str,
    pub reconciliation: Reconciliation,
}

impl<'a> FactorObject<'a> {
    pub const fn new(walk: &'a str, type_word: &'a str, reconciliation: Reconciliation) -> Self {
        Self {
            walk,
            type_word,
            reconciliation,
        }
    }

    /// Legacy/single-word factoring is the diagonal `(walk,type) = (w,w)`.
    pub const fn identity(word: &'a str) -> Self {
        Self::new(word, word, Reconciliation::Identity)
    }
}

/// Relational check for the frozen FRAME_WORK defect class.
/// Exactly one adjacent `∈∋` in the walk must become `∈ ... ∋` in the type,
/// with every other mark byte-for-byte unchanged.  We verify the relation
/// rather than guessing how long `τ` is; this is the Stage-24 rule that each
/// application is checked, not generalised by fiat.
fn frame_work_holds(walk: &str, type_word: &str) -> bool {
    let w: Vec<char> = walk.chars().collect();
    let t: Vec<char> = type_word.chars().collect();
    if w.len() != t.len() {
        return false;
    }

    for i in 0..w.len().saturating_sub(1) {
        if w[i] != FSPLIT || w[i + 1] != FFUSE {
            continue;
        }
        let mut wr = w.clone();
        wr.remove(i + 1);
        // The fuse must move strictly to the right, leaving work inside.
        for j in (i + 2)..t.len() {
            if t[j] != FFUSE {
                continue;
            }
            let mut tr = t.clone();
            tr.remove(j);
            if wr == tr {
                return true;
            }
        }
    }
    false
}

fn fork_balance(word: &[char]) -> Option<i32> {
    let mut depth = 0i32;
    for &g in word {
        if g == FSPLIT {
            depth += 1;
        } else if g == FFUSE {
            depth -= 1;
            if depth < 0 {
                return None;
            }
        }
    }
    Some(depth)
}

/// Relational check for CLOSE_FRAME.  The walk has exactly one net dangling
/// split; the type is the same word with exactly one `∋` inserted at some later
/// structural boundary, yielding zero surplus.  Its exact insertion point is
/// part of the supplied type and is therefore audited, not guessed.
fn close_frame_holds(walk: &str, type_word: &str) -> bool {
    let w: Vec<char> = walk.chars().collect();
    let t: Vec<char> = type_word.chars().collect();
    if t.len() != w.len() + 1 || fork_balance(&w) != Some(1) || fork_balance(&t) != Some(0) {
        return false;
    }
    for j in 0..t.len() {
        if t[j] != FFUSE {
            continue;
        }
        let mut tr = t.clone();
        tr.remove(j);
        if tr == w {
            return true;
        }
    }
    false
}

/// Check only the walk/type relation.  This does not claim factoring
/// completeness; that is a separate type-side audit below.
pub fn audit_reconciliation(object: &FactorObject<'_>) -> Result<(), String> {
    let holds = match object.reconciliation {
        Reconciliation::Identity => object.walk == object.type_word,
        Reconciliation::FrameWork => frame_work_holds(object.walk, object.type_word),
        Reconciliation::CloseFrame => close_frame_holds(object.walk, object.type_word),
    };

    if !holds {
        return Err(format!(
            "walk/type reconciliation mismatch: {:?} does not relate walk {} to type {}",
            object.reconciliation, object.walk, object.type_word
        ));
    }
    Ok(())
}

/// The type face must not merely close structurally: it must decompose into a
/// factoring-complete resident carrier.  This is the VOX_SEMANTICS distinction
/// between a genuine PHASE carrier and a carrier that can actually factor.
fn require_factoring_complete(tower: &[&[char]]) -> Result<(), String> {
    let has = |op: &[char]| tower.iter().any(|t| *t == op);
    // Fusion verifies and returns the pair inside this descent. A separate
    // latch is an optional output boundary, not an extraction prerequisite.
    if has(SEMIPRIME_DESCENT) {
        return Ok(());
    }
    let mut missing = Vec::new();
    // EXTRACT, ECM and UNBRAID each fold advance, decide and continue into one boundary.
    if !has(EXTRACT) && !has(EXTRACT_BANKED) && !has(ECM) && !has(UNBRAID) {
        if !has(ARITHMETIC) {
            missing.push("ARITHMETIC (remainder and gcd readout)");
        }
        if !has(SELECT) {
            missing.push("SELECT (decide)");
        }
        if !has(CONTINUE) {
            missing.push("CONTINUE (step the candidate)");
        }
    }
    if !has(FIX) && !has(FIX_BANKED) {
        missing.push("FIX (latch)");
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "operator type does not carry factoring; missing: {}",
            missing.join(", ")
        ))
    }
}

/// Audit the structural/type face and return its resident factoring tower.
pub fn audit_factor_object(object: &FactorObject<'_>) -> Result<Vec<&'static [char]>, String> {
    audit_reconciliation(object)?;
    let tower = construct_carrier(object.type_word)?;
    require_factoring_complete(&tower)?;
    audit_execution_projection(object, &tower)?;
    Ok(tower)
}

/// The morphism executor acts on the work carried by a boundary; VINIT/TANCH
/// are interfaces and FSPLIT/FFUSE are the structural frame.  Stage-25's
/// EXECUTION PRESERVATION obligation therefore becomes a byte-level equality
/// between the walk's non-frame work and the work carried by the typed tower.
fn work_projection(word: &[char]) -> Vec<char> {
    word.iter()
        .copied()
        .filter(|g| *g != VINIT && *g != TANCH && *g != FSPLIT && *g != FFUSE)
        .collect()
}

fn tower_work_projection(tower: &[&[char]]) -> Vec<char> {
    let mut out = Vec::new();
    for op in tower {
        out.extend(work_projection(op));
    }
    out
}

/// Marks which the carrier grammar permits outside a dispatch motif.  They are
/// carried across the resident object but deliberately emit no morphism from
/// `construct_carrier`.
fn non_dispatch_carry(g: char) -> bool {
    g == AREV || g == '⊞' || g == IMSCRIB
}

/// The morphism tower is a dispatch projection, not a byte-for-byte copy of
/// the walk: standalone carry marks are intentionally absent from it.  Check
/// that the tower work can be obtained from the preserved walk work by deleting
/// only those legal non-dispatch carries.
///
/// A small DP is used rather than a greedy subsequence walk because the same
/// glyphs (AREV/ENGAGR/IMSCRIB) may also occur *inside* a real morphism motif;
/// at such a position the mark may either be consumed by the tower or be a
/// standalone carry, and the later context decides which reading is valid.
fn dispatch_projection_holds(walk_work: &[char], tower_work: &[char]) -> bool {
    let mut reachable = vec![false; tower_work.len() + 1];
    reachable[0] = true;

    for &g in walk_work {
        let mut next = vec![false; tower_work.len() + 1];
        for j in 0..=tower_work.len() {
            if !reachable[j] {
                continue;
            }

            // The carrier constructor explicitly permits this mark to pass
            // without dispatching a morphism.
            if non_dispatch_carry(g) {
                next[j] = true;
            }

            // Or this occurrence can belong to the next dispatched morphism.
            if j < tower_work.len() && g == tower_work[j] {
                next[j + 1] = true;
            }
        }
        reachable = next;
    }

    reachable[tower_work.len()]
}

pub fn audit_execution_projection(
    object: &FactorObject<'_>,
    tower: &[&[char]],
) -> Result<(), String> {
    let walk: Vec<char> = object.walk.chars().collect();
    let type_word: Vec<char> = object.type_word.chars().collect();
    let walk_work = work_projection(&walk);
    let type_work = work_projection(&type_word);
    let tower_work = tower_work_projection(tower);

    if walk_work != type_work {
        return Err("execution preservation failed: walk/type work projections differ".into());
    }
    if !dispatch_projection_holds(&walk_work, &tower_work) {
        return Err(
            "execution preservation failed: walk work does not match typed morphism tower".into(),
        );
    }
    Ok(())
}

/// Factor using a frozen `(walk,type)` VOX object.
///
/// `type_word` supplies the structural decomposition into resident morphisms;
/// `walk` supplies the executable work projection.  `audit_factor_object`
/// proves reconciliation, factoring completeness, and byte-equality of the
/// executable work before the morphism-level executor is allowed to run.
pub fn factor_object(object: &FactorObject<'_>, n_word: &str) -> Result<String, String> {
    factor_object_with_phase_base(object, n_word, &two())
}

/// Factor a numeral using a phase base supplied as an encoded IMASM numeral.
/// The base is a membrane input, not a source-level constant.
pub fn factor_object_with_phase_base(
    object: &FactorObject<'_>,
    n_word: &str,
    phase_base: &[char],
) -> Result<String, String> {
    let tower = audit_factor_object(object)?;

    let n = parse_numeral(n_word)?;
    if cmp(&n, &two()) == core::cmp::Ordering::Less {
        return Err("numeral has no non-trivial factor".into());
    }
    if tower.iter().any(|op| *op == SEMIPRIME_DESCENT) {
        if word_parts(object.walk).is_err() {
            return run_carrier_rounds(&tower, &n, u64::MAX)
                .map(|factor| emit_numeral(&factor))
                .ok_or_else(|| "carrier exhausted its round budget without latching".into());
        }
        let report = crate::semiprime_descent::until_closed(
            n, two(), one(), object.walk, 100_000, |_, _, _| Ok(()),
        )?;
        return Ok(emit_numeral(&report.pair.unwrap().p));
    }
    let (_, even) = divmod(&n, &two());
    if zero(&even) {
        return Ok(emit_numeral(&two()));
    }
    if cmp(phase_base, &one()) != core::cmp::Ordering::Greater {
        return Err("phase base must be greater than one".into());
    }
    let base_gcd = gcd(phase_base.to_vec(), n.clone());
    if cmp(&base_gcd, &one()) == core::cmp::Ordering::Greater
        && cmp(&base_gcd, &n) == core::cmp::Ordering::Less
    {
        return Ok(emit_numeral(&base_gcd));
    }
    if base_gcd != one() {
        return Err("phase base must be coprime to N or expose a proper factor".into());
    }
    let tower_refs: Vec<&[char]> = tower.iter().map(|t| *t).collect();
    match run_carrier_rounds_with_phase_base(&tower_refs, &n, phase_base, u64::MAX) {
        Some(f) => Ok(emit_numeral(&f)),
        None => Err("carrier exhausted its round budget without latching".into()),
    }
}

/// Backward-compatible single-word surface.  Under Stage-25 semantics this is
/// exactly the diagonal object `(walk,type) = (word,word)` with IDENTITY
/// reconciliation.
pub fn factor_with(operator_word: &str, n_word: &str) -> Result<String, String> {
    factor_object(&FactorObject::identity(operator_word), n_word)
}

/// Single-word factoring surface with a dynamically baked IMASM phase base.
pub fn factor_with_phase_base(
    operator_word: &str,
    n_word: &str,
    phase_base_word: &str,
) -> Result<String, String> {
    let phase_base = parse_numeral(phase_base_word)?;
    factor_object_with_phase_base(&FactorObject::identity(operator_word), n_word, &phase_base)
}

/// Run a carrier tower on a tape for at most `max_rounds` rounds, returning the
/// factor as soon as an arm latches, or None when the budget is spent. The
/// bounded form is what lets the HARD branch nest the whole nine-arm carrier
/// OUTSIDE the sieve: the cheap and mid arms (witness, extract, p-1, p+1, SQUFOF,
/// Lehman, ECM) all get their throttled rounds first, and the quadratic sieve is
/// the deepest fallback, run only after this returns None. `u64::MAX` is the
/// unbounded run factor_with wants.
pub fn run_carrier_rounds(tower: &[&[char]], n_in: &[char], max_rounds: u64) -> Option<Tape> {
    run_carrier_rounds_with_phase_base(tower, n_in, &two(), max_rounds)
}

/// Bounded carrier execution with a dynamic phase base already parsed from its
/// IMASM numeral. All non-phase morphisms remain unchanged.
pub fn run_carrier_rounds_with_phase_base(
    tower: &[&[char]],
    n_in: &[char],
    phase_base: &[char],
    max_rounds: u64,
) -> Option<Tape> {
    let n = trim(n_in.to_vec());
    if cmp(&n, &two()) == core::cmp::Ordering::Less {
        return None;
    }
    let (_, even) = divmod(&n, &two());
    if zero(&even) && !tower.iter().any(|op| *op == SEMIPRIME_DESCENT) {
        return Some(two());
    }
    let mut a_seed = isqrt(&n);
    if cmp(&mul(&a_seed, &a_seed), &n) == core::cmp::Ordering::Less {
        a_seed = add(&a_seed, &one());
    }
    let mut state = initial_carrier_state(n, phase_base, a_seed);
    let mut r = 0u64;
    loop {
        state.round = add(&state.round, &one());
        execute_nested(tower, &mut state);
        if let Some(ref selected) = state.selected {
            return Some(selected.clone());
        }
        if state.carrier_closed {
            return None;
        }
        r += 1;
        if r >= max_rounds {
            return None;
        }
    }
}

fn initial_carrier_state(n: Tape, phase_base: &[char], a_seed: Tape) -> State {
    State {
        n,
        phase_base: trim(phase_base.to_vec()),
        candidate: add(&two(), &one()),
        remainder: vec![EVALT],
        x: two(),
        y: two(),
        phase: one(),
        eml_partners: None,
        eml_phase_done: false,
        negative_support: Vec::new(),
        descent_evidence: Four::N,
        descent_request: None,
        descent_report: None,
        divisor: one(),
        a: a_seed,
        pm_a: two(),
        pm_e: two(),
        ecm_seed: two(),
        ecm_round: vec![EVALT],
        pp_base: tape_u64(3),
        lehman_k: one(),
        lehman_sweep: None,
        witness_done: false,
        prime12_support: None,
        power_done: false,
        squfof_done: false,
        squfof_cycle: SqufofCycle::default(),
        round: vec![EVALT],
        exhausted: false,
        selected: None,
        selected_by: None,
        unbraid_stack: Vec::new(),
        unbraid_pair: None,
        unbraid_p_bits: 0,
        unbraid_q_bits: 0,
        unbraid_p_bound: vec![EVALT],
        unbraid_q_bound: vec![EVALT],
        unbraid_started: false,
        unbraid_done: false,
        carrier_closed: false,
        bridge_count: 0,
    }
}

pub(crate) fn execute_descent_carrier(
    source: Tape,
    seed: Tape,
    constant: Tape,
    word: &str,
    limits: Limits,
    evidence: Four,
) -> Result<Report, String> {
    let canonical: String = word_parts(word)?.0.iter().collect();
    let tower = audit_factor_object(&FactorObject::identity(&canonical))?;
    if !tower.iter().any(|op| *op == SEMIPRIME_DESCENT) {
        return Err("carrier does not dispatch the five-stage descent".into());
    }
    // No pre-frame parity, phase-base GCD, or other factor-producing arm.
    let mut state = initial_carrier_state(source, &two(), one());
    state.descent_request = Some((seed, constant, canonical, limits, evidence));
    apply_morphism(SEMIPRIME_DESCENT, &mut state);
    state.descent_report.ok_or("descent dispatcher omitted its readout")?
}

fn apply_descent_stages(
    source: Tape,
    seed: Tape,
    constant: Tape,
    word: &str,
    limits: Limits,
    evidence: Four,
) -> Result<Report, String> {
    // The same registered carrier is audited for CLI and nested execution.
    let tower = audit_factor_object(&FactorObject::identity(word))?;
    if !tower.iter().any(|op| *op == SEMIPRIME_DESCENT) {
        return Err("carrier does not dispatch the five-stage descent".into());
    }
    if arithmetic::cmp(&source, &arithmetic::two()) == Less {
        return Err("source must be an integer at least two".to_string());
    }
    if limits.attempts == 0 || limits.steps_per_attempt == 0 || limits.total_steps == 0 {
        return Err("descent budgets must be positive".to_string());
    }
    let (glyphs, start) = word_parts(word)?;
    let mut report = Report {
        source: source.clone(),
        word: glyphs.iter().collect(),
        events: Vec::new(),
        evidence,
        attempts: 0,
        steps: 0,
        pair: None,
        reason: "budget_exhausted",
    };
    for (i, g) in glyphs[..start].iter().copied().enumerate() {
        event(
            &mut report,
            i,
            g,
            if g == '⊢' {
                "bind_source"
            } else {
                "retain_source"
            },
            "\"factor_extraction\":false".to_string(),
        );
    }
    let mut seed = arithmetic::modulo(&seed, &source);
    let mut constant = arithmetic::modulo(&constant, &source);
    for attempt in 1..=limits.attempts {
        if report.steps == limits.total_steps {
            break;
        }
        report.attempts = attempt;
        let budget = limits
            .steps_per_attempt
            .min(limits.total_steps - report.steps);
        let (collision, steps) = cyclic_split(&source, &seed, &constant, budget);
        report.steps += steps;
        let probe = collision
            .as_ref()
            .map(|(x, y)| arithmetic::gcd(difference(x, y), source.clone()));
        let strict_probe = probe.as_ref().is_some_and(|g| {
            arithmetic::cmp(g, &arithmetic::one()) == Greater && arithmetic::cmp(g, &source) == Less
        });
        let observations = match &collision {
            Some((x, y)) => format!("\"seed\":\"{}\",\"c\":\"{}\",\"steps\":{},\"computational_channel\":\"{}\",\"leakage_channel\":\"{}\",\"collision\":\"{}\",\"collision_gcd\":\"{}\",\"distinct_mod_source\":{}",
                arithmetic::dec_of(&seed), arithmetic::dec_of(&constant), steps,
                arithmetic::dec_of(x), arithmetic::dec_of(y), if strict_probe { "hidden_factor" } else { "recurrence_image" }, arithmetic::dec_of(probe.as_ref().unwrap()), x != y),
            None => format!("\"seed\":\"{}\",\"c\":\"{}\",\"steps\":{},\"collision\":null",
                arithmetic::dec_of(&seed), arithmetic::dec_of(&constant), steps),
        };
        event(&mut report, start, '∈', "cyclic_split", observations);
        if collision.is_none() {
            report.events.last_mut().unwrap().status = "unclosed";
        }
        let lifted = if strict_probe {
            Some(square_lift(&source, probe.as_ref().unwrap())?)
        } else {
            collision.clone()
        };
        let squares = lifted.as_ref().map(|(x, y)| {
            (
                arithmetic::mul_mod(x, x, &source),
                arithmetic::mul_mod(y, y, &source),
            )
        });
        let congruent = squares.as_ref().map(|(x2, y2)| x2 == y2);
        event(
            &mut report,
            start + 1,
            '⊤',
            "square_congruence",
            format!(
                "\"lifted_from_collision\":{},\"X\":{},\"Y\":{},\"square_congruence\":{},\"x_squared_mod_n\":{},\"y_squared_mod_n\":{}",
                strict_probe,
                lifted.as_ref().map(|(x,_)| format!("\"{}\"", arithmetic::dec_of(x))).unwrap_or("null".to_string()),
                lifted.as_ref().map(|(_,y)| format!("\"{}\"", arithmetic::dec_of(y))).unwrap_or("null".to_string()),
                congruent
                    .map(|v| if v { "true" } else { "false" })
                    .unwrap_or("null"),
                squares
                    .as_ref()
                    .map(|(x2, _)| format!("\"{}\"", arithmetic::dec_of(x2)))
                    .unwrap_or("null".to_string()),
                squares
                    .as_ref()
                    .map(|(_, y2)| format!("\"{}\"", arithmetic::dec_of(y2)))
                    .unwrap_or("null".to_string())
            ),
        );
        if squares.is_none() {
            report.events.last_mut().unwrap().status = "not_run";
        }
        let g = lifted
            .as_ref()
            .map(|(x, y)| arithmetic::gcd(difference(x, y), source.clone()));
        let deposit = match &g {
            Some(g)
                if congruent == Some(true)
                    && arithmetic::cmp(g, &arithmetic::one()) == Greater
                    && arithmetic::cmp(g, &source) == Less =>
            {
                Four::T
            }
            Some(_) => Four::F,
            None => Four::N,
        };
        event(
            &mut report,
            start + 2,
            '⊥',
            "gcd_severing",
            format!(
                "\"gcd\":{},\"deposit\":\"{}\",\"hidden_factor_collision\":{}",
                g.as_ref()
                    .map(|g| format!("\"{}\"", arithmetic::dec_of(g)))
                    .unwrap_or("null".to_string()),
                deposit.label(),
                deposit == Four::T
            ),
        );
        if g.is_none() {
            report.events.last_mut().unwrap().status = "not_run";
        }
        let before = report.evidence;
        report.evidence = report.evidence.join(deposit);
        let after = report.evidence;
        event(&mut report, start + 3, '⊞', "knowledge_join",
            format!("\"proposition\":\"attempt_nontrivial_factor\",\"before\":\"{}\",\"deposit\":\"{}\",\"after\":\"{}\",\"retry\":{}",
                    before.label(), deposit.label(), after.label(), deposit != Four::T));
        if deposit == Four::T {
            let g = g.expect("support requires a measured gcd");
            let (q, rem) = arithmetic::divmod(&source, &g);
            if !arithmetic::zero(&rem) {
                return Err("nonzero cofactor remainder".to_string());
            }
            let pair = verify_pair(&source, &g, &q)?;
            event(&mut report, start + 4, '∋', "verify_and_fuse",
                format!("\"p\":\"{}\",\"q\":\"{}\",\"product\":\"{}\",\"godel_product_verified\":true,\"support_polynomial_product_verified\":true,\"mu_delta_source_return\":true",
                    arithmetic::dec_of(&pair.p), arithmetic::dec_of(&pair.q), arithmetic::dec_of(&pair.product)));
            report.pair = Some(pair);
            report.reason = "verified";
            break;
        }
        constant = arithmetic::modulo(&arithmetic::add(&constant, &arithmetic::one()), &source);
        if arithmetic::zero(&constant) {
            seed = arithmetic::modulo(&arithmetic::add(&seed, &arithmetic::one()), &source);
        }
    }
    if report.pair.is_some() {
        let mut forwarded = report.pair.clone();
        let mut linked = None;
        let mut latch: Option<(Pair, Four)> = None;
        for (i, g) in glyphs.iter().copied().enumerate().skip(start + 5) {
            let operation = match g {
                '≻' => "advance_verified_pair",
                '⋈' => "link_product_witness",
                '≺' => "return_source",
                '⊡' => "latch_pair",
                '⊣' => "release_pair",
                _ => unreachable!(),
            };
            match g {
                '≻' => forwarded = report.pair.clone(),
                '⋈' => {
                    let pair = forwarded.as_ref().unwrap();
                    linked = Some((
                        encode_cell_binary(&natural(&pair.p)),
                        encode_cell_binary(&natural(&pair.q)),
                    ));
                }
                '≺' => {
                    let pair = forwarded.as_ref().unwrap();
                    verify_pair(&source, &pair.p, &pair.q)?;
                }
                '⊡' => latch = Some((forwarded.as_ref().unwrap().clone(), report.evidence)),
                '⊣' => {
                    let pair = latch
                        .as_ref()
                        .map(|(p, _)| p)
                        .unwrap_or_else(|| forwarded.as_ref().unwrap());
                    verify_pair(&source, &pair.p, &pair.q)?;
                }
                _ => unreachable!(),
            }
            event(
                &mut report,
                i,
                g,
                operation,
                format!(
                    "\"factor_extraction\":false,\"source_return\":{},\"linked\":{},\"latched\":{}",
                    forwarded.as_ref().unwrap().product == source,
                    linked.is_some(),
                    latch.is_some()
                ),
            );
        }
    }
    Ok(report)
}

fn apply_morphism(operator: &[char], state: &mut State) {
    if operator == SIEVE_SOURCE {
        if state.selected.is_none() {
            if let Some(returned) = source_divisor(&state.n) {
                state.selected = Some(returned.divisor);
                state.selected_by = Some(returned.producer);
            }
        }
        return;
    }
    // Dispatch is read from the operator word itself. Each operator therefore
    // remains both the boundary and the action performed at that boundary.
    if operator == SEMIPRIME_DESCENT {
        if let Some((seed, constant, word, limits, evidence)) = state.descent_request.take() {
            state.descent_report = Some(apply_descent_stages(
                state.n.clone(), seed, constant, &word, limits, evidence,
            ));
            return;
        }
        match apply_descent_stages(
            state.n.clone(), state.x.clone(), state.phase.clone(),
            crate::semiprime_descent::DESCENT,
            Limits { attempts: 1, steps_per_attempt: 100_000, total_steps: 100_000 },
            state.descent_evidence,
        ) {
            Ok(report) => {
                state.descent_evidence = report.evidence;
                if let Some(pair) = report.pair {
                    state.divisor = pair.p.clone();
                    state.selected = Some(pair.p);
                } else {
                    state.phase = modulo(&add(&state.phase, &one()), &state.n);
                    if zero(&state.phase) {
                        state.x = modulo(&add(&state.x, &one()), &state.n);
                    }
                }
            }
            Err(_) => state.carrier_closed = true,
        }
    } else if operator == PHASE_EML {
        apply_morphism(PHASE, state);
        if state.selected.is_none() {
            apply_morphism(EML_FRAME, state);
        }
        if state.selected.is_none() {
            apply_morphism(EML_EXP_MINUS_LOG, state);
            apply_morphism(EML_EXP_DIV_LOG, state);
            apply_morphism(EML_REVERSE_NEGATIVE, state);
        }
        if state.selected.is_none()
            && (state.eml_phase_done || state.negative_support.len() >= 8)
            && state.bridge_count < 12
        {
            apply_morphism(STRUCTURAL_BRIDGE, state);
        }
    } else if operator == STRUCTURAL_BRIDGE {
        // The bridge word closes to N by design. Its computational action is
        // to carry the banked negative support into one new phase-base frame.
        // Pairwise differences are the comultiplicative read: a residue
        // collision modulo either hidden factor yields a proper gcd without
        // enumerating candidate divisors.
        for left in 0..state.negative_support.len() {
            for right in (left + 1)..state.negative_support.len() {
                let g = gcd(
                    abs_diff(
                        &state.negative_support[left].1,
                        &state.negative_support[right].1,
                    ),
                    state.n.clone(),
                );
                if cmp(&g, &one()) == core::cmp::Ordering::Greater
                    && cmp(&g, &state.n) == core::cmp::Ordering::Less
                {
                    state.selected = Some(trim(g));
                    return;
                }
            }
        }
        state.bridge_count += 1;
        state.phase_base = add(&state.phase_base, &one());
        state.eml_partners = None;
        state.eml_phase_done = false;
    } else if operator == EML_FRAME {
        // One EML firing advances one phase observation for each bit in both
        // factor registers before the deeper carrier arms execute. The sweep
        // width is derived from the encoded source and has no fixed cap.
        for _ in 0..(state.n.len() * 2) {
            advance_eml_phase(state);
            if state.eml_phase_done || state.selected.is_some() {
                break;
            }
        }
        // Reassemble the adjacent joint states exactly before passing the
        // numeral to the next morphism in the tower.
        state.n = state
            .n
            .chunks(2)
            .flat_map(|joint_state| joint_state.iter().copied())
            .collect();
    } else if operator == PHASE {
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
    } else if operator == EXTRACT || operator == EXTRACT_BANKED {
        // Instant extract: one frame forks three arms and fuses on the first to
        // close, the two-arm converge circuit plus trial.
        // Arm 1, the square frontier (Fermat): a walks up from ceil(sqrt N);
        // when a*a - N is a perfect square b*b, the pair is (a-b, a+b). This
        // closes a balanced semiprime in steps set by the gap, not by sqrt(p),
        // so it gets past the rho ceiling for factors near the root.
        {
            // Once the complete negative phase aperture is banked, the
            // modulus has already failed the short near-root support read.
            // Preserve the arm, but stop paying its full-width square-root
            // cost on every round; the multiplicative arms remain live.
            if state.negative_support.len() < 8 {
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
        if state.prime12_support.is_none() {
            state.prime12_support = Some(prime12_support(&state.n));
        }
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
        // Retain the sweep between firings so a wide source cannot hold the
        // nested continuation inside a single multiplier's entire window.
        let sweep = state.lehman_sweep
            .get_or_insert_with(|| lehman_sweep(&state.n, &state.lehman_k));
        if let Some(g) = lehman_resume(&state.n, sweep, 16) {
            state.selected = Some(g);
        }
        if cmp(&sweep.a, &sweep.limit) == core::cmp::Ordering::Greater {
            state.lehman_sweep = None;
            state.lehman_k = add(&state.lehman_k, &one());
        }
    } else if operator == SQUFOF {
        // Heavy fallback: its cycle bound grows like N^(1/4), so it is nested
        // deep in time. Begin after the cheap arms have had a few hundred
        // rounds, then retain each cycle across bounded firings so the later
        // stages continue to execute while the square forms are searched.
        if !state.squfof_done && tape_to_u64(&state.round) >= 256 {
            if let Some(g) = squfof_resume(&state.n, &mut state.squfof_cycle, 16) {
                state.selected = Some(g);
            }
            state.squfof_done = state.squfof_cycle.done();
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
    } else if operator == UNBRAID {
        if state.unbraid_done {
            return;
        }
        if !state.unbraid_started {
            state.unbraid_started = true;
            let total_bits = trim(state.n.clone()).len();
            state.unbraid_p_bits = (total_bits + 1) / 2;
            state.unbraid_q_bits = total_bits + 1 - state.unbraid_p_bits;
            // Seed: p0 = q0 = 1 (both odd, since N is odd -- the caller already
            // peeled the even case before entering the tower).
            state.unbraid_stack.push((vec![EVALF], vec![EVALF], vec![EVALT], 1));
        }
        let Some((p, q, carry, position)) = state.unbraid_stack.pop() else {
            // This bit-width split failed. Close the outer carrier so the
            // caller can distinguish exhausted support from a live re-entry.
            state.unbraid_done = true;
            state.exhausted = true;
            state.carrier_closed = true;
            return;
        };
        let p_bits = state.unbraid_p_bits;
        let q_bits = state.unbraid_q_bits;
        let p_bound = &state.unbraid_p_bound;
        let q_bound = &state.unbraid_q_bound;
        if p.len() == p_bits && q.len() == q_bits {
            let prod = trim(mul(&p, &q));
            if prod == trim(state.n.clone()) {
                state.unbraid_done = true;
                state.unbraid_pair = Some((trim(p), trim(q)));
                state.selected = state.unbraid_pair.as_ref().map(|(left, _)| left.clone());
            }
            return;
        }
        let p_open = p.len() < p_bits;
        let q_open = q.len() < q_bits;
        // c's parity fixes p_bit XOR q_bit; the two candidates at odd parity
        // (p_bit,q_bit) = (0,1) and (1,0) are related by exchanging the two
        // factors' identities for every bit still to come, since N=p*q is
        // symmetric in p and q. That exchange is only a real symmetry of the
        // FUTURE search when the tapes-so-far being exchanged are themselves
        // identical -- swapping labels only revisits an already-reachable
        // solution when there is nothing yet to distinguish which label is
        // which. The instant p and q diverge (any earlier bit unequal), the
        // "mirror" branch is no longer a relabelling of the surviving one; it
        // is a distinct, uncovered region of the search, and skipping it
        // loses real solutions. So the skip applies only at the genuine
        // criticality point p==q (true only while both still read [1,1,...]
        // from the shared seed), never merely because the two targets share a
        // bit length.
        let symmetric_split = p_bits == q_bits && p == q;
        let mut pushed_swap_pair = false;
        for &pb in &[0u8, 1u8] {
            if (!p_open && pb == 1) || (p_open && p.len() + 1 == p_bits && pb == 0) {
                continue;
            }
            let mut new_p = p.clone();
            if p_open {
                new_p.push(if pb == 1 { EVALF } else { EVALT });
            }
            let mut coefficient_without_q_bit = carry.clone();
            for i in 0..=position {
                let j = position - i;
                if i < new_p.len()
                    && j < q.len()
                    && new_p[i] == EVALF
                    && q[j] == EVALF
                {
                    coefficient_without_q_bit = add(&coefficient_without_q_bit, &one());
                }
            }
            let q_bit_is_constrained = q_open && p.first() == Some(&EVALF);
            let target_bit = state.n.get(position).copied().unwrap_or(EVALT) == EVALF;
            let required_q_bit = if q_bit_is_constrained {
                Some(u8::from(target_bit
                    ^ (coefficient_without_q_bit.first().copied() == Some(EVALF))))
            } else {
                None
            };
            let q_choices: &[u8] = match required_q_bit {
                Some(0) => &[0],
                Some(1) => &[1],
                _ => &[0, 1],
            };
            for &qb in q_choices {
                if (!q_open && qb == 1) || (q_open && q.len() + 1 == q_bits && qb == 0) {
                    continue;
                }
                if symmetric_split && pb != qb && pushed_swap_pair {
                    // The mirror of the (pb,qb) pair already pushed this round.
                    continue;
                }
                let mut new_q = q.clone();
                if q_open {
                    new_q.push(if qb == 1 { EVALF } else { EVALT });
                }
                let mut coefficient = carry.clone();
                for i in 0..=position {
                    if i < new_p.len()
                        && position - i < new_q.len()
                        && new_p[i] == EVALF
                        && new_q[position - i] == EVALF
                    {
                        coefficient = add(&coefficient, &one());
                    }
                }
                let (next_carry, output_bit) = divmod(&coefficient, &two());
                let source_bit = state.n.get(position).copied().unwrap_or(EVALT);
                if output_bit.first().copied().unwrap_or(EVALT) == source_bit
                    && cmp(&new_p, p_bound) != core::cmp::Ordering::Greater
                    && cmp(&new_q, q_bound) != core::cmp::Ordering::Greater
                {
                    state.unbraid_stack.push((new_p.clone(), new_q, next_carry, position + 1));
                    if symmetric_split && pb != qb {
                        pushed_swap_pair = true;
                    }
                }
            }
        }
    } else if operator == FIX || operator == FIX_BANKED {
        if let Some(value) = state.selected.take() {
            state.selected = Some(trim(value));
        }
    }
}

/// Separate a semiprime by the IMASM bit-carry unfolding alone.
/// Width pairs cover every pair of nontrivial factor bitlengths whose product
/// can have the source width. Each branch extends the LSB-first lanes and is
/// retained only when its low product frame matches the corresponding source
/// frame. No divisor oracle participates in this path.
fn unbraid_shift_right(tape: &[char], places: usize) -> Tape {
    if places >= tape.len() {
        return vec![EVALT];
    }
    trim(tape[places..].to_vec())
}

/// Search a factor-width pair from its leading frames downward. Each partial
/// pair denotes an interval of possible factors; multiplying the interval
/// endpoints gives an exact product interval, so frames that cannot contain
/// N are discarded before lower bits are opened.
fn range_unbraid_width(n: &[char], p_bits: usize, q_bits: usize) -> Option<(Tape, Tape)> {
    let mut p_high = vec![EVALT; p_bits];
    p_high[p_bits - 1] = EVALF;
    let mut q_high = vec![EVALT; q_bits];
    q_high[q_bits - 1] = EVALF;
    let mut stack = vec![(trim(p_high), trim(q_high), p_bits.max(q_bits) as isize - 2)];

    while let Some((p, q, pos)) = stack.pop() {
        if pos < 0 {
            if trim(mul(&p, &q)) == n {
                return Some((trim(p), trim(q)));
            }
            continue;
        }

        let k = pos as usize;
        let span = vec![EVALF; k + 1];
        let p_max = add(&p, &span);
        let q_max = add(&q, &span);
        let low = mul(&p, &q);
        let high = mul(&p_max, &q_max);
        if cmp(n, &low) == core::cmp::Ordering::Less
            || cmp(n, &high) == core::cmp::Ordering::Greater
        {
            continue;
        }

        let p_free = k > 0 && k < p_bits.saturating_sub(1);
        let q_free = k > 0 && k < q_bits.saturating_sub(1);
        let p_choices: &[bool] = if p_free { &[false, true] } else if k == 0 { &[true] } else { &[false] };
        let q_choices: &[bool] = if q_free { &[false, true] } else if k == 0 { &[true] } else { &[false] };

        // Stack order preserves the zero-first traversal used by the G-MO
        // fold. The value tapes grow only when a set bit is selected.
        for &p_bit in p_choices.iter().rev() {
            let next_p = if p_bit {
                let mut bit = vec![EVALT; k];
                bit.push(EVALF);
                add(&p, &bit)
            } else {
                p.clone()
            };
            for &q_bit in q_choices.iter().rev() {
                let next_q = if q_bit {
                    let mut bit = vec![EVALT; k];
                    bit.push(EVALF);
                    add(&q, &bit)
                } else {
                    q.clone()
                };
                stack.push((next_p.clone(), next_q, pos - 1));
            }
        }
    }
    None
}

pub fn unbraid_semiprime(word: &str) -> Result<(String, String), String> {
    let n = parse_numeral(word)?;
    let n = trim(n);
    let total_bits = n.len();
    if total_bits < 3 {
        return Err("semiprime needs two nontrivial factor lanes".into());
    }

    // The compiler readout fixes the admissible width sums before extraction:
    // the two factor widths sum to either the source width or source width + 1.
    // Enumerate those dynamic tape widths from balanced outward, then close
    // each candidate through the LSB-first product carries.
    let mut width_pairs = Vec::new();
    for width_sum in [total_bits, total_bits + 1] {
        for p_bits in 2..=width_sum / 2 {
            if width_sum > p_bits {
                let q_bits = width_sum - p_bits;
                if (2..total_bits).contains(&q_bits) && p_bits <= q_bits {
                    width_pairs.push((p_bits, q_bits));
                }
            }
        }
    }
    width_pairs.sort_by_key(|(p_bits, q_bits)| p_bits.abs_diff(*q_bits));
    for (p_bits, q_bits) in width_pairs {
            if let Some((p, q)) = range_unbraid_width(&n, p_bits, q_bits) {
                if cmp(&p, &one()) == core::cmp::Ordering::Greater
                    && cmp(&q, &one()) == core::cmp::Ordering::Greater
                    && trim(mul(&p, &q)) == n
                {
                    return Ok((emit_numeral(&p), emit_numeral(&q)));
                }
            }
            let mut state = State {
                n: n.clone(),
                phase_base: two(),
                candidate: add(&two(), &one()),
                remainder: vec![EVALT],
                x: two(),
                y: two(),
                phase: one(),
                eml_partners: None,
                eml_phase_done: false,
                negative_support: Vec::new(),
                descent_evidence: Four::N,
                descent_request: None,
                descent_report: None,
                divisor: one(),
                a: two(),
                pm_a: two(),
                pm_e: two(),
                ecm_seed: two(),
                ecm_round: vec![EVALT],
                pp_base: tape_u64(3),
                lehman_k: one(),
                lehman_sweep: None,
                witness_done: false,
                prime12_support: None,
                power_done: false,
                squfof_done: false,
                squfof_cycle: SqufofCycle::default(),
                round: vec![EVALT],
                exhausted: false,
                selected: None,
        selected_by: None,
                unbraid_stack: Vec::new(),
                unbraid_pair: None,
                unbraid_p_bits: p_bits,
                unbraid_q_bits: q_bits,
                unbraid_p_bound: unbraid_shift_right(&n, q_bits - 1),
                unbraid_q_bound: unbraid_shift_right(&n, p_bits - 1),
                unbraid_started: true,
                unbraid_done: false,
                carrier_closed: false,
                bridge_count: 0,
            };

            if bit(state.n[0])? {
                state.unbraid_stack.push((vec![EVALF], vec![EVALF], vec![EVALT], 1));
            } else {
                state.unbraid_stack.push((vec![EVALF], vec![EVALT], vec![EVALT], 1));
                state.unbraid_stack.push((vec![EVALT], vec![EVALF], vec![EVALT], 1));
            }

            while !state.unbraid_done {
                apply_morphism(UNBRAID, &mut state);
            }
            if let Some((p, q)) = state.unbraid_pair {
                if cmp(&p, &one()) == core::cmp::Ordering::Greater
                    && cmp(&q, &one()) == core::cmp::Ordering::Greater
                    && trim(mul(&p, &q)) == n
                {
                    return Ok((emit_numeral(&p), emit_numeral(&q)));
                }
            }
    }
    Err("no nontrivial exact bit-carry lane pair closes to the source".into())
}

fn execute_nested(operators: &[&[char]], state: &mut State) {
    execute_nested_with_source_port(operators, state, &mut |n| Ok(source_divisor(n)))
        .expect("default source port cannot return a transport error");
}

fn execute_nested_with_source_port(operators: &[&[char]], state: &mut State,
    port: &mut impl FnMut(&[char]) -> Result<Option<SourceDivisor>,String>) -> Result<(),String> {
    #[cfg(feature = "carrier-profile")]
    {
        use core::sync::atomic::Ordering::Relaxed;
        VOX_FACTOR_COUNTERS[0].store(state.n.len() as u64, Relaxed);
        VOX_FACTOR_COUNTERS[1].store(tape_to_u64(&state.round), Relaxed);
    }
    let source = state.n.clone();
    execute_nested_preserving_source(operators, state, &source, port)
}

// Check the source register returned by each executed stage. The working
// registers may evolve, but every stage must retain the same bound source.
// This also applies to a single-stage descent without imposing a tower size.
fn execute_nested_preserving_source(operators: &[&[char]], state: &mut State, source: &[char],
    port: &mut impl FnMut(&[char]) -> Result<Option<SourceDivisor>,String>) -> Result<(),String> {
    if state.carrier_closed {
        return Ok(());
    }
    if let Some((operator, continuation)) = operators.split_first() {
        // A selected arm stops further search, but its enclosing fixation
        // boundaries still have to execute before the carrier can return.
        if state.selected.is_some() && *operator != FIX && *operator != FIX_BANKED {
            return execute_nested_preserving_source(continuation,state,source,port);
        }
        #[cfg(feature = "carrier-profile")]
        {
            use core::sync::atomic::Ordering::Relaxed;
            let stage = [WITNESS, POWER, EXTRACT, SQUFOF, P_MINUS, P_PLUS, LEHMAN, ECM, FIX, SIEVE_SOURCE]
                .iter().position(|candidate| candidate == operator).map_or(0, |i| i + 1);
            VOX_FACTOR_COUNTERS[2].store(stage as u64, Relaxed);
            VOX_FACTOR_COUNTERS[3].fetch_add(1, Relaxed);
            VOX_FACTOR_COUNTERS[4].store(1, Relaxed);
        }
        let was_selected = state.selected.is_some();
        if *operator == SIEVE_SOURCE {
            if let Some(returned) = port(&state.n)? {
                if cmp(&returned.divisor,&one()).is_gt() && cmp(&returned.divisor,&state.n).is_lt()
                    && zero(&divmod(&state.n,&returned.divisor).1) {
                    state.selected = Some(returned.divisor);
                    state.selected_by = Some(returned.producer);
                }
            }
        } else { apply_morphism(operator, state); }
        if !was_selected && state.selected.is_some() && state.selected_by.is_none() {
            state.selected_by = Some(morphism_name(operator));
        }
        #[cfg(feature = "carrier-profile")]
        {
            use core::sync::atomic::Ordering::Relaxed;
            VOX_FACTOR_COUNTERS[4].store(2, Relaxed);
            VOX_FACTOR_COUNTERS[5].store(state.selected.as_ref().map_or(0, |v| v.len()) as u64, Relaxed);
            VOX_FACTOR_COUNTERS[6].store(u64::from(state.n == source), Relaxed);
            VOX_FACTOR_COUNTERS[7].store(state.squfof_cycle.multiplier as u64, Relaxed);
            VOX_FACTOR_COUNTERS[8].store(state.squfof_cycle.frame.as_ref().map_or(0, |f| tape_to_u64(&f.steps)), Relaxed);
        }
        if state.n != source {
            state.selected = None;
            state.carrier_closed = true;
            return Ok(());
        }
        execute_nested_preserving_source(continuation, state, source, port)?;
        if state.n != source {
            state.selected = None;
            state.carrier_closed = true;
        }
    }
    Ok(())
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
        phase_base: two(),
        candidate: add(&two(), &one()),
        remainder: vec![EVALT],
        x: two(),
        y: two(),
        phase: one(),
        eml_partners: None,
        eml_phase_done: false,
        negative_support: Vec::new(),
        divisor: one(),
        descent_evidence: Four::N,
        a: a_seed,
        descent_request: None,
        descent_report: None,
        pm_a: two(),
        pm_e: two(),
        ecm_seed: two(),
        ecm_round: vec![EVALT],
        pp_base: tape_u64(3),
        lehman_k: one(),
        lehman_sweep: None,
        witness_done: false,
        prime12_support: None,
        power_done: false,
        squfof_done: false,
        squfof_cycle: SqufofCycle::default(),
        round: vec![EVALT],
        exhausted: false,
        selected: None,
        selected_by: None,
        unbraid_stack: Vec::new(),
        unbraid_pair: None,
        unbraid_p_bits: 0,
        unbraid_q_bits: 0,
        unbraid_p_bound: vec![EVALT],
        unbraid_q_bound: vec![EVALT],
        unbraid_started: false,
        unbraid_done: false,
        carrier_closed: false,
        bridge_count: 0,
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
    // Lift-proven emission: scalar .rev() walk, never an in-place reverse.
    // The in-place digits.reverse() autovectorizes into punpcklbw/pshuflw
    // once the decimal string grows and mis-decodes in the x86 lift — the
    // hazard gpu_shor_one.rs contracts away. Same digits, same string.
    digits
        .into_iter()
        .rev()
        .map(|d| d as char)
        .collect::<String>()
}

/// Shape scout: cheap probes, cheapest first, each of which reads one shape of N
/// and, when it hits, hands the factor. The first hit both names the shape and
/// gives the pair, so the width-heavy arms never run for a shape they do not
/// fit. Returns the pair with the shape name, and a log of what each probe saw.
/// A None with "prime" is a certified prime; a None with "HARD" is the
/// random-equal-size-far-apart shape that wants the sub-exponential tier.
pub fn scout_factor(n_in: &[char]) -> (Option<(Tape, Tape, &'static str)>, String) {
    scout_factor_with_primality(n_in, true)
}

/// Fast route for a baked semiprime input. The caller supplies the semiprime
/// shape, so the scout proceeds directly to its factor frontier instead of
/// spending a modular witness pass proving the input composite.
pub fn scout_semiprime(n_in: &[char]) -> (Option<(Tape, Tape, &'static str)>, String) {
    scout_factor_with_primality(n_in, false)
}

/// Keep the cheap Fermat frontier's work fixed as source width grows. Wider
/// balanced inputs move promptly to the sub-exponential sieve instead of
/// multiplying a low-yield square-gap scan at every increase in bit width.
const NEAR_ROOT_FRONTIER_STEPS: u64 = 16_384;

/// Scout a composite already rejected by the caller's primality frame.
/// Keeping the witness pass at the membrane boundary avoids running the same
/// modular exponentiation twice on every composite input.
fn scout_factor_with_primality(
    n_in: &[char],
    test_primality: bool,
) -> (Option<(Tape, Tape, &'static str)>, String) {
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
    if test_primality && miller_rabin(&n) {
        return (None, "shape: prime (witness, no factor to find)\n".into());
    }
    // short frontier first: closes at once iff the factors sit near the root, so
    // a near-root N never pays the width-heavy rho below.
    // The near-root probe has a fixed work budget; wider residual shapes belong
    // to the folded sieve rather than an exponentially growing Fermat scan.
    {
        let mut a = isqrt(&n);
        if cmp(&mul(&a, &a), &n) == Less {
            a = add(&a, &one());
        }
        let bound = NEAR_ROOT_FRONTIER_STEPS;
        let mut i = 0u64;
        while i < bound {
            let a2 = mul(&a, &a);
            if cmp(&a2, &n) != Less {
                let dl = sub(&a2, &n);
                if let Some(bb) = is_square(&dl) {
                    let p = sub(&a, &bb);
                    if cmp(&p, &one()) == Greater {
                        let q = divmod(&n, &p).0;
                        return (
                            Some((p.clone(), q, "frontier")),
                            format!("shape: near-root, closed at frontier step {}\n", i),
                        );
                    }
                }
            }
            a = add(&a, &one());
            i += 1;
        }
    }

    // Perfect-power splitting is the fallback after the semiprime frontier.
    // Testing every exponent first repeats expensive root searches on the
    // balanced semiprimes the fast route is designed to close immediately.
    let bits = n.len();
    let mut b = 2usize;
    while b <= bits.min(16) {
        let r = iroot(&n, b);
        if cmp(&r, &one()) == Greater && cmp(&ipow(&r, b), &n) == Equal {
            let q = divmod(&n, &r).0;
            return (
                Some((r.clone(), q, "perfect-power")),
                format!("shape: perfect power, base {}\n", dec_of(&r)),
            );
        }
        b += 1;
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
pub const HARD_CARRIER_ROUNDS: u64 = u64::MAX;

/// The full nine-arm carrier word, the deepest routing in one string.
pub const NINE_ARM: &str = "⊢∈⊤≺⊥∈⊤⊞⊥∈≻⊤≺⊥⊞⋈∈⊤≺⊞⊥∈⊙⊞⋈∈⊙≺⋈∈≻⋈⊤⊥∈⊙≻⋈∋∋∋∋∋∋∋∋⊙⊡⊣";

/// Nest the source square/correlation morphism inside the existing carrier,
/// before its enclosing fuses. This word binds its executable source port.
pub fn source_carrier_word() -> String {
    let mut word = String::new();
    let mut inserted = false;
    for mark in NINE_ARM.chars() {
        if mark == FFUSE && !inserted {
            word.extend(SIEVE_SOURCE[1..SIEVE_SOURCE.len() - 1].iter());
            inserted = true;
        }
        word.push(mark);
    }
    word
}

/// N alone enters one carrier register; the actual latching arm is retained.
pub fn source_carrier_divisor(n_in: &[char]) -> Result<Option<SourceDivisor>, String> {
    source_carrier_divisor_with_port(n_in, |n| Ok(source_divisor(n)))
}

pub fn source_carrier_divisor_with_port(n_in: &[char],
    mut port: impl FnMut(&[char]) -> Result<Option<SourceDivisor>,String>)
    -> Result<Option<SourceDivisor>,String> {
    let n = trim(n_in.to_vec());
    if cmp(&n, &two()).is_lt() { return Ok(None); }
    if zero(&divmod(&n, &two()).1) && cmp(&n, &two()).is_gt() {
        return Ok(Some(SourceDivisor { divisor: two(), producer: "source-parity" }));
    }
    let tower = construct_carrier(&source_carrier_word())?;
    let mut root = isqrt(&n);
    if cmp(&mul(&root, &root), &n).is_lt() { root = add(&root, &one()); }
    let mut state = initial_carrier_state(n.clone(), &two(), root);
    loop {
        state.round = add(&state.round, &one());
        execute_nested_with_source_port(&tower, &mut state, &mut port)?;
        if let Some(divisor) = state.selected.take() {
            if cmp(&divisor, &one()).is_gt() && cmp(&divisor, &n).is_lt()
                && zero(&divmod(&n, &divisor).1) {
                return Ok(Some(SourceDivisor { divisor, producer: state.selected_by.unwrap_or("carrier-source") }));
            }
            return Ok(None);
        }
        if state.carrier_closed { return Ok(None); }
    }
}

pub struct SourceDivisor {
    pub divisor: Tape,
    pub producer: &'static str,
}

/// One N-only source split using the same hard arms as smart_factor.
/// The quotient is derived only after a proper divisor has been produced.
pub fn source_divisor(n_in: &[char]) -> Option<SourceDivisor> {
    source_divisor_observed(n_in, |_| {})
}

pub fn source_divisor_observed(n_in: &[char], mut observe: impl FnMut(&'static str))
    -> Option<SourceDivisor> {
    source_divisor_observed_with_return(n_in, &mut observe, || None)
}

pub fn source_divisor_observed_with_return(n_in: &[char], mut observe: impl FnMut(&'static str),
    mut resident_return: impl FnMut() -> Option<SourceDivisor>) -> Option<SourceDivisor> {
    let n = trim(n_in.to_vec());
    observe("source-primality");
    if cmp(&n, &one()) != core::cmp::Ordering::Greater || miller_rabin(&n) {
        return None;
    }
    observe("source-scout");
    if let (Some((p, _, _)), _) = scout_factor_with_primality(&n, false) {
        if cmp(&p, &one()) == core::cmp::Ordering::Greater
            && cmp(&p, &n) == core::cmp::Ordering::Less && zero(&divmod(&n, &p).1) {
            return Some(SourceDivisor { divisor: p, producer: "source-scout" });
        }
    }
    hard_source_divisor_with_return(&n, &mut observe, &mut resident_return)
}

fn hard_source_divisor(n: &Tape) -> Option<SourceDivisor> {
    hard_source_divisor_observed(n, |_| {})
}

fn hard_source_divisor_observed(n: &Tape, mut observe: impl FnMut(&'static str)) -> Option<SourceDivisor> {
    hard_source_divisor_with_return(n, &mut observe, || None)
}

fn hard_source_divisor_with_return(n: &Tape, mut observe: impl FnMut(&'static str),
    mut resident_return: impl FnMut() -> Option<SourceDivisor>) -> Option<SourceDivisor> {
    let good = |p: &Tape| cmp(p, &one()) == core::cmp::Ordering::Greater
        && cmp(p, n) == core::cmp::Ordering::Less && zero(&divmod(n, p).1);
    let (bound, m) = crate::sieve::sieve_params(n);
    let mut dialectic = crate::dialectic_reentry::DialecticObject::new(n.clone()).ok();
    let mut producer = "mpqs-source";
    observe("mpqs-source");
    if let Some((p, arm)) = crate::sieve::mpqs_with_source_port(n, bound, 32_768, 64, || {
        if let Some(returned) = resident_return() {
            if good(&returned.divisor) {
                producer = returned.producer;
                return Some(returned.divisor);
            }
        }
        let object = dialectic.take()?;
        observe("dialectic-source-return");
        match object.descend() {
            crate::dialectic_reentry::Descent::T(closed) => {
                let p = closed.carrier.p;
                if good(&p) && mul(&p, &closed.carrier.q) == *n {
                    producer = "dialectic-source";
                    return Some(p);
                }
            },
            crate::dialectic_reentry::Descent::B(next)
                | crate::dialectic_reentry::Descent::N(next) => dialectic = Some(next),
            crate::dialectic_reentry::Descent::F => {},
        }
        None
    }).filter(|(p, _)| good(p)) {
        if arm != "retained-source-port" { producer = arm; }
        return Some(SourceDivisor { divisor: p, producer });
    }
    observe("qs-source");
    if let Some(p) = crate::sieve::qs(n, bound, m, 64).filter(&good) {
        return Some(SourceDivisor { divisor: p, producer: "qs-source" });
    }
    observe("nine-arm-source");
    let tower = construct_carrier(NINE_ARM).expect("NINE_ARM is a valid carrier");
    let tower_refs: Vec<&[char]> = tower.iter().map(|t| *t).collect();
    if let Some(p) = run_carrier_rounds(&tower_refs, n, HARD_CARRIER_ROUNDS).filter(&good) {
        return Some(SourceDivisor { divisor: p, producer: "nine-arm-source" });
    }
    observe("dixon-source");
    crate::sieve::dixon(n, bound, 8, u64::MAX).filter(&good)
        .map(|divisor| SourceDivisor { divisor, producer: "dixon-source" })
}

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
        let (res, l) = scout_factor_with_primality(&c, false);
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
                // MPQS is the primary hard arm: a fresh polynomial per step keeps
                // the values small, so it is faster than the single polynomial
                // everywhere they overlap and reaches to the machine-integer width.
                // Single-poly QS is the fallback for the narrow N MPQS declines,
                // then the carrier's rho, then Dixon.
                // MPQS sizes its polynomial window by source width. The much
                // wider single-polynomial QS interval is only for the fallback.
                let hit = hard_source_divisor(&c).map(|result| result.divisor);
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
        Some((p, q, shape)) => format!(
            "N={n}\n{log}  {n} = {} x {}  [{shape}]",
            dec_of(&p),
            dec_of(&q)
        ),
        None => format!("N={n}\n{log}"),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn source_sieve_is_nested_and_a_cheap_closure_skips_it() {
        let word = source_carrier_word();
        let marks: Vec<char> = word.chars().collect();
        let (_, open, overfused) = crate::vox::pairing(&marks);
        assert!(open.is_empty() && overfused.is_empty());
        let tower = construct_carrier(&word).unwrap();
        let names: Vec<_> = tower.iter().map(|operator| morphism_name(operator)).collect();
        assert_eq!(names[names.len() - 2..], ["SIEVE_SOURCE", "FIX"]);
        let source = tape_u64(91);
        let result = source_carrier_divisor(&source).unwrap().unwrap();
        assert_eq!(result.producer, "EXTRACT");
        assert_eq!(result.divisor, tape_u64(7));
        assert!(zero(&divmod(&source, &result.divisor).1));
    }

    use super::*;

    #[test]
    fn descent_carrier_dispatches_standalone_and_sealed_words() {
        for word in [crate::semiprime_descent::DESCENT, crate::semiprime_descent::PROTOCOL, " ∈⊤⊥⊞∋ "] {
            let canonical: String = word.chars().filter(|g| !g.is_whitespace()).collect();
            let tower = audit_factor_object(&FactorObject::identity(&canonical)).unwrap();
            assert_eq!(tower[0], SEMIPRIME_DESCENT);
            let n = decimal_to_tape("15").unwrap();
            let report = execute_descent_carrier(
                n.clone(), one(), tape_u64(6), word,
                Limits { attempts: 1, steps_per_attempt: 1, total_steps: 1 }, Four::N,
            ).unwrap();
            let pair = report.pair.unwrap();
            assert_eq!(mul(&pair.p, &pair.q), n);
            assert_eq!(report.evidence, Four::T);
            let extraction: Vec<char> = report.events.iter()
                .filter(|e| matches!(e.operation, "cyclic_split" | "square_congruence" | "gcd_severing" | "knowledge_join" | "verify_and_fuse"))
                .map(|e| e.symbol).collect();
            assert_eq!(extraction, SEMIPRIME_DESCENT);
        }
    }

    #[test]
    fn descent_nested_carrier_uses_verified_fusion() {
        let tower = construct_carrier(crate::semiprime_descent::DESCENT).unwrap();
        for source in ["15", "14", "49", "8051"] {
            let n = decimal_to_tape(source).unwrap();
            let g = run_carrier_rounds(&tower, &n, 64).unwrap();
            let (q, rem) = divmod(&n, &g);
            assert!(zero(&rem));
            verify_pair(&n, &g, &q).unwrap();
            let factor = factor_with(crate::semiprime_descent::DESCENT, &emit_numeral(&n)).unwrap();
            let p = parse_numeral(&factor).unwrap();
            assert!(cmp(&p, &one()) == Greater && cmp(&p, &n) == Less);
        }
        let mixed = "⊢∈⊤⊥⊞∋⊙⊡⊣";
        let tower = audit_factor_object(&FactorObject::identity(mixed)).unwrap();
        assert_eq!(tower, vec![SEMIPRIME_DESCENT, FIX]);
        let n = decimal_to_tape("8051").unwrap();
        let factor = factor_with(mixed, &emit_numeral(&n)).unwrap();
        let p = parse_numeral(&factor).unwrap();
        let (q, _) = divmod(&n, &p);
        verify_pair(&n, &p, &q).unwrap();
    }

    #[test]
    fn normalized_limb_division_closes_quotient_and_remainder() {
        let numerators = [
            vec![u64::MAX, 0, 1],
            vec![0x1234_5678_9abc_def0, u64::MAX, 0x8000_0000_0000_0001],
            vec![u64::MAX, u64::MAX, u64::MAX, 0x7fff_ffff_ffff_ffff],
        ];
        let divisors = [
            vec![3],
            vec![0xffff_ffff_ffff_ffc5],
            vec![0x8000_0000_0000_0001, 0x4000_0000_0000_0000],
            vec![0xffff_ffff_ffff_fffb, 0x7fff_ffff_ffff_ffff],
        ];
        for n in &numerators {
            for d in &divisors {
                let (q, r) = l_divmod(n, d);
                assert_eq!(l_add(&l_mul(&q, d), &r), l_trim(n.clone()));
                assert_eq!(l_cmp(&r, d), core::cmp::Ordering::Less);
            }
        }
    }

    #[test]
    fn fused_modular_products_match_tape_arithmetic_across_limb_widths() {
        for (a, b, c, n) in [
            ("123456789", "987654321", "12345", "1000000007"),
            (
                "18446744073709551617",
                "340282366920938463463374607431768211457",
                "98765432101234567890",
                "6277101735386680763835789423207666416102355444464034512895",
            ),
        ] {
            let a = decimal_to_tape(a).unwrap();
            let b = decimal_to_tape(b).unwrap();
            let c = decimal_to_tape(c).unwrap();
            let n = decimal_to_tape(n).unwrap();
            let expected = modulo(&add(&mul(&a, &b), &c), &n);
            assert_eq!(mul_mod_add(&a, &b, &c, &n), expected);
            assert_eq!(mul_mod(&a, &b, &n), modulo(&mul(&a, &b), &n));
        }
    }

    #[test]
    fn smart_factor_gives_full_multiset() {
        let (fs, _) = smart_factor(&tape_u64(360));
        let prod = fs.iter().fold(1u64, |a, f| {
            let mut v = 0u64;
            for &c in trim(f.clone()).iter().rev() {
                v = (v << 1) | if c == EVALF { 1 } else { 0 };
            }
            a * v
        });
        assert_eq!(prod, 360);
        assert_eq!(fs.len(), 6); // 2^3 * 3^2 * 5
        assert!(
            repl_smart_factor(&tape_u64(8051)).contains("83 x 97")
                || repl_smart_factor(&tape_u64(8051)).contains("97 x 83")
        );
    }

    #[test]
    fn scout_reads_the_shape_and_routes() {
        // Each shape routes to its cheapest probe; the name is the reading.
        // near-root balanced: the frontier probe closes it.
        let (r, _) = scout_factor(&tape_u64(1000003 * 1000033));
        assert_eq!(r.unwrap().2, "frontier");
        // The scout hands this shape to the membrane; rho is now fused into
        // the sieve. Verify both the handoff and the complete factorization.
        let n = tape_u64(1000003 * 1000000007);
        let (r, log) = scout_factor(&n);
        assert!(r.is_none(), "{log}");
        assert!(log.contains("HARD"), "{log}");
        let (factors, route) = smart_factor(&n);
        assert_eq!(
            factors,
            vec![tape_u64(1000003), tape_u64(1000000007)],
            "{route}"
        );
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

    #[test]
    fn prime12_read_is_information_support_not_a_primality_test() {
        for p in [5u64, 7, 11, 13, 17, 19] {
            let n = tape_u64(p);
            assert!(prime12_support(&n));
            assert!(miller_rabin(&n));
        }
        let composite = tape_u64(25);
        assert!(prime12_support(&composite));
        assert!(!miller_rabin(&composite));
        assert!(!prime12_support(&tape_u64(3)));
    }

    #[test]
    fn exhausted_nested_arm_closes_the_outer_carrier() {
        let tower: [&[char]; 1] = [UNBRAID];
        assert!(
            run_carrier_rounds_with_phase_base(&tower, &tape_u64(17), &tape_u64(2), u64::MAX,)
                .is_none()
        );
    }

    // A single operator word whose interior is the six motif interiors in
    // canonical order. The carrier constructor recovers the full tower.
    const FULL: &str = "⊢∈≻⊤⊥∋∈⋈⊤⊥∋∈⊤⊥∋∈⊙∋≻⋈⊙⊡⊣";

    #[test]
    fn constructor_recovers_the_full_tower() {
        let tower = construct_carrier(FULL).unwrap();
        let names: Vec<&str> = tower.iter().map(|t| morphism_name(t)).collect();
        assert_eq!(
            names,
            ["PHASE", "ARITHMETIC", "BRANCH", "SELECT", "CONTINUE", "FIX"]
        );
    }

    #[test]
    fn rsa_200_and_256_bit_selected_arms_execute_the_enclosing_fixations() {
        for decimal in [
            "1156514714917773145849996001252587703581994899993461612691909",
            "101560191607051872909385412079844080615251494997013823952605603214371184345809",
        ] {
            let source = decimal_to_tape(decimal).unwrap();
            assert!(source.len() >= 200);
            let payload = isqrt(&source);
            let mut padded = payload.clone();
            padded.extend([EVALT,EVALT]);
            let mut state = initial_carrier_state(source.clone(),&two(),payload.clone());
            // Inject an already-selected resident payload to isolate the
            // return path. This is a closure check, not a factoring readout.
            state.selected = Some(padded);
            let tower: [&[char];5] = [EXTRACT,FIX,PHASE,FIX_BANKED,FIX];
            execute_nested_preserving_source(&tower,&mut state,&source,&mut |_| Ok(None)).unwrap();
            assert_eq!(state.n,source);
            assert_eq!(state.selected,Some(payload));
            assert!(!state.carrier_closed);
        }
    }

    #[test]
    fn nine_arm_carrier_is_a_nested_frobenius_stack() {
        let word: Vec<char> = NINE_ARM.chars().collect();
        let (regions, unanswered_splits, unanswered_fuses) = crate::vox::pairing(&word);
        assert!(unanswered_splits.is_empty());
        assert!(unanswered_fuses.is_empty());
        assert_eq!(regions.len(), 8);
        assert!(regions.iter().all(|region| region.substantial));
        let mut boundaries: Vec<(usize, usize)> = regions
            .into_iter()
            .map(|region| (region.split, region.fuse))
            .collect();
        boundaries.sort_unstable_by_key(|(split, _)| *split);
        assert!(boundaries.windows(2).all(|pair| pair[0].1 > pair[1].1));
        assert_eq!(crate::vox::verdict(&word), 'T');

        let tower = construct_carrier(NINE_ARM).unwrap();
        audit_execution_projection(&FactorObject::identity(NINE_ARM), &tower).unwrap();
        let mut missing_stage = tower.clone();
        missing_stage.remove(3);
        assert!(audit_execution_projection(&FactorObject::identity(NINE_ARM), &missing_stage).is_err());
        let names: Vec<&str> = tower
            .iter()
            .map(|operator| morphism_name(operator))
            .collect();
        assert_eq!(
            names,
            [
                "WITNESS",
                "POWER",
                "EXTRACT",
                "SQUFOF",
                "P_MINUS",
                "P_PLUS",
                "LEHMAN",
                "ECM",
                "FIX"
            ]
        );
        let source = decimal_to_tape("8051").unwrap();
        let factor = run_carrier_rounds(&tower, &source, 64).unwrap();
        let (cofactor, remainder) = divmod(&source, &factor);
        assert!(zero(&remainder));
        verify_pair(&source, &factor, &cofactor).unwrap();
    }

    #[test]
    fn constructor_accepts_source_bound_fragments_and_retained_terminal_output() {
        let expected = construct_carrier(FULL).unwrap();
        let glyphs: Vec<char> = FULL.chars().collect();
        for (start,end) in [(0,glyphs.len()-1),(1,glyphs.len()),(1,glyphs.len()-1)] {
            let fragment: String = glyphs[start..end].iter().collect();
            assert_eq!(construct_carrier(&fragment).unwrap(),expected);
        }
        assert_eq!(construct_carrier(&format!("{FULL}⊙")).unwrap(),expected);
        assert!(construct_carrier("").unwrap().is_empty());
        assert!(construct_carrier("⊢").unwrap().is_empty());
        assert!(construct_carrier("⊣").unwrap().is_empty());
        let error = construct_carrier("⊢≻⊙∈⊤⋈⊥⊙≻⋈⊤⊥⊞≻⋈⊤≺⊞⊙⊤⊥⊞⊙⊤≻⋈⊥≺⊞≻⋈≺⊙≺⋈∋⊞⊡≺⋈⊣⊙").unwrap_err();
        assert!(error.contains("begins no factoring morphism"),"{error}");
    }

    #[test]
    fn constructor_recognizes_kernel_eml_frame_and_composes_factor_arm() {
        const EML: &str = "⊢≻≺∈⊤⊥⊞≺⊙∋⊡⊣";
        const NESTED: &str = "⊢≻≺∈⊤⊥⊞≺⊙∋⊡∈≻⊤≺⊥⊞⋈∋⊙⊡⊣";
        const EML_PHASE: &str = "⊢≻≺∈⊤⊥⊞≺⊙∋⊡∈≻⊤⊥∋∈⊙∋≻⋈⊙⊡⊣";
        const EML_FULL: &str = "⊢≻≺∈⊤⊥⊞≺⊙∋⊡∈≻⊤⊥∋∈⋈⊤⊥∋∈⊤⊥∋∈⊙∋≻⋈⊙⊡⊣";
        const EML_NINE: &str =
            "⊢∈≻⊤⊥≻≺∈⊤⊥⊞≺⊙∋⊡∋∈⊤≺⊥∋∈⊤⊞⊥∋∈≻⊤≺⊥⊞⋈∋∈⊤≺⊞⊥∋∈⊙⊞⋈∋∈⊙≺⋈∋∈≻⋈⊤⊥∋∈⊙≻⋈∋∈⊙≻⊤≺⊥⋈∋⊙⊡⊣";

        let eml = construct_carrier(EML).unwrap();
        assert_eq!(
            eml.iter().map(|op| morphism_name(op)).collect::<Vec<_>>(),
            ["EML_FRAME"]
        );

        let nested = construct_carrier(NESTED).unwrap();
        assert_eq!(
            nested
                .iter()
                .map(|op| morphism_name(op))
                .collect::<Vec<_>>(),
            ["EML_FRAME", "EXTRACT", "FIX"]
        );
        let factor = factor_with(NESTED, &numeral(8051)).unwrap();
        assert!(factor == numeral(83) || factor == numeral(97));

        let err = factor_with(EML_PHASE, &numeral(8051)).unwrap_err();
        assert!(err.contains("ARITHMETIC (remainder and gcd readout)"));

        let factors = construct_carrier(EML_FULL).unwrap();
        assert_eq!(
            factors
                .iter()
                .map(|op| morphism_name(op))
                .collect::<Vec<_>>(),
            [
                "EML_FRAME",
                "PHASE",
                "ARITHMETIC",
                "BRANCH",
                "SELECT",
                "CONTINUE",
                "FIX"
            ]
        );
        let factor = factor_with(EML_FULL, &numeral(8051)).unwrap();
        assert!(factor == numeral(83) || factor == numeral(97));

        let source_word = numeral(12_289 * 1_000_000_007);
        let source = parse_numeral(&source_word).unwrap();
        let factor_word = factor_with(EML_FULL, &source_word).unwrap();
        let factor = parse_numeral(&factor_word).unwrap();
        let p = parse_numeral(&numeral(12_289)).unwrap();
        let q = parse_numeral(&numeral(1_000_000_007)).unwrap();
        assert!(factor == p || factor == q);
        let (cofactor, remainder) = divmod(&source, &factor);
        assert!(zero(&remainder));
        assert_eq!(mul(&factor, &cofactor), source);

        let wide_p = 3_221_225_473u64;
        let wide_q = 180_143_985_094_819_841u64;
        let wide_source = decimal_to_tape("580284393595165992175009793").unwrap();
        let wide_word = emit_numeral(&wide_source);
        let phase_tower: [&[char]; 2] = [EML_FRAME, FIX];
        let phase_factor = run_carrier_rounds(&phase_tower, &wide_source, 1024).unwrap();
        let wide_left = parse_numeral(&numeral(wide_p)).unwrap();
        let wide_right = parse_numeral(&numeral(wide_q)).unwrap();
        assert!(phase_factor == wide_left || phase_factor == wide_right);
        assert_eq!(
            construct_carrier(EML_NINE)
                .unwrap()
                .iter()
                .map(|op| morphism_name(op))
                .collect::<Vec<_>>(),
            [
                "PHASE_EML",
                "WITNESS",
                "POWER",
                "EXTRACT",
                "SQUFOF",
                "P_MINUS",
                "P_PLUS",
                "LEHMAN",
                "ECM",
                "UNBRAID",
                "FIX"
            ]
        );
        let wide_factor = parse_numeral(&factor_with(EML_NINE, &wide_word).unwrap()).unwrap();
        assert!(wide_factor == wide_left || wide_factor == wide_right);
        let (wide_cofactor, wide_remainder) = divmod(&wide_source, &wide_factor);
        assert!(zero(&wide_remainder));
        assert_eq!(mul(&wide_factor, &wide_cofactor), wide_source);
    }

    #[test]
    fn repaired_structural_bridge_is_a_closure_marker() {
        let bridge = "⊢∋∈⊥⊙≻⋈⊞⊤≺⊡⊣";
        let carrier = construct_carrier(bridge).unwrap();
        assert_eq!(carrier, vec![STRUCTURAL_BRIDGE]);
        assert_eq!(morphism_name(carrier[0]), "STRUCTURAL_BRIDGE");
    }

    #[test]
    fn eml_variants_are_distinct_kernel_arms() {
        let words = [
            ("⊢⊙∈≻⊤≺⊥∋⊞⋈⊡⊣", "EML_EXP_MINUS_LOG"),
            ("⊢⊙∈≻⊤≺⊥⊞∋⋈⊡⊣", "EML_EXP_DIV_LOG"),
            ("⊢∈≻⊤≺⊥∋⊞⋈⊙⊡⊣", "EML_REVERSE_NEGATIVE"),
        ];
        for (word, expected) in words {
            let carrier = construct_carrier(word).unwrap();
            assert_eq!(carrier.len(), 1);
            assert_eq!(morphism_name(carrier[0]), expected);
        }
    }

    #[test]
    fn phase_base_is_a_baked_imasm_input_not_a_source_literal() {
        const EML_NINE: &str =
            "⊢∈≻⊤⊥≻≺∈⊤⊥⊞≺⊙∋⊡∋∈⊤≺⊥∋∈⊤⊞⊥∋∈≻⊤≺⊥⊞⋈∋∈⊤≺⊞⊥∋∈⊙⊞⋈∋∈⊙≺⋈∋∈≻⋈⊤⊥∋∈⊙≻⋈∋∈⊙≻⊤≺⊥⋈∋⊙⊡⊣";
        for base in [2, 3, 5] {
            let factor = factor_with_phase_base(EML_NINE, &numeral(8051), &numeral(base)).unwrap();
            assert!(factor == numeral(83) || factor == numeral(97));
        }
        assert_eq!(
            factor_with_phase_base(EML_NINE, &numeral(8051), &numeral(83)).unwrap(),
            numeral(83)
        );
        assert!(factor_with_phase_base(EML_NINE, &numeral(8051), &numeral(1)).is_err());
    }

    #[test]
    fn factor_object_identity_is_the_legacy_diagonal() {
        let object = FactorObject::identity(FULL);
        assert!(audit_reconciliation(&object).is_ok());
        let tower = audit_factor_object(&object).unwrap();
        assert_eq!(
            tower.iter().map(|t| morphism_name(t)).collect::<Vec<_>>(),
            ["PHASE", "ARITHMETIC", "BRANCH", "SELECT", "CONTINUE", "FIX"]
        );
        assert_eq!(
            factor_object(&object, &numeral(8051)).unwrap(),
            factor_with(FULL, &numeral(8051)).unwrap()
        );
    }

    #[test]
    fn frozen_walk_type_reconciliation_is_first_class() {
        let frame = FactorObject::new("∈∋⊤≻⊡", "∈⊤≻⊡∋", Reconciliation::FrameWork);
        assert!(audit_reconciliation(&frame).is_ok());

        let close = FactorObject::new("⊢∈≻⊤⊣", "⊢∈≻⊤∋⊣", Reconciliation::CloseFrame);
        assert!(audit_reconciliation(&close).is_ok());
    }

    #[test]
    fn reconciliation_mismatch_is_not_silently_normalized() {
        let wrong = FactorObject::new("∈∋⊤≻⊡", "∈∋⊤≻⊡", Reconciliation::FrameWork);
        let err = audit_reconciliation(&wrong).unwrap_err();
        assert!(err.contains("reconciliation mismatch"));
    }

    #[test]
    fn frame_work_object_executes_walk_projection_and_types_with_full_carrier() {
        // Same factoring work as FULL, but its first PHASE frame is shipped as
        // an adjacent walk dyad. FRAME_WORK changes frame placement only.
        let walk = "⊢∈∋≻⊤⊥∈⋈⊤⊥∋∈⊤⊥∋∈⊙∋≻⋈⊙⊡⊣";
        let object = FactorObject::new(walk, FULL, Reconciliation::FrameWork);
        let tower = audit_factor_object(&object).unwrap();
        assert!(audit_execution_projection(&object, &tower).is_ok());
        let f = factor_object(&object, &numeral(8051)).unwrap();
        assert!(f == numeral(83) || f == numeral(97));
    }

    #[test]
    fn close_frame_object_executes_same_work_after_structural_repair() {
        // Remove the first PHASE fuse from FULL: the walk is B/unclosed by one
        // frame, while the supplied type restores exactly that fuse.
        let walk = "⊢∈≻⊤⊥∈⋈⊤⊥∋∈⊤⊥∋∈⊙∋≻⋈⊙⊡⊣";
        let object = FactorObject::new(walk, FULL, Reconciliation::CloseFrame);
        let tower = audit_factor_object(&object).unwrap();
        assert!(audit_execution_projection(&object, &tower).is_ok());
        let f = factor_object(&object, &numeral(8051)).unwrap();
        assert!(f == numeral(83) || f == numeral(97));
    }

    #[test]
    fn changed_work_is_rejected_even_when_a_type_is_factoring_complete() {
        let walk = "⊢∈∋≺⊤⊥∈⋈⊤⊥∋∈⊤⊥∋∈⊙∋≻⋈⊙⊡⊣";
        let object = FactorObject::new(walk, FULL, Reconciliation::FrameWork);
        let err = audit_factor_object(&object).unwrap_err();
        assert!(err.contains("reconciliation mismatch") || err.contains("execution preservation"));
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
        assert!(err.contains("ARITHMETIC (remainder and gcd readout)"));
        assert!(err.contains("CONTINUE"));
    }

    #[test]
    fn instant_extractor_decomposes_and_factors() {
        // The instant semiprime extractor carries AREV ≺ and ENGAGR ⊞ inside
        // its evaluate frame; that frame is the EXTRACT morphism, a one-frame
        // fold of advance, decide and continue. The word decomposes to
        // EXTRACT then FIX and factors on the carrier built from itself.
        let w = "⊢∈≻⊤⋈⊙≺⊥⊞∋⊡⋈⊙⊣";
        let tower = construct_carrier(w).unwrap();
        let names: Vec<&str> = tower.iter().map(|t| morphism_name(t)).collect();
        assert_eq!(names, ["EXTRACT", "FIX"]);
        // The frontier arm may return either factor of the pair; both are valid.
        let f = factor_with(w, &numeral(8051)).unwrap();
        assert!(f == numeral(83) || f == numeral(97));
    }

    #[test]
    fn extract_banks_both_deposits_across_its_internal_arev() {
        let split = EXTRACT_BANKED
            .iter()
            .position(|&mark| mark == FSPLIT)
            .unwrap();
        let reversal = EXTRACT_BANKED
            .iter()
            .position(|&mark| mark == AREV)
            .unwrap();
        let fuse = EXTRACT_BANKED
            .iter()
            .position(|&mark| mark == FFUSE)
            .unwrap();
        assert!(split < reversal && reversal < fuse);
        assert_eq!(
            EXTRACT_BANKED
                .iter()
                .filter(|&&mark| mark == FSPLIT)
                .count(),
            1
        );
        assert_eq!(
            EXTRACT_BANKED.iter().filter(|&&mark| mark == FFUSE).count(),
            1
        );
        for deposit in [EVALT, EVALF] {
            let at = EXTRACT_BANKED
                .iter()
                .position(|&mark| mark == deposit)
                .unwrap();
            assert!(
                split < at && at < fuse,
                "{deposit} must remain inside EXTRACT's bank"
            );
        }
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
            .unwrap()
            .iter()
            .map(|t| morphism_name(t))
            .collect();
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
            construct_carrier(carrier)
                .unwrap()
                .iter()
                .map(|t| morphism_name(t))
                .collect::<Vec<_>>(),
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
            construct_carrier(carrier)
                .unwrap()
                .iter()
                .map(|t| morphism_name(t))
                .collect::<Vec<_>>(),
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
        assert_eq!(
            lucas_v(&tape_u64(4), &tape_u64(3), &tape_u64(1_000_000_007)),
            tape_u64(47)
        );
    }

    #[test]
    fn lehman_sweep_retains_cursor_and_returns_to_continuation() {
        let n = tape_u64(101);
        let mut sweep = LehmanSweep {
            kn4: tape_u64(404), a: tape_u64(21), limit: tape_u64(100),
        };
        assert!(lehman_resume(&n, &mut sweep, 1).is_none());
        assert_eq!(sweep.a, tape_u64(22));
        assert!(lehman_resume(&n, &mut sweep, 1).is_none());
        assert_eq!(sweep.a, tape_u64(23));

        let n = tape_u64(8051);
        let mut sweep = lehman_sweep(&n, &one());
        assert_eq!(lehman_resume(&n, &mut sweep, 16), lehman_step(&n, &one()));

        let wide = decimal_to_tape("233108530344407544527637656910680524145619812480305449042948611968495918245135782867888369318577116418213919268572658314913060672626911354027609793166341626693946596196427744273886601876896313468704059066746903123910748277606548649151920812699309766587514735456594993207").unwrap();
        let mut state = initial_carrier_state(wide.clone(), &two(), isqrt(&wide));
        state.round = tape_u64(2);
        execute_nested(&[LEHMAN, FIX], &mut state);
        assert_eq!(state.n, wide);
        assert!(state.lehman_sweep.is_some());
        let cursor = state.lehman_sweep.as_ref().unwrap().a.clone();
        state.round = tape_u64(6);
        execute_nested(&[LEHMAN, FIX], &mut state);
        assert!(cmp(&state.lehman_sweep.as_ref().unwrap().a, &cursor).is_gt());
    }

    #[test]
    fn folded_square_roots_close_across_limb_boundaries() {
        for value in 0..1024 {
            let n = tape_u64(value);
            let r = isqrt(&n);
            assert!(cmp(&mul(&r, &r), &n).is_le());
            let next = add(&r, &one());
            assert!(cmp(&mul(&next, &next), &n).is_gt());
        }
        for width in [63, 64, 65, 127, 128, 129, 447, 448, 449, 895, 1024] {
            let mut root = vec![EVALT; width];
            root[0] = EVALF;
            root[width - 1] = EVALF;
            let square = mul(&root, &root);
            assert_eq!(isqrt(&square), root);
            assert_eq!(isqrt(&sub(&square, &one())), sub(&root, &one()));
            assert_eq!(isqrt(&add(&square, &one())), root);
            let mut padded = square.clone();
            padded.extend([EVALT; 64]);
            assert_eq!(isqrt(&padded), root);
        }
    }

    #[test]
    fn resumable_square_forms_preserve_factors_and_wide_cycle_bound() {
        for value in [15, 77, 8051, 10403, 1000036000099] {
            let n = tape_u64(value);
            let mut cycle = SqufofCycle::default();
            let expected = squfof(&n);
            let mut found = None;
            for _ in 0..100_000 {
                found = squfof_resume(&n, &mut cycle, 1);
                if found.is_some() || cycle.done() { break; }
            }
            assert_eq!(found, expected, "source {value}");
            if let Some(p) = found {
                let (q, remainder) = divmod(&n, &p);
                assert!(zero(&remainder));
                verify_pair(&n, &p, &q).unwrap();
            }
        }
        let n = decimal_to_tape("233108530344407544527637656910680524145619812480305449042948611968495918245135782867888369318577116418213919268572658314913060672626911354027609793166341626693946596196427744273886601876896313468704059066746903123910748277606548649151920812699309766587514735456594993207").unwrap();
        let mut cycle = SqufofCycle::default();
        assert!(squfof_resume(&n, &mut cycle, 16).is_none());
        let frame = cycle.frame.as_ref().unwrap();
        assert!(frame.cap.len() > 64);
        let steps = frame.steps.clone();
        assert!(squfof_resume(&n, &mut cycle, 16).is_none());
        assert!(cmp(&cycle.frame.as_ref().unwrap().steps, &steps).is_gt());
        assert!(!cycle.done());
    }

    #[test]
    fn full_membrane_certifies_factors_and_takes_perfect_powers() {
        // WITNESS -> POWER -> EXTRACT -> P_MINUS -> P_PLUS -> ECM -> FIX
        let carrier = "⊢∈⊤≺⊥∋∈⊤⊞⊥∋∈≻⊤≺⊥⊞⋈∋∈⊙⊞⋈∋∈⊙≺⋈∋∈⊙≻⋈∋⊙⊡⊣";
        assert_eq!(
            construct_carrier(carrier)
                .unwrap()
                .iter()
                .map(|t| morphism_name(t))
                .collect::<Vec<_>>(),
            ["WITNESS", "POWER", "EXTRACT", "P_MINUS", "P_PLUS", "ECM", "FIX"]
        );
        assert_eq!(
            factor_with(carrier, &numeral(2147483647)).unwrap(),
            numeral(2147483647)
        );
        let f = factor_with(carrier, &numeral(8051)).unwrap();
        assert!(f == numeral(83) || f == numeral(97));
        // POWER first (before EXTRACT) takes a prime square to its base.
        let pw = "⊢∈⊤⊞⊥∋∈≻⊤≺⊥⊞⋈∋⊙⊡⊣";
        assert_eq!(
            construct_carrier(pw)
                .unwrap()
                .iter()
                .map(|t| morphism_name(t))
                .collect::<Vec<_>>(),
            ["POWER", "EXTRACT", "FIX"]
        );
        // 9973 is prime; POWER returns the base 9973 of 9973^2.
        assert_eq!(
            factor_with(pw, &numeral(9973 * 9973)).unwrap(),
            numeral(9973)
        );
    }

    #[test]
    fn witness_first_certifies_a_large_prime_without_the_sqrt_walk() {
        // WITNESS nested first: a strong prime test selects N at once, so a
        // large prime is not walked out to sqrt(N). Carrier
        // WITNESS -> EXTRACT -> P_MINUS -> ECM -> FIX.
        let carrier = "⊢∈⊤≺⊥∋∈≻⊤≺⊥⊞⋈∋∈⊙⊞⋈∋∈⊙≻⋈∋⊙⊡⊣";
        assert_eq!(
            construct_carrier(carrier)
                .unwrap()
                .iter()
                .map(|t| morphism_name(t))
                .collect::<Vec<_>>(),
            ["WITNESS", "EXTRACT", "P_MINUS", "ECM", "FIX"]
        );
        // 2^31-1 is prime: selected as itself.
        assert_eq!(
            factor_with(carrier, &numeral(2147483647)).unwrap(),
            numeral(2147483647)
        );
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
            construct_carrier(ecm)
                .unwrap()
                .iter()
                .map(|t| morphism_name(t))
                .collect::<Vec<_>>(),
            ["ECM", "FIX"]
        );
        let f = factor_with(ecm, &numeral(8051)).unwrap();
        assert!(f == numeral(83) || f == numeral(97));
        // Nested with EXTRACT: EXTRACT -> ECM -> FIX composes and factors.
        let nested = "⊢∈≻⊤≺⊥⊞⋈∋∈⊙≻⋈∋⊙⊡⊣";
        assert_eq!(
            construct_carrier(nested)
                .unwrap()
                .iter()
                .map(|t| morphism_name(t))
                .collect::<Vec<_>>(),
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

/// Bounded `factor`: drive the fixed tower for at most `max_steps` and return
/// Err naming the budget when no arm latches. The unbounded `factor` loops
/// until an arm selects; this makes the consumption a phase readout that
/// always returns -- a factor, or a named exhaustion.
pub fn factor_bounded(word: &str, max_steps: usize) -> Result<String, String> {
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
        phase_base: two(),
        candidate: add(&two(), &one()),
        remainder: vec![EVALT],
        x: two(),
        y: two(),
        phase: one(),
        eml_partners: None,
        eml_phase_done: false,
        negative_support: Vec::new(),
        divisor: one(),
        descent_evidence: Four::N,
        a: a_seed,
        descent_request: None,
        descent_report: None,
        pm_a: two(),
        pm_e: two(),
        ecm_seed: two(),
        ecm_round: vec![EVALT],
        pp_base: tape_u64(3),
        lehman_k: one(),
        lehman_sweep: None,
        witness_done: false,
        prime12_support: None,
        power_done: false,
        squfof_done: false,
        squfof_cycle: SqufofCycle::default(),
        round: vec![EVALT],
        exhausted: false,
        selected: None,
        selected_by: None,
        unbraid_stack: Vec::new(),
        unbraid_pair: None,
        unbraid_p_bits: 0,
        unbraid_q_bits: 0,
        unbraid_p_bound: vec![EVALT],
        unbraid_q_bound: vec![EVALT],
        unbraid_started: false,
        unbraid_done: false,
        carrier_closed: false,
        bridge_count: 0,
    };
    let tower: [&[char]; 6] = [PHASE, ARITHMETIC, BRANCH, SELECT, CONTINUE, FIX];
    for _ in 0..max_steps {
        execute_nested(&tower, &mut state);
        if let Some(ref selected) = state.selected {
            return Ok(emit_numeral(selected));
        }
    }
    Err(format!(
        "no factor in reach within {} tower steps",
        max_steps
    ))
}

#[cfg(test)]
mod direct_comparison_tests {
    use super::*;
    #[test]
    fn glyph_order_matches_limb_order_for_padded_and_wide_tapes() {
        let mut tapes = vec![Vec::new(), vec![EVALT; 130]];
        for value in 0..128 {
            let mut tape = tape_u64(value);
            tape.extend([EVALT; 3]);
            tapes.push(tape);
        }
        for width in [65, 129, 257, 1025, 2049] {
            let mut tape = vec![EVALT; width];
            tape[width - 1] = EVALF;
            tape[0] = EVALF;
            tapes.push(tape);
        }
        for a in &tapes {
            for b in &tapes { assert_eq!(cmp(a, b), l_cmp(&fold(a), &fold(b))); }
        }
    }
}

/// Number of 32-bit device limbs required by an arbitrary tape width.
pub fn limb_count_for_bits(bit_count: usize) -> usize {
    bit_count.div_ceil(32)
}

/// Convert a tape to device limbs. The requested count is minimum padding;
/// every source cell survives when the source requires a larger buffer.
pub fn tape_to_limbs(tape: &[char], limb_count: usize) -> Vec<u32> {
    let mut limbs = vec![0u32; limb_count.max(limb_count_for_bits(tape.len()))];
    for (i, &bit) in tape.iter().enumerate() {
        if bit == EVALF {
            limbs[i / 32] |= 1u32 << (i % 32);
        }
    }
    limbs
}

/// Recover the complete IMASM numeral from a device limb buffer.
pub fn limbs_to_tape(limbs: &[u32]) -> Vec<char> {
    let mut tape = Vec::new();
    for &limb in limbs {
        for bit in 0..32 {
            tape.push(if (limb >> bit) & 1 == 1 { EVALF } else { EVALT });
        }
    }
    trim(tape)
}

#[cfg(test)]
mod device_limb_tests {
    use super::*;
    #[test]
    fn minimum_device_padding_never_truncates_a_wider_source() {
        let mut tape = vec![EVALT; 2051];
        for i in [0, 31, 32, 63, 64, 2048, 2050] { tape[i] = EVALF; }
        let limbs = tape_to_limbs(&tape, 1);
        assert_eq!(limbs.len(), 65);
        assert_eq!(limbs_to_tape(&limbs), tape);
        assert_eq!(limb_count_for_bits(usize::MAX), usize::MAX / 32 + 1);
    }
}
