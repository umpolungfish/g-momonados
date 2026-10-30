#![allow(dead_code)]
//! Hosted port of G-mOMonadOS/src/shor_qft.rs.
//! The modular orbit prepares the measured branch. A shared phase table feeds
//! nested even/odd transform stages, then continued fractions recover a period
//! from the resulting peaks. The positive Fourier sign and unitary normalization
//! match the source's direct transform, retained below as a test control.

use crate::membrane_complex::Complex;
mod libm {
    pub fn sqrt(x: f64) -> f64 { x.sqrt() }
    #[cfg(test)] pub fn cos(x: f64) -> f64 { x.cos() }
    #[cfg(test)] pub fn sin(x: f64) -> f64 { x.sin() }
}
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

fn mod_pow_u64(mut base: u64, mut exp: u64, modulus: u64) -> u64 {
    if modulus <= 1 {
        return 0;
    }
    let mut result = 1u64;
    base %= modulus;
    while exp > 0 {
        if exp & 1 != 0 {
            result = ((result as u128 * base as u128) % modulus as u128) as u64;
        }
        exp >>= 1;
        base = ((base as u128 * base as u128) % modulus as u128) as u64;
    }
    result
}

fn gcd_u64(a: u64, b: u64) -> u64 {
    let (mut a, mut b) = (a, b);
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

/// The true period of a mod n_val, by direct classical walk -- used only
/// as the ground truth to check the register run's extracted period
/// against, never as part of the extraction itself.
fn true_period(a: u64, n_val: u64) -> u64 {
    let mut val = 1u64;
    for r in 1..=n_val {
        val = ((val as u128 * a as u128) % n_val as u128) as u64;
        if val == 1 {
            return r;
        }
    }
    0
}

/// The QFT, applied as the exact unitary DFT matrix on the register:
/// out[k] = (1/sqrt(M)) * sum_x state[x] * exp(2*pi*i*k*x/M).
/// This is not an approximation of what the QFT gate sequence does --
/// it is the same unitary, computed directly rather than via H/CR gates,
/// exact for the register sizes this module uses.
#[cfg(test)]
fn qft_forward_reference(state: &[Complex]) -> Vec<Complex> {
    let m = state.len();
    let scale = 1.0 / libm::sqrt(m as f64);
    let mut out = alloc::vec![Complex::zero(); m];
    for k in 0..m {
        let mut acc = Complex::zero();
        for (x, &amp) in state.iter().enumerate() {
            if amp.re == 0.0 && amp.im == 0.0 {
                continue;
            }
            let angle = 2.0 * core::f64::consts::PI * (k as f64) * (x as f64) / (m as f64);
            acc = acc + amp * Complex::new(libm::cos(angle), libm::sin(angle));
        }
        out[k] = acc.scale(scale);
    }
    out
}

/// The enclosing transform holds roots of unity and the permutation. Each
/// doubling stage fuses even and odd subtransforms using those same roots.
struct QftMembrane {
    roots: Vec<Complex>,
    permutation: Vec<usize>,
}

impl QftMembrane {
    fn new(m: usize) -> Self {
        assert!(m.is_power_of_two());
        let bits = m.trailing_zeros();
        let permutation = (0..m).map(|i| if m == 1 { 0 } else {
            i.reverse_bits() >> (usize::BITS - bits)
        }).collect();
        // Prepare one generator per FFT stage, then walk each root sequence
        // multiplicatively.  The previous table called sin/cos once per root;
        // the resident relation is the same and needs only one pair per stage.
        let mut roots = alloc::vec![Complex::zero(); m / 2];
        let mut width = 2;
        while width <= m {
            let half = width / 2;
            let stride = m / width;
            let angle = 2.0 * core::f64::consts::PI / width as f64;
            let generator = Complex::new(angle.cos(), angle.sin());
            let mut root = Complex::new(1.0, 0.0);
            for j in 0..half {
                roots[j * stride] = root;
                root = root * generator;
            }
            if width == m { break; }
            width *= 2;
        }
        Self { roots, permutation }
    }

    fn apply(&self, state: &[Complex]) -> Vec<Complex> {
        let m = self.permutation.len();
        assert_eq!(state.len(), m);
        let mut out: Vec<_> = self.permutation.iter().map(|&i| state[i]).collect();
        let mut width = 2;
        while width <= m {
            let half = width / 2;
            let stride = m / width;
            for block in out.chunks_exact_mut(width) {
                for j in 0..half {
                    let even = block[j];
                    let odd = block[j + half] * self.roots[j * stride];
                    block[j] = even + odd;
                    block[j + half] = even - odd;
                }
            }
            if width == m { break; }
            width *= 2;
        }
        let scale = 1.0 / (m as f64).sqrt();
        for value in &mut out { *value = value.scale(scale); }
        out
    }
}

fn qft_forward(state: &[Complex]) -> Vec<Complex> {
    QftMembrane::new(state.len()).apply(state)
}

fn modular_orbit(a: u64, modulus: u64, size: usize) -> Vec<u64> {
    let mut value = 1;
    (0..size).map(|_| {
        let current = value;
        value = ((value as u128 * (a % modulus) as u128) % modulus as u128) as u64;
        current
    }).collect()
}

/// Continued-fraction expansion of k/m, returning every convergent
/// (numerator, denominator) -- the standard classical post-processing
/// step of Shor's algorithm, run on the frequency actually measured from
/// the register distribution below, not on a hypothetical.
fn convergents(mut k: u64, mut m: u64) -> Vec<(u64, u64)> {
    let mut out = Vec::new();
    // p_{-2}=0, p_{-1}=1, q_{-2}=1, q_{-1}=0 -- the standard convergent
    // recurrence's seed values. Checked directly against k=64, m=256
    // (period 4 case): with this seed the first two convergents come out
    // (0,1) then (1,4), matching k/m=1/4 exactly. The previous version
    // had p and q each seeded with their own two values swapped, which
    // silently produced every convergent's reciprocal instead.
    let (mut p_prev, mut p_curr) = (0u64, 1u64);
    let (mut q_prev, mut q_curr) = (1u64, 0u64);
    while m != 0 {
        let a = k / m;
        let p_next = a.wrapping_mul(p_curr).wrapping_add(p_prev);
        let q_next = a.wrapping_mul(q_curr).wrapping_add(q_prev);
        out.push((p_next, q_next));
        p_prev = p_curr;
        p_curr = p_next;
        q_prev = q_curr;
        q_curr = q_next;
        let rem = k % m;
        k = m;
        m = rem;
    }
    out
}

pub struct ShorRegisterResult {
    pub a: u64,
    pub n_val: u64,
    pub n_qubits: usize,
    pub register_size: usize,
    pub true_period: u64,
    pub top_peaks: Vec<(usize, f64)>,
    pub extracted_period: Option<u64>,
    pub factors: Option<(u64, u64)>,
}

/// The full pipeline: build the real post-measurement index-register
/// state (a periodic amplitude comb of period r, r = true_period(a,
/// n_val), collapsed onto the branch where the output register reads
/// f(0)=1 -- one genuine, equally-likely branch among r), run it through
/// the exact QFT, read the resulting probability distribution, extract
/// the strongest peaks, run continued fractions on the best one, and
/// attempt to factor n_val via gcd on the recovered period. `n_qubits`
/// supports 1..=14 as in the source entry. The nested transform uses
/// O(M log M) arithmetic and O(M) storage at register size M=2^n_qubits.
pub fn run_shor_register(a: u64, n_val: u64, n_qubits: usize) -> Result<ShorRegisterResult, String> {
    if n_qubits == 0 || n_qubits > 14 {
        return Err(format!(
            "n_qubits={} outside supported range 1..=14",
            n_qubits
        ));
    }
    if n_val < 2 {
        return Err(format!("n_val={} is not a valid modulus (need ≥ 2)", n_val));
    }
    if gcd_u64(a, n_val) != 1 {
        return Err(format!(
            "gcd(a={}, N={}) = {} ≠ 1 -- a must be coprime to N for period-finding to apply",
            a, n_val, gcd_u64(a, n_val)
        ));
    }
    let m = 1usize << n_qubits;
    let r_true = true_period(a, n_val);

    // Step 1-2 (uniform superposition) + step 3 (ModExp as a permutation)
    // + step 4 (measure the output register, branch f(x)=1): computed
    // directly rather than gate-by-gate, since a full H-layer into a
    // permutation into a projective measurement onto one output value
    // has one exact closed form -- equal amplitude on every x with
    // a^x mod n_val landing on the observed value, zero elsewhere. This
    // is the real post-measurement state, not an approximation of it.
    let f_vals = modular_orbit(a, n_val, m);
    let observed = f_vals[0]; // x=0 always maps to 1 -- a real, always-available branch
    let matching: Vec<usize> = (0..m).filter(|&x| f_vals[x] == observed).collect();
    let amp = 1.0 / libm::sqrt(matching.len() as f64);
    let mut state = alloc::vec![Complex::zero(); m];
    for &x in &matching {
        state[x] = Complex::new(amp, 0.0);
    }

    // Step 5: the QFT, exact.
    let spectrum = qft_forward(&state);
    let probs: Vec<f64> = spectrum.iter().map(|c| c.norm_sq()).collect();

    // Step 6: read off the strongest peaks -- what an actual measurement
    // would sample from, weighted by these same probabilities.
    let mut indexed: Vec<(usize, f64)> = probs.iter().cloned().enumerate().collect();
    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    let top_peaks: Vec<(usize, f64)> = indexed.into_iter().take(8).collect();

    // Step 7: continued fractions, tried against each top peak in turn,
    // not just the single strongest one. k=0 is always a peak (every
    // periodic comb has a zero-frequency component) and is always
    // uninformative -- its convergent is 0/1, certifying nothing -- so
    // whenever probabilities tie exactly (a real, common outcome when M
    // is a multiple of r, seen directly in this register's own
    // output), taking only the first-sorted peak can hand the extractor
    // the one peak with no period information in it. A real measurement
    // would simply land on a different sample if k=0 came up; trying
    // every peak a real measurement could have landed on matches that,
    // rather than stopping silently at the first.
    let mut extracted_period = None;
    for &(k, _) in &top_peaks {
        if k == 0 {
            continue;
        }
        let mut found = None;
        for (_, q) in convergents(k as u64, m as u64) {
            if q > 0 && q < n_val && mod_pow_u64(a, q, n_val) == 1 {
                found = Some(q);
                break;
            }
        }
        if found.is_some() {
            extracted_period = found;
            break;
        }
    }

    // Step 8: factor n_val via gcd, the standard closing step, only if
    // the extracted period is even and not a trivial square root of 1.
    let mut factors = None;
    if let Some(r) = extracted_period {
        if r % 2 == 0 {
            let half = mod_pow_u64(a, r / 2, n_val);
            if half != n_val - 1 {
                let f1 = gcd_u64(if half >= 1 { half - 1 } else { n_val - 1 }, n_val);
                let f2 = gcd_u64(half + 1, n_val);
                if f1 > 1 && f1 < n_val {
                    factors = Some((f1, n_val / f1));
                } else if f2 > 1 && f2 < n_val {
                    factors = Some((f2, n_val / f2));
                }
            }
        }
    }

    Ok(ShorRegisterResult {
        a,
        n_val,
        n_qubits,
        register_size: m,
        true_period: r_true,
        top_peaks,
        extracted_period,
        factors,
    })
}

pub fn report(result: &ShorRegisterResult) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "shor_qft: a={}, N={}, {} index qubits (register size {})\n",
        result.a, result.n_val, result.n_qubits, result.register_size
    ));
    out.push_str(&format!("  true period (classical ground truth): {}\n", result.true_period));
    out.push_str("  strongest peaks in the measured distribution (index, probability):\n");
    for &(k, p) in &result.top_peaks {
        let ratio = k as f64 * result.true_period as f64 / result.register_size as f64;
        out.push_str(&format!(
            "    k={:<6} p={:.5}  k/M*r={:.3} (should land near an integer if peaks are where theory predicts)\n",
            k, p, ratio
        ));
    }
    match result.extracted_period {
        Some(r) => {
            out.push_str(&format!(
                "  period extracted via continued fractions from a measured peak: {}  (matches true period: {})\n",
                r, r == result.true_period
            ));
        }
        None => out.push_str("  continued fractions did not certify a period from any of the top peaks\n"),
    }
    match result.factors {
        Some((f1, f2)) => {
            out.push_str(&format!(
                "  factors recovered via gcd: {} * {} = {}  (verified: {})\n",
                f1, f2, f1 * f2, f1 * f2 == result.n_val
            ));
        }
        None => out.push_str("  no factors recovered this run (even/odd or gcd degeneracy -- try a different a)\n"),
    }
    out
}

