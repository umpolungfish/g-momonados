//! Gödel-grounded arbitrary-width factor extraction with a checked route ladder.
use crate::factor_routes::{big_gcd, is_prime, PrimeVerdict};
use crate::factor_routes::{
    congruence_split, order_multiple_leaping, winding_bridge, BRIDGE_BOUND, CONGRUENCE_FB_BOUND,
    CONGRUENCE_TRIALS, LEAP_STEPS, WINDING_BASES,
};
use crate::godel_analyzer::{prime_sieve_read, PrimeSieveRead};
use crate::godel_calculus::{check, decode, encode_cell_binary, Family, Nat, Operator};
use crate::native_numeral::{
    add_via_word, divmod_via_word, mod_pow_walk, modulo_via_word, multiply_via_word,
    subtract_via_word, to_bits_low_first,
};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use num_bigint::BigUint;
use num_traits::{One, Zero};

const SEMIPRIME_PROTOCOL: &str = "⊢⊙∈⊤⊥⊞∋≻⋈≺⊡⊣";
const PRIME_PROTOCOL: &str = "⊢⊙∈⊤≻⋈⊥≺∋⊞⊡⊣";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    Trivial,
    SieveLane,
    DifferenceOfSquares,
    WindingBridge,
    CongruenceSieve,
    OrderWinding,
    Rho,
    AnyonPhase,
    NativeFactorEngine,
    Prime,
}

