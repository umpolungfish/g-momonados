//! Schütte search extracted from G-mOMonadOS/src/erdos_walks.rs.
// Original arithmetic search retained as the independent test control.
#[cfg(test)]
/// A tournament on `n` vertices from a quadratic-residue rule: `i` beats `j`
/// when `j − i` is a nonzero square mod `n`. For `n ≡ 3 (mod 4)` this is
/// antisymmetric, which is what makes the Paley construction a tournament.
fn paley_beats(n: u64, i: u64, j: u64) -> bool {
    if i == j { return false; }
    let d = (j + n - i) % n;
    let mut is_sq = false;
    let mut k = 1u64;
    while k < n {
        if (k * k) % n == d { is_sq = true; break; }
        k += 1;
    }
    is_sq
}

/// Does every `k`-subset have a common dominator? That is the Schütte property.
#[cfg(test)]
fn has_property(n: u64, k: u32) -> bool {
    // Every k-subset, by bitmask over n < 64 vertices.
    let total = 1u64 << n;
    let mut mask = 0u64;
    while mask < total {
        if (mask.count_ones()) == k {
            let mut dominated = false;
            let mut v = 0u64;
            while v < n {
                if (mask >> v) & 1 == 0 {
                    let mut all = true;
                    let mut u = 0u64;
                    while u < n {
                        if (mask >> u) & 1 == 1 && !paley_beats(n, v, u) { all = false; break; }
                        u += 1;
                    }
                    if all { dominated = true; break; }
                }
                v += 1;
            }
            if !dominated { return false; }
        }
        mask += 1;
    }
    true
}

pub struct SchutteMembrane {
    n: u32,
    dominators: Vec<u64>,
}

#[derive(Debug)]
pub struct Reading {
    pub holds: bool,
    pub subsets_checked: u64,
    pub counterexample: Option<u64>,
}

impl SchutteMembrane {
    pub fn new(n: u32) -> Result<Self, String> {
        if n == 0 || n >= 64 { return Err("Schutte vertex count must be in 1..=63".into()); }
        let mut residues = vec![false; n as usize];
        for k in 1..n { residues[((k * k) % n) as usize] = true; }
        let dominators = (0..n).map(|u| {
            let mut mask = 0;
            for v in 0..n {
                if u != v && residues[((u + n - v) % n) as usize] {
                    mask |= 1u64 << v;
                }
            }
            mask
        }).collect();
        Ok(Self { n, dominators })
    }

    pub fn check(&self, k: u32) -> Reading {
        let mut reading = Reading { holds: true, subsets_checked: 0, counterexample: None };
        if k > self.n { return reading; }
        let limit = 1u64 << self.n;
        let mut subset = (1u64 << k) - 1;
        loop {
            reading.subsets_checked += 1;
            let mut candidates = (limit - 1) & !subset;
            let mut members = subset;
            while members != 0 && candidates != 0 {
                let u = members.trailing_zeros() as usize;
                candidates &= self.dominators[u];
                members &= members - 1;
            }
            if candidates == 0 {
                reading.holds = false;
                reading.counterexample = Some(subset);
                return reading;
            }
            if k == 0 { break; }
            // Next mask of the same cardinality, in the original mask order.
            let low = subset & subset.wrapping_neg();
            let ripple = subset + low;
            let next = ripple | (((ripple ^ subset) >> 2) / low);
            if next >= limit { break; }
            subset = next;
        }
        reading
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn membrane_matches_original_search() {
        for n in 1..=12 {
            let membrane = SchutteMembrane::new(n).unwrap();
            for k in 0..=n+1 {
                let reading = membrane.check(k);
                assert_eq!(reading.holds, has_property(n as u64, k), "n={n} k={k}");
                if let Some(mask) = reading.counterexample {
                    assert_eq!(mask.count_ones(), k);
                    assert!(!(0..n).any(|v| (mask >> v) & 1 == 0 &&
                        (0..n).all(|u| (mask >> u) & 1 == 0 || paley_beats(n as u64, v as u64, u as u64))));
                }
            }
        }
        assert!(SchutteMembrane::new(0).is_err());
        assert!(SchutteMembrane::new(64).is_err());
        assert!(SchutteMembrane::new(63).unwrap().check(0).holds);
        assert!(!SchutteMembrane::new(63).unwrap().check(63).holds);
    }

    #[test]
    fn membrane_timing_control() {
        let n = std::hint::black_box(23);
        let start = std::time::Instant::now();
        let reference = has_property(n, 2);
        let old = start.elapsed();
        let start = std::time::Instant::now();
        let reading = SchutteMembrane::new(n as u32).unwrap().check(2);
        let nested = start.elapsed();
        assert_eq!(reading.holds, reference);
        assert!(reading.holds);
        assert_eq!(reading.subsets_checked, 253);
        println!("schutte n=23 k=2 original={old:?} nested_with_setup={nested:?} subsets={}", reading.subsets_checked);
    }
}