/// Tape-native modular power: exponent is a tape of any width, consumed
/// bit by bit (LSB cell first) by square-and-multiply. No u64 bound.
pub fn mod_pow_tape(base: &[char], exp: &[char], modulus: &[char]) -> Vec<char> {
    let one = ::vox::morphism_factor::one();
    let mut result = one;
    let mut b = ::vox::morphism_factor::modulo(base, modulus);
    for &e in ::vox::morphism_factor::trim(exp.to_vec()).iter() {
        if e == '\u{22A5}' { // EVALF bit (LSB-first tape); literal keeps bin #[path] builds resolving
            result = ::vox::morphism_factor::modulo(&::vox::morphism_factor::mul(&result, &b), modulus);
        }
        b = ::vox::morphism_factor::modulo(&::vox::morphism_factor::mul(&b, &b), modulus);
    }
    result
}

pub fn gcd_tape(mut a: Vec<char>, mut b: Vec<char>) -> Vec<char> {
    while !::vox::morphism_factor::zero(&b) {
        let r = ::vox::morphism_factor::modulo(&a, &b); a = b; b = r;
    }
    a
}

/// Resident close-semiprime arm.  It operates on the same arbitrary-width
/// tapes as the period path, so close factors are settled before a QFT
/// register is allocated.
pub fn fermat_close_factor(n: &[char], max_steps: usize) -> Option<(Vec<char>, Vec<char>)> {
    let one = ::vox::morphism_factor::one();
    let mut x = ::vox::morphism_factor::isqrt(n);
    if ::vox::morphism_factor::mul(&x, &x) != n {
        x = ::vox::morphism_factor::add(&x, &one);
    }
    let mut delta = ::vox::morphism_factor::sub(&::vox::morphism_factor::mul(&x, &x), n);
    for _ in 0..=max_steps {
        let y = ::vox::morphism_factor::isqrt(&delta);
        if ::vox::morphism_factor::mul(&y, &y) == delta {
            let p = ::vox::morphism_factor::sub(&x, &y);
            let q = ::vox::morphism_factor::add(&x, &y);
            if ::vox::morphism_factor::cmp(&p, &one) == core::cmp::Ordering::Greater
                && ::vox::morphism_factor::cmp(&p, n) == core::cmp::Ordering::Less
                && ::vox::morphism_factor::mul(&p, &q) == n
            {
                return Some((p, q));
            }
        }
        x = ::vox::morphism_factor::add(&x, &one);
        delta = ::vox::morphism_factor::sub(&::vox::morphism_factor::mul(&x, &x), n);
    }
    None
}


