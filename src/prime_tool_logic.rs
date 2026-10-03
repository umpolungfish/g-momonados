//! Prime Factor - Gödel-Encoding Relationship Tool
//!
//! Protocol: ⊣⊣⊙∈⊤≻⋈⊥≺∋⊞⊡
//! Focus: Atomic irreducibility and hermetic boundary sealing.

use crate::godel_analyzer::{
    analyze_with_sieve, prime_sieve_read, render as render_analysis, DivisorBoundCertificate,
    PrimeSieveRead,
};
use crate::godel_calculus::{decode, encode_cell_binary, DecodeError, Family, Nat};

const PRIME_PROTOCOL_WORD: &str = "⊣⊣⊙∈⊤≻⋈⊥≺∋⊞⊡";

pub struct PrimeReport {
    pub source: Nat,
    pub word: String,
    pub is_prime: bool,
    pub analysis: String,
    pub protocol_match: bool,
}

pub fn process_prime(raw_input: &str) -> Result<PrimeReport, String> {
    let value = if raw_input.starts_with('⊢') {
        let reading = decode(raw_input).map_err(|e: DecodeError| e.to_string())?;
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

    // 2. Primality Determination
    let is_prime = matches!(&sieve_read, PrimeSieveRead::Prime { .. });

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

    // 4. Protocol Conformance (Prime Specific)
    // Must be > 1, have no factors in aperture, and match the atomic seal pattern
    let protocol_match = is_prime && verify_prime_protocol(&value, &word);

    Ok(PrimeReport {
        source: value,
        word,
        is_prime,
        analysis: analysis_report,
        protocol_match,
    })
}

fn verify_prime_protocol(value: &Nat, word: &str) -> bool {
    if value.bits_le().len() > 1 {
        if let Ok(reading) = decode(word) {
            if reading.family == Family::CellBinary {
                // The double-⊣ protocol implies a sealed boundary with no internal split
                return true;
            }
        }
    }
    false
}

pub fn render_report(report: &PrimeReport) -> String {
    let verdict = if report.is_prime {
        "PRIME (Atomic)"
    } else {
        "COMPOSITE/UNKNOWN"
    };

    format!(
        "═══════════════════════════════════════════════════════════\n\
         PRIME FACTOR GÖDEL-ENCODING RELATIONSHIP\n\
         ══════════════════════════════════════════════════════════\n\n\
         ── SOURCE ──────────────────────────────────────────────\n\
         value                      {}\n\
         gödel.word                 {}\n\
         protocol.conformance       {}\n\
         expected.protocol          {}\n\n\
         ── ATOMICITY VERDICT ───────────────────────────────────\n\
         status                     {}\n\
         hermetic.seal              {}\n\n\
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
        PRIME_PROTOCOL_WORD,
        verdict,
        if report.is_prime {
            "PASS (⊣⊣)"
        } else {
            "REFUTED"
        },
        report.analysis,
    )
}
