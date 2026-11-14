//! binary_membrane.rs — G-mOMonadOS tool porting Imscribing.Paraconsistent.BinaryMembrane.
//! The Lean source (p4rakernel/p4ramill/Imscribing/Paraconsistent/BinaryMembrane.lean)
//! defines a binary catalytic membrane integration:
//!   δ(N) = (N mod 2, N / 2)       FSPLIT
//!   μ(b,q) = 2q + b               FFUSE
//!   μ ∘ δ = id                    IMSCRIB
//!   ENGAGR: 1 < gcd(candidate,N) < N
//!   IFIX: exact arithmetic factor certificates
//! All operations run on BigUint. Belnap4 values (T/F/B/N) model the phase.
#![allow(dead_code)]
extern crate alloc;
use alloc::string::String;
use alloc::format;
use alloc::vec::Vec;
use num_bigint::BigUint;
use num_traits::Zero;

/// Belnap four values: T, F, B (both), N (neither).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Belnap4 { T, F, B, N }
impl Belnap4 {
    pub fn is_glut(self) -> bool { matches!(self, Belnap4::B) }
    pub fn to_str(self) -> &'static str {
        match self { Belnap4::T => "T", Belnap4::F => "F", Belnap4::B => "B", Belnap4::N => "N" }
    }
}

/// Proper factor pair: 1 < p, 1 < q, p*q = N, p < N, q < N.
#[derive(Clone, Debug)]
pub struct ProperFactorPair { pub p: BigUint, pub q: BigUint }
/// Verified factor pair: p*q = N (p or q may be unit).
#[derive(Clone, Debug)]
pub struct VerifiedFactorPair { pub p: BigUint, pub q: BigUint }
/// Binary membrane output: even lane, odd lane, closure, factors, phase.
#[derive(Clone, Debug)]
pub struct BinaryMembraneOutput {
    pub even_lane: BigUint,
    pub odd_lane: BigUint,
    pub binary_closure: bool,
    pub factors: VerifiedFactorPair,
    pub phase: Belnap4,
}

/// δ: binarySplit(N) = (N mod 2, N / 2) — even lane first, odd lane second.
pub fn binary_split(n: &BigUint) -> (BigUint, BigUint) { (n % 2u32, n / 2u32) }
/// μ: binaryFuse((b, q)) = 2q + b.
pub fn binary_fuse(lanes: (BigUint, BigUint)) -> BigUint { let (b, q) = lanes; 2u32 * q + b }
/// μ ∘ δ = id — the IMSCRIB round-trip certificate.
pub fn binary_roundtrip(n: &BigUint) -> bool { binary_fuse(binary_split(n)) == *n }
/// Even channel: (binarySplit N).1 = N mod 2.
pub fn binary_even_channel(n: &BigUint) -> BigUint { binary_split(n).0 }
/// Odd channel: (binarySplit N).2 = N / 2.
pub fn binary_odd_channel(n: &BigUint) -> BigUint { binary_split(n).1 }

/// Euclidean GCD for BigUint (avoids num-integer dependency).
fn big_gcd(a: &BigUint, b: &BigUint) -> BigUint {
    let mut x = a.clone();
    let mut y = b.clone();
    while !y.is_zero() {
        let t = x.clone() % &y;
        x = y;
        y = t;
    }
    x
}

/// Integer square root via Newton's method.
fn big_sqrt(n: &BigUint) -> BigUint {
    if n.is_zero() { return BigUint::zero(); }
    let mut x = n.clone();
    loop {
        let y = (x.clone() + n / &x) / 2u32;
        if y >= x { return x; }
        x = y;
    }
}

/// ENGAGR paradox condition: 1 < gcd(candidate, N) < N.
pub fn paradox_condition(n: &BigUint, candidate: &BigUint) -> bool {
    let g = big_gcd(candidate, n);
    let one = BigUint::from(1u32);
    g > one && g < *n
}