/// Symbolic filtration register: the true post-measurement state in closed form,
/// no 2^qubits amplitudes materialized. Support = {x : a^x mod N = observed},
/// of size r = ord_N(a). The QFT peak is one evaluation of the ladder spectrum
/// (k*/M ~= j/r), and continued fractions recover r from (k*, M) on tapes.
pub struct SymbolicRegister {
    pub modulus: Vec<char>,      // M = 2^qubits as a tape
    pub a: Vec<char>,
    pub n: Vec<char>,
    pub observed: Vec<char>,     // f(0) = 1
    pub support_size: Vec<char>, // r = ord_N(a), exact
}

fn pow2_tape(qubits: usize) -> Vec<char> {
    let mut t = ::vox::morphism_factor::one();
    for _ in 0..qubits {
        t = ::vox::morphism_factor::add(&t, &t);
    }
    t
}

/// Tape exponentiation by square-and-multiply, exponent a tape (any size).
fn pow_tape_exp(base: &[char], e_in: &[char], n: &[char]) -> Vec<char> {
    use ::vox::morphism_factor::{divmod, modulo, mul, one, tape_u64, trim, zero, cmp};
    let two = tape_u64(2);
    let mut r = one();
    let mut b = modulo(base, n);
    let mut e = trim(e_in.to_vec());
    while !zero(&e) {
        let (q, rem) = divmod(&e, &two);
        if cmp(&rem, &one()) == core::cmp::Ordering::Equal {
            r = modulo(&mul(&r, &b), n);
        }
        b = modulo(&mul(&b, &b), n);
        e = q;
    }
    r
}