impl Route {
    pub fn label(self) -> &'static str {
        match self {
            Self::Trivial => "trivial (bit-support)",
            Self::SieveLane => "small-prime sieve lane",
            Self::DifferenceOfSquares => "difference of squares",
            Self::WindingBridge => "winding bridge (p-1)",
            Self::CongruenceSieve => "congruence sieve",
            Self::OrderWinding => "order winding",
            Self::Rho => "rho",
            Self::AnyonPhase => "anyon phase readout",
            Self::NativeFactorEngine => "native factor engine",
            Self::Prime => "prime (recursion bottom)",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Step {
    pub value: String,
    pub route: Route,
    pub factor: Option<String>,
    pub detail: String,
}

/// A candidate supplied by the source-bound anyon phase executor. Its divisor
/// is checked by this module before either descendant enters the factor tree.
pub struct AnyonCandidate {
    pub factor: BigUint,
    pub detail: String,
}

#[derive(Clone, Debug)]
pub struct Extraction {
    pub source: String,
    pub word: String,
    pub factors: Vec<(String, u32)>,
    pub steps: Vec<Step>,
    pub verified: bool,
    pub leftover: Option<String>,
    pub protocol_match: bool,
    pub expected_protocol: &'static str,
}

impl Extraction {
    pub fn render(&self) -> String {
        let mut out = format!(
            "arbitrary factor extraction\n  source   {}\n  word     {}\n  routes:\n",
            self.source, self.word
        );
        for step in &self.steps {
            out.push_str(&format!(
                "    {:<14} {:<26} {}\n",
                truncate(&step.value, 14),
                step.route.label(),
                step.detail
            ));
        }
        out.push_str("  factorization:\n    ");
        if self.factors.is_empty() {
            out.push_str("(none)\n");
        } else {
            let mut terms: Vec<String> = self
                .factors
                .iter()
                .map(|(p, e)| {
                    if *e == 1 {
                        p.clone()
                    } else {
                        format!("{}^{}", p, e)
                    }
                })
                .collect();
            if let Some(leftover) = &self.leftover {
                terms.push(format!("{} (unresolved)", leftover));
            }
            out.push_str(&format!("{} = {}\n", self.source, terms.join(" × ")));
        }
        out.push_str(&format!(
            "  product-closure  {}\n",
            if self.verified {
                "closed (godel check mul)"
            } else {
                "OPEN"
            }
        ));
        out.push_str(&format!(
            "  protocol-conformance  {}\n  expected-protocol     {}\n",
            if self.protocol_match {
                "PASS"
            } else if !self.verified {
                "UNRESOLVED"
            } else {
                "NOT PRIME / SEMIPRIME"
            },
            self.expected_protocol
        ));
        if let Some(left) = &self.leftover {
            out.push_str(&format!("  leftover cofactor  {}\n", left));
        }
        out
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        format!("{}…{}", &s[..n / 2], &s[s.len() - n / 2..])
    }
}

fn nat_to_biguint(n: &Nat) -> BigUint {
    let mut out = BigUint::zero();
    for (i, bit) in n.bits_le().iter().copied().enumerate() {
        if bit {
            out |= BigUint::one() << i;
        }
    }
    out
}

fn biguint_to_nat(n: &BigUint) -> Nat {
    let mut value = n.clone();
    let mut bits = Vec::new();
    while !value.is_zero() {
        bits.push((&value & BigUint::one()) == BigUint::one());
        value >>= 1usize;
    }
    Nat::from_bits_le(bits)
}

fn word_of(n: &BigUint) -> String {
    encode_cell_binary(&biguint_to_nat(n))
}

fn parse_source(raw: &str) -> Result<(BigUint, String), String> {
    let input = raw.trim();
    if input.starts_with('⊢') {
        let reading = decode(input).map_err(|e| e.to_string())?;
        if reading.family != Family::CellBinary {
            return Err("factor input must be cell-binary or decimal".into());
        }
        let canonical = encode_cell_binary(&reading.value);
        if canonical != input {
            return Err("cell-binary input is not canonical".into());
        }
        Ok((nat_to_biguint(&reading.value), canonical))
    } else {
        let value = BigUint::parse_bytes(input.as_bytes(), 10)
            .ok_or_else(|| format!("not a natural number: {}", raw))?;
        let nat =
            Nat::from_decimal(input).ok_or_else(|| format!("not a natural number: {}", raw))?;
        Ok((value, encode_cell_binary(&nat)))
    }
}

fn verify_split(source_word: &str, p: &BigUint, q: &BigUint) -> bool {
    if p.is_zero() || q.is_zero() {
        return false;
    }
    matches!(check(&word_of(p), Operator::Mul, &word_of(q), source_word), Ok(eq) if eq.valid)
}

fn verified_factor(n: &BigUint, word: &str, p: &BigUint) -> bool {
    if p <= &BigUint::one() || p >= n {
        return false;
    }
    let Some((q, remainder)) = divmod_via_word(n, p) else {
        return false;
    };
    remainder.is_zero() && q > BigUint::one() && verify_split(word, p, &q)
}

fn difference_of_squares_bounded(n: &BigUint, steps: u64) -> Option<BigUint> {
    use crate::native_numeral::isqrt;
    let one = BigUint::one();
    let mut a = isqrt(n);
    if multiply_via_word(&a, &a) < *n {
        a = add_via_word(&a, &one);
    }
    for _ in 0..steps {
        let square = multiply_via_word(&a, &a);
        let gap = subtract_via_word(&square, n)?;
        let b = isqrt(&gap);
        if multiply_via_word(&b, &b) == gap {
            let p = subtract_via_word(&a, &b)?;
            if p > one && p < *n && modulo_via_word(n, &p)?.is_zero() {
                return Some(p);
            }
        }
        a = add_via_word(&a, &one);
    }
    None
}

fn pollard_brent(n: &BigUint, budget: u64) -> Option<BigUint> {
    pollard_brent_seeded(n, budget, 0)
}

fn pollard_brent_seeded(n: &BigUint, budget: u64, seed: u64) -> Option<BigUint> {
    let one = BigUint::one();
    let two = BigUint::from(2u32);
    if n <= &one {
        return None;
    }
    if (n % &two).is_zero() {
        return Some(two);
    }
    let c = BigUint::from(seed) * 2u32 + &one;
    let m = 128u64;
    let f = |x: &BigUint| -> BigUint { (x * x + &c) % n };
    let (mut y, mut r, mut g) = ((BigUint::from(seed) + 2u32) % n, 1u64, one.clone());
    let (mut x, mut ys) = (y.clone(), y.clone());
    let mut spent = 0u64;
    while g == one && spent < budget {
        x = y.clone();
        for _ in 0..r {
            if spent >= budget {
                break;
            }
            y = f(&y);
            spent += 1;
        }
        let mut k = 0u64;
        while k < r && g == one && spent < budget {
            ys = y.clone();
            let mut q = one.clone();
            let lim = core::cmp::min(m, r - k);
            for _ in 0..lim {
                if spent >= budget {
                    break;
                }
                y = f(&y);
                let diff = if x >= y { &x - &y } else { &y - &x };
                q = (&q * &diff) % n;
                spent += 1;
            }
            g = big_gcd(q.clone(), n.clone());
            k = k.saturating_add(m);
        }
        r = r.saturating_mul(2);
    }
    if g == *n {
        while spent < budget {
            ys = f(&ys);
            let diff = if x >= ys { &x - &ys } else { &ys - &x };
            g = big_gcd(diff, n.clone());
            spent += 1;
            if g > one {
                break;
            }
        }
    }
    if g > one && &g < n {
        Some(g)
    } else {
        None
    }
}

fn one_factor(n: &BigUint, steps: &mut Vec<Step>) -> Option<BigUint> {
    let one = BigUint::one();
    let two = BigUint::from(2u32);
    let value = n.to_str_radix(10);
    let word = word_of(n);
    if modulo_via_word(n, &two)?.is_zero() && verified_factor(n, &word, &two) {
        steps.push(Step {
            value,
            route: Route::Trivial,
            factor: Some("2".into()),
            detail: "low cell ⊤ (even)".into(),
        });
        return Some(two);
    }
    match prime_sieve_read(&word, 1usize << 16) {
        Ok(PrimeSieveRead::Factor { p, .. }) => {
            let p = nat_to_biguint(&p);
            if verified_factor(n, &word, &p) {
                steps.push(Step {
                    value,
                    route: Route::SieveLane,
                    factor: Some(p.to_string()),
                    detail: "least prime divisor ≤ 2^16".into(),
                });
                return Some(p);
            }
        }
        Ok(PrimeSieveRead::Prime { .. }) => return None,
        _ => {}
    }
    steps.push(Step {
        value: value.clone(),
        route: Route::SieveLane,
        factor: None,
        detail: "no factor ≤ 2^16".into(),
    });
    // Perfect squares close immediately, before a native engine is needed.
    if let Some(p) = difference_of_squares_bounded(n, 1).filter(|p| verified_factor(n, &word, p)) {
        steps.push(Step {
            value,
            route: Route::DifferenceOfSquares,
            factor: Some(p.to_string()),
            detail: "first square bridge closes".into(),
        });
        return Some(p);
    }
    if let Some(p) = native_factor_candidate(n).filter(|p| verified_factor(n, &word, p)) {
        steps.push(Step {
            value,
            route: Route::NativeFactorEngine,
            factor: Some(p.to_string()),
            detail: "PARI factor candidate; Gödel multiplication closed".into(),
        });
        return Some(p);
    }
    const SQUARES_WORK: u64 = 4_096;
    if let Some(p) = difference_of_squares_bounded(n, SQUARES_WORK) {
        if verified_factor(n, &word, &p) {
            steps.push(Step {
                value: value.clone(),
                route: Route::DifferenceOfSquares,
                factor: Some(p.to_string()),
                detail: format!("n = a² − b² within {} Fermat steps", SQUARES_WORK),
            });
            return Some(p);
        }
    }
    steps.push(Step {
        value: value.clone(),
        route: Route::DifferenceOfSquares,
        factor: None,
        detail: format!(
            "no close-factor bridge within {} Fermat steps",
            SQUARES_WORK
        ),
    });
    // These native-word routes use exact word operations; stage practical
    // apertures here so an unproductive route cannot monopolize extraction.
    const BRIDGE_WORK: u64 = 512;
    const RELATION_WORK: u64 = 512;
    const ORDER_WORK: u64 = 64;
    const RHO_WORK: u64 = 2_000_000;
    if let Some(p) =
        winding_bridge(n, BRIDGE_BOUND.min(BRIDGE_WORK)).filter(|p| verified_factor(n, &word, p))
    {
        steps.push(Step {
            value: value.clone(),
            route: Route::WindingBridge,
            factor: Some(p.to_string()),
            detail: format!("p−1 winding bound {}", BRIDGE_WORK.min(BRIDGE_BOUND)),
        });
        return Some(p);
    }
    steps.push(Step {
        value: value.clone(),
        route: Route::WindingBridge,
        factor: None,
        detail: "no smooth winding at this bound".into(),
    });
    if let Some((p, _)) =
        congruence_split(n, CONGRUENCE_FB_BOUND, CONGRUENCE_TRIALS.min(RELATION_WORK))
    {
        if verified_factor(n, &word, &p) {
            steps.push(Step {
                value: value.clone(),
                route: Route::CongruenceSieve,
                factor: Some(p.to_string()),
                detail: "parity cancellation: X² ≡ Y² (mod n)".into(),
            });
            return Some(p);
        }
    }
    steps.push(Step {
        value: value.clone(),
        route: Route::CongruenceSieve,
        factor: None,
        detail: "no relation in trial budget".into(),
    });
    for &base in &WINDING_BASES {
        let a = modulo_via_word(&BigUint::from(base), n)?;
        if a < two {
            continue;
        }
        let shared = big_gcd(a.clone(), n.clone());
        if verified_factor(n, &word, &shared) {
            steps.push(Step {
                value: value.clone(),
                route: Route::OrderWinding,
                factor: Some(shared.to_string()),
                detail: format!("base {} shares a factor", base),
            });
            return Some(shared);
        }
        let Some(mut r) = order_multiple_leaping(&a, n, LEAP_STEPS.min(ORDER_WORK)) else {
            continue;
        };
        while modulo_via_word(&r, &two)?.is_zero() {
            let half = divmod_via_word(&r, &two)?.0;
            let a_half = mod_pow_walk(&a, &to_bits_low_first(&half), n);
            if a_half == one {
                r = half;
                continue;
            }
            if a_half == subtract_via_word(n, &one)? {
                break;
            }
            let p = big_gcd(subtract_via_word(&a_half, &one)?, n.clone());
            if verified_factor(n, &word, &p) {
                steps.push(Step {
                    value: value.clone(),
                    route: Route::OrderWinding,
                    factor: Some(p.to_string()),
                    detail: format!("winding r = {} for base {}", r, base),
                });
                return Some(p);
            }
            break;
        }
    }
    steps.push(Step {
        value: value.clone(),
        route: Route::OrderWinding,
        factor: None,
        detail: "no base closed in leap budget".into(),
    });
    if let Some(p) =
        pollard_brent(n, LEAP_STEPS.min(RHO_WORK)).filter(|p| verified_factor(n, &word, p))
    {
        steps.push(Step {
            value,
            route: Route::Rho,
            factor: Some(p.to_string()),
            detail: "Brent cycle hit".into(),
        });
        return Some(p);
    }
    steps.push(Step {
        value,
        route: Route::Rho,
        factor: None,
        detail: "rho exhausted its step budget".into(),
    });
    None
}

/// The local native engine receives N alone. Its output is a candidate and
/// passes the same word-level split gate as every other route.
#[cfg(feature = "hosted")]
fn native_factor_candidate(n: &BigUint) -> Option<BigUint> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    let mut child = Command::new("gp")
        .args(["-q", "-f", "-s", "64000000"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    let input = format!("print(factor({})[1,1]);quit(0)\n", n.to_str_radix(10));
    let written = child
        .stdin
        .take()
        .and_then(|mut pipe| pipe.write_all(input.as_bytes()).ok());
    if written.is_none() {
        let _ = child.kill();
        let _ = child.wait();
        return None;
    }
    // This optional route must yield to the remaining extraction routes.
    // Killing and reaping our own GP child leaves no abandoned factoring job.
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                eprintln!(
                    "factor extraction: {}-bit native candidate attempt ended; advancing routes",
                    n.bits()
                );
                return None;
            }
        }
    }
    let output = child.wait_with_output().ok()?;
    if !output.status.success() {
        return None;
    }
    let candidate = core::str::from_utf8(&output.stdout).ok()?.trim();
    if candidate.is_empty() || !candidate.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    BigUint::parse_bytes(candidate.as_bytes(), 10)
}

