//! factor_operator.rs — the synthesized factor operator F_N.
//!
//! The primitive is no longer "extract the factor" but "construct a factor
//! state whose nesting is fixed under N": S = (P, Q, K) with P, Q ∈ 𝟒^m two
//! ambient factor lanes and K the still-unresolved coupling register. Seed
//! P_i = Q_i = N (ambient, not guessed Boolean), clamp the product trace Φ_N
//! as evidence, and propagate the multiplication constraints monotonically.
//!
//! The circuit is cyclic. The gates are monotone in the information order, so
//! the evaluator's bounded Kleene iteration from all-N converges to the least
//! fixed point — no oscillation. Certification is the fixed-point rule:
//!
//!   F_N(P, Q, K) = (P, Q, K)
//!
//! Five conditions the circuit enforces:
//!   1. Reconstruction:  digit(MulAmbient(P,Q)) = Φ_N  (no Boolean reading inside).
//!   2. Terminal classicality: Ω(P) = Ω(Q) = ∅  (terminal factor cells in {T,F}).
//!   3. Ambient coupling preserved: N and B stay inside K and the intermediate
//!      lanes; no r/c retraction before fixation.
//!   4. Factor-exchange symmetry: (P,Q) ~ (Q,P).
//!   5. Fixed-point certification: F_N(P,Q,K) = (P,Q,K).

#![allow(dead_code)]

use crate::belnap::B4;
use crate::dqi_ambient::{b4_schoolbook_mul, b4_digit_channel};
use num_bigint::BigUint;
use num_traits::{One, Zero};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// The candidate ambient factor state.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FactorState {
    pub p: Vec<B4>,
    pub q: Vec<B4>,
    pub k: Vec<B4>,
}

/// Product trace Φ_N as B4 (Boolean: T/F per bit, LSB first, 2m bits).
fn product_trace(n: &BigUint, bits: usize) -> Vec<B4> {
    (0..bits).map(|i| if n.bit(i as u64) { B4::T } else { B4::F }).collect()
}

/// Is every cell classical (T or F)?  Terminal-classicality check on one lane.
fn classical(lane: &[B4]) -> bool {
    lane.iter().all(|&v| v == B4::T || v == B4::F)
}

/// Seed the state at ambient N: P_i = Q_i = N, K empty (all N).
pub fn seed_ambient(m: usize) -> FactorState {
    FactorState {
        p: vec![B4::N; m],
        q: vec![B4::N; m],
        k: vec![B4::N; 2 * m],
    }
}

/// Lift a BigUint lane to Boolean B4 cells (LSB first), m cells.
pub fn lane_from_biguint(x: &BigUint, m: usize) -> Vec<B4> {
    (0..m).map(|i| if x.bit(i as u64) { B4::T } else { B4::F }).collect()
}

// ── MulAmbient: the four-valued schoolbook product over ambient cells ──────
//
// This is the circuit's reconstruction primitive. Every column k of the
// product is a B4 state whose (is_true, is_false) pair is (digit, carry_parity):
// a column with no contribution and no carry is N, a lone 1 without carry is T,
// an even sum with carry parity 1 is F, and an odd sum with carry parity 1 is B.
// Reading the digit channel (is_true) recovers the Boolean product; the carry parity
// channel preserves Γ(x) = c_k mod 2 without saturation collapse, so the ambient cells
// retain the balanced four-valued coupling structure. P, Q stay 𝟒-valued the whole
// time — no r/c retraction happens inside the product.

pub fn mul_ambient(p: &[B4], q: &[B4]) -> Vec<B4> {
    b4_schoolbook_mul(p, q)
}

/// The digit (is_true) channel of MulAmbient — the Boolean product trace.
pub fn mul_digit(p: &[B4], q: &[B4]) -> Vec<B4> {
    b4_digit_channel(&b4_schoolbook_mul(p, q))
}

// ── Knowledge-order consensus (lub in ≤k) ───────────────────────────────────
//
// In the information order, N is bottom, B is top, T and F are incomparable.
// The lub is bitwise OR: N⊕x=x, T⊕T=T, F⊕F=F, T⊕F=B, B⊕x=B. Every refinement
// below is a consensus of the current cell with some forced cell, so a cell
// only ever moves UP (N→T, N→F, T→B, F→B). That is the monotonicity that makes
// the Kleene iteration converge instead of oscillate.

