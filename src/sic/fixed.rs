//! Deterministic fixed-point FFT and WH overlap tapes. Same WH convention as wh.
use super::{error, SicError};
use crate::phase_unbraid::{FixedComplex, FixedPointFormat};
use num_bigint::{BigInt, BigUint};
use num_traits::Zero;
fn zero() -> FixedComplex {
    FixedComplex {
        re: BigInt::zero(),
        im: BigInt::zero(),
    }
}
fn phase(n: BigInt, d: usize, f: &FixedPointFormat) -> Result<FixedComplex, SicError> {
    FixedComplex::winding_twiddle(&n, &BigUint::from(d), f).map_err(|e| SicError(e.to_string()))
}
fn add(a: &FixedComplex, b: &FixedComplex) -> FixedComplex {
    FixedComplex {
        re: &a.re + &b.re,
        im: &a.im + &b.im,
    }
}
fn sub(a: &FixedComplex, b: &FixedComplex) -> FixedComplex {
    FixedComplex {
        re: &a.re - &b.re,
        im: &a.im - &b.im,
    }
}
fn radix(a: &mut [FixedComplex], inverse: bool, f: &FixedPointFormat) -> Result<(), SicError> {
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
        let root = phase(BigInt::from(if inverse { 1 } else { -1 }), span, f)?;
        for start in (0..n).step_by(span) {
            let mut w = FixedComplex {
                re: f.scale(),
                im: BigInt::zero(),
            };
            for k in 0..span / 2 {
                let u = a[start + k].clone();
                let v = a[start + k + span / 2].mul(&w, f);
                a[start + k] = add(&u, &v);
                a[start + k + span / 2] = sub(&u, &v);
                w = w.mul(&root, f);
            }
        }
        span *= 2;
    }
    if inverse {
        for z in a {
            z.re /= n;
            z.im /= n;
        }
    }
    Ok(())
}
pub fn fft_positive(
    input: &[FixedComplex],
    f: &FixedPointFormat,
) -> Result<Vec<FixedComplex>, SicError> {
    let n = input.len();
    if n == 0 {
        return Err(error("empty fixed FFT tape"));
    }
    if n.is_power_of_two() {
        let mut a: Vec<_> = input
            .iter()
            .map(|z| FixedComplex {
                re: z.re.clone(),
                im: -&z.im,
            })
            .collect();
        radix(&mut a, false, f)?;
        for z in &mut a {
            z.im = -&z.im;
        }
        return Ok(a);
    }
    let length = n
        .checked_mul(2)
        .and_then(|x| x.checked_sub(1))
        .and_then(|x| x.checked_next_power_of_two())
        .ok_or_else(|| error("fixed FFT width overflow"))?;
    let mut a = vec![zero(); length];
    let mut b = a.clone();
    for k in 0..n {
        let chirp = phase(BigInt::from(k) * BigInt::from(k), 2 * n, f)?;
        a[k] = input[k].mul(&chirp, f);
        b[k] = FixedComplex {
            re: chirp.re,
            im: -chirp.im,
        };
        if k != 0 {
            b[length - k] = b[k].clone();
        }
    }
    radix(&mut a, false, f)?;
    radix(&mut b, false, f)?;
    for k in 0..length {
        a[k] = a[k].mul(&b[k], f);
    }
    radix(&mut a, true, f)?;
    (0..n)
        .map(|k| Ok(a[k].mul(&phase(BigInt::from(k) * BigInt::from(k), 2 * n, f)?, f)))
        .collect()
}
pub fn wh_overlaps(
    fiducial: &[FixedComplex],
    f: &FixedPointFormat,
) -> Result<Vec<FixedComplex>, SicError> {
    let d = fiducial.len();
    if d < 2 {
        return Err(error("invalid fixed WH dimension"));
    }
    let mut out = Vec::with_capacity(d * d);
    for p in 0..d {
        let tape: Vec<_> = (0..d)
            .map(|n| {
                FixedComplex {
                    re: fiducial[n].re.clone(),
                    im: -&fiducial[n].im,
                }
                .mul(&fiducial[(n + d - p) % d], f)
            })
            .collect();
        for (q, z) in fft_positive(&tape, f)?.into_iter().enumerate() {
            out.push(z.mul(
                &phase(
                    -BigInt::from(d + 1) * BigInt::from(p) * BigInt::from(q),
                    2 * d,
                    f,
                )?,
                f,
            ));
        }
    }
    Ok(out)
}
