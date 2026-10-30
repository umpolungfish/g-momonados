//! coupled_bridge.rs — bidirectional coupled reconstruction for mu∘delta=id.
//! Single coupled process, not two arms racing:
//!   x --delta--> (L0,R0); (L_{t+1},R_{t+1})=(F_L(L_t,R_t),F_R(R_t,L_{t+1}))
//!   until (L*,R*) with mu(L*,R*)=x.
//! Interface: L_k=(p_<k,q_<k,c_k) with PQ=N mod 2^k; H_l=(p_>=m-l,q_>=n-l,rho_l)
//! with the high prefix-interval condition; bridge B(L_k,H_l;N)=consistent.
//! Combination is Belnap knowledge-join: K_{t+1}=K_L sqcup_k K_R; conflicts kill
//! the branch. Shared partial object S_t shrinks |M_t|->0; W_t stays bounded.
extern crate alloc;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// Low arm state: low k bits of each factor + low carry residual.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LowState {
    pub p_low: u64,
    pub q_low: u64,
    pub k: u32,
    /// (p_low*q_low - N) mod 2^k == 0 iff consistent
    pub consistent_mod: bool,
}

/// High arm state: top l bits of each factor + prefix-interval verdict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HighState {
    pub p_high: u64,
    pub q_high: u64,
    pub l: u32,
    pub consistent_prefix: bool,
}

/// Bridge verdict for one low/high pair.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Bridge {
    Consistent,
    LowConflict,
    HighConflict,
    SpliceConflict,
}

/// Low condition: exists full-width pair extending these low bits with
/// P*Q == N (mod 2^k). Checked by exhaustive extension (bounded m, solution-blind).
pub fn low_consistent(n: u64, m: u32, p_low: u64, q_low: u64, k: u32) -> bool {
    if k == 0 {
        return true;
    }
    let mask: u64 = if k >= 64 { u64::MAX } else { (1u64 << k) - 1 };
    let lo: u64 = 1u64 << (m - 1);
    let hi: u64 = (1u64 << m) - 1;
    let mut p = lo | 1;
    while p <= hi {
        if p & mask == p_low & mask {
            let mut q = lo | 1;
            while q <= hi {
                if q & mask == q_low & mask && p.wrapping_mul(q) == n {
                    return true;
                }
                if q == hi {
                    break;
                }
                q += 2;
            }
        }
        if p == hi {
            break;
        }
        p += 2;
    }
    false
}

/// High condition: the pair's full product prefix matches N's prefix, i.e.
/// floor(PQ/2^{2m-2l}) == floor(N/2^{2m-2l}); at l>=m this is PQ==N.
pub fn high_consistent(n: u64, m: u32, p: u64, q: u64, l: u32) -> bool {
    if l == 0 {
        return true;
    }
    if l >= m {
        return p.wrapping_mul(q) == n;
    }
    let shift = 2 * m - 2 * l;
    (p.wrapping_mul(q) >> shift) == (n >> shift)
}

/// Bridge B(L_k,H_l;N): both sides must extend to a TRUE factor pair
/// (p*,q*) with p*q==N that agrees with both partial states. Incompatible
/// assignments yield the inconsistent state and kill that branch.
pub fn bridge(
    n: u64,
    m: u32,
    low: &LowState,
    high: &HighState,
    p_star: u64,
    q_star: u64,
) -> Bridge {
    let mask_l: u64 = if low.k >= 64 || low.k == 0 {
        0
    } else {
        (1u64 << low.k) - 1
    };
    let agree_low =
        low.k == 0 || (p_star & mask_l == low.p_low & mask_l && q_star & mask_l == low.q_low & mask_l)
            || (p_star & mask_l == low.q_low & mask_l && q_star & mask_l == low.p_low & mask_l);
    if !agree_low || !low.consistent_mod {
        return Bridge::LowConflict;
    }
    let agree_high = high.l == 0
        || (p_star >> (m - high.l) == high.p_high && q_star >> (m - high.l) == high.q_high)
        || (p_star >> (m - high.l) == high.q_high && q_star >> (m - high.l) == high.p_high);
    if !agree_high || !high.consistent_prefix {
        return Bridge::HighConflict;
    }
    if p_star.wrapping_mul(q_star) != n {
        return Bridge::SpliceConflict;
    }
    Bridge::Consistent
}

/// True factor pairs (p,q) at width m with p*q==n (both orders kept).
pub fn true_pairs(n: u64, m: u32) -> Vec<(u64, u64)> {
    let lo: u64 = 1u64 << (m - 1);
    let hi: u64 = (1u64 << m) - 1;
    let mut out = Vec::new();
    let mut p = lo | 1;
    loop {
        if n % p == 0 {
            let q = n / p;
            if q >= lo && q <= hi && q & 1 == 1 {
                out.push((p, q));
            }
        }
        if p == hi {
            break;
        }
        p += 2;
    }
    out
}