#[cfg(not(feature = "hosted"))]
fn native_factor_candidate(_: &BigUint) -> Option<BigUint> {
    None
}

/// Exhaustion advances seeds and work budgets. A steadily advancing exact
/// divisor lane runs beside the retries, with no retained state history.
fn continue_factor(n: &BigUint, steps: &mut Vec<Step>) -> Result<BigUint, String> {
    let word = word_of(n);
    let slot = steps.len();
    steps.push(Step {
        value: n.to_string(),
        route: Route::Rho,
        factor: None,
        detail: "continuing extraction".into(),
    });
    let mut attempt = 1u64;
    let mut budget = 2_000_000u64;
    let mut divisor = BigUint::from(65_537u32);
    loop {
        #[cfg(feature = "hosted")]
        std::eprintln!(
            "factor extraction: {}-bit cofactor, retry {attempt}, rho work {budget}",
            n.bits()
        );
        if let Some(p) =
            pollard_brent_seeded(n, budget, attempt).filter(|p| verified_factor(n, &word, p))
        {
            steps[slot].factor = Some(p.to_string());
            steps[slot].detail = format!("retry {attempt} closes through Gödel multiplication");
            return Ok(p);
        }
        for _ in 0..1_024 {
            if &divisor * &divisor > *n {
                return Err("primality and exhaustive divisor readings disagree".into());
            }
            if (n % &divisor).is_zero() && verified_factor(n, &word, &divisor) {
                steps[slot].route = Route::SieveLane;
                steps[slot].factor = Some(divisor.to_string());
                steps[slot].detail =
                    "continuing divisor lane closes through Gödel multiplication".into();
                return Ok(divisor);
            }
            divisor += 2u32;
        }
        attempt = attempt.wrapping_add(1);
        budget = budget.saturating_mul(2).min(64_000_000);
        steps[slot].detail = format!("continuing at retry {attempt}, rho work {budget}");
    }
}

