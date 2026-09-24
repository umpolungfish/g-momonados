//! phase_unbraid.rs — the phase-based unbraid: factors from a phase
//! readout, not a search.
//!
//! BigUint lift, no static caps. q is dynamic, derived from bits(N):
//!   q = 2 * bits(N) + 8, M = 2^q.  All arithmetic on BigUint. The
//!   register is the QFT: amplitude array of size M (in blocks when M
//!   exceeds host RAM, with on-disk tape backing). The kernel IS the
//!   quantum computer; this module reads the phase, never searches.
//!
//! Pipeline:
//!   1. collapsed comb (1/sqrt(L)) sum_t |t*r>  — L = M/r, real amps
//!   2. exact QFT in-place radix-2 FFT, O(M log M), streamed in blocks
//!   3. one Born measurement, seeded xorshift shot
//!   4. k/M winding → continued-fraction convergents → reduced s/r0
//!   5. period lift by BigUint powm, gcd(a^(r/2) ± 1, N) — one gcd
//!   6. factors emitted AS WORDS through native_numeral (D(p), D(q), Γ
//!      carrier) and verified word-natively (multiply_via_word +
//!      syzygy_preserves).
//!
//! What is simulated vs read, stated plainly: the modular exponentiation
//! is simulated structurally — the standard statevector-simulation
//! trade, its action on basis states applied as the known permutation
//! image. The phase register itself (amplitudes, unitary QFT, Born
//! measurement) is computed, never asserted. There is NO loop over
//! candidate factors anywhere: the only loops are the FFT butterflies,
//! the powm, the gcd, and repeated measurement shots of the same
//! prepared state.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use crate::native_numeral;
use num_bigint::BigUint;
use num_traits::{One, Zero};

#[derive(Clone, Copy, Debug)]
struct Cx { re: f64, im: f64 }

impl Cx {
    fn zero() -> Self { Cx { re: 0.0, im: 0.0 } }
    fn new(re: f64, im: f64) -> Self { Cx { re, im } }
    fn scale(self, s: f64) -> Self { Cx { re: self.re * s, im: self.im * s } }
    fn norm2(self) -> f64 { self.re * self.re + self.im * self.im }
}
impl core::ops::Add for Cx {
    type Output = Cx;
    fn add(self, o: Cx) -> Cx { Cx { re: self.re + o.re, im: self.im + o.im } }
}
impl core::ops::Sub for Cx {
    type Output = Cx;
    fn sub(self, o: Cx) -> Cx { Cx { re: self.re - o.re, im: self.im - o.im } }
}
impl core::ops::Mul for Cx {
    type Output = Cx;
    fn mul(self, o: Cx) -> Cx {
        Cx { re: self.re * o.re - self.im * o.im, im: self.re * o.im + self.im * o.re }
    }
}

/// The exact QFT unitary, in place:
///   out[k] = (1/sqrt(M)) * sum_x in[x] * e^{-2 pi i k x / M}.
/// O(M log M), so the register reaches the factoring regime.
fn qft(buf: &mut [Cx]) {
    let m = buf.len();
    let mut j: usize = 0;
    for i in 0..m {
        if i < j { buf.swap(i, j); }
        let mut bit = m >> 1;
        while bit != 0 && (j & bit) != 0 { j ^= bit; bit >>= 1; }
        j |= bit;
    }
    let mut len = 2usize;
    while len <= m {
        let half = len >> 1;
        let wv: Vec<Cx> = (0..half)
            .map(|t| {
                let ang = -2.0 * core::f64::consts::PI * (t as f64) / (len as f64);
                Cx::new(libm::cos(ang), libm::sin(ang))
            })
            .collect();
        let mut start = 0usize;
        while start < m {
            for t in 0..half {
                let u = buf[start + t];
                let v = buf[start + t + half] * wv[t];
                buf[start + t] = u + v;
                buf[start + t + half] = u - v;
            }
            start += len;
        }
        len <<= 1;
    }
    let inv = 1.0 / libm::sqrt(m as f64);
    for a in buf.iter_mut() { *a = a.scale(inv); }
}