fn consensus(a: B4, b: B4) -> B4 {
    a.join(b)
}

fn consensus_lane(a: &[B4], b: &[B4]) -> Vec<B4> {
    a.iter().zip(b.iter()).map(|(&x, &y)| x.join(y)).collect()
}

/// Integer reading of a lane: confirmed-T bits only (matches the product's
/// contribution rule, where exactly T·T counts). N, F, B all read as 0 here.
pub fn lane_to_biguint(lane: &[B4]) -> BigUint {
    let mut acc = BigUint::from(0u32);
    for (i, &v) in lane.iter().enumerate() {
        if v == B4::T {
            acc |= BigUint::one() << i;
        }
    }
    acc
}

// ── Newton/Hensel inverse mod 2^m ──────────────────────────────────────────
//
// Same lift as closure_nested::big_inv_pow2 (verified there against m=66 and
// m=82), kept local so the fixed-point circuit carries no cross-module private
// dependency.

fn big_inv_pow2(a: &BigUint, m: usize) -> BigUint {
    let two = BigUint::from(2u32);
    let two_m = BigUint::one() << m;
    let mask = &two_m - BigUint::one();
    let mut x = BigUint::one();
    let iters = (m as u32).ilog2() as usize + 3;
    for _ in 0..iters {
        let ax = (a * &x) & &mask;
        let t = if ax == BigUint::one() {
            BigUint::one()
        } else {
            &two_m - (ax - &two)
        };
        x = (x * t) & &mask;
    }
    x
}

// ── The circuit step F_N ────────────────────────────────────────────────────
//
// F_N(P,Q,K) = (P', Q', K') with
//   P' = P ⊔ lane(N · Q⁻¹ mod 2^m)
//   Q' = Q ⊔ lane(N · P⁻¹ mod 2^m)
//   K' = K ⊔ MulAmbient(P,Q)
//
// Each lane is only ever refined by consensus, so F_N is extensive: S ≤k F_N(S).
// The true factor pair (p,q) is a fixed point because p·q ≡ N mod 2^m forces
// p = N·q⁻¹ and q = N·p⁻¹, and the product trace closes on all 2m bits. A wrong
// guess is raised cell-by-cell toward B (paradox) rather than retracted, which
// is the ambient coupling the certification reads. The factor-exchange symmetry
// is exact: swapping P,Q swaps the two refinement legs, so F_N commutes with
// (P,Q) ↦ (Q,P).

fn known_bit(x: B4) -> Option<u8> { match x { B4::T => Some(1), B4::F => Some(0), _ => None } }
fn from_bit(v: u8) -> B4 { if v & 1 == 1 { B4::T } else { B4::F } }

pub fn fn_step(n: &BigUint, m: usize, s: &FactorState) -> FactorState {
    let phi = product_trace(n, 2 * m);
    let mut p = s.p.clone();
    let mut q = s.q.clone();

    // N odd forces both low bits to T (P·Q odd ⇒ P₀=Q₀=1). The one piece of
    // evidence the clamp injects before any lane bit is resolvable.
    if phi.first() == Some(&B4::T) {
        p[0] = consensus(p[0], B4::T);
        q[0] = consensus(q[0], B4::T);
    }

    // The 2-adic inverse leg, resolved one bit at a time so a partial seed
    // Hensel-lifts across. With p_0=q_0=1, bit k of p·q is C_k + p_k + q_k
    // (mod 2), where C_k is bit k of the already-fixed low parts' product; the
    // congruence p·q ≡ N then gives p_k ⊕ q_k = r_k := N_k ⊕ C_k. On a classical
    // low run below k: one side known forces the other classically, both unknown
    // is a free coupling whose two completions join to B (the paradox), both
    // known is the consistency a true factor carries. A cell only ever rises in
    // the knowledge order (N → {T,F} → B), so the step is extensive.
    for k in 1..m {
        let low_classical = (0..k).all(|i| known_bit(p[i]).is_some() && known_bit(q[i]).is_some());
        if !low_classical { break; }
        let plow: BigUint = (0..k).map(|i| BigUint::from(known_bit(p[i]).unwrap()) << i).sum();
        let qlow: BigUint = (0..k).map(|i| BigUint::from(known_bit(q[i]).unwrap()) << i).sum();
        let c_k = if (&plow * &qlow).bit(k as u64) { 1u8 } else { 0u8 };
        let r_k = (n.bit(k as u64) as u8) ^ c_k;
        match (known_bit(p[k]), known_bit(q[k])) {
            (Some(pk), None) => { q[k] = consensus(q[k], from_bit(r_k ^ pk)); }
            (None, Some(qk)) => { p[k] = consensus(p[k], from_bit(r_k ^ qk)); }
            (None, None) => { p[k] = consensus(p[k], B4::B); q[k] = consensus(q[k], B4::B); break; }
            (Some(_), Some(_)) => {}
        }
    }
    // symmetric consensus above the break: unresolved coupling stabilises at B,
    // not at unknown — the moat, the free factor bits held as paradox.
    for i in 1..m {
        if p[i] == B4::N { p[i] = B4::B; }
        if q[i] == B4::N { q[i] = B4::B; }
    }

    // The coupling register: the ambient product's carry structure, joined in
    // monotonically.
    let prod = mul_ambient(&s.p, &s.q);
    let k_next = consensus_lane(&s.k, &prod);

    FactorState { p, q, k: k_next }
}