/// Build a ProperFactorPair from an engaged witness.
fn gcd_proper_pair(n: &BigUint, candidate: &BigUint) -> Option<ProperFactorPair> {
    if !paradox_condition(n, candidate) { return None; }
    let g = big_gcd(candidate, n);
    let q = n / &g;
    let one = BigUint::from(1u32);
    if g > one && q > one { Some(ProperFactorPair { p: g, q }) } else { None }
}

/// Build a VerifiedFactorPair from an engaged witness.
fn gcd_verified_pair(n: &BigUint, candidate: &BigUint) -> Option<VerifiedFactorPair> {
    gcd_proper_pair(n, candidate).map(|pp| VerifiedFactorPair { p: pp.p, q: pp.q })
}

/// Unit fallback: ⟨N, 1⟩.
fn unit_fallback(n: &BigUint) -> VerifiedFactorPair {
    VerifiedFactorPair { p: n.clone(), q: BigUint::from(1u32) }
}

/// membraneFactors: proper pair if engaged, unit fallback otherwise.
pub fn membrane_factors(n: &BigUint, candidate: &BigUint) -> VerifiedFactorPair {
    if paradox_condition(n, candidate) {
        gcd_verified_pair(n, candidate).unwrap_or_else(|| unit_fallback(n))
    } else { unit_fallback(n) }
}

/// runBinaryMembrane: the complete pipeline.
pub fn run_binary_membrane(n: &BigUint, candidate: &BigUint) -> BinaryMembraneOutput {
    let even_lane = binary_even_channel(n);
    let odd_lane = binary_odd_channel(n);
    let binary_closure = binary_roundtrip(n);
    let factors = membrane_factors(n, candidate);
    let phase = if paradox_condition(n, candidate) { Belnap4::B } else { Belnap4::T };
    BinaryMembraneOutput { even_lane, odd_lane, binary_closure, factors, phase }
}

/// engagementTest: boolean decision procedure for ENGAGR.
pub fn engagement_test(n: &BigUint, candidate: &BigUint) -> bool { paradox_condition(n, candidate) }
/// engagementSearch: O(N) scan, first candidate below N satisfying ENGAGR.
pub fn engagement_search(n: &BigUint) -> Option<BigUint> {
    let mut c = BigUint::from(2u32);
    let one = BigUint::from(1u32);
    while c < *n { if paradox_condition(n, &c) { return Some(c); } c += &one; }
    None
}
/// normalizeCandidate: gcd(candidate, N) — idempotent projection.
pub fn normalize_candidate(n: &BigUint, candidate: &BigUint) -> BigUint {
    big_gcd(candidate, n)
}
/// divisorEngagementCandidates: O(sqrt N) trial-divisor scan.
pub fn divisor_engagement_candidates(n: &BigUint) -> Vec<BigUint> {
    let mut out = Vec::new();
    let sqrt = big_sqrt(n);
    let mut d = BigUint::from(2u32);
    let one = BigUint::from(1u32);
    while d <= sqrt { if n % &d == BigUint::zero() { out.push(d.clone()); } d += &one; }
    out
}
/// divisorEngagementSearch: first trial divisor, or none.
pub fn divisor_engagement_search(n: &BigUint) -> Option<BigUint> {
    divisor_engagement_candidates(n).into_iter().next()
}
/// divisorSearchFactorPair: exact factor pair from a successful search.
pub fn divisor_search_factor_pair(n: &BigUint, candidate: &BigUint) -> Option<ProperFactorPair> {
    if n % candidate != BigUint::zero() { return None; }
    let q = n / candidate;
    let one = BigUint::from(1u32);
    if *candidate > one && q > one { Some(ProperFactorPair { p: candidate.clone(), q }) } else { None }
}
/// display a BigUint as decimal.
fn show(n: &BigUint) -> String { n.to_string() }