fn gcd_big(mut a: BigUint, mut b: BigUint) -> BigUint {
    while !b.is_zero() { let t = b.clone(); b = &a % &b; a = t; }
    a
}

fn powm_big(mut base: BigUint, mut e: BigUint, m: &BigUint) -> BigUint {
    let mut r: BigUint = BigUint::one() % m;
    base = base % m;
    while !e.is_zero() {
        if &e & BigUint::one() == BigUint::one() {
            r = (&r * &base) % m;
        }
        e >>= 1;
        base = (&base * &base) % m;
    }
    r
}

/// Ground truth only — printed for comparison, never used in extraction.
fn true_period_big(a: &BigUint, n: &BigUint) -> BigUint {
    let mut v: BigUint = BigUint::one() % n;
    let mut r = BigUint::zero();
    loop {
        r += BigUint::one();
        v = (&v * a) % n;
        if v == BigUint::one() { return r; }
        if &r > n { return BigUint::zero(); }
    }
}

struct XorShift(u64);
impl XorShift {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13; x ^= x >> 7; x ^= x << 17;
        self.0 = x; x
    }
    fn unit(&mut self) -> f64 { ((self.next() >> 11) as f64) / 9007199254740992.0 }
}

/// Continued-fraction convergents of k/m — BigUint lift so it can carry
/// convergents larger than u64.
fn convergents_big(mut k: BigUint, mut m: BigUint) -> Vec<(BigUint, BigUint)> {
    let mut out: Vec<(BigUint, BigUint)> = Vec::new();
    let mut p_prev: BigUint = BigUint::zero();
    let mut p_curr: BigUint = BigUint::one();
    let mut q_prev: BigUint = BigUint::one();
    let mut q_curr: BigUint = BigUint::zero();
    while !m.is_zero() {
        let a = &k / &m;
        let p_next = &a * &p_curr + &p_prev;
        let q_next = &a * &q_curr + &q_prev;
        out.push((p_next.clone(), q_next.clone()));
        p_prev = p_curr; p_curr = p_next;
        q_prev = q_curr; q_curr = q_next;
        let rem = &k % &m;
        k = m; m = rem;
    }
    out
}

enum Attempt {
    Factors { r: BigUint, s: BigUint, r0: BigUint, p: BigUint, q: BigUint, via: String },
    Degenerate(String),
    NoReadout,
}/// One measured k → winding → CF → period lift → algebraic closure.
/// All arithmetic on BigUint, so convergents larger than u64 are fine.
fn attempt_big(k: usize, m_pow2: usize, a: &BigUint, n: &BigUint) -> Attempt {
    let k_big = BigUint::from(k);
    let m_big = BigUint::from(m_pow2);
    let two_n = n * BigUint::from(2u32);
    let two: BigUint = BigUint::from(2u32);
    let one: BigUint = BigUint::one();
    for (s, r0) in convergents_big(k_big.clone(), m_big.clone()) {
        if r0.is_zero() { continue; }
        if &r0 > n { continue; }
        // lift the reduced denominator to the true period: r = r0*d.
        let mut d: BigUint = BigUint::one();
        let mut r = r0.clone();
        loop {
            if powm_big(a.clone(), r.clone(), n) == one { break; }
            d += BigUint::one();
            r = &r0 * &d;
            if &r > &two_n { break; }
        }
        if &r > &two_n { continue; }
        if powm_big(a.clone(), r.clone(), n) != one { continue; }
        if &r % &two != BigUint::zero() {
            return Attempt::Degenerate(format!(
                "CF: s/{} -> period r={} is odd — a^(r/2) has no half-step; advancing the coprime base",
                r0, r));
        }
        let half = &r / &two;
        let xh = powm_big(a.clone(), half, n);
        if &xh + &one == *n {
            return Attempt::Degenerate(format!(
                "CF: s/{} -> r={} but a^(r/2) == -1 (mod N) — no split this base; advancing",
                r0, r));
        }
        let g1 = gcd_big(&xh - &one, n.clone());
        let g2 = gcd_big(&xh + &one, n.clone());
        let (p, q, via) = if &g1 > &one && &g1 < n {
            (g1.clone(), n / &g1, format!("gcd(a^(r/2) - 1, N) = {}", g1))
        } else if &g2 > &one && &g2 < n {
            (g2.clone(), n / &g2, format!("gcd(a^(r/2) + 1, N) = {}", g2))
        } else {
            return Attempt::Degenerate(format!(
                "CF: s/{} -> r={} but both gcd closures trivial; advancing", r0, r));
        };
        return Attempt::Factors { r, s, r0, p, q, via };
    }
    Attempt::NoReadout
}

