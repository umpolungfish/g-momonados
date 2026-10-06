//! Gödel-grounded arbitrary-width factor extraction — anyonic ququart phase readout ONLY.
//! All classical fallback routes (sieve, Fermat, Pollard, ECM, etc.) have been removed.
//! The only route is Route::AnyonPhase via extract_with_anyons.

use crate::godel_calculus::{check, decode, encode_cell_binary, Family, Nat, Operator};
use crate::native_numeral::{
    divmod_via_word, multiply_via_word, modulo_via_word,
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
    AnyonPhase,
    Prime,
}

impl Route {
    pub fn label(self) -> &'static str {
        match self {
            Self::Trivial => "trivial (bit-support)",
            Self::AnyonPhase => "anyon phase readout",
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

pub(crate) fn nat_to_biguint(n: &Nat) -> BigUint {
    let mut out = BigUint::zero();
    for (i, bit) in n.bits_le().iter().copied().enumerate() {
        if bit {
            out |= BigUint::one() << i;
        }
    }
    out
}

pub fn nat_to_biguint_pub(n: &Nat) -> BigUint {
    nat_to_biguint(n)
}

pub(crate) fn biguint_to_nat(n: &BigUint) -> Nat {
    let mut value = n.clone();
    let mut bits = Vec::new();
    while !value.is_zero() {
        bits.push((&value & BigUint::one()) == BigUint::one());
        value >>= 1usize;
    }
    Nat::from_bits_le(bits)
}

pub(crate) fn word_of(n: &BigUint) -> String {
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

/// Extract prime powers in ascending order; every split closes through Gödel multiplication.
/// ONLY Route::AnyonPhase is used — the splitter callback must supply factors via
/// the anyonic ququart phase readout device. No classical fallbacks.
pub fn extract(_raw: &str) -> Result<Extraction, String> {
    Err("classical extract() removed — use extract_with_anyons with a ququart device".into())
}

/// Try measured anyon splits before the bounded ladder for each composite
/// descendant of at least 128 bits.
pub fn extract_with_anyons<F>(raw: &str, splitter: F) -> Result<Extraction, String>
where
    F: FnMut(&BigUint) -> Result<Option<AnyonCandidate>, String>,
{
    extract_inner(raw, splitter)
}

fn extract_inner<F>(raw: &str, mut splitter: F) -> Result<Extraction, String>
where
    F: FnMut(&BigUint) -> Result<Option<AnyonCandidate>, String>,
{
    let (source_value, word) = parse_source(raw)?;
    if source_value.bits() < 128 {
        return Err("anyonic extraction requires a source of at least 128 bits".into());
    }
    let source = source_value.to_str_radix(10);
    if source_value.is_zero() {
        return Err("0 has no finite factorization".into());
    }
    let one = BigUint::one();
    let two = BigUint::from(2u32);
    let mut steps = Vec::new();
    let mut factors: Vec<(BigUint, u32)> = Vec::new();
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
        // Strip factor 2 (trivial bit-support) — only classical micro-step retained
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
        // Prime check — recursion bottom
        if crate::factor_routes::is_prime(&n.to_string()) == crate::factor_routes::PrimeVerdict::Prime {
            steps.push(Step {
                value: n.to_string(),
                route: Route::Prime,
                factor: Some(n.to_string()),
                detail: "prime (recursion bottom)".into(),
            });
            factors.push((n.clone(), 1));
            continue;
        }
        // ANYONIC QUQUART PHASE READOUT — the ONLY composite route
        let measured_factor = match splitter(&n)? {
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
        };
        let p = match measured_factor {
            Some(candidate) => candidate,
            None => {
                return Err("anyon phase readout did not yield a factor — no classical fallback".into());
            }
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
    let closed = product == source_value
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
        leftover: None,
        protocol_match,
        expected_protocol,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        extract_with_anyons, verified_factor, word_of, AnyonCandidate, Route,
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
        let failure =
            extract_with_anyons(&source, |_| Err("device readout failed".into())).unwrap_err();
        assert_eq!(failure, "device readout failed");
    }

    #[test]
    fn preserves_stripped_prime_power_multiplicity_on_a_128_bit_source() {
        let prime = (BigUint::one() << 89usize) - BigUint::one();
        let source = &prime << 39usize;
        assert_eq!(source.bits(), 128);
        let report = extract_with_anyons(&source.to_str_radix(10), |_| Ok(None)).unwrap();
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
        let report = extract_with_anyons(&semiprime.to_str_radix(10), |_| Ok(None)).unwrap();
        assert!(report.verified, "{}", report.render());
        assert!(report.protocol_match, "{}", report.render());
        assert_eq!(report.expected_protocol, SEMIPRIME_PROTOCOL);
        assert_eq!(report.factors, vec![(prime.to_str_radix(10), 2)]);
        let word_report = extract_with_anyons(&word_of(&semiprime), |_| Ok(None)).unwrap();
        assert!(word_report.verified, "{}", word_report.render());
        assert!(word_report.protocol_match, "{}", word_report.render());
    }

    #[test]
    fn factors_an_unbalanced_128_bit_semiprime() {
        let p = BigUint::from(2_097_143u64);
        let q = BigUint::parse_bytes(b"162259276829213363391578010288127", 10).unwrap();
        let semiprime = &p * &q;
        assert_eq!(semiprime.bits(), 128);
        let source = semiprime.to_str_radix(10);
        let report = extract_with_anyons(&source, |_| Ok(None)).unwrap();
        assert!(report.verified, "{}", report.render());
        assert!(report.protocol_match, "{}", report.render());
        assert_eq!(report.expected_protocol, SEMIPRIME_PROTOCOL);
        assert_eq!(report.factors, vec![(p.to_string(), 1), (q.to_string(), 1)]);
    }
}