/// REPL entry point for the binary_membrane command.
pub fn repl_binary_membrane(args: &[&str]) -> String {
    if args.is_empty() || args[0] == "help" {
        return String::from(
            "binary_membrane — port of Imscribing.Paraconsistent.BinaryMembrane\n\
             help                      show this help\n\
             split <N>                 δ(N) = (N mod 2, N / 2)\n\
             fuse <b> <q>              μ(b, q) = 2q + b\n\
             roundtrip <N>             certify μ(δ(N)) = N\n\
             channel <N>               even and odd lanes\n\
             engage <N> <candidate>    test ENGAGR: 1 < gcd(candidate,N) < N\n\
             factors <N> <candidate>   membrane factor pair (proper or unit fallback)\n\
             run <N> <candidate>       complete pipeline output\n\
             search <N>                O(N) engagement search, first candidate\n\
             normalize <N> <candidate> gcd(candidate, N) idempotent projection\n\
             divisors <N>              O(sqrt N) trial-divisor candidates\n\
             dsearch <N>               first trial divisor, or none\n\
             dsfactor <N> <candidate>  exact factor pair from divisor search\n"
        );
    }

    match args[0] {
        "help" => repl_binary_membrane(&[]),
        "split" => {
            if args.len() != 2 { return String::from("usage: binary_membrane split <N>"); }
            let n: BigUint = match args[1].parse() { Ok(v) => v, Err(_) => return String::from("invalid N") };
            let (e, o) = binary_split(&n);
            format!("δ({}) = ({}, {})", n, e, o)
        }
        "fuse" => {
            if args.len() != 3 { return String::from("usage: binary_membrane fuse <b> <q>"); }
            let b: BigUint = match args[1].parse() { Ok(v) => v, Err(_) => return String::from("invalid b") };
            let q: BigUint = match args[2].parse() { Ok(v) => v, Err(_) => return String::from("invalid q") };
            let fused = binary_fuse((b.clone(), q.clone()));
            format!("μ({}, {}) = {}", b, q, fused)
        }
        "roundtrip" => {
            if args.len() != 2 { return String::from("usage: binary_membrane roundtrip <N>"); }
            let n: BigUint = match args[1].parse() { Ok(v) => v, Err(_) => return String::from("invalid N") };
            let ok = binary_roundtrip(&n);
            format!("μ(δ({})) = {}  IMSCRIB: {}", n, ok, if ok { "T" } else { "F" })
        }
        "channel" => {
            if args.len() != 2 { return String::from("usage: binary_membrane channel <N>"); }
            let n: BigUint = match args[1].parse() { Ok(v) => v, Err(_) => return String::from("invalid N") };
            let (e, o) = binary_split(&n);
            format!("δ({}) = even={} odd={}", n, e, o)
        }
        "engage" => {
            if args.len() != 3 { return String::from("usage: binary_membrane engage <N> <candidate>"); }
            let n: BigUint = match args[1].parse() { Ok(v) => v, Err(_) => return String::from("invalid N") };
            let c: BigUint = match args[2].parse() { Ok(v) => v, Err(_) => return String::from("invalid candidate") };
            let engaged = paradox_condition(&n, &c);
            let g = big_gcd(&c, &n);
            format!("engage({}, {}) = {}  gcd = {}", n, c, if engaged { "T" } else { "F" }, g)
        }
        "factors" => {
            if args.len() != 3 { return String::from("usage: binary_membrane factors <N> <candidate>"); }
            let n: BigUint = match args[1].parse() { Ok(v) => v, Err(_) => return String::from("invalid N") };
            let c: BigUint = match args[2].parse() { Ok(v) => v, Err(_) => return String::from("invalid candidate") };
            let fp = membrane_factors(&n, &c);
            let proper = paradox_condition(&n, &c);
            format!("factors({}, {}) = p={} q={} proper={}", n, c, fp.p, fp.q, proper)
        }
        "run" => {
            if args.len() != 3 { return String::from("usage: binary_membrane run <N> <candidate>"); }
            let n: BigUint = match args[1].parse() { Ok(v) => v, Err(_) => return String::from("invalid N") };
            let c: BigUint = match args[2].parse() { Ok(v) => v, Err(_) => return String::from("invalid candidate") };
            let out = run_binary_membrane(&n, &c);
            let engaged = paradox_condition(&n, &c);
            let mu_result = &out.even_lane * BigUint::from(2u32) + &out.odd_lane;
            let factors_product = &out.factors.p * &out.factors.q;
            format!(
                "BinaryMembrane({} candidate={}):\n  δ(N) = ({}, {})\n  μ(δ(N)) = {}  closure = {}\n  phase = {}  (isGlut = {})\n  factors: P0 = {}, P1 = {}\n  P0 * P1 = {}\n  proper = {}",
                n, c,
                out.even_lane, out.odd_lane,
                mu_result, out.binary_closure,
                out.phase.to_str(), out.phase.is_glut(),
                out.factors.p, out.factors.q,
                factors_product,
                if engaged { "yes" } else { "no (unit fallback)" }
            )
        }
        "search" => {
            if args.len() != 2 { return String::from("usage: binary_membrane search <N>"); }
            let n: BigUint = match args[1].parse() { Ok(v) => v, Err(_) => return String::from("invalid N") };
            match engagement_search(&n) {
                Some(c) => format!("engagement search for {}: first candidate = {} (gcd = {})", n, c, big_gcd(&c, &n)),
                None => format!("engagement search for {}: no candidate found (no factorization)", n),
            }
        }
        "normalize" => {
            if args.len() != 3 { return String::from("usage: binary_membrane normalize <N> <candidate>"); }
            let n: BigUint = match args[1].parse() { Ok(v) => v, Err(_) => return String::from("invalid N") };
            let c: BigUint = match args[2].parse() { Ok(v) => v, Err(_) => return String::from("invalid candidate") };
            let g = normalize_candidate(&n, &c);
            let idem = normalize_candidate(&n, &g) == g;
            format!("normalize({}, {}) = {}  idempotent = {}", n, c, g, if idem { "T" } else { "F" })
        }
        "divisors" => {
            if args.len() != 2 { return String::from("usage: binary_membrane divisors <N>"); }
            let n: BigUint = match args[1].parse() { Ok(v) => v, Err(_) => return String::from("invalid N") };
            let ds = divisor_engagement_candidates(&n);
            format!("divisors of {} up to sqrt: {}", n, ds.iter().map(|d| d.to_string()).collect::<Vec<_>>().join(", "))
        }
        "dsearch" => {
            if args.len() != 2 { return String::from("usage: binary_membrane dsearch <N>"); }
            let n: BigUint = match args[1].parse() { Ok(v) => v, Err(_) => return String::from("invalid N") };
            match divisor_engagement_search(&n) {
                Some(d) => format!("dsearch({}) = {} (divisor ≤ sqrt)", n, d),
                None => format!("dsearch({}) = none (no proper divisor ≤ sqrt)", n),
            }
        }
        "dsfactor" => {
            if args.len() != 3 { return String::from("usage: binary_membrane dsfactor <N> <candidate>"); }
            let n: BigUint = match args[1].parse() { Ok(v) => v, Err(_) => return String::from("invalid N") };
            let c: BigUint = match args[2].parse() { Ok(v) => v, Err(_) => return String::from("invalid candidate") };
            match divisor_search_factor_pair(&n, &c) {
                Some(pp) => {
                    let product = &pp.p * &pp.q;
                    format!("dsfactor({}, {}) = ({}, {})  P0*P1 = {}", n, c, pp.p, pp.q, product)
                }
                None => format!("dsfactor({}, {}) = none (not a proper factor)", n, c),
            }
        }
        _ => String::from("unknown subcommand (try help)"),
    }
}