#[cfg(test)]
mod tests {
    use super::{
        extract, extract_with_anyons, verified_factor, word_of, AnyonCandidate, Route,
        SEMIPRIME_PROTOCOL,
    };
    use num_bigint::BigUint;
    use num_traits::One;

    #[test]
    fn closes_certified_unstructured_candidate_fixtures_at_all_required_widths() {
        let manifest = include_str!("../measurements/anyon-extractor-width-controls.tsv");
        let mut widths = Vec::new();
        for line in manifest.lines().skip(1) {
            let fields: Vec<_> = line.split('\t').collect();
            let width: u64 = fields[1].parse().unwrap();
            let source = BigUint::parse_bytes(fields[2].as_bytes(), 10).unwrap();
            let p = BigUint::parse_bytes(fields[3].as_bytes(), 10).unwrap();
            let q = BigUint::parse_bytes(fields[4].as_bytes(), 10).unwrap();
            assert_eq!(source.bits(), width);
            assert_eq!(&p * &q, source);
            let mut requests = 0;
            // This callback supplies a certified fixture candidate. It tests
            // recursive extraction and closure, not measured device phases.
            let report = extract_with_anyons(fields[2], |requested| {
                requests += 1;
                assert_eq!(requested, &source);
                Ok(Some(AnyonCandidate {
                    factor: p.clone(),
                    detail: "certified candidate fixture".into(),
                }))
            })
            .unwrap();
            assert_eq!(requests, 1);
            assert!(report.verified, "{width}-bit candidate failed closure");
            let mut expected = vec![(p.to_string(), 1), (q.to_string(), 1)];
            if p > q {
                expected.swap(0, 1);
            }
            assert_eq!(report.factors, expected);
            assert_eq!(report.expected_protocol, SEMIPRIME_PROTOCOL);
            widths.push(width);
        }
        assert_eq!(widths, vec![128, 256, 512, 1024, 2048]);
    }

