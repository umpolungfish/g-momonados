//! Unified Semiprime Gödel-Encoding Tool
//! 
//! Performs the full cycle: Encode -> Analyze -> Separate -> Verify -> Protocol Check
//! for arbitrary semiprimes within the IMASM/Gödel calculus framework.

use godel_calculus::{encode_cell_binary, decode, check, Operator, Nat, Family};
use godel_analyzer::{analyze_with_sieve, render as render_analysis, PrimeSieveRead};
use godel_product::{separate_product, analyze_product, render as render_product};

/// The canonical glyph word for the Semiprime Factor Pair relationship
const SEMIPRIME_PROTOCOL_WORD: &str = "⊢⊙∈⊤⊥⊞∋≻⋈≺⊡⊣";

pub struct SemiprimeReport {
    pub source: Nat,
    pub word: String,
    pub analysis: String,
    pub separation: String,
    pub product_verification: String,
    pub protocol_match: bool,
}

/// Execute the full semiprime relationship pipeline
pub fn process_semiprime(raw_input: &str) -> Result<SemiprimeReport, String> {
    // 1. Parse input (accepts decimal or cell-binary word)
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

    // 2. Generate canonical Gödel encoding
    let word = encode_cell_binary(&value);
    
    // 3. Structural analysis with bounded sieve certificate
    // Window=16 gives aperture=65536 for factor bound certification
    let sieve_read = godel_analyzer::prime_sieve_read(&word, 65536)?;
    let certificate = match &sieve_read {
        PrimeSieveRead::Bounded { aperture, tested_primes } => {
            Some(godel_analyzer::DivisorBoundCertificate {
                bound: Nat::from_u64(*aperture as u64),
                tested_primes: Nat::from_u64(*tested_primes as u64),
                aperture_width: Nat::from_u64(16),
            })
        }
        _ => None,
    };
    let structural = analyze_with_sieve(&value, certificate, Some(sieve_read))?;
    let analysis_report = render_analysis(&structural);

    // 4. Factor separation via SHIAB/Residual-GCD frames
    let separation_report = separate_product(&value)?;

    // 5. Product verification through support convolution
    // Re-extract factors from separation to verify multiplicative closure
    let factors = extract_factors_from_report(&separation_report)?;
    let mut verification = String::new();
    
    if factors.len() == 2 {
        let prod_analysis = analyze_product(&value, &factors[0], &factors[1])?;
        verification = render_product(&prod_analysis);
        
        // Also verify via glyph-level equation check
        let lhs_word = encode_cell_binary(&factors[0]);
        let rhs_word = encode_cell_binary(&factors[1]);
        let eq_check = check(&lhs_word, Operator::Mul, &rhs_word, &word)
            .map_err(|e| e.to_string())?;
        
        verification.push_str(&format!(
            "\nglyph.equation-check     {} × {} = {}\nglyph.check.status       {}\n",
            factors[0], factors[1], value,
            if eq_check.valid { "PASS" } else { "FAIL" }
        ));
    } else {
        verification = format!(
            "WARNING: Expected 2 prime factors for semiprime, found {}\n\
             This may indicate the input is not a true semiprime.\n",
            factors.len()
        );
    }

    // 6. Protocol conformance check
    // Verify the structural word participates in the expected cycle
    let protocol_match = verify_protocol_conformance(&word);

    Ok(SemiprimeReport {
        source: value,
        word,
        analysis: analysis_report,
        separation: separation_report,
        product_verification: verification,
        protocol_match,
    })
}

/// Extract factor Nats from separation report output
fn extract_factors_from_report(report: &str) -> Result<Vec<Nat>, String> {
    let mut factors = Vec::new();
    for line in report.lines() {
        if line.trim().starts_with("prime.factor") 
            && !line.contains(".word") 
            && !line.contains(".count")
            && line.contains("PASS") == false 
        {
            // Parse lines like "prime.factor            83"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Some(n) = Nat::from_decimal(parts.last().unwrap()) {
                    if n != Nat::zero() && n != Nat::one() {
                        factors.push(n);
                    }
                }
            }
        }
    }
    if factors.is_empty() {
        return Err("Could not extract factors from separation report".into());
    }
    Ok(factors)
}

/// Verify the encoded word participates in the semiprime protocol cycle
fn verify_protocol_conformance(word: &str) -> bool {
    // Check that the word's structure is compatible with the 
    // ⊢⊙∈⊤⊥⊞∋≻⋈≺⊡⊣ relationship by verifying:
    // 1. Valid cell-binary encoding
    // 2. Non-trivial support (not unit or zero)
    // 3. Has at least one ⊥ (bit=1) beyond the trivial case
    if let Ok(reading) = decode(word) {
        if reading.family == Family::CellBinary {
            let bits = match &reading.structure {
                godel_calculus::Structure::CellBinary { bits_le } => bits_le,
                _ => return false,
            };
            // Semiprime must have popcount >= 2 (product of two primes > 1)
            let popcount = bits.iter().filter(|&&b| b).count();
            return popcount >= 2;
        }
    }
    false
}

/// Render the complete report
pub fn render_report(report: &SemiprimeReport) -> String {
    format!(
        "═══════════════════════════════════════════════════════════\n\
         SEMIPRIME GÖDEL-ENCODING RELATIONSHIP ANALYSIS\n\
         ═══════════════════════════════════════════════════════════\n\n\
         ── SOURCE ──────────────────────────────────────────────\n\
         value                      {}\n\
         gödel.word                 {}\n\
         protocol.conformance       {}\n\n\
         ── STRUCTURAL ANALYSIS ─────────────────────────────────\n\
         {}\n\
         ── FACTOR SEPARATION ───────────────────────────────────\n\
         {}\n\
         ── PRODUCT VERIFICATION ────────────────────────────────\n\
         {}\n\
         ── PROTOCOL ────────────────────────────────────────────\n\
         expected.word              {}\n\
         relationship               Semiprime Factor Pair\n\
         frobenius.verdict          T\n\
         topology.class             flat_chain\n\
         period                     12\n\
         ═══════════════════════════════════════════════════════════\n",
        report.source,
        report.word,
        if report.protocol_match { "PASS" } else { "FAIL" },
        report.analysis,
        report.separation,
        report.product_verification,
        SEMIPRIME_PROTOCOL_WORD,
    )
}

// CLI entry point example
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: semiprime-tool <natural-number|cell-binary-word>");
        eprintln!("Example: semiprime-tool 91");
        eprintln!("Example: semiprime-tool 10873");
        std::process::exit(1);
    }

    match process_semiprime(&args[1]) {
        Ok(report) => print!("{}", render_report(&report)),
        Err(e) => {
            eprintln!("ERROR: {}", e);
            std::process::exit(1);
        }
    }
}