// ── Bounded Kleene iteration from the seed ──────────────────────────────────
//
// Because F_N is extensive (S ≤k F_N(S)) on the finite 4-valued lattice, the
// orbit from any seed is an ascending chain and terminates at a fixed point in
// at most a few passes — no oscillation is possible. max_iters is a safety cap
// only.

pub struct FixRun {
    pub state: FactorState,
    pub iterations: usize,
    pub converged: bool,
}

pub fn fixed_point(n: &BigUint, m: usize, s0: &FactorState, max_iters: usize) -> FixRun {
    let mut s = s0.clone();
    for i in 0..max_iters {
        let s2 = fn_step(n, m, &s);
        if s2 == s {
            return FixRun { state: s, iterations: i + 1, converged: true };
        }
        s = s2;
    }
    FixRun { state: s, iterations: max_iters, converged: false }
}

// ── Certification: the five conditions ──────────────────────────────────────

#[derive(Debug)]
pub struct Certification {
    pub fixed_point: bool,
    pub reconstruction: bool,
    pub classical_p: bool,
    pub classical_q: bool,
    pub coupling_preserved: bool,
    pub symmetric: bool,
}

impl Certification {
    pub fn all_pass(&self) -> bool {
        self.fixed_point
            && self.reconstruction
            && self.classical_p
            && self.classical_q
            && self.coupling_preserved
            && self.symmetric
    }
}

pub fn certify(n: &BigUint, m: usize, s: &FactorState) -> Certification {
    let phi = product_trace(n, 2 * m);
    let digit = mul_digit(&s.p, &s.q);
    let prod = mul_ambient(&s.p, &s.q);

    // 1. Reconstruction: digit(MulAmbient(P,Q)) = Φ_N over all 2m bits.
    let reconstruction = digit == phi;

    // 2. Terminal classicality: Ω(P)=Ω(Q)=∅.
    let classical_p = classical(&s.p);
    let classical_q = classical(&s.q);

    // 3. Ambient coupling preserved: K equals the ambient product, uncollapsed.
    let coupling_preserved = s.k == prod;

    // 5. Fixed-point certification.
    let fixed_point = fn_step(n, m, s) == *s;

    // 4. Factor-exchange symmetry: F_N commutes with (P,Q) ↦ (Q,P).
    let swapped = FactorState { p: s.q.clone(), q: s.p.clone(), k: s.k.clone() };
    let a = fn_step(n, m, s);
    let b = fn_step(n, m, &swapped);
    let symmetric = a.p == b.q && a.q == b.p && a.k == b.k;

    Certification {
        fixed_point,
        reconstruction,
        classical_p,
        classical_q,
        coupling_preserved,
        symmetric,
    }
}

fn fmt_bool(b: bool) -> &'static str {
    if b { "PASS" } else { "FAIL" }
}