    #[test]
    fn verifies_an_anyon_candidate_for_a_balanced_128_bit_source() {
        let source = "296650821743515430283258444261036507151";
        let value = BigUint::parse_bytes(source.as_bytes(), 10).unwrap();
        let p = BigUint::from(16_925_480_323_643_806_501u64);
        let q = &value / &p;
        let mut requests = 0;
        // The candidate fixture checks the extractor interface. It is not a
        // device readout or an execution of phase estimation.
        let report = extract_with_anyons(source, |requested| {
            requests += 1;
            assert_eq!(requested, &value);
            Ok(Some(AnyonCandidate {
                factor: p.clone(),
                detail: "candidate fixture".into(),
            }))
        })
        .unwrap();
        assert_eq!(requests, 1);
        assert!(report.verified, "{}", report.render());
        assert_eq!(report.factors, vec![(p.to_string(), 1), (q.to_string(), 1)]);
        assert_eq!(report.expected_protocol, SEMIPRIME_PROTOCOL);
        assert!(report
            .steps
            .iter()
            .any(|step| step.route == Route::AnyonPhase && step.factor.is_some()));
        assert!(!report.steps.iter().any(|step| step.route == Route::Rho));
    }

    #[test]
    fn descends_recursively_when_an_anyon_candidate_is_itself_composite() {
        let prime = BigUint::parse_bytes(b"18446744073709551557", 10).unwrap();
        let square = &prime * &prime;
        let source = &square * &square;
        assert_eq!(source.bits(), 256);
        let mut widths = Vec::new();
        let report = extract_with_anyons(&source.to_string(), |requested| {
            widths.push(requested.bits());
            let factor = if requested == &source {
                square.clone()
            } else {
                prime.clone()
            };
            Ok(Some(AnyonCandidate {
                factor,
                detail: "candidate fixture".into(),
            }))
        })
        .unwrap();
        assert_eq!(widths, vec![256, 128, 128]);
        assert!(report.verified, "{}", report.render());
        assert_eq!(report.factors, vec![(prime.to_string(), 4)]);
    }

