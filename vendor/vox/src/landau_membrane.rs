//! landau_membrane kernel extracted from G-mOMonadOS/src/erdos_walks.rs.
/// Each prime owns one stage. Its powers are alternatives inside that stage;
/// all capacities consume the preceding stage, so a prime is selected once.
/// Distinct prime powers realize the LCM at their summed partition cost.
pub struct LandauMembrane {
    maxima: Vec<u128>,
}

impl LandauMembrane {
    pub fn new(limit: usize) -> Result<Self, String> {
        let len = limit.checked_add(1).ok_or("Landau table size overflow")?;
        let mut maxima = Vec::new();
        maxima.try_reserve_exact(len).map_err(|_| "Landau table allocation failed")?;
        maxima.resize(len, 1u128);
        let mut composite = Vec::new();
        composite.try_reserve_exact(len).map_err(|_| "Prime table allocation failed")?;
        composite.resize(len, false);
        for p in 2..=limit {
            if composite[p] { continue; }
            for m in (p..=limit).step_by(p) { composite[m] = true; }
            let previous = maxima.clone();
            let mut power = p;
            loop {
                for capacity in power..=limit {
                    let value = previous[capacity - power].checked_mul(power as u128)
                        .ok_or("Landau value exceeds u128")?;
                    maxima[capacity] = maxima[capacity].max(value);
                }
                match power.checked_mul(p) {
                    Some(next) if next <= limit => power = next,
                    _ => break,
                }
            }
        }
        Ok(Self { maxima })
    }

    pub fn at(&self, n: usize) -> Option<u128> { self.maxima.get(n).copied() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn membrane_matches_partition_descent() {
        let membrane = LandauMembrane::new(32).unwrap();
        for n in 0..=32 {
            assert_eq!(membrane.at(n), Some(reference::landau(n as u64) as u128), "n={n}");
        }
        assert_eq!(membrane.at(10), Some(30));
        assert_eq!(membrane.at(15), Some(105));
        assert_eq!(membrane.at(33), None);
    }
    #[test]
    fn membrane_timing_control() {
        let n = std::hint::black_box(45);
        let start = std::time::Instant::now();
        let original = reference::landau(n);
        let old = start.elapsed();
        let start = std::time::Instant::now();
        let membrane = LandauMembrane::new(n as usize).unwrap();
        let elapsed = start.elapsed();
        assert_eq!(membrane.at(n as usize), Some(original as u128));
        println!("landau n={n} value={original} partition_descent={old:?} nested_all_capacities={elapsed:?}");
    }
}

#[cfg(test)]
mod reference {
fn gcd(a: u64, b: u64) -> u64 { if b == 0 { a } else { gcd(b, a % b) } }
fn lcm(a: u64, b: u64) -> u64 { a / gcd(a, b) * b }

/// The largest lcm of a partition of `n`, by exhaustive descent over parts.
pub(super) fn landau(n: u64) -> u64 {
    fn go(remaining: u64, max_part: u64, acc: u64) -> u64 {
        if remaining == 0 { return acc; }
        let mut best = acc;
        let mut p = if max_part < remaining { max_part } else { remaining };
        while p >= 1 {
            let v = go(remaining - p, p, lcm(acc, p));
            if v > best { best = v; }
            if p == 1 { break; }
            p -= 1;
        }
        best
    }
    go(n, n, 1)
}


}