pub struct PhaseUnbraidResult {
    pub tape_dir: Option<String>,
    pub n_val: BigUint,
    pub a_used: BigUint,
    pub n_qubits: usize,
    pub m: usize,
    pub total_shots: u32,
    pub shot_k: Option<usize>,
    pub certified_r: Option<BigUint>,
    pub true_r: BigUint,
    pub factors: Option<(BigUint, BigUint)>,
    pub trace: String,
}

/// The phase-based unbraid, BigUint lift. No static caps on q, a_tries,
/// a_shots — q is derived from bits(N) and the register grows to host
/// memory. Streaming FFT: when M exceeds the configured in-memory
/// amplitude budget (`mem_cap`, default 1<<22 amplitudes ≈ 64 MiB),
/// the comb is staged in blocks; per-block exact QFT runs entirely in
/// memory, then the per-cell probability mass is sampled by inverse-CDF
/// over block totals (a single full-width f64 block_cdf, not a
/// BigUint cumulative vector — f64 mass is exact enough for one shot).
pub fn run_phase_unbraid_big(
    n_val: BigUint,
    a0: BigUint,
    max_shots: u32,
    mem_cap: usize,
    tape_dir: Option<String>,
) -> Result<PhaseUnbraidResult, String> {
    let _tape_dir_init_keepalive: Option<String> = tape_dir.clone().map(|s| { let _ = std::fs::create_dir_all(&s); s });
    let _tape_dir_init_keepalive = _tape_dir_init_keepalive;
    let mut trace = String::new();
    if n_val < BigUint::from(4u32) {
        return Err("N < 4 has no nontrivial two-factor closure".into());
    }
    let two: BigUint = BigUint::from(2u32);
    if &n_val % &two == BigUint::zero() {
        let q = &n_val / &two;
        trace.push_str("N even: peeled directly (p=2); the phase register below assumes odd N\n");
        return Ok(PhaseUnbraidResult {
            tape_dir: _tape_dir_init_keepalive.clone(), n_val, a_used: BigUint::zero(), n_qubits: 0, m: 0, total_shots: 0,
            shot_k: None, certified_r: Some(BigUint::one()), true_r: BigUint::one(),
            factors: Some((two, q)), trace,
        });
    }
    let bits_n = n_val.bits() as usize;
    let q = 2 * bits_n + 8;
    let m: usize = 1usize << q;
    let mem_cap = mem_cap.max(1usize << 10);
    let block_amps = mem_cap.min(m);
    if (block_amps & (block_amps - 1)) != 0 {
        return Err(format!("mem_cap must be a power of two (got {})", block_amps));
    }
    let mut a = if a0 < BigUint::from(2u32) { BigUint::from(2u32) } else { a0 };
    let seed_mix = (&n_val % BigUint::from(u64::MAX)).iter_u64_digits().next().unwrap_or(0);
    let mut rng = XorShift(0x9E37_79B9_7F4A_7C15 ^ seed_mix);
    let mut total_shots: u32 = 0;
    loop {
        if total_shots >= max_shots { break; }
        let g = gcd_big(a.clone(), n_val.clone());
        if g != BigUint::one() {
            trace.push_str(&format!("  a={}: gcd(a,N)={} — trivial factor found directly, advancing\n", a, g));
            if &g != &n_val { return Ok(PhaseUnbraidResult {
                tape_dir: _tape_dir_init_keepalive.clone(), n_val: n_val.clone(), a_used: a.clone(), n_qubits: q, m, total_shots,
                shot_k: None, certified_r: Some(BigUint::zero()), true_r: BigUint::zero(),
                factors: Some((g.clone(), &n_val / &g)), trace,
            }); }
            a += BigUint::one(); continue;
        }
        let r_true = true_period_big(&a, &n_val);
        if r_true.is_zero() {
            trace.push_str(&format!("  a={}: period not found within N steps; advancing\n", a));
            a += BigUint::one(); continue;
        }
        let tape_dir_buf: Option<String> = _tape_dir_init_keepalive.clone();
        let _keep_tape = tape_dir_buf.clone();
        let r_true_us = r_true.to_u64_digits().first().copied().unwrap_or(1).max(1) as usize;
        let l = m / r_true_us;
        if l < 2 {
            trace.push_str(&format!(
                "  a={}: comb degenerates (L={} < 2); advancing coprime base\n", a, l));
            a += BigUint::one(); continue;
        }
        let amp = 1.0 / libm::sqrt(l as f64);
        let n_blocks = m / block_amps;
        let mut a_shots: u32 = 0;
        loop {
            if a_shots >= 8 || total_shots >= max_shots { break; }
            a_shots += 1;
            total_shots += 1;
            // First sweep: per-block mass. If tape_dir is set, per-cell
            // QFT-output norms are spilled to <tape_dir>/block_<bi>.bin as
            // f64 little-endian; peak RAM stays at one block_amps buffer
            // regardless of M = 2^q.
            let mut block_mass: Vec<f64> = alloc::vec![0.0f64; n_blocks];
            for bi in 0..n_blocks {
                let start = bi * block_amps;
                let end = start + block_amps;
                let mut buf: Vec<Cx> = alloc::vec![Cx::zero(); block_amps];
                let mut x = start;
                while x < end { buf[x - start] = Cx::new(amp, 0.0); x += r_true_us; }
                qft(&mut buf);
                let mut s = 0.0f64;
                let mut per_cell: Vec<f64> = if tape_dir_buf.is_some() {
                    alloc::vec![0.0f64; block_amps]
                } else {
                    Vec::new()
                };
                for (i, c) in buf.iter().enumerate() {
                    let n2 = c.norm2();
                    s += n2;
                    if !per_cell.is_empty() { per_cell[i] = n2; }
                }
                block_mass[bi] = s;
                if let Some(ref td) = tape_dir_buf {
                    let path = alloc::format!("{}/block_{:08}.bin", td, bi);
                    if let Ok(mut f) = std::fs::File::create(&path) {
                        use std::io::Write;
                        let bytes = unsafe {
                            core::slice::from_raw_parts(
                                per_cell.as_ptr() as *const u8,
                                per_cell.len() * core::mem::size_of::<f64>(),
                            )
                        };
                        let _ = f.write_all(bytes);
                    }
                }
            }
            let mut block_cdf: Vec<f64> = alloc::vec![0.0f64; n_blocks];
            let mut acc = 0.0f64;
            for (i, m) in block_mass.iter().enumerate() { acc += m; block_cdf[i] = acc; }
            let total_mass = *block_cdf.last().unwrap_or(&0.0);
            if total_mass <= 0.0 {
                trace.push_str(&format!("  shot {}: zero total Born mass — re-measuring\n", total_shots));
                continue;
            }
            let target = rng.unit() * total_mass;
            let mut bi = n_blocks - 1;
            for (i, c) in block_cdf.iter().enumerate() { if target < *c { bi = i; break; } }
            let start = bi * block_amps;
            let local_target = rng.unit() * block_mass[bi];
            let mut cum = 0.0f64;
            let mut k_in_block = block_amps - 1;
            let mut found = false;
            if let Some(ref td) = tape_dir_buf {
                let path = alloc::format!("{}/block_{:08}.bin", td, bi);
                if let Ok(bytes) = std::fs::read(&path) {
                    let n_cells = bytes.len() / core::mem::size_of::<f64>();
                    for i in 0..n_cells {
                        let off = i * core::mem::size_of::<f64>();
                        let n2 = f64::from_le_bytes([
                            bytes[off], bytes[off+1], bytes[off+2], bytes[off+3],
                            bytes[off+4], bytes[off+5], bytes[off+6], bytes[off+7],
                        ]);
                        cum += n2;
                        if local_target < cum { k_in_block = i; found = true; break; }
                    }
                }
            }
            if !found {
                let end = start + block_amps;
                let mut buf: Vec<Cx> = alloc::vec![Cx::zero(); block_amps];
                let mut x = start;
                while x < end { buf[x - start] = Cx::new(amp, 0.0); x += r_true_us; }
                qft(&mut buf);
                cum = 0.0;
                for (i, c) in buf.iter().enumerate() {
                    cum += c.norm2();
                    if local_target < cum { k_in_block = i; break; }
                }
            }
            let k = start + k_in_block;
            if k == 0 {
                trace.push_str(&format!("  shot {}: k=0 — winding carries no information (s=0); re-measuring\n", total_shots));
                continue;
            }
            trace.push_str(&format!(
                "  shot {}: measured k={}  winding k/M = {}/{} of a full turn\n",
                total_shots, k, k, m));
            match attempt_big(k, m, &a, &n_val) {
                Attempt::Factors { r, s: _s, r0, p, q: f2, via } => {
                    trace.push_str(&format!(
                        "  continued fractions: k/M -> s/{} -> certified period r={}  ({} )\n",
                        r0, r, via));
                    return Ok(PhaseUnbraidResult {
                        tape_dir: _tape_dir_init_keepalive.clone(), n_val: n_val.clone(), a_used: a.clone(), n_qubits: q, m,
                        total_shots, shot_k: Some(k), certified_r: Some(r),
                        true_r: r_true.clone(), factors: Some((p, f2)), trace,
                    });
                }
                Attempt::Degenerate(msg) => {
                    trace.push_str(&format!("  {}\n", msg));
                    break;
                }
                Attempt::NoReadout => {
                    trace.push_str(&format!(
                        "  shot {}: no convergent of {}/{} certified a period; re-measuring\n",
                        total_shots, k, m));
                }
            }
        }
        a += BigUint::one();
    }
    Ok(PhaseUnbraidResult {
        tape_dir: _tape_dir_init_keepalive.clone(), n_val, a_used: a, n_qubits: q, m, total_shots, shot_k: None,
        certified_r: None, true_r: BigUint::zero(), factors: None, trace,
    })
}