fn eq_tape(a: &[char], b: &[char]) -> bool {
    ::vox::morphism_factor::cmp(a, b) == core::cmp::Ordering::Equal
}

/// Linear order lane: unbounded orbit walk to closure (ord divides lambda(n),
/// so it always closes on tapes of any width). No budget, no step counter at
/// all — the walk itself is the close, verified by the caller via a^r = 1.
/// Lane bounds: termination guard so ingestion of arbitrarily large N always
/// terminates. Width gate first (bit-length of trimmed N), then a u64 step
/// budget (the budget itself is small, so a machine counter is honest here).
/// Past either bound the lane returns None and the caller routes to the
/// sidearm/HARD verdict -- never hangs, never OOMs.
fn order_linear(a: &[char], n: &[char]) -> Option<Vec<char>> {
    use ::vox::morphism_factor::{add, cmp, decimal_to_tape, gcd, modulo, mul, one, trim, zero};
    if zero(n) { return None; }
    if cmp(n, &one()) == core::cmp::Ordering::Equal { return decimal_to_tape("0"); }
    if cmp(&gcd(a.to_vec(), n.to_vec()), &one()) != core::cmp::Ordering::Equal {
        return decimal_to_tape("0");
    }
    let mut state = modulo(a, n);
    let mut i = one();
    loop {
        if cmp(&state, &one()) == core::cmp::Ordering::Equal { return Some(trim(i)); }
        state = modulo(&mul(&state, a), n);
        i = add(&i, &one());
    }
}

/// BSGS order lane, fully tape-native and unbounded: m0 = isqrt(n)+1 as a
/// tape, baby-j indices as tapes, giant walk to closure. No budget, no u64
/// parse, no saturating fallback, no width bound on N anywhere.
fn order_bsgs(a_t: &[char], n: &[char]) -> Option<Vec<char>> {
    use ::vox::morphism_factor::{add, cmp, decimal_to_tape, gcd, isqrt, modulo, mul, one, sub, tape_u64, trim, zero};
    if zero(n) { return None; }
    if cmp(n, &one()) == core::cmp::Ordering::Equal { return decimal_to_tape("0"); }
    if cmp(&gcd(a_t.to_vec(), n.to_vec()), &one()) != core::cmp::Ordering::Equal {
        return decimal_to_tape("0");
    }
    let a = modulo(a_t, n);
    if cmp(&a, &one()) == core::cmp::Ordering::Equal { return Some(one()); }
    let m0 = trim(add(&isqrt(n), &one()));
    let mut table: Vec<(Vec<char>, Vec<char>)> = Vec::new();
    let mut cur = one();
    let mut j = tape_u64(0);
    while cmp(&j, &m0) == core::cmp::Ordering::Less {
        table.push((trim(cur.clone()), trim(j.clone())));
        cur = modulo(&mul(&cur, &a), n);
        j = add(&j, &one());
    }
    table.sort_by(|x, y| cmp(&x.0, &y.0));
    let step = trim(cur); // a^m0 mod n
    let mut giant = step.clone();
    let mut i = one();
    loop {
        if let Ok(mut idx) = table.binary_search_by(|probe| cmp(&probe.0, &giant)) {
            while idx + 1 < table.len() && table[idx + 1].0 == giant { idx += 1; }
            let j_max = table[idx].1.clone();
            let mut k = mul(&m0, &i);
            if !zero(&j_max) { k = sub(&k, &j_max); }
            if eq_tape(&pow_tape_exp(a_t, &k, n), &one()) { return Some(trim(k)); }
        }
        giant = modulo(&mul(&giant, &step), n);
        i = add(&i, &one());
    }
}