    #[test]
    fn falls_through_an_invalid_anyon_candidate_and_preserves_device_errors() {
        let prime = BigUint::parse_bytes(b"18446744073709551557", 10).unwrap();
        let source = (&prime * &prime).to_string();
        let report = extract_with_anyons(&source, |_| {
            Ok(Some(AnyonCandidate {
                factor: &prime + BigUint::one(),
                detail: "invalid candidate fixture".into(),
            }))
        })
        .unwrap();
        assert!(report.verified, "{}", report.render());
        assert!(report
            .steps
            .iter()
            .any(|step| step.route == Route::AnyonPhase && step.factor.is_none()));
        assert!(report
            .steps
            .iter()
            .any(|step| step.route == Route::DifferenceOfSquares && step.factor.is_some()));
        let failure =
            extract_with_anyons(&source, |_| Err("device readout failed".into())).unwrap_err();
        assert_eq!(failure, "device readout failed");
    }

    #[test]
    fn continues_the_ladder_after_an_unclosed_shot_budget() {
        let p = BigUint::from(2_097_143u64);
        let q = BigUint::parse_bytes(b"162259276829213363391578010288127", 10).unwrap();
        let source = &p * &q;
        assert_eq!(source.bits(), 128);
        let report = extract_with_anyons(&source.to_string(), |_| Ok(None)).unwrap();
        assert!(report.verified, "{}", report.render());
        assert_eq!(report.factors, vec![(p.to_string(), 1), (q.to_string(), 1)]);
        assert!(report
            .steps
            .iter()
            .any(|step| step.route == Route::AnyonPhase && step.factor.is_none()));
    }

    #[test]
    fn preserves_stripped_prime_power_multiplicity_on_a_128_bit_source() {
        let prime = (BigUint::one() << 89usize) - BigUint::one();
        let source = &prime << 39usize;
        assert_eq!(source.bits(), 128);
        let report = extract(&source.to_str_radix(10)).unwrap();
        assert!(report.verified, "{}", report.render());
        assert_eq!(
            report.factors,
            vec![("2".to_string(), 39), (prime.to_string(), 1)]
        );
    }

    #[test]
    fn rejects_invalid_route_candidates_against_the_128_bit_source_word() {
        let source = BigUint::parse_bytes(b"296650821743515430283258444261036507151", 10).unwrap();
        let prime = BigUint::from(16_925_480_323_643_806_501u64);
        assert_eq!(source.bits(), 128);
        let word = word_of(&source);
        assert!(verified_factor(&source, &word, &prime));
        assert!(!verified_factor(&source, &word, &(&prime + BigUint::one())));
        assert!(!verified_factor(
            &source,
            &word_of(&(&source + BigUint::one())),
            &prime
        ));
        assert!(!verified_factor(&source, &word, &BigUint::one()));
        assert!(!verified_factor(&source, &word, &source));
    }