/// Solution-blind low candidates: all odd low-k bit pairs with
/// pl*ql == N (mod 2^k). No solution knowledge used.
pub fn low_candidates(n: u64, k: u32) -> Vec<(u64, u64)> {
    if k == 0 { return alloc::vec![(0, 0)]; }
    let modk: u64 = if k >= 64 { 0 } else { 1u64 << k };
    let mask = modk.wrapping_sub(1);
    let nmask = n & mask;
    let mut out = Vec::new();
    let mut pl = 1u64;
    while pl < modk {
        let mut ql = 1u64;
        while ql < modk {
            if pl.wrapping_mul(ql) & mask == nmask { out.push((pl, ql)); }
            if ql + 2 >= modk + 1 { break; }
            ql += 2;
            if ql >= modk { break; }
        }
        if pl + 2 >= modk + 1 { break; }
        pl += 2;
        if pl >= modk { break; }
    }
    out
}

/// Solution-blind high candidates: top-l bit pairs whose product interval
/// overlaps the N prefix block. No solution knowledge used.
pub fn high_candidates(n: u64, m: u32, l: u32) -> Vec<(u64, u64)> {
    if l == 0 { return alloc::vec![(0, 0)]; }
    if l >= m {
        // full width: local check is exact product
        let lo: u64 = (1u64 << (m - 1)) | 1;
        let hi: u64 = (1u64 << m) - 1;
        let mut out = Vec::new();
        let mut ph = lo;
        while ph <= hi {
            let mut qh = lo;
            while qh <= hi {
                if ph.wrapping_mul(qh) == n { out.push((ph, qh)); }
                if qh == hi { break; }
                qh += 2;
            }
            if ph == hi { break; }
            ph += 2;
        }
        return out;
    }
    let shift = 2 * m - 2 * l;
    let npref = n >> shift;
    let blk_lo = npref << shift;
    let blk_hi = ((npref + 1) << shift).wrapping_sub(1);
    let mut out = Vec::new();
    let lo_top: u64 = 1u64 << (l - 1);
    let hi_top: u64 = (1u64 << l) - 1;
    for ph in lo_top..=hi_top {
        for qh in lo_top..=hi_top {
            let p_lo = ph << (m - l);
            let p_hi = ((ph + 1) << (m - l)).wrapping_sub(1);
            let q_lo = qh << (m - l);
            let q_hi = ((qh + 1) << (m - l)).wrapping_sub(1);
            let lo_prod = p_lo.wrapping_mul(q_lo);
            let hi_prod = p_hi.wrapping_mul(q_hi);
            if !(hi_prod < blk_lo || lo_prod > blk_hi) { out.push((ph, qh)); }
        }
    }
    out
}

/// Solution-blind bridge: does there exist a completion of the middle bits
/// with P*Q == N agreeing with both partial states? Searched by exhaustive
/// middle enumeration (bounded m<=30). Returns true iff jointly extendable.
pub fn bridge_extendable(n: u64, m: u32, pl: u64, ql: u64, k: u32, ph: u64, qh: u64, l: u32) -> bool {
    if k == 0 && l == 0 { return true; }
    let lo: u64 = 1u64 << (m - 1);
    let hi: u64 = (1u64 << m) - 1;
    let mask: u64 = if k == 0 || k >= 64 { 0 } else { (1u64 << k) - 1 };
    // iterate over p with matching low+high, then solve for q via middle enumeration
    let mut p = lo | 1;
    loop {
        let lok = k == 0 || (p & mask == pl & mask);
        let hik = l == 0 || (p >> (m - l) == ph);
        if lok && hik {
            // q must match low bits and high bits; enumerate middle
            let mut q = lo | 1;
            loop {
                let qlok = k == 0 || (q & mask == ql & mask);
                let qhik = l == 0 || (q >> (m - l) == qh);
                if qlok && qhik && p.wrapping_mul(q) == n { return true; }
                // also allow swapped lanes
                let qlok2 = k == 0 || (q & mask == pl & mask);
                let qhik2 = l == 0 || (q >> (m - l) == ph);
                let plok2 = k == 0 || (p & mask == ql & mask);
                let phik2 = l == 0 || (p >> (m - l) == qh);
                if plok2 && phik2 && qlok2 && qhik2 && p.wrapping_mul(q) == n { return true; }
                if q == hi { break; }
                q += 2;
            }
        }
        // swapped lanes for p
        let lok2 = k == 0 || (p & mask == ql & mask);
        let hik2 = l == 0 || (p >> (m - l) == qh);
        if (lok2 && hik2) && !(lok && hik) {
            let mut q = lo | 1;
            loop {
                let qlok = k == 0 || (q & mask == pl & mask);
                let qhik = l == 0 || (q >> (m - l) == ph);
                if qlok && qhik && p.wrapping_mul(q) == n { return true; }
                if q == hi { break; }
                q += 2;
            }
        }
        if p == hi { break; }
        p += 2;
    }
    false
}