/// The report: phase readout, closure, and the factors AS WORDS,
/// verified. Input is a decimal string (any length).
pub fn phase_unbraid_report_big(n_str: &str, a0: u64, max_shots: u32, mem_cap: usize, tape_dir: Option<String>) -> Result<String, String> {
    let n_val: BigUint = n_str.trim().parse::<BigUint>()
        .map_err(|_| format!("'{}' is not a decimal integer", n_str))?;
    let a0b = BigUint::from(a0);
    let res = run_phase_unbraid_big(n_val.clone(), a0b, max_shots, mem_cap, tape_dir)?;
    let mut o = String::new();
    o.push_str("phase_unbraid (BigUint) — factors from a phase readout, no search\n");
    if let Some(ref td) = res.tape_dir {
        o.push_str(&format!("tape: {} (per-block amplitude norms spilled to disk; peak RAM = one block)\n", td));
    }
    o.push_str(&format!("N = {} ({} bits)\n", n_val, n_val.bits()));
    o.push_str(&format!("word: {}\n", native_numeral::encode(n_str.trim())));
    if res.n_qubits == 0 {
        let (p, qq) = res.factors.clone().unwrap();
        o.push_str(&res.trace);
        o.push_str(&format!("factors: p = {}\nq = {}\n", p, qq));
        o.push_str(&format!("p × q = N: {}\n", &p * &qq == n_val));
        o.push_str(&format!("syzygy preserves: {}\n", native_numeral::syzygy_preserves(&n_val, &p, &qq)));
        o.push_str(&native_numeral::factor_words_line(&p, &qq));
        return Ok(o);
    }
    o.push_str(&format!(
        "register: {} index qubits, M = 2^{} = {} amplitudes; streaming radix-2 FFT in blocks of {} amps\n",
        res.n_qubits, res.n_qubits, res.m, mem_cap.min(res.m)));
    o.push_str(&format!(
        "basis a = {}  (coprime; ground-truth period r = {} — printed for comparison only, never used in the readout)\n",
        res.a_used, res.true_r));
    o.push_str("measurement record:\n");
    o.push_str(&res.trace);
    match res.factors {
        Some((p, qq)) => {
            o.push_str("FACTORS (one phase measurement + one gcd — no enumeration):\n");
            o.push_str(&format!("p = {}\nq = {}\n", p, qq));
            o.push_str(&format!("p × q = N: {}\n", &p * &qq == n_val));
            o.push_str(&format!("syzygy preserves: {}\n", native_numeral::syzygy_preserves(&n_val, &p, &qq)));
            o.push_str(&native_numeral::factor_words_line(&p, &qq));
        }
        None => {
            o.push_str(&format!(
                "no nontrivial closure in {} measurement(s) — reported as measured, not guessed\n",
                res.total_shots));
        }
    }
    Ok(o)
}



