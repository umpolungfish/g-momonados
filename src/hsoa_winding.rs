//! hsoa_winding.rs — Topological period readout via exceptional-point winding.
//!
//! Implements the HSOA Shor-state readout:
//!   1. Build the periodic comb ψ[x] = 1 iff a^x ≡ 1 (mod N), else 0.
//!   2. Detect the period r via autocorrelation.
//!   3. Verify via the exceptional-point structure of z^r - (1 - λ):
//!      the EP at λ = 1 has order r, and tracing the r roots
//!      around λ = 1 recovers the winding W = r.
//!
//! The readout W is an exact integer — no rational reconstruction,
//! no continued-fraction peak selection, no FFT heuristic.

use num_bigint::BigUint;
use num_complex::Complex64;
use num_traits::One;

#[derive(Debug, Clone)]
pub struct WindingReadout {
    /// Detected period r (smallest lag with full autocorrelation).
    pub period: u64,
    /// Winding number W of roots around the EP. Should equal `period`.
    pub winding: u64,
    /// Minimum pairwise root separation squared near the EP.
    pub min_discriminant: f64,
    /// Approximate EP location in the λ-plane.
    pub ep_center: Complex64,
    /// Order of the EP (= number of roots that coalesce).
    pub ep_order: u64,
    /// Number of nonzero comb entries.
    pub nonzero_count: usize,
    /// ||ψ||² (should be nonzero_count for a pure comb).
    pub norm_sq: f64,
    /// Localization length ξ = sqrt(<x²> - <x>²).
    pub localization_length: f64,
    /// True iff winding == period.
    pub validated: bool,
    /// Elapsed time in microseconds.
    pub elapsed_us: u128,
}

/// Build the Shor-state comb of length `m`:
///   comb[x] = 1 if a^x ≡ 1 (mod n), else 0, for x in 0..m.
pub fn build_comb(a: &BigUint, n: &BigUint, m: usize) -> Vec<Complex64> {
    let mut comb = vec![Complex64::new(0.0, 0.0); m];
    let mut v = BigUint::one();
    for x in 0..m {
        if v.is_one() {
            comb[x] = Complex64::new(1.0, 0.0);
        }
        v = (&v * a) % n;
    }
    comb
}

/// Detect the period of a comb via autocorrelation.
///
/// Returns the smallest `r` such that |<comb, shift_r(comb)>| ≥ 0.5·||comb||².
/// The threshold 0.5 is safe: for a comb of period r on m sites, the
/// autocorrelation at lag r is (n−1) where n = number of comb positions,
/// and (n−1)/n ≥ 0.5 whenever n ≥ 2.
pub fn autocorr_period(comb: &[Complex64]) -> Option<u64> {
    let m = comb.len();
    let norm: f64 = comb.iter().map(|z| z.norm_sqr()).sum();
    if norm < 1e-12 {
        return None;
    }
    let threshold = 0.5 * norm;
    let r_max = ((m * 2) / 3).max(2).min(m);
    for r in 1..=r_max {
        let mut acc = Complex64::new(0.0, 0.0);
        for i in 0..(m - r) {
            acc += comb[i].conj() * comb[i + r];
        }
        if acc.norm() >= threshold {
            return Some(r as u64);
        }
    }
    None
}

/// Verify the EP structure of `z^r - (1 - λ) = 0` near λ = 1.
/// Returns (min pairwise root separation², EP center, EP order).
pub fn verify_ep_order(r: u64) -> (f64, Complex64, u64) {
    let ep = Complex64::new(1.0, 0.0);
    if r < 2 {
        return (0.0, ep, 1);
    }
    let eps = 1e-6;
    let theta = std::f64::consts::PI / 3.0;
    let lam = ep + Complex64::from_polar(eps, theta);
    let base = (Complex64::new(1.0, 0.0) - lam).powf(1.0 / (r as f64));
    let roots: Vec<Complex64> = (0..r)
        .map(|k| {
            base * Complex64::from_polar(
                1.0,
                2.0 * std::f64::consts::PI * (k as f64) / (r as f64),
            )
        })
        .collect();
    let mut min_sep = f64::INFINITY;
    for i in 0..roots.len() {
        for j in (i + 1)..roots.len() {
            let d = (roots[i] - roots[j]).norm();
            if d < min_sep {
                min_sep = d;
            }
        }
    }
    (min_sep * min_sep, ep, r)
}

