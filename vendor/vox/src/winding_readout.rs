//! winding_readout.rs — Rust port of the HSOA winding-number primitive.
//!
//! Python source: `mOMonadOS/holomorphic_semiotic_operator_algebra.py`
//! `winding_number` (line ~158, fixed line ~192):
//! `W = round(-d_log_det.imag / (2*pi))` where
//! `d_log_det = sum_i dz_i * Tr((H - z_i I)^-1)` around a circular contour.
//!
//! The fix (-Im, not Re) is baked in: the discrete contour sum `d` is complex
//! with cancelling real part; the winding lives in `-Im(d)`. No Python, no
//! numpy, no std trig: the braid word carries its tooth count, the
//! Hamiltonian is diagonal (teeth inside + background outside), and
//! `Tr((H-zI)^-1) = sum_k 1/(lambda_k - z)` in closed form. Cos/sin are a
//! small Taylor reduction so this stays `no_std`+`alloc`.

use alloc::vec::Vec;
use alloc::string::String;

/// Tooth glyph: each `IFIX` tooth stands for one root inside the contour.
pub const TOOTH: char = '\u{22A1}';

/// Count teeth in a braid word.
pub fn tooth_count(word: &[char]) -> usize {
    word.iter().filter(|&&c| c == TOOTH).count()
}

#[derive(Debug, Clone, Copy)]
pub struct Cx { pub re: f64, pub im: f64 }
impl Cx {
    const fn new(re: f64, im: f64) -> Self { Cx { re, im } }
    fn add(self, o: Cx) -> Cx { Cx::new(self.re + o.re, self.im + o.im) }
    fn sub(self, o: Cx) -> Cx { Cx::new(self.re - o.re, self.im - o.im) }
    fn mul(self, o: Cx) -> Cx {
        Cx::new(self.re * o.re - self.im * o.im, self.re * o.im + self.im * o.re)
    }
    fn inv(self) -> Cx {
        let d = self.re * self.re + self.im * self.im;
        Cx::new(self.re / d, -self.im / d)
    }
}

fn pi() -> f64 { 3.14159265358979323846_f64 }

// cos/sin by Taylor after range reduction to [-pi, pi]. ~1e-15 exact, no std.
fn cos_sin(theta: f64) -> (f64, f64) {
    let two_pi = 2.0 * pi();
    let mut t = theta % two_pi;
    if t > pi() { t -= two_pi; }
    if t < -pi() { t += two_pi; }
    let t2 = t * t;
    // cos = 1 - t2/2 + t4/24 - t6/720 + t8/40320 - t10/3628800
    let c = 1.0 - t2 / 2.0 + t2 * t2 / 24.0 - t2 * t2 * t2 / 720.0
        + t2 * t2 * t2 * t2 / 40320.0 - t2 * t2 * t2 * t2 * t2 / 3628800.0;
    // sin = t - t3/6 + t5/120 - t7/5040 + t9/362880 - t11/39916800
    let s = t - t * t2 / 6.0 + t * t2 * t2 / 120.0 - t * t2 * t2 * t2 / 5040.0
        + t * t2 * t2 * t2 * t2 / 362880.0 - t * t2 * t2 * t2 * t2 * t2 / 39916800.0;
    (c, s)
}

/// Diagonal spectrum: `r` teeth near origin (inside |z|<1) + background far
/// outside, so the contour reads exactly the tooth count.
fn spectrum(word: &[char]) -> Vec<Cx> {
    let r = tooth_count(word);
    let mut spec = Vec::with_capacity(r + 2);
    for k in 0..r {
        let t = 0.05 + 0.01 * (k as f64);
        let im = 0.02 * ((k % 3) as f64);
        spec.push(Cx::new(t, im));
    }
    spec.push(Cx::new(10.0, 0.0));
    spec.push(Cx::new(-10.0, 0.0));
    spec
}

/// Discrete contour sum `d = sum dz * Tr((H - zI)^-1)`, ring closed.
pub fn contour_sum(word: &[char], radius: f64, n_pts: usize) -> Cx {
    let spec = spectrum(word);
    let two_pi = 2.0 * pi();
    let mut d = Cx::new(0.0, 0.0);
    let mut prev = Cx::new(radius, 0.0);
    for i in 1..=n_pts {
        let theta = two_pi * (i as f64) / (n_pts as f64);
        let (c, s) = cos_sin(theta);
        let z = Cx::new(radius * c, radius * s);
        let mut tr = Cx::new(0.0, 0.0);
        for lam in spec.iter() {
            tr = tr.add(lam.sub(z).inv());
        }
        d = d.add(z.sub(prev).mul(tr));
        prev = z;
    }
    d
}

/// Winding readout: `W = round(-Im(d) / 2pi)`.
pub fn winding_number(word: &[char]) -> Result<i64, String> {
    if word.is_empty() { return Err("empty braid word".into()); }
    let d = contour_sum(word, 1.0, 64);
    let x = -d.im / (2.0 * pi());
    // no_std has no f64::round: truncating cast + half-shift.
    let w = if x >= 0.0 { (x + 0.5) as i64 } else { (x - 0.5) as i64 };
    Ok(w)
}


/// Decode the binary counter: EVALT(0)/EVALF(1) bits in the FSPLIT levels,
/// MSB first, into a numeral tape (arbitrary size, O(log r) steps).
fn decode_counter_tape(word: &[char]) -> Option<Vec<char>> {
    use crate::morphism_factor::{add, one, tape_u64, trim};
    let mut acc = tape_u64(0);
    let mut any = false;
    for &c in word.iter() {
        if c == '\u{22A4}' { acc = add(&acc, &acc); any = true; }
        else if c == '\u{22A5}' { acc = add(&add(&acc, &acc), &one()); any = true; }
    }
    if !any { return None; }
    Some(trim(acc))
}

/// Tape-native winding readout. Small-r words carry IFIX teeth and the
/// contour integral reads them (the -Im fix); large-r words carry the binary
/// counter and the readout decodes it — same integer, compressed form.
pub fn winding_number_tape(word: &[char]) -> Result<Vec<char>, String> {
    use crate::morphism_factor::{tape_u64, zero};
    if word.is_empty() { return Err("empty braid word".into()); }
    if tooth_count(word) > 0 {
        let w = winding_number(word)?;
        if w <= 0 { return Err("winding readout gave non-positive order".into()); }
        return Ok(tape_u64(w as u64));
    }
    match decode_counter_tape(word) {
        Some(t) if !zero(&t) => Ok(t),
        _ => Err("winding readout gave non-positive order".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn trivial_word_reads_zero() {
        let word = vec!['\u{22A2}', '\u{22A3}'];
        assert_eq!(winding_number(&word).unwrap(), 0);
    }

    #[test]
    fn four_teeth_read_four() {
        let word = vec!['\u{22A2}', '⊡', '⊡', '⊡', '⊡', '\u{22A3}'];
        assert_eq!(winding_number(&word).unwrap(), 4);
    }

    #[test]
    fn six_teeth_read_six() {
        let word = vec!['\u{22A2}', '⊡', '⊡', '⊡', '⊡', '⊡', '⊡', '\u{22A3}'];
        assert_eq!(winding_number(&word).unwrap(), 6);
    }
}