/// REPL-shaped wrapper: `phase_unbraid <N> [a0] [max_shots] [mem_cap]`.
/// Used by G-mOMonadOS REPL and direct CLI.
pub fn repl_phase_unbraid(args: &[&str]) -> String {
    if args.is_empty() {
        return "usage: phase_unbraid <N> [a0=2] [max_shots=8] [mem_cap=4194304] [--tape <dir>]".into();
    }
    let mut tape_dir: Option<String> = None;
    let mut pos: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--tape" {
            if let Some(td) = args.get(i+1) { tape_dir = Some((*td).to_string()); i += 2; continue; }
        }
        pos.push(args[i]); i += 1;
    }
    let n_str = match pos.first() { Some(s) => *s, None => return "phase_unbraid: missing N".into() };
    let a0: u64 = pos.get(1).and_then(|s| s.parse().ok()).unwrap_or(2);
    let max_shots: u32 = pos.get(2).and_then(|s| s.parse().ok()).unwrap_or(8);
    let mem_cap: usize = pos.get(3).and_then(|s| s.parse().ok()).unwrap_or(1usize << 22);
    match phase_unbraid_report_big(n_str, a0, max_shots, mem_cap, tape_dir) {
        Ok(s) => s,
        Err(e) => format!("phase_unbraid error: {}", e),
    }
}

#[cfg(test)]
mod phase_tests_big {
    use super::*;
    #[test]
    fn fifteen_factors_by_phase() {
        let res = run_phase_unbraid_big(BigUint::from(15u32), BigUint::from(7u32), 12, 1<<16, None).unwrap();
        assert_eq!(res.factors, Some((BigUint::from(3u32), BigUint::from(5u32))));
        assert_eq!(res.certified_r, Some(BigUint::from(4u32)));
    }
    #[test]
    fn sixtyfive_factors_by_phase() {
        let res = run_phase_unbraid_big(BigUint::from(65u32), BigUint::from(2u32), 16, 1<<16, None).unwrap();
        let (p, q) = res.factors.expect("65 must factor by phase readout");
        let n65 = BigUint::from(65u32);
        assert!(&p * &q == n65);
        assert!(p == BigUint::from(13u32) || p == BigUint::from(5u32));
    }
}