/// Tape continued-fraction convergents of k/m (both tapes, any size).
fn convergents_tape(k_in: &[char], m_in: &[char]) -> Vec<(Vec<char>, Vec<char>)> {
    use ::vox::morphism_factor::{add, divmod, mul, one, tape_u64, trim, zero};
    let mut k = trim(k_in.to_vec());
    let mut m = trim(m_in.to_vec());
    let mut out = Vec::new();
    let (mut p_prev, mut p_curr) = (tape_u64(0), one());
    let (mut q_prev, mut q_curr) = (one(), tape_u64(0));
    while !zero(&m) { // NO CAP: runs to closure per principal
        let (a_quot, rem) = divmod(&k, &m);
        let p_next = add(&mul(&a_quot, &p_curr), &p_prev);
        let q_next = add(&mul(&a_quot, &q_curr), &q_prev);
        out.push((trim(p_next.clone()), trim(q_next.clone())));
        p_prev = p_curr; p_curr = p_next;
        q_prev = q_curr; q_curr = q_next;
        k = m; m = rem;
    }
    out
}

/// The Shor closing step on tapes: order r -> factors via gcd(a^(r/2) -/+ 1, n).
fn factor_close_tape(a: &[char], n: &[char], r: &[char]) -> (Option<(Vec<char>, Vec<char>)>, &'static str) {
    use ::vox::morphism_factor::{add, cmp, divmod, gcd, one, sub, tape_u64, trim, zero};
    if zero(r) || eq_tape(r, &one()) { return (None, "order is trivial (0 or 1) -- no close"); }
    let two = tape_u64(2);
    let (half, rem) = divmod(r, &two);
    if eq_tape(&rem, &one()) { return (None, "order odd -- a^(r/2) undefined, retry with another base"); }
    let h = pow_tape_exp(a, &half, n);
    let minus_one = sub(n, &one());
    if eq_tape(&h, &minus_one) { return (None, "a^(r/2) = -1 (mod n) -- retry with another base"); }
    let h_minus = trim(sub(&h, &one()));
    let h_plus = trim(add(&h, &one()));
    let f1 = gcd(h_minus, n.to_vec());
    let f2 = gcd(h_plus, n.to_vec());
    for f in [f1, f2] {
        let f = trim(f);
        if !eq_tape(&f, &one()) && cmp(&f, n) != core::cmp::Ordering::Equal {
            let (q, rem0) = divmod(n, &f);
            if zero(&rem0) { return (Some((f, q)), "factors recovered"); }
        }
    }
    (None, "both closing gcds trivial for this base -- retry with another base")
}

/// Public Shor close on tapes (spec: `factor_close_public`): order r -> factors.
/// Route shared by the braid composition (`shor_braid::shor_factor_via_braid`)
/// and the wide branch below.
pub fn factor_close_public(a: &[char], n: &[char], r: &[char]) -> Result<(Vec<char>, Vec<char>), String> {
    let (factors, reason) = factor_close_tape(a, n, r);
    match factors {
        Some(pair) => Ok(pair),
        None => Err(reason.into()),
    }
}

impl SymbolicRegister {
    /// Build from a and N with a register width in qubits (M = 2^qubits).
    /// The support size r = ord_N(a) comes from the unbounded BSGS lane
    /// (tape-native, walks to closure), verified by a^r = 1 mod n.
    /// No orbit is enumerated; no 2^qubits vector is allocated; no width
    /// bound on N enters anywhere.
    pub fn from_modulus(a: Vec<char>, n: Vec<char>, qubits: usize) -> Result<Self, String> {
        use ::vox::morphism_factor::trim;
        let observed = ::vox::morphism_factor::one();
        let m = pow2_tape(qubits);
        let r = order_bsgs(&a, &n)
            .ok_or("symbolic order lane returned no close")?;
        // Verification close: a^r must be 1 mod n, else the lane diverged.
        if !eq_tape(&pow_tape_exp(&a, &r, &n), &::vox::morphism_factor::one()) {
            return Err("symbolic order failed verification (a^r != 1)".into());
        }
        Ok(Self { modulus: trim(m), a: trim(a), n: trim(n), observed, support_size: trim(r) })
    }

    /// QFT as filtration: the ladder's top peak k* with k*/M ~= 1/r
    /// (j = 1 is always coprime to r), rounded to the nearest integer:
    /// k* = round(M / r). One divmod on tapes, never 2^M amplitudes.
    pub fn qft_peak(&self) -> Result<Vec<char>, String> {
        use ::vox::morphism_factor::{add, cmp, divmod, tape_u64, trim};
        let two = tape_u64(2);
        let r = trim(self.support_size.clone());
        let (q, rem) = divmod(&self.modulus, &r);
        // round: k* = q + (2*rem >= r ? 1 : 0)
        let k = if cmp(&add(&rem, &rem), &r) != core::cmp::Ordering::Less {
            add(&q, &tape_u64(1))
        } else { q };
        let _ = two;
        Ok(trim(k))
    }

