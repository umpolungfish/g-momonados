//! Rep-tiling membrane extracted from G-mOMonadOS/src/erdos_walks.rs.
//! Prepare square membership once, then answer all baked n values by lookup.
pub struct RepTilingMembrane { admissible: Vec<bool> }

impl RepTilingMembrane {
    pub fn new(limit: usize) -> Result<Self, String> {
        if limit > 2_000_000 { return Err("rep-tiling limit exceeds 2000000".into()); }
        let mut square = vec![false; limit + 1];
        let mut r = 0usize;
        while r <= limit / r.max(1) {
            let Some(value) = r.checked_mul(r) else { break };
            if value > limit { break; }
            square[value] = true;
            r += 1;
        }
        let mut admissible = vec![false; limit + 1];
        for n in 0..=limit {
            admissible[n] = square[n]
                || (n % 3 == 0 && square[n / 3])
                || (1..=n).any(|a| square[a] && a > 0 && {
                    let rest = n - a;
                    rest > 0 && square[rest]
                });
        }
        Ok(Self { admissible })
    }
    pub fn at(&self, n: usize) -> Option<bool> { self.admissible.get(n).copied() }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn isqrt(n: u64) -> u64 {
        let mut r = (n as f64).sqrt() as u64;
        while (r + 1).checked_mul(r + 1).is_some_and(|v| v <= n) { r += 1; }
        while r * r > n { r -= 1; }
        r
    }
    fn reference(n: u64) -> bool {
        let sq = |v| isqrt(v) * isqrt(v) == v;
        sq(n) || (n % 3 == 0 && sq(n / 3))
            || (1..=n).any(|a| sq(a) && n > a && sq(n-a))
    }
    #[test]
    fn source_classification_and_boundaries() {
        let membrane = RepTilingMembrane::new(1000).unwrap();
        for n in 0..=1000 { assert_eq!(membrane.at(n), Some(reference(n as u64)), "n={n}"); }
        assert_eq!(membrane.at(1), Some(true));
        assert_eq!(membrane.at(6), Some(false));
        assert_eq!(membrane.at(1001), None);
        assert!(RepTilingMembrane::new(2_000_001).is_err());
    }
    #[test]
    fn timing_control() {
        let queries: Vec<usize> = (1..=1000).collect();
        let start = std::time::Instant::now();
        let original: Vec<_> = queries.iter().map(|&n| reference(n as u64)).collect();
        let old = start.elapsed();
        let start = std::time::Instant::now();
        let membrane = RepTilingMembrane::new(1000).unwrap();
        let prepared: Vec<_> = queries.iter().map(|&n| membrane.at(n).unwrap()).collect();
        let new = start.elapsed();
        assert_eq!(original, prepared);
        println!("rep-tiling queries=1000 original={old:?} prepared={new:?}");
    }
}