pub fn certification_report(n: &BigUint, m: usize, s: &FactorState) -> String {
    let c = certify(n, m, s);
    let mut out = String::new();
    out.push_str(&format!(
        "F_N certification for N={} (m={}): {}\n",
        n,
        m,
        if c.all_pass() { "ALL PASS" } else { "OPEN" }
    ));
    out.push_str(&format!("  1. reconstruction  digit(MulAmbient(P,Q))=Φ_N : {}\n", fmt_bool(c.reconstruction)));
    out.push_str(&format!("  2. classicality    Ω(P)=Ω(Q)=∅              : {} / {}\n", fmt_bool(c.classical_p), fmt_bool(c.classical_q)));
    out.push_str(&format!("  3. coupling        K=MulAmbient(P,Q)         : {}\n", fmt_bool(c.coupling_preserved)));
    out.push_str(&format!("  4. symmetry        F_N∘swap = swap∘F_N       : {}\n", fmt_bool(c.symmetric)));
    out.push_str(&format!("  5. fixed point     F_N(P,Q,K)=(P,Q,K)        : {}\n", fmt_bool(c.fixed_point)));
    out
}

// ── REPL arm ────────────────────────────────────────────────────────────────
//
//   factor_operator ambient <n> <m> [max_iters]   — all-N seed, iterate, certify
//   factor_operator cert <n> <m> <p> <q>          — seed with factors, certify

fn parse_big(s: &str) -> Option<BigUint> {
    s.parse::<BigUint>().ok()
}

/// Integer square root by Newton iteration on BigUint (no f64).
fn isqrt_big(n: &BigUint) -> BigUint {
    if n.is_zero() { return BigUint::zero(); }
    let one = BigUint::one();
    let mut x = n.clone();
    let mut y = (&x + &one) >> 1;
    while y < x {
        x = y;
        y = (&x + n / &x) >> 1;
    }
    x
}

/// Fermat correction: walk a upward from isqrt(N), close exactly when
/// a^2 - N is a perfect square b^2, then N = (a-b)(a+b). Bounded by max_steps.
/// This is the fix for the all-N seed, which converged the 2-adic Hensel lift
/// to the trivial p=q=N fixed point (reconstruction FAIL): the correction seed
/// is the Fermat root, not the zero-information N cell.
pub fn fermat_correct(n: &BigUint, max_steps: u64) -> Option<(BigUint, BigUint)> {
    let one = BigUint::one();
    let mut a = isqrt_big(n);
    if &a * &a < *n { a += &one; }
    let mut steps = 0u64;
    loop {
        let b2 = &a * &a - n;
        let b = isqrt_big(&b2);
        if &b * &b == b2 {
            let p = &a - &b;
            let q = &a + &b;
            if p > one && q > one && &p * &q == *n {
                return Some((p, q));
            }
        }
        a += &one;
        steps += 1;
        if steps >= max_steps { return None; }
    }
}

/// Resolve the moat: fork each free bit into its two completions (the δ move),
/// keep only the branch the exact winding constraint admits — p·q ≡ N mod
/// 2^{k+1} at every level, and p ≤ √N as the magnitude bound — and lift bit by
/// bit until a completion closes on p·q = N (the μ fixation). Returns the factor
/// pair and the number of branch nodes walked: the measured cost of collapsing
/// the moat, read off the run rather than asserted.
pub fn resolve_moat(n: &BigUint, max_nodes: u64) -> (Option<(BigUint, BigUint)>, u64, bool) {
    let one = BigUint::one();
    let root = isqrt_big(n);
    let bits = n.bits() as u32;
    let mut nodes: u64 = 0;
    let mut found: Option<(BigUint, BigUint)> = None;
    // DFS stack of (level k, p_low, q_low)
    let mut stack: Vec<(u32, BigUint, BigUint)> = vec![(0, BigUint::zero(), BigUint::zero())];
    while let Some((k, plo, qlo)) = stack.pop() {
        if found.is_some() || nodes >= max_nodes { break; }
        if k >= bits + 1 { continue; }
        let modk = (&one << (k + 1)) - &one; // 2^{k+1} - 1
        for pk in 0u32..2 {
            for qk in 0u32..2 {
                let p = &plo | (BigUint::from(pk) << k);
                let q = &qlo | (BigUint::from(qk) << k);
                nodes += 1;
                let prod = &p * &q;
                // 2-adic winding constraint at this level
                if (&prod & &modk) != (n & &modk) { continue; }
                if &prod == n && p > one && q > one {
                    found = Some(if p <= q { (p.clone(), q.clone()) } else { (q.clone(), p.clone()) });
                }
                // magnitude bound: the smaller factor cannot exceed √N.
                if p > root && q > root { continue; }
                stack.push((k + 1, p, q));
            }
        }
    }
    let capped = nodes >= max_nodes;
    (found, nodes, capped)
}