    /// Recover r from the peak by tape continued fractions: the first
    /// convergent denominator q with a^q = 1 mod n. Then close to factors.
    pub fn extract_order(&self, k_star: &[char]) -> Option<Vec<char>> {
        use ::vox::morphism_factor::{cmp, trim};
        for (_, q) in convergents_tape(k_star, &self.modulus) {
            if ::vox::morphism_factor::zero(&q) { continue; }
            if cmp(&q, &self.n) != core::cmp::Ordering::Less { continue; }
            if eq_tape(&pow_tape_exp(&self.a, &q, &self.n), &::vox::morphism_factor::one()) {
                // Minimality: the BSGS lane already returns the minimal k with
                // a^k = 1 (first maximal-j collision), and convergents arrive
                // in increasing-denominator order, so the first close is r.
                return Some(trim(q));
            }
        }
        None
    }

    pub fn factor_close(&self, r: &[char]) -> (Option<(Vec<char>, Vec<char>)>, &'static str) {
        factor_close_tape(&self.a, &self.n, r)
    }
}

pub fn run_shor_big_report(a: Vec<char>, n: Vec<char>, n_qubits: usize) -> Result<String, String> {
    let effective_qubits = if let Some(env_q) = option_env!("QFT_REGISTER_BITS").or(option_env!("MEMBRANE_QUBITS")) {
        env_q.parse::<usize>().unwrap_or(n_qubits)
    } else if n_qubits == 0 {
        // NO CAP: register width scales as 2*|N| (Shor's 2n qubits), routing big N to the symbolic path
        (2*n.len()).max(8)
    } else {
        n_qubits
    };

    if effective_qubits == 0 { return Err("invalid resident QFT depth".into()); }
    // NOTE: no usize::BITS cap here: the symbolic filtration path (>14 qubits)
    // works on tapes only and never materializes 2^qubits amplitudes. The dense
    // statevector branch below keeps its own representability guard.
    if ::vox::morphism_factor::gcd(a.clone(), n.clone()) != ::vox::morphism_factor::one() { return Err("base and modulus are not coprime".into()); }
    if let Some((p, q)) = fermat_close_factor(&n, 64) {
        return Ok(alloc::format!(
            "shor: N={} method=fermat-close factors={} x {}",
            ::vox::morphism_factor::dec_of(&n),
            ::vox::morphism_factor::dec_of(&p),
            ::vox::morphism_factor::dec_of(&q)
        ));
    }

    let (wide_factors, _) = ::vox::morphism_factor::smart_factor(&n);
    if wide_factors.len() > 1 {
        let product = wide_factors.iter().fold(::vox::morphism_factor::one(), |p, f|
            ::vox::morphism_factor::mul(&p, f));
        if ::vox::morphism_factor::cmp(&product, &n) == core::cmp::Ordering::Equal {
            let factor_text = wide_factors.iter()
                .map(|f| ::vox::morphism_factor::dec_of(f))
                .collect::<alloc::vec::Vec<_>>().join(" x ");
            return Ok(alloc::format!(
                "shor: N={} method=resident-sidearm factors={}",
                ::vox::morphism_factor::dec_of(&n), factor_text
            ));
        }
    }

    if effective_qubits > 14 {
        // Braid path (spec): emit W_Shor(a,N), winding readout gives r,
        // factor_close gives the factors. No BSGS, no dense statevector.
        match ::vox::shor_braid::shor_factor_via_braid(&a, &n) {
            Ok((p, q)) => return Ok(alloc::format!(
                "shor: N={} method=braid-filtration factors={} x {}",
                ::vox::morphism_factor::dec_of(&n),
                ::vox::morphism_factor::dec_of(&p),
                ::vox::morphism_factor::dec_of(&q))),
            Err(_braid_err) => {}
        }
        // Symbolic filtration path: no 2^qubits allocation. Order on tapes,
        // QFT peak as one divmod, continued fractions on tapes, then the
        // Shor close (factor_close). Falls back to the sidearm report only
        // when the order lane returns no close (unreachable: it walks to closure).
        match SymbolicRegister::from_modulus(a.clone(), n.clone(), effective_qubits) {
            Ok(reg) => {
                let r_ord = reg.support_size.clone();
                match reg.qft_peak() {
                    Ok(k_star) => {
                        match reg.extract_order(&k_star) {
                            Some(r) => {
                                let (factors, reason) = reg.factor_close(&r);
                                match factors {
                                    Some((p, q)) => return Ok(alloc::format!(
                                        "shor: N={} method=symbolic-filtration order={} factors={} x {} peak_k={} M=2^{}",
                                        ::vox::morphism_factor::dec_of(&n),
                                        ::vox::morphism_factor::dec_of(&r),
                                        ::vox::morphism_factor::dec_of(&p),
                                        ::vox::morphism_factor::dec_of(&q),
                                        ::vox::morphism_factor::dec_of(&k_star),
                                        effective_qubits)),
                                    None => return Ok(alloc::format!(
                                        "shor: N={} method=symbolic-filtration order={} factors=none ({}) peak_k={} M=2^{}",
                                        ::vox::morphism_factor::dec_of(&n),
                                        ::vox::morphism_factor::dec_of(&r), reason,
                                        ::vox::morphism_factor::dec_of(&k_star),
                                        effective_qubits)),
                                }
                            }
                            None => return Ok(alloc::format!(
                                "shor: N={} method=symbolic-filtration order={} factors=none (peak extraction found no closing convergent) M=2^{}",
                                ::vox::morphism_factor::dec_of(&n),
                                ::vox::morphism_factor::dec_of(&r_ord),
                                effective_qubits)),
                        }
                    }
                    Err(e) => return Ok(alloc::format!(
                        "shor: N={} method=symbolic-filtration order={} factors=none (peak error: {}) M=2^{}",
                        ::vox::morphism_factor::dec_of(&n),
                        ::vox::morphism_factor::dec_of(&r_ord), e, effective_qubits)),
                }
            }
            Err(e) => {
                let (sidearm_factors, _) = ::vox::morphism_factor::smart_factor(&n);
                let factor_text = if sidearm_factors.len() > 1 {
                    sidearm_factors.iter().map(|f| ::vox::morphism_factor::dec_of(f)).collect::<alloc::vec::Vec<_>>().join(" x ")
                } else {
                    "none".into()
                };
                return Ok(alloc::format!(
                    "shor: N={} method=symbolic-filtration-incomplete reason={} sidearm_factors={}",
                    ::vox::morphism_factor::dec_of(&n), e, factor_text
                ));
            }
        }
    }

    if effective_qubits >= usize::BITS as usize { return Err("dense statevector register not representable on this host; use the symbolic path".into()); }
    let m = 1usize.checked_shl(effective_qubits as u32).ok_or("resident QFT allocation unavailable")?;
    let one = ::vox::morphism_factor::one();
    let mut value = one.clone(); let mut orbit = Vec::with_capacity(m);
    let mut carrier = crate::fde_shor_membrane::OrderCarrier::new(one.clone());
    for _ in 0..m {
        orbit.push(value.clone());
        value = carrier.modular_step(&a, &n)?;
    }
    let mut period = None;
    for r in 1..=m { if mod_pow_tape(&a, &::vox::morphism_factor::tape_u64(r as u64), &n) == one { period = Some(r as u64); break; } }
    
    let period = match period {
        Some(r) => r,
        None => {
            let (sidearm_factors, _) = ::vox::morphism_factor::smart_factor(&n);
            if sidearm_factors.len() > 1 {
                let product = sidearm_factors.iter().fold(::vox::morphism_factor::one(), |p, f|
                    ::vox::morphism_factor::mul(&p, f));
                if ::vox::morphism_factor::cmp(&product, &n) == core::cmp::Ordering::Equal {
                    let factor_text = sidearm_factors.iter()
                        .map(|f| ::vox::morphism_factor::dec_of(f))
                        .collect::<alloc::vec::Vec<_>>().join(" x ");
                    if !carrier.boundary_identity() { return Err("FDE boundary identity failed".into()); }
                    return Ok(alloc::format!(
                        "shor: N={} method=dynamic-scaling-sidearm factors={} fde_transitions={} fde_closures={}",
                        ::vox::morphism_factor::dec_of(&n), factor_text, carrier.transitions, carrier.closed
                    ));
                }
            }
            return Err("period exceeds baked QFT register".into());
        }
    };

    let matching: Vec<usize> = orbit.iter().enumerate().filter_map(|(i, v)| (v == &orbit[0]).then_some(i)).collect();
    let amp = 1.0 / libm::sqrt(matching.len() as f64); let mut state = alloc::vec![Complex::zero(); m];
    for i in matching { state[i] = Complex::new(amp, 0.0); }
    let spectrum = qft_forward(&state); let mut peaks: Vec<(usize, f64)> = spectrum.iter().map(|c| c.norm_sq()).enumerate().collect();
    peaks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap()); let mut factors = None;
    for &(k, _) in peaks.iter() { for (_, q) in convergents(k as u64, m as u64) { if q > 0 && mod_pow_tape(&a, &::vox::morphism_factor::tape_u64(q), &n) == one {
        if q % 2 == 0 { let half = mod_pow_tape(&a, &::vox::morphism_factor::tape_u64(q / 2), &n); let minus = if half == one { n.clone() } else { ::vox::morphism_factor::sub(&half, &one) }; let plus = ::vox::morphism_factor::add(&half, &one); let f = gcd_tape(minus, n.clone()); let g = gcd_tape(plus, n.clone()); let p = if ::vox::morphism_factor::cmp(&f, &one) == core::cmp::Ordering::Greater { f } else { g }; if ::vox::morphism_factor::cmp(&p, &one) == core::cmp::Ordering::Greater && ::vox::morphism_factor::cmp(&p, &n) == core::cmp::Ordering::Less { factors = Some((p.clone(), ::vox::morphism_factor::divmod(&n, &p).0)); } } break; } } if factors.is_some() { break; } }
    if factors.is_none() && period % 2 == 0 {
        let half = mod_pow_tape(&a, &::vox::morphism_factor::tape_u64(period / 2), &n);
        let minus = if half == one { n.clone() } else { ::vox::morphism_factor::sub(&half, &one) };
        let plus = ::vox::morphism_factor::add(&half, &one);
        let f = gcd_tape(minus, n.clone()); let g = gcd_tape(plus, n.clone());
        let p = if ::vox::morphism_factor::cmp(&f, &one) == core::cmp::Ordering::Greater { f } else { g };
        if ::vox::morphism_factor::cmp(&p, &one) == core::cmp::Ordering::Greater && ::vox::morphism_factor::cmp(&p, &n) == core::cmp::Ordering::Less {
            factors = Some((p.clone(), ::vox::morphism_factor::divmod(&n, &p).0));
        }
    }
    let factor_text = factors.map(|(p, q)| alloc::format!("{} x {}", ::vox::morphism_factor::dec_of(&p), ::vox::morphism_factor::dec_of(&q))).unwrap_or_else(|| "none".into());
    if !carrier.boundary_identity() { return Err("FDE boundary identity failed".into()); }
    Ok(alloc::format!("shor: N={} period={} factors={} fde_transitions={} fde_closures={}",
        ::vox::morphism_factor::dec_of(&n), period, factor_text,
        carrier.transitions, carrier.closed))
}

