//! Hosted kernels from G-mOMonadOS/src/erdos_walks.rs (row_gcd, lcm_to_n).
//! A shared outer sieve prepares prime-power events; each readout is indexed.
pub struct PrimePowerMembrane {
    rows: Vec<u64>,
    lcms: Vec<Option<u128>>,
}

impl PrimePowerMembrane {
    pub fn new(limit: usize) -> Result<Self, String> {
        if limit > 1_000_000 { return Err("Prime-power table limit exceeds 1000000".into()); }
        let mut composite = vec![false; limit + 1];
        let mut rows = vec![1; limit + 1];
        rows[0] = 0;
        if limit >= 1 { rows[1] = 0; }
        for p in 2..=limit {
            if composite[p] { continue; }
            for multiple in (p..=limit).step_by(p) { composite[multiple] = true; }
            let mut power = p;
            loop {
                rows[power] = p as u64;
                match power.checked_mul(p) {
                    Some(next) if next <= limit => power = next,
                    _ => break,
                }
            }
        }
        let mut lcms = vec![Some(1u128); limit + 1];
        for n in 2..=limit {
            lcms[n] = lcms[n - 1].and_then(|value| value.checked_mul(rows[n] as u128));
        }
        Ok(Self { rows, lcms })
    }

    pub fn row_gcd(&self, n: usize) -> Option<u64> { self.rows.get(n).copied() }
    /// Outer None means outside the prepared domain; inner None means overflow.
    pub fn lcm(&self, n: usize) -> Option<Option<u128>> { self.lcms.get(n).copied() }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn gcd(mut a: u128, mut b: u128) -> u128 {
        while b != 0 { (a, b) = (b, a % b); } a
    }
    // Original binomial-row kernel, retaining its saturating u64 arithmetic.
    fn binom(n: u64, k: u64) -> u64 {
        if k > n { return 0; }
        let (mut num, mut den) = (1u64, 1u64);
        let kk = if k > n - k { n - k } else { k };
        for i in 0..kk {
            num = num.saturating_mul(n - i);
            den = den.saturating_mul(i + 1);
            let g = gcd(num as u128, den as u128) as u64;
            num /= g; den /= g;
        }
        num / den
    }
    fn row_gcd(n: u64) -> u64 {
        (1..n).fold(0, |g, k| gcd(g as u128, binom(n, k) as u128) as u64)
    }
    fn lcm_to_n(n: usize) -> Option<u128> {
        (2..=n).try_fold(1u128, |a, k| (a / gcd(a, k as u128)).checked_mul(k as u128))
    }
    #[test]
    fn exhaustive_controls_and_boundaries() {
        let table = PrimePowerMembrane::new(200).unwrap();
        for n in 0..=30 { assert_eq!(table.row_gcd(n), Some(row_gcd(n as u64))); }
        // Independent exact Pascal addition, avoiding the original's saturation.
        let mut row = vec![1u128];
        for n in 1..=120 {
            let mut next = vec![1u128; n + 1];
            for k in 1..n { next[k] = row[k-1] + row[k]; }
            let g = next[1..n].iter().copied().fold(0, gcd);
            assert_eq!(table.row_gcd(n), Some(g as u64));
            row = next;
        }
        for n in 0..=200 { assert_eq!(table.lcm(n), Some(lcm_to_n(n))); }
        for n in [81, 16, 30, 81, 0, 1] {
            assert_eq!(table.row_gcd(n), PrimePowerMembrane::new(n).unwrap().row_gcd(n));
        }
        assert_eq!(table.row_gcd(201), None);
        assert_eq!(table.lcm(201), None);
        assert_eq!(table.lcm(200), Some(None));
        assert!(PrimePowerMembrane::new(1_000_001).is_err());
    }
    #[test]
    fn timing_controls() {
        let repeats = std::hint::black_box(1000);
        let start = std::time::Instant::now();
        let mut original_rows = 0;
        for _ in 0..repeats { for n in 2..=30 { original_rows += row_gcd(std::hint::black_box(n)); } }
        let row_time = start.elapsed();
        let start = std::time::Instant::now();
        let mut original_lcms = 0u128;
        for _ in 0..repeats { for n in 0..=40 { original_lcms += lcm_to_n(std::hint::black_box(n)).unwrap(); } }
        let lcm_time = start.elapsed();
        let start = std::time::Instant::now();
        let table = PrimePowerMembrane::new(40).unwrap();
        let prepare = start.elapsed();
        let start = std::time::Instant::now();
        let mut rows = 0;
        for _ in 0..repeats { for n in 2..=30 { rows += table.row_gcd(std::hint::black_box(n)).unwrap(); } }
        let row_read = start.elapsed();
        let start = std::time::Instant::now();
        let mut lcms = 0;
        for _ in 0..repeats { for n in 0..=40 { lcms += table.lcm(std::hint::black_box(n)).unwrap().unwrap(); } }
        let lcm_read = start.elapsed();
        assert_eq!(rows, original_rows); assert_eq!(lcms, original_lcms);
        println!("prime-power repeats={repeats} prepare={prepare:?} original_rows={row_time:?} prepared_rows={row_read:?} original_lcms={lcm_time:?} prepared_lcms={lcm_read:?}");
    }
}