/// Full factorization: strip the easy part first, cross the moat only for the
/// hard balanced remainder. The 2-part comes off by the winding parity (⊡),
/// small odd primes by the divisor leg (trial to a cheap bound), and whatever
/// balanced core survives is handed to resolve_moat. Returns the factor multiset
/// (sorted) with the total moat-node cost and whether the budget held.
pub fn full_resolve(n0: &BigUint, small_bound: u64, max_nodes: u64) -> (Vec<BigUint>, u64, bool) {
    let one = BigUint::one();
    let two = BigUint::from(2u32);
    let mut factors: Vec<BigUint> = Vec::new();
    let mut moat_nodes: u64 = 0;
    let mut capped = false;
    // divisor/winding pre-strip: 2, then odd primes up to the cheap bound.
    let mut n = n0.clone();
    while (&n % &two).is_zero() { factors.push(two.clone()); n /= &two; }
    let sb = BigUint::from(small_bound);
    let mut d = BigUint::from(3u32);
    while d <= sb && (&d * &d) <= n {
        while (&n % &d).is_zero() { factors.push(d.clone()); n /= &d; }
        d += &two;
    }
    // n is now free of factors <= small_bound. Resolve the balanced core(s).
    let sb2 = &sb * &sb;
    let mut stack = vec![n];
    while let Some(c) = stack.pop() {
        if c == one { continue; }
        // prime if no factor <= sqrt survived the strip and c < small_bound^2
        if c < sb2 { factors.push(c); continue; }
        let (res, nodes, cap) = resolve_moat(&c, max_nodes);
        moat_nodes += nodes;
        if cap { capped = true; factors.push(c); continue; }
        match res {
            Some((p, q)) => { stack.push(p); stack.push(q); }
            None => factors.push(c), // resolver walked to exhaustion: prime
        }
    }
    factors.sort_unstable();
    (factors, moat_nodes, capped)
}

/// The two-arm convergence circuit: ∈ forks, the ≻⊤ arm walks the square
/// frontier out from √N (Fermat, wins when the factors sit close), the ≺⊥ arm
/// runs a rho cycle (wins when one factor is small), and ∋ fuses on whichever
/// closes first. Interleaving costs at most twice the winning arm, so it
/// converges for any gap: close or balanced-near-root by the frontier, far
/// apart by rho. The 2-part is stripped first so an even N never stalls rho.
pub fn converge(n0: &BigUint, max_rounds: u64) -> Option<(BigUint, BigUint, &'static str, u64)> {
    use num_traits::Zero;
    let one = BigUint::one();
    let two = BigUint::from(2u32);
    if (n0 % &two).is_zero() { return Some((two.clone(), n0 / &two, "two-part", 0)); }
    let n = n0.clone();

    // ≻⊤ frontier arm state
    let mut a = isqrt_big(&n);
    if &a * &a < n { a += &one; }
    // ≺⊥ rho arm state (Floyd, restartable c on cycle collapse)
    let mut c = one.clone();
    let mut x = two.clone();
    let mut y = two.clone();
    let f = |v: &BigUint, c: &BigUint, n: &BigUint| ((v * v) + c) % n;

    let batch = 256u64;
    let mut rounds = 0u64;
    while rounds < max_rounds {
        // frontier batch
        for _ in 0..batch {
            let b2 = &a * &a - &n;
            let b = isqrt_big(&b2);
            if &b * &b == b2 {
                let p = &a - &b; let q = &a + &b;
                if p > one && q > one { return Some((p, q, "frontier", rounds)); }
            }
            a += &one;
        }
        // rho batch
        for _ in 0..batch {
            x = f(&x, &c, &n);
            y = f(&f(&y, &c, &n), &c, &n);
            let d = if x >= y { &x - &y } else { &y - &x };
            let g = gcd_big(&d, &n);
            if g > one && g < n { return Some((g.clone(), &n / &g, "rho", rounds)); }
            if g == n { // cycle collapsed: re-seed the arm with a new constant
                c += &one; x = two.clone(); y = two.clone();
            }
        }
        rounds += 1;
    }
    None
}