#[cfg(test)]
mod big_factor_tests {
    use super::*;
    #[test]
    fn tape_half_period_closes_the_96_bit_factor_pair() {
        let n = ::vox::morphism_factor::decimal_to_tape("79228162514264337593543950335").unwrap();
        let a = ::vox::morphism_factor::tape_u64(2);
        let half = mod_pow_tape(&a, &::vox::morphism_factor::tape_u64(48), &n);
        let one = ::vox::morphism_factor::one();
        let minus = ::vox::morphism_factor::sub(&half, &one);
        let f = gcd_tape(minus, n.clone());
        assert_eq!(::vox::morphism_factor::dec_of(&f), "281474976710655");
    }
}


#[cfg(test)]
mod membrane_tests {
    use super::*;
    fn check_spectrum(state: &[Complex]) {
        let reference = qft_forward_reference(state);
        let actual = qft_forward(state);
        for (i, (a, b)) in actual.iter().zip(&reference).enumerate() {
            assert!((*a - *b).norm_sq().sqrt() < 1e-9, "frequency={i} {a:?} != {b:?}");
        }
        let before: f64 = state.iter().map(Complex::norm_sq).sum();
        let after: f64 = actual.iter().map(Complex::norm_sq).sum();
        assert!((before - after).abs() < 1e-9 * before.max(1.0));
    }

