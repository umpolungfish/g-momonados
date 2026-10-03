//! Gödel-grounded arbitrary-width factor extraction with a checked route ladder.
use crate::native_numeral::{
    add_via_word, divmod_via_word, mod_pow_walk, modulo_via_word, multiply_via_word,
    subtract_via_word, to_bits_low_first,
};
use crate::prime_winding::{big_gcd, is_prime, PrimeVerdict};
use crate::trilattice_factor::{
    congruence_split, order_multiple_leaping, winding_bridge, BRIDGE_BOUND, CONGRUENCE_FB_BOUND,
    CONGRUENCE_TRIALS, LEAP_STEPS, WINDING_BASES,
};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use g_momonados::godel_analyzer::{prime_sieve_read, PrimeSieveRead};
use g_momonados::godel_calculus::{check, decode, encode_cell_binary, Family, Nat, Operator};
use num_bigint::BigUint;
use num_traits::{One, Zero};

const SEMIPRIME_PROTOCOL: &str = "⊢∋∈⊤⊥⊞∋≻⋈≺⊡⊣";
const PRIME_PROTOCOL: &str = "⊣⊣⊙∈⊤≻⋈⊥≺∋⊞⊡";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    Trivial,
    SieveLane,
    DifferenceOfSquares,
    WindingBridge,
    CongruenceSieve,
    OrderWinding,
    Rho,
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
            let terms: Vec<String> = self
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
            if self.protocol_match { "PASS" } else { "FAIL" },
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
    let one = BigUint::one();
    let two = BigUint::from(2u32);
    if n <= &one {
        return None;
    }
    if modulo_via_word(n, &two)?.is_zero() {
        return Some(two);
    }
    let c = one.clone();
    let m = 128u64;
    let f = |x: &BigUint| -> Option<BigUint> {
        modulo_via_word(&add_via_word(&multiply_via_word(x, x), &c), n)
    };
    let (mut y, mut r, mut q, mut g) = (BigUint::from(2u32), 1u64, one.clone(), one.clone());
    let (mut x, mut ys) = (y.clone(), y.clone());
    let mut spent = 0u64;
    while g == one && spent < budget {
        x = y.clone();
        for _ in 0..r {
            if spent >= budget {
                break;
            }
            y = f(&y)?;
            spent += 1;
        }
        let mut k = 0u64;
        while k < r && g == one && spent < budget {
            ys = y.clone();
            let lim = core::cmp::min(m, r - k);
            for _ in 0..lim {
                if spent >= budget {
                    break;
                }
                y = f(&y)?;
                let diff = if x >= y {
                    subtract_via_word(&x, &y)?
                } else {
                    subtract_via_word(&y, &x)?
                };
                q = modulo_via_word(&multiply_via_word(&q, &diff), n)?;
                spent += 1;
            }
            g = big_gcd(q.clone(), n.clone());
            k = k.saturating_add(m);
        }
        r = r.saturating_mul(2);
    }
    if g == *n {
        while spent < budget {
            ys = f(&ys)?;
            let diff = if x >= ys {
                subtract_via_word(&x, &ys)?
            } else {
                subtract_via_word(&ys, &x)?
            };
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
    if modulo_via_word(n, &two)?.is_zero() {
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
            if p > one && &p < n {
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
    const SQUARES_WORK: u64 = 4_096;
    if let Some(p) = difference_of_squares_bounded(n, SQUARES_WORK) {
        if p > one && &p < n {
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
    const RHO_WORK: u64 = 4_096;
    if let Some(p) = winding_bridge(n, BRIDGE_BOUND.min(BRIDGE_WORK)).filter(|p| p > &one && p < n)
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
        if p > one && &p < n {
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
        if shared > one && &shared < n {
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
            if p > one && &p < n {
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
    if let Some(p) = pollard_brent(n, LEAP_STEPS.min(RHO_WORK)).filter(|p| p > &one && p < n) {
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

#[cfg(test)]
mod tests {
    use super::{extract, word_of, Route, SEMIPRIME_PROTOCOL};
    use num_bigint::BigUint;

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
}

/// Extract prime powers in ascending order; every split closes through Gödel multiplication.
pub fn extract(raw: &str) -> Result<Extraction, String> {
    let (mut n, word) = parse_source(raw)?;
    let source = n.to_str_radix(10);
    if n.is_zero() {
        return Err("0 has no finite factorization".into());
    }
    let one = BigUint::one();
    let two = BigUint::from(2u32);
    let mut steps = Vec::new();
    let mut factors: Vec<(BigUint, u32)> = Vec::new();
    let mut leftover = None;
    let mut iterations = 0usize;
    while n > one && iterations < 4096 {
        iterations += 1;
        if is_prime(&n.to_string()) == PrimeVerdict::Prime {
            steps.push(Step {
                value: n.to_string(),
                route: Route::Prime,
                factor: Some(n.to_string()),
                detail: "prime (recursion bottom)".into(),
            });
            factors.push((n.clone(), 1));
            n = one.clone();
            break;
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
            continue;
        }
        match one_factor(&n, &mut steps) {
            Some(p) => {
                let (q, rem) = divmod_via_word(&n, &p)
                    .ok_or_else(|| "word division failed for route candidate".to_string())?;
                if !rem.is_zero() || !verify_split(&word_of(&n), &p, &q) {
                    return Err(format!(
                        "route split failed Gödel multiplication closure: {} × {} against {}",
                        p, q, n
                    ));
                }
                let mut exponent = 0u32;
                while modulo_via_word(&n, &p)
                    .ok_or_else(|| {
                        "word remainder failed while extracting prime power".to_string()
                    })?
                    .is_zero()
                {
                    n = divmod_via_word(&n, &p)
                        .ok_or_else(|| {
                            "word division failed while extracting prime power".to_string()
                        })?
                        .0;
                    exponent = exponent
                        .checked_add(1)
                        .ok_or_else(|| "factor exponent overflow".to_string())?;
                }
                if exponent == 0 {
                    return Err("route made no descent".into());
                }
                factors.push((p, exponent));
            }
            None => {
                leftover = Some(n.clone());
                break;
            }
        }
    }
    if n > one && leftover.is_none() {
        leftover = Some(n.clone());
    }
    factors.sort_by(|a, b| a.0.cmp(&b.0));
    let mut product = one.clone();
    for (p, exponent) in &factors {
        for _ in 0..*exponent {
            product = multiply_via_word(&product, p);
        }
    }
    let source_value = BigUint::parse_bytes(source.as_bytes(), 10)
        .ok_or_else(|| "source conversion failed".to_string())?;
    let closed = leftover.is_none()
        && product == source_value
        && check(&word_of(&product), Operator::Mul, &word_of(&one), &word).is_ok_and(|eq| eq.valid);
    let total_multiplicity: u32 = factors.iter().map(|(_, exponent)| *exponent).sum();
    let expected_protocol = if closed && total_multiplicity == 1 {
        PRIME_PROTOCOL
    } else if closed && total_multiplicity == 2 {
        SEMIPRIME_PROTOCOL
    } else {
        "UNKNOWN / COMPLEX COMPOSITE"
    };
    let protocol_match = match expected_protocol {
        PRIME_PROTOCOL => closed && factors.len() == 1 && factors[0].1 == 1,
        SEMIPRIME_PROTOCOL => closed && total_multiplicity == 2,
        _ => false,
    };
    Ok(Extraction {
        source,
        word,
        factors: factors
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
