//! Semiprime Factor Pair - Gödel-Encoding Relationship Tool
//!
//! Protocol: ⊢∋∈⊤⊥⊞∋≻⋈≺⊡⊣
//! Focus: Conjunctive reconstruction of N = p * q via SHIAB/Residual frames.

use crate::godel_analyzer::{
    analyze_with_sieve, prime_sieve_read, render as render_analysis, DivisorBoundCertificate,
    PrimeSieveRead,
};
use crate::godel_calculus::{check, decode, encode_cell_binary, Family, Nat, Operator, Structure};

const SEMIPRIME_PROTOCOL_WORD: &str = "⊢∋∈⊤⊥⊞∋≻⋈≺⊡⊣";

pub struct SemiprimeReport {
    pub source: Nat,
    pub word: String,
    pub factors: Option<(Nat, Nat)>,
    pub analysis: String,
    pub protocol_match: bool,
}

pub fn process_semiprime(raw_input: &str) -> Result<SemiprimeReport, String> {
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
    let factors = match &sieve_read {
        PrimeSieveRead::Factor { p, q, .. } => {
            let p_word = encode_cell_binary(p);
            let q_word = encode_cell_binary(q);
            let product =
                check(&p_word, Operator::Mul, &q_word, &word).map_err(|error| error.to_string())?;
            let cofactor_read = prime_sieve_read(&q_word, 65536)?;
            if product.valid
                && q != &Nat::one()
                && matches!(cofactor_read, PrimeSieveRead::Prime { .. })
            {
                Some((p.clone(), q.clone()))
            } else {
                None
            }
        }
        _ => None,
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
    let protocol_match = factors.is_some() && verify_semiprime_protocol(&word);

    Ok(SemiprimeReport {
        source: value,
        word,
        factors,
        analysis: analysis_report,
        protocol_match,
    })
}

fn verify_semiprime_protocol(word: &str) -> bool {
    if let Ok(reading) = decode(word) {
        if reading.family == Family::CellBinary {
            let bits = match &reading.structure {
                Structure::CellBinary { bits_le } => bits_le,
                _ => return false,
            };
            // Semiprime protocol requires popcount >= 2 and valid closure
            return bits.iter().filter(|&&b| b).count() >= 2;
        }
    }
    false
}

pub fn render_report(report: &SemiprimeReport) -> String {
    let factor_str = if let Some((p, q)) = &report.factors {
        format!("{} × {}", p, q)
    } else {
        "none (not a verified semiprime)".to_string()
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
         ── STRUCTURAL ANALYSIS ─────────────────────────────────\n\
         {}\n\
         ══════════════════════════════════════════════════════════\n",
        report.source,
        report.word,
        if report.protocol_match {
            "PASS"
        } else {
            "FAIL"
        },
        SEMIPRIME_PROTOCOL_WORD,
        factor_str,
        if report.factors.is_some() {
            "PASS (∋)"
        } else {
            "FAIL"
        },
        report.analysis,
    )
}