    #[test]
    fn closes_a_128_bit_prime_square_through_godel_multiplication() {
        let prime = BigUint::parse_bytes(b"18446744073709551557", 10).unwrap();
        let semiprime = &prime * &prime;
        assert!(semiprime.bits() >= 128);
        let report = extract(&semiprime.to_str_radix(10)).unwrap();
        assert!(report.verified, "{}", report.render());
        assert!(report.protocol_match, "{}", report.render());
        assert_eq!(report.expected_protocol, SEMIPRIME_PROTOCOL);
        assert_eq!(report.factors, vec![(prime.to_str_radix(10), 2)]);
        assert!(report
            .steps
            .iter()
            .any(|step| step.route == Route::DifferenceOfSquares));
        let word_report = extract(&word_of(&semiprime)).unwrap();
        assert!(word_report.verified, "{}", word_report.render());
        assert!(word_report.protocol_match, "{}", word_report.render());
    }

    #[test]
    fn factors_an_unbalanced_128_bit_semiprime() {
        // The factors are deliberately far apart, so Fermat's close-factor
        // route cannot solve this case within its budget. Both prime factors
        // exceed the sieve aperture and their product remains 128-bit.
        let p = BigUint::from(2_097_143u64); // prime, well above the sieve aperture
        let q = BigUint::parse_bytes(b"162259276829213363391578010288127", 10).unwrap(); // 2^107 - 1, prime
        let semiprime = &p * &q;
        assert_eq!(semiprime.bits(), 128);
        let source = semiprime.to_str_radix(10);
        let report = extract(&source).unwrap();
        assert!(report.verified, "{}", report.render());
        assert!(report.protocol_match, "{}", report.render());
        assert_eq!(report.expected_protocol, SEMIPRIME_PROTOCOL);
        assert_eq!(report.factors, vec![(p.to_string(), 1), (q.to_string(), 1)]);
        assert!(!report
            .steps
            .iter()
            .any(|step| step.route == Route::DifferenceOfSquares && step.factor.is_some()));
    }
}

/// Extract prime powers in ascending order; every split closes through Gödel multiplication.
pub fn extract(raw: &str) -> Result<Extraction, String> {
    extract_inner(raw, |_| Ok(None), false)
}

/// Try measured anyon splits before the bounded ladder for each composite
/// descendant of at least 128 bits. Smaller descendants use the ladder.
pub fn extract_with_anyons<F>(raw: &str, splitter: F) -> Result<Extraction, String>
where
    F: FnMut(&BigUint) -> Result<Option<AnyonCandidate>, String>,
{
    extract_inner(raw, splitter, true)
}