fn gcd_big(a: &BigUint, b: &BigUint) -> BigUint {
    let (mut x, mut y) = (a.clone(), b.clone());
    while !num_traits::Zero::is_zero(&y) { let r = &x % &y; x = y; y = r; }
    x
}

pub fn repl_factor_operator(args: &[&str]) -> String {
    if args.is_empty() {
        return String::from(
            "factor_operator ambient <n> <m> [max_iters]\n\
             factor_operator cert <n> <m> <p> <q>",
        );
    }
    match args[0] {
        "ambient" => {
            if args.len() < 3 {
                return String::from("usage: factor_operator ambient <n> <m> [max_iters]");
            }
            let n = match parse_big(args[1]) { Some(x) => x, None => return String::from("bad n") };
            let m: usize = match args[2].parse() { Ok(x) => x, Err(_) => return String::from("bad m") };
            let max_iters: usize = if args.len() > 3 {
                args[3].parse().unwrap_or(64 * m + 16)
            } else {
                64 * m + 16
            };
            let seed = seed_ambient(m);
            let run = fixed_point(&n, m, &seed, max_iters);
            let mut out = String::new();
            out.push_str(&format!(
                "ambient seed (all-N) → {} iterations, converged={}\n",
                run.iterations, run.converged
            ));
            out.push_str(&certification_report(&n, m, &run.state));
            // The all-N seed reaches the symmetric paradox fixed point: bit 0 is
            // the odd anchor, every free factor bit is B. The moat width is the
            // count of B lanes — the hardness as a positive quantity, not a
            // hidden search. No Fermat fallback: the operator states the moat, it
            // does not go around it.
            let moat = run.state.p.iter().chain(run.state.q.iter())
                .filter(|&&b| b == B4::B).count();
            out.push_str(&format!(
                "  moat width (B lanes over P and Q): {} of {} — the free factor bits, held as paradox\n",
                moat, 2 * m
            ));
            out
        }
        "cert" => {
            if args.len() < 5 {
                return String::from("usage: factor_operator cert <n> <m> <p> <q>");
            }
            let n = match parse_big(args[1]) { Some(x) => x, None => return String::from("bad n") };
            let m: usize = match args[2].parse() { Ok(x) => x, Err(_) => return String::from("bad m") };
            let p = match parse_big(args[3]) { Some(x) => x, None => return String::from("bad p") };
            let q = match parse_big(args[4]) { Some(x) => x, None => return String::from("bad q") };
            let state = FactorState {
                p: lane_from_biguint(&p, m),
                q: lane_from_biguint(&q, m),
                k: mul_ambient(&lane_from_biguint(&p, m), &lane_from_biguint(&q, m)),
            };
            certification_report(&n, m, &state)
        }
        "resolve" => {
            if args.len() < 2 {
                return String::from("usage: factor_operator resolve <n> [max_nodes]");
            }
            let n = match parse_big(args[1]) { Some(x) if x >= BigUint::from(3u32) => x, _ => return String::from("bad n (need integer >= 3)") };
            let max_nodes: u64 = if args.len() > 2 { args[2].parse().unwrap_or(50_000_000) } else { 50_000_000 };
            let small_bound: u64 = 1 << 20;
            let (factors, moat_nodes, capped) = full_resolve(&n, small_bound, max_nodes);
            let mut out = String::new();
            let prod: BigUint = factors.iter().fold(BigUint::one(), |a, b| a * b);
            let terms: Vec<String> = factors.iter().map(|f| f.to_string()).collect();
            out.push_str(&format!("resolve: {} = {}  (verified {})\n", n, terms.join(" × "), prod == n));
            out.push_str(&format!("  small part stripped below {}; moat nodes for the balanced core: {}{}\n",
                small_bound, moat_nodes, if capped { " (a balanced core did not collapse within budget)" } else { "" }));
            out
        }
        "converge" => {
            if args.len() < 2 { return String::from("usage: factor_operator converge <n> [max_rounds]"); }
            let n = match parse_big(args[1]) { Some(x) if x >= BigUint::from(3u32) => x, _ => return String::from("bad n (need integer >= 3)") };
            let max_rounds: u64 = if args.len() > 2 { args[2].parse().unwrap_or(20_000_000) } else { 20_000_000 };
            match converge(&n, max_rounds) {
                Some((p, q, arm, rounds)) => format!(
                    "converge: {} = {} × {}  (verified {})\n  arm: {}  rounds: {}  (∈ forks frontier ≻⊤ and rho ≺⊥, ∋ fuses on first closure)\n",
                    n, p, q, &p * &q == n, arm, rounds),
                None => format!("converge: no factor within {} rounds (n prime, or both arms past budget)\n", max_rounds),
            }
        }
        _ => String::from("unknown subcommand (ambient | cert | resolve | converge)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn big(s: &str) -> BigUint {
        s.parse::<BigUint>().unwrap()
    }

    /// The true factor pair is a certified fixed point on all three known
    /// semiprimes: all five conditions pass.
    #[test]
    fn true_factors_certify_all_pass() {
        for (n, m, p, q) in [
            ("2147712859", 16usize, "32779", "65521"),
            ("8796158033869", 22, "2097169", "4194301"),
            ("140737614184421", 24, "8388617", "16777213"),
        ] {
            let (n, p, q) = (big(n), big(p), big(q));
            let lp = lane_from_biguint(&p, m);
            let lq = lane_from_biguint(&q, m);
            let state = FactorState { p: lp.clone(), q: lq.clone(), k: mul_ambient(&lp, &lq) };
            let c = certify(&n, m, &state);
            assert!(c.all_pass(), "m={} {:?}", m, c);
        }
    }

    /// The all-N seed converges (no oscillation) to a fixed point that keeps
    /// factor-exchange symmetry but stays ambient — monotone closure alone does
    /// not retract to the Boolean factor. This is the measured closure verdict: monotone closure alone does not retract to the Boolean factor.
    #[test]
    fn ambient_seed_converges_to_symmetric_paradox() {
        for (n, m) in [
            ("2147712859", 16usize),
            ("8796158033869", 22),
            ("140737614184421", 24),
        ] {
            let n = big(n);
            let run = fixed_point(&n, m, &seed_ambient(m), 64 * m + 16);
            assert!(run.converged, "m={} did not converge", m);
            let c = certify(&n, m, &run.state);
            assert!(c.fixed_point, "m={} not a fixed point", m);
            assert!(c.symmetric, "m={} broke symmetry", m);
            assert!(!c.reconstruction, "m={} ambient seed should not reconstruct", m);
        }
    }

    /// F_N is extensive in the knowledge order: S ≤k F_N(S), so every cell only
    /// moves up. This is the monotonicity that bans oscillation.
    #[test]
    fn fn_step_is_extensive() {
        let n = big("2147712859");
        let m = 16;
        // A deliberately mixed seed: N, T, F, B scattered through both lanes.
        let mut p = vec![B4::N; m];
        let mut q = vec![B4::N; m];
        for i in 0..m {
            p[i] = B4::from_u8((i % 4) as u8);
            q[i] = B4::from_u8(((i * 3 + 1) % 4) as u8);
        }
        let s = FactorState { p, q, k: vec![B4::N; 2 * m] };
        let t = fn_step(&n, m, &s);
        for i in 0..m {
            assert!(s.p[i].approx_le(t.p[i]), "p[{}] went down", i);
            assert!(s.q[i].approx_le(t.q[i]), "q[{}] went down", i);
        }
        for i in 0..2 * m {
            assert!(s.k[i].approx_le(t.k[i]), "k[{}] went down", i);
        }
    }

    /// F_N commutes with factor exchange (P,Q) ↦ (Q,P).
    #[test]
    fn fn_step_commutes_with_swap() {
        let n = big("8796158033869");
        let m = 22;
        let s = seed_ambient(m);
        let swapped = FactorState { p: s.q.clone(), q: s.p.clone(), k: s.k.clone() };
        let a = fn_step(&n, m, &s);
        let b = fn_step(&n, m, &swapped);
        assert_eq!(a.p, b.q);
        assert_eq!(a.q, b.p);
        assert_eq!(a.k, b.k);
    }

    /// The digit channel of MulAmbient(P,Q) is exactly Φ_N for the true pair.
    #[test]
    fn mul_digit_recovers_product_trace() {
        let (n, m, p, q) = ("2147712859", 16usize, "32779", "65521");
        let (n, p, q) = (big(n), big(p), big(q));
        let digit = mul_digit(&lane_from_biguint(&p, m), &lane_from_biguint(&q, m));
        assert_eq!(digit, product_trace(&n, 2 * m));
    }
}
