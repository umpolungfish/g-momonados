//! Frozen convention: tau=-exp(pi i/d), D[p,q]=tau^(pq) X^p Z^q.
//! Consequently <psi|D|psi>=tau^(-pq) DFT_+[conj(psi[n])psi[n-p]].
use super::frame::{Complex, Operator, Sic};
use super::{error, SicError};
use std::f64::consts::PI;

pub(crate) fn turn_phase(numerator: u128, denominator: u128) -> Complex {
    Complex::phase(2.0 * PI * ((numerator % denominator) as f64) / (denominator as f64))
}
pub(crate) fn chirp(k: usize, n: usize) -> Complex {
    turn_phase((k as u128) * (k as u128), 2 * n as u128)
}

fn radix_fft(a: &mut [Complex], inverse: bool) {
    let n = a.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            a.swap(i, j);
        }
    }
    let mut span = 2;
    while span <= n {
        let root = Complex::phase((if inverse { 2.0 } else { -2.0 }) * PI / span as f64);
        for start in (0..n).step_by(span) {
            let mut w = Complex::new(1.0, 0.0);
            for k in 0..span / 2 {
                let u = a[start + k];
                let v = a[start + k + span / 2] * w;
                a[start + k] = u + v;
                a[start + k + span / 2] = u - v;
                w = w * root;
            }
        }
        span *= 2;
    }
    if inverse {
        for z in a {
            *z = z.scale(1.0 / n as f64);
        }
    }
}

/// Positive-sign, unnormalized FFT for every length, via Bluestein convolution.
pub fn fft_positive(input: &[Complex]) -> Result<Vec<Complex>, SicError> {
    let n = input.len();
    if n == 0 {
        return Err(error("empty FFT tape"));
    }
    if input.iter().any(|z| !z.finite()) {
        return Err(error("invalid FFT scalar"));
    }
    if n.is_power_of_two() {
        let mut a: Vec<_> = input.iter().map(|z| z.conj()).collect();
        radix_fft(&mut a, false);
        for z in &mut a {
            *z = z.conj();
        }
        return Ok(a);
    }
    let length = n
        .checked_mul(2)
        .and_then(|x| x.checked_sub(1))
        .and_then(|x| x.checked_next_power_of_two())
        .ok_or_else(|| error("FFT width overflow"))?;
    let mut a = vec![Complex::default(); length];
    let mut b = a.clone();
    for k in 0..n {
        let phase = chirp(k, n);
        a[k] = input[k] * phase;
        b[k] = phase.conj();
        if k != 0 {
            b[length - k] = b[k];
        }
    }
    radix_fft(&mut a, false);
    radix_fft(&mut b, false);
    for k in 0..length {
        a[k] = a[k] * b[k];
    }
    radix_fft(&mut a, true);
    Ok((0..n).map(|k| a[k] * chirp(k, n)).collect())
}

pub struct WhSic {
    d: usize,
    fiducial: Vec<Complex>,
}
pub struct OverlapField {
    pub dimension: usize,
    pub values: Vec<Complex>,
}
impl WhSic {
    pub fn new(fiducial: Vec<Complex>) -> Result<Self, SicError> {
        if fiducial.len() < 2
            || fiducial.len().checked_mul(fiducial.len()).is_none()
            || fiducial.iter().any(|z| !z.finite())
        {
            return Err(error("invalid WH fiducial"));
        }
        Ok(Self {
            d: fiducial.len(),
            fiducial,
        })
    }
    pub fn fiducial(&self) -> &[Complex] {
        &self.fiducial
    }
    pub(crate) fn phase(&self, p: usize, q: usize) -> Complex {
        turn_phase(
            (self.d as u128 + 1) * (p as u128) * (q as u128),
            2 * self.d as u128,
        )
    }
    pub fn displaced(&self, p: usize, q: usize) -> Result<Vec<Complex>, SicError> {
        if p >= self.d || q >= self.d {
            return Err(error("WH displacement index out of range"));
        }
        Ok((0..self.d)
            .map(|n| {
                let k = (n + self.d - p) % self.d;
                self.phase(p, q)
                    * turn_phase((q as u128) * (k as u128), self.d as u128)
                    * self.fiducial[k]
            })
            .collect())
    }
    pub fn overlaps(&self) -> Result<OverlapField, SicError> {
        let mut values = Vec::with_capacity(self.d * self.d);
        for p in 0..self.d {
            let tape: Vec<_> = (0..self.d)
                .map(|n| self.fiducial[n].conj() * self.fiducial[(n + self.d - p) % self.d])
                .collect();
            let row = fft_positive(&tape)?;
            values.extend(
                row.into_iter()
                    .enumerate()
                    .map(|(q, z)| z * self.phase(p, q).conj()),
            );
        }
        Ok(OverlapField {
            dimension: self.d,
            values,
        })
    }
}
impl Sic for WhSic {
    fn dimension(&self) -> usize {
        self.d
    }
    fn projector(&self, i: usize) -> Result<Operator, SicError> {
        if i >= self.d * self.d {
            return Err(error("WH vertex index out of range"));
        }
        Operator::projector(&self.displaced(i / self.d, i % self.d)?)
    }
}