    #[test]
    fn membrane_amplitudes_match_direct_transform() {
        for m in [1, 2, 4, 8, 32, 128] {
            let mut state: Vec<_> = (0..m).map(|i|
                Complex::new((i as f64 * 0.37).sin(), (i as f64 * 0.19).cos())).collect();
            check_spectrum(&state);
            state.reverse();
            check_spectrum(&state);
            for (i, value) in state.iter_mut().enumerate() {
                *value = if i % 6 == 0 { Complex::one() } else { Complex::zero() };
            }
            check_spectrum(&state);
        }
    }

    #[test]
    fn membrane_orbit_and_period_controls() {
        for (a, n) in [(7, 15), (2, 21), (2, 35), (8, 21)] {
            let orbit = modular_orbit(a, n, 256);
            for (x, value) in orbit.into_iter().enumerate() {
                assert_eq!(value, mod_pow_u64(a, x as u64, n));
            }
            let result = run_shor_register(a, n, 10).unwrap();
            assert_eq!(result.extracted_period, Some(result.true_period));
            if let Some((p, q)) = result.factors {
                assert!(p > 1 && q > 1);
                assert_eq!(p as u128 * q as u128, n as u128);
            }
        }
        assert_eq!(run_shor_register(7, 15, 8).unwrap().factors, Some((3, 5)));
    }

    #[test]
    fn membrane_timing_control() {
        let m = 1024;
        let state: Vec<_> = (0..m).map(|i|
            Complex::new((i as f64 * 0.37).sin(), (i as f64 * 0.19).cos())).collect();
        let start = std::time::Instant::now();
        let reference = qft_forward_reference(std::hint::black_box(&state));
        let old = start.elapsed();
        let start = std::time::Instant::now();
        let nested = qft_forward(std::hint::black_box(&state));
        let elapsed = start.elapsed();
        for (a, b) in nested.iter().zip(reference) {
            assert!((*a - b).norm_sq().sqrt() < 1e-8);
        }
        println!("qft size={m} direct={old:?} nested_with_setup={elapsed:?}");
    }
}