fn extract_inner<F>(raw: &str, mut splitter: F, anyonic: bool) -> Result<Extraction, String>
where
    F: FnMut(&BigUint) -> Result<Option<AnyonCandidate>, String>,
{
    let (source_value, word) = parse_source(raw)?;
    if anyonic && source_value.bits() < 128 {
        return Err("arbitrary anyonic extraction requires a source of at least 128 bits".into());
    }
    let source = source_value.to_str_radix(10);
    if source_value.is_zero() {
        return Err("0 has no finite factorization".into());
    }
    let one = BigUint::one();
    let two = BigUint::from(2u32);
    let mut steps = Vec::new();
    let mut factors: Vec<(BigUint, u32)> = Vec::new();
    let leftover: Option<BigUint> = None;
    let mut pending = alloc::vec![source_value.clone()];
    let iteration_limit = (source_value.bits() as usize)
        .saturating_mul(2)
        .saturating_add(1);
    let mut iterations = 0usize;
    while let Some(mut n) = pending.pop() {
        if n <= one {
            continue;
        }
        iterations += 1;
        if iterations > iteration_limit {
            return Err("strict-descendant extraction exceeded its structural node bound".into());
        }
        if is_prime(&n.to_string()) == PrimeVerdict::Prime {
            steps.push(Step {
                value: n.to_string(),
                route: Route::Prime,
                factor: Some(n.to_string()),
                detail: "prime (recursion bottom)".into(),
            });
            factors.push((n.clone(), 1));
            continue;
        }
        if modulo_via_word(&n, &two)
            .ok_or_else(|| "word remainder failed while stripping factor 2".to_string())?
            .is_zero()
        {
            let mut exponent = 0u32;
            while modulo_via_word(&n, &two)
                .ok_or_else(|| "word remainder failed while stripping factor 2".to_string())?
                .is_zero()
            {
                n = divmod_via_word(&n, &two)
                    .ok_or_else(|| "word division failed while stripping factor 2".to_string())?
                    .0;
                exponent = exponent
                    .checked_add(1)
                    .ok_or_else(|| "factor exponent overflow".to_string())?;
            }
            factors.push((two.clone(), exponent));
            steps.push(Step {
                value: n.to_string(),
                route: Route::Trivial,
                factor: Some("2".into()),
                detail: format!("stripped 2^{}", exponent),
            });
            if n > one {
                pending.push(n);
            }
            continue;
        }
        let measured_factor = if anyonic && n.bits() >= 128 {
            match splitter(&n)? {
                Some(candidate) if verified_factor(&n, &word_of(&n), &candidate.factor) => {
                    steps.push(Step {
                        value: n.to_string(),
                        route: Route::AnyonPhase,
                        factor: Some(candidate.factor.to_string()),
                        detail: candidate.detail,
                    });
                    Some(candidate.factor)
                }
                Some(_) => {
                    steps.push(Step {
                        value: n.to_string(),
                        route: Route::AnyonPhase,
                        factor: None,
                        detail: "discarded candidate without Gödel split closure".into(),
                    });
                    None
                }
                None => {
                    steps.push(Step {
                        value: n.to_string(),
                        route: Route::AnyonPhase,
                        factor: None,
                        detail: "shot budget exhausted without a factor split".into(),
                    });
                    None
                }
            }
        } else {
            None
        };
        let p = match measured_factor.or_else(|| one_factor(&n, &mut steps)) {
            Some(candidate) => candidate,
            None => continue_factor(&n, &mut steps)?,
        };
        let (q, rem) = divmod_via_word(&n, &p)
            .ok_or_else(|| "word division failed for route candidate".to_string())?;
        if !rem.is_zero() || !verify_split(&word_of(&n), &p, &q) {
            return Err(format!(
                "route split failed Gödel multiplication closure: {} × {} against {}",
                p, q, n
            ));
        }
        if p <= one || q <= one || p >= n || q >= n {
            return Err("route did not produce two strict descendants".into());
        }
        pending.push(p);
        pending.push(q);
    }
    factors.sort_by(|a, b| a.0.cmp(&b.0));
    let mut compressed: Vec<(BigUint, u32)> = Vec::new();
    for (prime, count) in factors {
        if let Some((last, exponent)) = compressed.last_mut() {
            if *last == prime {
                *exponent = exponent
                    .checked_add(count)
                    .ok_or_else(|| "factor exponent overflow".to_string())?;
                continue;
            }
        }
        compressed.push((prime, count));
    }
    let mut product = one.clone();
    for (p, exponent) in &compressed {
        for _ in 0..*exponent {
            product = multiply_via_word(&product, p);
        }
    }
    let source_value = BigUint::parse_bytes(source.as_bytes(), 10)
        .ok_or_else(|| "source conversion failed".to_string())?;
    let closed = leftover.is_none()
        && product == source_value
        && check(&word_of(&product), Operator::Mul, &word_of(&one), &word).is_ok_and(|eq| eq.valid);
    if !closed {
        return Err(
            "factorization did not reconstruct the source through Gödel multiplication".into(),
        );
    }
    let total_multiplicity: u32 = compressed.iter().map(|(_, exponent)| *exponent).sum();
    let expected_protocol = if !closed {
        "UNRESOLVED"
    } else if total_multiplicity == 1 {
        PRIME_PROTOCOL
    } else if total_multiplicity == 2 {
        SEMIPRIME_PROTOCOL
    } else {
        "UNKNOWN / COMPLEX COMPOSITE"
    };
    let protocol_match = match expected_protocol {
        PRIME_PROTOCOL => closed && compressed.len() == 1 && compressed[0].1 == 1,
        SEMIPRIME_PROTOCOL => closed && total_multiplicity == 2,
        _ => false,
    };
    Ok(Extraction {
        source,
        word,
        factors: compressed
            .into_iter()
            .map(|(p, e)| (p.to_string(), e))
            .collect(),
        steps,
        verified: closed,
        leftover: leftover.map(|v| v.to_string()),
        protocol_match,
        expected_protocol,
    })
}