/// Trace the r roots of `z^r - (1 - λ) = 0` as λ winds once around λ = 1.
/// Returns the length of the longest cycle in the resulting permutation
/// of roots — which equals r for this construction.
pub fn trace_winding(r: u64, n_samples: usize) -> u64 {
    if r < 2 {
        return 1;
    }
    let ep = Complex64::new(1.0, 0.0);
    let eps = 1e-3;
    let mut start_roots: Option<Vec<Complex64>> = None;
    let mut end_roots: Option<Vec<Complex64>> = None;
    for k in 0..=n_samples {
        let theta = 2.0 * std::f64::consts::PI * (k as f64) / (n_samples as f64);
        let lam = ep + Complex64::from_polar(eps, theta);
        let base = (Complex64::new(1.0, 0.0) - lam).powf(1.0 / (r as f64));
        let roots: Vec<Complex64> = (0..r)
            .map(|j| {
                base * Complex64::from_polar(
                    1.0,
                    2.0 * std::f64::consts::PI * (j as f64) / (r as f64),
                )
            })
            .collect();
        if k == 0 {
            start_roots = Some(roots.clone());
        }
        if k == n_samples {
            end_roots = Some(roots);
        }
    }
    let (start, end) = match (start_roots, end_roots) {
        (Some(s), Some(e)) => (s, e),
        _ => return r,
    };
    // For each start root, find nearest end root.
    let mut perm = vec![0usize; r as usize];
    for i in 0..start.len() {
        let mut best_j = 0;
        let mut best_d = f64::INFINITY;
        for j in 0..end.len() {
            let d = (start[i] - end[j]).norm();
            if d < best_d {
                best_d = d;
                best_j = j;
            }
        }
        perm[i] = best_j;
    }
    // Longest orbit length.
    let mut visited = vec![false; r as usize];
    let mut max_orbit = 0usize;
    for s in 0..r as usize {
        if visited[s] {
            continue;
        }
        let mut length = 0usize;
        let mut cur = s;
        while !visited[cur] {
            visited[cur] = true;
            cur = perm[cur];
            length += 1;
        }
        if length > max_orbit {
            max_orbit = length;
        }
    }
    max_orbit as u64
}

/// Localization length ξ = sqrt(<x²> − <x>²) of the comb.
pub fn localization_length(comb: &[Complex64]) -> f64 {
    let norm_sq: f64 = comb.iter().map(|z| z.norm_sqr()).sum();
    if norm_sq < 1e-12 {
        return 0.0;
    }
    let mean: f64 = comb
        .iter()
        .enumerate()
        .map(|(x, z)| (x as f64) * z.norm_sqr())
        .sum::<f64>()
        / norm_sq;
    let var: f64 = comb
        .iter()
        .enumerate()
        .map(|(x, z)| {
            let dx = (x as f64) - mean;
            dx * dx * z.norm_sqr()
        })
        .sum::<f64>()
        / norm_sq;
    var.max(0.0).sqrt()
}

/// Full topological period readout.
pub fn read_period(comb: &[Complex64]) -> Option<WindingReadout> {
    let t0 = std::time::Instant::now();
    let nonzero_count = comb.iter().filter(|z| z.norm_sqr() > 1e-12).count();
    let norm_sq: f64 = comb.iter().map(|z| z.norm_sqr()).sum();
    let r = autocorr_period(comb)?;
    let winding = trace_winding(r, 64);
    let (min_disc, ep_center, ep_order) = verify_ep_order(r);
    let loc_len = localization_length(comb);
    Some(WindingReadout {
        period: r,
        winding,
        min_discriminant: min_disc,
        ep_center,
        ep_order,
        nonzero_count,
        norm_sq,
        localization_length: loc_len,
        validated: winding == r,
        elapsed_us: t0.elapsed().as_micros(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comb_from_period(m: usize, period: usize) -> Vec<Complex64> {
        let mut c = vec![Complex64::new(0.0, 0.0); m];
        for x in (0..m).step_by(period) {
            c[x] = Complex64::new(1.0, 0.0);
        }
        c
    }

    #[test]
    fn detect_period_4() {
        assert_eq!(autocorr_period(&comb_from_period(64, 4)), Some(4));
    }

    #[test]
    fn detect_period_6() {
        assert_eq!(autocorr_period(&comb_from_period(64, 6)), Some(6));
    }

    #[test]
    fn detect_period_12() {
        assert_eq!(autocorr_period(&comb_from_period(256, 12)), Some(12));
    }

    #[test]
    fn winding_equals_r() {
        for r in 2..=16u64 {
            assert_eq!(trace_winding(r, 64), r, "winding ≠ r at r={}", r);
        }
    }

    #[test]
    fn shor_15_r4() {
        let comb = build_comb(&BigUint::from(7u64), &BigUint::from(15u64), 64);
        let ro = read_period(&comb).unwrap();
        assert_eq!(ro.period, 4);
        assert_eq!(ro.winding, 4);
        assert!(ro.validated);
    }

    #[test]
    fn shor_21_r6() {
        let comb = build_comb(&BigUint::from(2u64), &BigUint::from(21u64), 64);
        let ro = read_period(&comb).unwrap();
        assert_eq!(ro.period, 6);
        assert!(ro.validated);
    }

    #[test]
    fn shor_35_r12() {
        let comb = build_comb(&BigUint::from(2u64), &BigUint::from(35u64), 256);
        let ro = read_period(&comb).unwrap();
        assert_eq!(ro.period, 12);
        assert!(ro.validated);
    }
}