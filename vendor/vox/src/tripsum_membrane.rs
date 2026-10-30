//! tripsum_membrane kernel extracted from G-mOMonadOS/src/erdos_walks.rs.
pub struct TripleSumMembrane {
    occupied: Vec<bool>,
    trail: Vec<usize>,
    current: Vec<u64>,
    best: Vec<u64>,
}

impl TripleSumMembrane {
    pub fn solve(limit: usize) -> Result<Vec<u64>, String> {
        let len = limit.checked_mul(3).and_then(|n| n.checked_add(1))
            .ok_or("Triple-sum table size overflow")?;
        let mut occupied = Vec::new();
        occupied.try_reserve_exact(len).map_err(|_| "Triple-sum table allocation failed")?;
        occupied.resize(len, false);
        let mut membrane = Self { occupied, trail: Vec::new(), current: Vec::new(), best: Vec::new() };
        membrane.descend(1, limit);
        Ok(membrane.best)
    }

    fn descend(&mut self, next: usize, limit: usize) {
        if self.current.len() > self.best.len() { self.best = self.current.clone(); }
        for candidate in next..=limit {
            if self.current.len() + (limit - candidate + 1) <= self.best.len() { return; }
            let checkpoint = self.trail.len();
            let mut distinct = true;
            'pairs: for i in 0..self.current.len() {
                for j in i + 1..self.current.len() {
                    let sum = self.current[i] as usize + self.current[j] as usize + candidate;
                    if self.occupied[sum] { distinct = false; break 'pairs; }
                    self.occupied[sum] = true;
                    self.trail.push(sum);
                }
            }
            if distinct {
                self.current.push(candidate as u64);
                self.descend(candidate + 1, limit);
                self.current.pop();
            }
            // The enclosing branch keeps its sums; this child's additions leave.
            while self.trail.len() > checkpoint {
                self.occupied[self.trail.pop().unwrap()] = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn membrane_matches_original_witness_and_restores_siblings() {
        for n in 0..=18 {
            let result = TripleSumMembrane::solve(n).unwrap();
            assert_eq!(result, reference::max_b3(n as u64), "n={n}");
            assert!(reference::triple_sums_distinct(&result));
        }
    }
    #[test]
    fn membrane_timing_control() {
        let n = std::hint::black_box(24);
        let start = std::time::Instant::now();
        let original = reference::max_b3(n);
        let old = start.elapsed();
        let start = std::time::Instant::now();
        let nested = TripleSumMembrane::solve(n as usize).unwrap();
        let elapsed = start.elapsed();
        assert_eq!(nested, original);
        println!("tripsum limit={n} witness={nested:?} original={old:?} nested_with_setup={elapsed:?}");
    }
}

#[cfg(test)]
mod reference {
pub(super) fn max_b3(limit: u64) -> Vec<u64> {
    let mut best: Vec<u64> = Vec::new();
    let mut cur: Vec<u64> = Vec::new();
    descend(1, limit, &mut cur, &mut best);
    best
}

fn descend(next: u64, limit: u64, cur: &mut Vec<u64>, best: &mut Vec<u64>) {
    if cur.len() > best.len() { *best = cur.clone(); }
    let mut n = next;
    while n <= limit {
        // Nothing left to gain: even taking every remaining integer cannot beat
        // the best already found.
        if cur.len() + ((limit - n + 1) as usize) <= best.len() { return; }
        cur.push(n);
        if triple_sums_distinct(cur) { descend(n + 1, limit, cur, best); }
        cur.pop();
        n += 1;
    }
}

/// Every three-element subset has a sum no other three-element subset shares.
pub(super) fn triple_sums_distinct(set: &[u64]) -> bool {
    let mut sums: Vec<u64> = Vec::new();
    let k = set.len();
    let mut i = 0;
    while i < k {
        let mut j = i + 1;
        while j < k {
            let mut m = j + 1;
            while m < k {
                let t = set[i] + set[j] + set[m];
                if sums.contains(&t) { return false; }
                sums.push(t);
                m += 1;
            }
            j += 1;
        }
        i += 1;
    }
    true
}

}