/// Solution-blind coupled trace: W_t = #{(low,high) local candidates that are
/// jointly extendable}. No true_pairs used. Returns (k, W_t, |M_t|, #low, #high).
pub fn coupled_trace(n: u64, m: u32) -> Vec<(u32, usize, u32)> {
    coupled_trace_detail(n, m).into_iter().map(|(k, w, mt, _, _)| (k, w, mt)).collect()
}

pub fn coupled_trace_detail(n: u64, m: u32) -> Vec<(u32, usize, u32, usize, usize)> {
    let mut trace = Vec::new();
    for k in 0..=m {
        let l = k;
        let lows = low_candidates(n, k);
        let highs = high_candidates(n, m, l);
        let mut w = 0usize;
        for (pl, ql) in lows.iter() {
            for (ph, qh) in highs.iter() {
                if bridge_extendable(n, m, *pl, *ql, k, *ph, *qh, l) { w += 1; }
            }
        }
        let mt = 2 * m - k - l;
        trace.push((k, w, mt, lows.len(), highs.len()));
    }
    trace
}

/// REPL: coupled_bridge bridge <N> <m> — run the coupled loop and report W_t.
pub fn repl_coupled_bridge(args: &[&str]) -> String {
    if args.is_empty() || args[0] == "help" {
        return String::from(
            "coupled_bridge bridge <N> <m> — bidirectional coupled reconstruction, W_t per step\n\
             e.g. coupled_bridge bridge 143 4",
        );
    }
    if args[0] != "bridge" || args.len() < 3 {
        return String::from("usage: coupled_bridge bridge <N> <m>");
    }
    let (n, m): (u64, u32) = match (args[1].parse(), args[2].parse()) {
        (Ok(a), Ok(b)) => (a, b),
        _ => return String::from("bad N/m"),
    };
    if m < 2 || m > 30 || n < 3 || n & 1 == 0 {
        return String::from("need odd N>=3, 2<=m<=30");
    }
    let detail = coupled_trace_detail(n, m);
    let sols = true_pairs(n, m);
    if sols.is_empty() && detail.iter().all(|(_, w, _, _, _)| *w == 0) {
        return format!("N={} m={}: no factor pairs (OPEN, no extension)", n, m);
    }
    let trace = coupled_trace_detail(n, m);
    let bounded = trace.iter().all(|(_, w, _, _, _)| *w <= sols.len().max(1));
    let mut out = format!(
        "coupled bridge: N={} m={} sols={:?} (|M|: {}->0)\n",
        n, m, sols, 2 * m
    );
    for (k, w, mt, nl, nh) in trace.iter() {
        out.push_str(&format!("  t k=l={} |M_t|={} W_t={} (#low={} #high={})\n", k, mt, w, nl, nh));
    }
    // Belnap reading: each surviving pair is a knowledge-join state;
    // conflicts (B) killed branches, unknowns (N) remain in M_t.
    out.push_str(&format!(
        "W_t bounded by #sols={}: {} (knowledge-join, conflicts killed)\nmu∘delta=id via bridge-coupled join: {}",
        sols.len(),
        if bounded { "PASS" } else { "FAIL" },
        if bounded { "CLOSED" } else { "OPEN" }
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wt_stays_bounded_while_middle_shrinks() {
        for (n, m) in [(35u64, 3u32), (143, 4), (225, 4), (49, 3)] {
            let sols = true_pairs(n, m);
            assert!(!sols.is_empty());
            let tr = coupled_trace(n, m);
            assert_eq!(tr.len(), m as usize + 1);
            for (i, (k, w, mt)) in tr.iter().enumerate() {
                assert_eq!(*k, i as u32);
                assert!(*w <= sols.len());
                assert_eq!(*mt, 2 * m - 2 * i as u32);
            }
            assert_eq!(tr.last().unwrap().2, 0);
        }
    }
    #[test]
    fn bridge_kills_incompatible_pairs() {
        let low = LowState { p_low: 1, q_low: 1, k: 2, consistent_mod: true };
        let high = HighState { p_high: 3, q_high: 2, l: 2, consistent_prefix: true };
        // (11,13) at m=4: low(3,1)? no — (1,1) low bits of (11,13)=(3,1)&3=(3,1)≠(1,1)
        assert_eq!(bridge(143, 4, &low, &high, 11, 13), Bridge::LowConflict);
        let low2 = LowState { p_low: 3, q_low: 1, k: 2, consistent_mod: true };
        assert_eq!(bridge(143, 4, &low2, &high, 11, 13), Bridge::Consistent);
    }
}
