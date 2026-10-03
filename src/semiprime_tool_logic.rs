//! Semiprime Factor Pair - Gödel-Encoding Relationship Tool
//!
//! Protocol: ⊢⊙∈⊤⊥⊞∋≻⋈≺⊡⊣
//! Focus: Conjunctive reconstruction of N = p * q via SHIAB/Residual frames.

use crate::godel_analyzer::{
    analyze_with_sieve, prime_sieve_read, render as render_analysis, DivisorBoundCertificate,
    PrimeSieveRead,
};
use crate::godel_calculus::{check, decode, encode_cell_binary, Family, Nat, Operator};

const SEMIPRIME_PROTOCOL_WORD: &str = "⊢⊙∈⊤⊥⊞∋≻⋈≺⊡⊣";

pub struct SemiprimeReport {
    pub source: Nat,
    pub word: String,
    pub factors: Option<(Nat, Nat)>,
    pub analysis: String,
    pub protocol_match: bool,
    pub extraction: crate::arbitrary_factor::Extraction,
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigUint;

    #[test]
    fn closes_a_128_bit_prime_square_through_the_shared_ladder() {
        let p = BigUint::from(18_446_744_073_709_551_557u64);
        let n = &p * &p;
        assert_eq!(n.bits(), 128);
        let report = process_semiprime(&n.to_string()).unwrap();
        assert!(report.protocol_match);
        assert_eq!(
            report.factors,
            Some((
                Nat::from_decimal(&p.to_string()).unwrap(),
                Nat::from_decimal(&p.to_string()).unwrap()
            ))
        );
        assert!(report
            .extraction
            .steps
            .iter()
            .any(|step| step.route == crate::arbitrary_factor::Route::DifferenceOfSquares));
    }

    #[test]
    fn retains_unresolved_balanced_128_bit_source_and_route_trace() {
        let source = "296650821743515430283258444261036507151";
        let report = process_semiprime(source).unwrap();
        assert!(!report.protocol_match);
        assert!(report.factors.is_none());
        assert_eq!(report.extraction.leftover.as_deref(), Some(source));
        assert!(report
            .extraction
            .steps
            .iter()
            .any(|step| step.route == crate::arbitrary_factor::Route::Rho));
        assert!(render_report(&report).contains("UNRESOLVED"));
    }
}

pub fn process_semiprime(raw_input: &str) -> Result<SemiprimeReport, String> {
    let raw_input = raw_input.trim();
    let value = if raw_input.starts_with('⊢') {
        let reading = decode(raw_input).map_err(|e| e.to_string())?;
        if reading.family != Family::CellBinary {
            return Err("Input must be a cell-binary word or decimal natural".into());
        }
        reading.value
    } else {
        Nat::from_decimal(raw_input)
            .ok_or_else(|| format!("Invalid natural number: {}", raw_input))?
    };

    let word = encode_cell_binary(&value);

    // 1. Sieve Witness Acquisition
    let sieve_read = prime_sieve_read(&word, 65536)?;

    // 2. Factor Pair Extraction and product closure
    let extraction = crate::arbitrary_factor::extract(raw_input)?;
    let mut pair = Vec::new();
    if extraction.verified {
        for (prime, exponent) in &extraction.factors {
            if *exponent > 2 || pair.len() + *exponent as usize > 2 {
                pair.clear();
                break;
            }
            let prime =
                Nat::from_decimal(prime).ok_or_else(|| "factor conversion failed".to_string())?;
            for _ in 0..*exponent {
                pair.push(prime.clone());
            }
        }
    }
    let factors = if pair.len() == 2 {
        let product = check(
            &encode_cell_binary(&pair[0]),
            Operator::Mul,
            &encode_cell_binary(&pair[1]),
            &word,
        )
        .map_err(|error| error.to_string())?;
        product.valid.then(|| (pair[0].clone(), pair[1].clone()))
    } else {
        None
    };

    // 3. Structural Analysis
    let certificate = match &sieve_read {
        PrimeSieveRead::Bounded {
            aperture,
            tested_primes,
        } => Some(DivisorBoundCertificate {
            bound: Nat::from_u64(*aperture as u64),
            tested_primes: Nat::from_u64(*tested_primes as u64),
            aperture_width: Nat::from_u64(16),
        }),
        _ => None,
    };
    let structural = analyze_with_sieve(&value, certificate, Some(sieve_read))?;
    let analysis_report = render_analysis(&structural);

    // 4. Protocol Conformance (Semiprime Specific)
    // Must have exactly two non-trivial prime factors and pass the conjunctive check
    let protocol_match = factors.is_some() && extraction.protocol_match;

    Ok(SemiprimeReport {
        source: value,
        word,
        factors,
        analysis: analysis_report,
        protocol_match,
        extraction,
    })
}

pub fn render_report(report: &SemiprimeReport) -> String {
    let factor_str = if let Some((p, q)) = &report.factors {
        format!("{} × {}", p, q)
    } else if report.extraction.verified {
        "none (complete factorization is not a prime pair)".to_string()
    } else {
        "unresolved (route ladder has no verified prime pair)".to_string()
    };

    format!(
        "═══════════════════════════════════════════════════════════\n\
         SEMIPRIME FACTOR PAIR GÖDEL-ENCODING RELATIONSHIP\n\
         ══════════════════════════════════════════════════════════\n\n\
         ── SOURCE ──────────────────────────────────────────────\n\
         value                      {}\n\
         gödel.word                 {}\n\
         protocol.conformance       {}\n\
         expected.protocol          {}\n\n\
         ── FACTORIZATION ───────────────────────────────────────\n\
         factor.pair                {}\n\
         conjunctive.reconstruction {}\n\n\
         ── EXTRACTION ──────────────────────────────────────────\n\
         {}\n\
         ── STRUCTURAL ANALYSIS ─────────────────────────────────\n\
         {}\n\
         ══════════════════════════════════════════════════════════\n",
        report.source,
        report.word,
        if report.protocol_match {
            "PASS"
        } else if report.extraction.verified {
            "NOT SEMIPRIME"
        } else {
            "UNRESOLVED"
        },
        SEMIPRIME_PROTOCOL_WORD,
        factor_str,
        if report.factors.is_some() {
            "PASS (∋)"
        } else {
            "OPEN"
        },
        report.extraction.render(),
        report.analysis,
    )
}
