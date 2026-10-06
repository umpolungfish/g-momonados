//! Semiprime Factor Pair - Gödel-Encoding Relationship Tool
//!
//! Protocol: ⊢⊙∈⊤⊥⊞∋≻⋈≺⊡⊣
//! Focus: Conjunctive reconstruction of N = p * q via anyonic ququart phase measurement.

use crate::anyon_pair::{PairMatrix, COMPUTATIONAL_CHANNELS, LEAKAGE_CHANNEL};
use crate::godel_calculus::{check, decode, encode_cell_binary, Family, Nat, Operator};
use crate::phase_unbraid::FixedPointFormat;
use crate::ququart_factor::{QuquartFactorExecutor, QuquartFactorShot};
use crate::ququart_folded_work::QuquartFoldedWorkDevice;
use alloc::{string::String, vec::Vec};
use num_bigint::{BigInt, BigUint};
use num_traits::{One, Zero};

const SEMIPRIME_PROTOCOL_WORD: &str = "⊢⊙∈⊤⊥⊞∋≻⋈≺⊡⊣";

pub struct SemiprimeReport {
    pub source: Nat,
    pub word: String,
    pub factors: Option<(Nat, Nat)>,
    pub analysis: String,
    pub protocol_match: bool,
    pub extraction_steps: Vec<ExtractionStep>,
}

#[derive(Clone, Debug)]
pub struct ExtractionStep {
    pub route: &'static str,
    pub factor: Option<String>,
    pub detail: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigUint;

    #[test]
    fn closes_a_128_bit_prime_square_through_ququart_extraction() {
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
            .extraction_steps
            .iter()
            .any(|step| step.route == "ququart phase measurement"));
    }

    #[test]
    fn closes_the_original_balanced_128_bit_source_through_ququart_extraction() {
        let source = "296650821743515430283258444261036507151";
        let report = process_semiprime(source).unwrap();
        assert!(report.protocol_match);
        assert_eq!(
            report.factors,
            Some((
                Nat::from_decimal("16925480323643806501").unwrap(),
                Nat::from_decimal("17526877587580975651").unwrap(),
            ))
        );
        assert!(report
            .extraction_steps
            .iter()
            .any(|step| step.route == "ququart phase measurement"));
        assert!(!render_report(&report).contains("UNRESOLVED"));
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
    let source_big = biguint_from_nat(&value);

    // Anyonic ququart phase measurement extraction
    let extraction_steps = extract_via_ququart(&source_big, &word)?;

    let factors = extraction_steps.iter()
        .find_map(|step| step.factor.as_ref())
        .and_then(|p_str| {
            Nat::from_decimal(p_str).and_then(|p| {
                divmod_nat(&value, &p).ok().and_then(|(q, rem)| {
                    let q_big = biguint_from_nat(&q);
                    let one_big = BigUint::one();
                    if rem.is_zero() && q_big > one_big {
                        Some((p, q))
                    } else {
                        None
                    }
                })
            })
        });

    let protocol_match = factors.is_some();

    // Structural analysis (sieve witness only)
    let sieve_read = crate::godel_analyzer::prime_sieve_read(&word, 65536)?;
    let certificate = match &sieve_read {
        crate::godel_analyzer::PrimeSieveRead::Bounded { aperture, tested_primes } => {
            Some(crate::godel_analyzer::DivisorBoundCertificate {
                bound: Nat::from_u64(*aperture as u64),
                tested_primes: Nat::from_u64(*tested_primes as u64),
                aperture_width: Nat::from_u64(16),
            })
        }
        _ => None,
    };
    let structural = crate::godel_analyzer::analyze_with_sieve(&value, certificate, Some(sieve_read))?;
    let analysis_report = crate::godel_analyzer::render(&structural);

    Ok(SemiprimeReport {
        source: value,
        word,
        factors,
        analysis: analysis_report,
        protocol_match,
        extraction_steps,
    })
}

fn biguint_from_nat(n: &Nat) -> BigUint {
    let mut out = BigUint::from(0u32);
    for (i, bit) in n.bits_le().iter().copied().enumerate() {
        if bit {
            out |= BigUint::one() << i;
        }
    }
    out
}

fn divmod_nat(n: &Nat, p: &Nat) -> Result<(Nat, Nat), String> {
    let n_big = biguint_from_nat(n);
    let p_big = biguint_from_nat(p);
    let q_big = &n_big / &p_big;
    let rem_big = &n_big % &p_big;
    Ok((nat_from_biguint(&q_big), nat_from_biguint(&rem_big)))
}

fn nat_from_biguint(n: &BigUint) -> Nat {
    let mut bits = Vec::new();
    let mut value = n.clone();
    while !value.is_zero() {
        bits.push((&value & BigUint::one()) == BigUint::one());
        value >>= 1usize;
    }
    Nat::from_bits_le(bits)
}

fn extract_via_ququart(source: &BigUint, source_word: &str) -> Result<Vec<ExtractionStep>, String> {
    let mut steps = Vec::new();
    let bit_len = source.bits();
    let phase_digits = 2 * (bit_len as usize + 4) + 1;

    steps.push(ExtractionStep {
        route: "ququart phase measurement",
        factor: None,
        detail: format!("initializing {}-bit source, {} phase digits", bit_len, phase_digits),
    });

    // Build Fourier braid matrix
    let format = FixedPointFormat::for_modulus(source)
        .map_err(|e| format!("format error: {}", e))?;
    let fourier = build_fourier_matrix(&format);

    // Use folded work device for in-memory execution
    let device = QuquartFoldedWorkDevice::new(source.clone(), fourier, 1729)
        .map_err(|e| format!("device init error: {}", e))?;

    let mut executor = QuquartFactorExecutor::new(device);

    // Execute shot with base 2
    let base = BigUint::from(2u32);
    let shot: QuquartFactorShot = executor.shot(source, &base)
        .map_err(|e| format!("ququart shot error: {}", e))?;

    if let Some(closure) = shot.closure {
        let (p_big, q_big) = closure.factors();
        let (p_word, q_word) = closure.factor_words();

        // Verify Gödel product closure
        let verified = check(p_word, Operator::Mul, q_word, source_word)
            .map(|eq| eq.valid)
            .unwrap_or(false);

        if verified {
            steps.push(ExtractionStep {
                route: "ququart phase measurement",
                factor: Some(p_big.to_string()),
                detail: format!("measured winding closed: p={} q={}", p_big, q_big),
            });
            return Ok(steps);
        }
    }

    // No SIC fallback — ququart phase measurement is the only route
    steps.push(ExtractionStep {
        route: "ququart phase measurement",
        factor: None,
        detail: "extraction did not close a factor pair — no classical/SIC fallback".into(),
    });
    Ok(steps)
}

fn build_fourier_matrix(format: &FixedPointFormat) -> PairMatrix {
    use crate::phase_unbraid::FixedComplex;
    let mut matrix = PairMatrix(core::array::from_fn(|_| FixedComplex {
        re: BigInt::zero(),
        im: BigInt::zero(),
    }));
    for (row, &r) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
        for (col, &c) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
            let mut z = FixedComplex::winding_twiddle(
                &BigInt::from(row * col),
                &BigUint::from(4u8),
                format,
            ).unwrap();
            z.re /= 2u8;
            z.im /= 2u8;
            matrix.0[5 * r + c] = z;
        }
    }
    matrix.0[5 * LEAKAGE_CHANNEL + LEAKAGE_CHANNEL].re = format.scale();
    matrix
}

pub fn render_report(report: &SemiprimeReport) -> String {
    let factor_str = if let Some((p, q)) = &report.factors {
        format!("{} × {}", p, q)
    } else {
        "no factor pair extracted".to_string()
    };

    let extraction_display = report.extraction_steps.iter()
        .map(|step| {
            let factor_info = step.factor.as_deref().unwrap_or("(none)");
            format!("    {:<26} {:<20} {}", factor_info, step.route, step.detail)
        })
        .collect::<Vec<_>>()
        .join("\n");

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
        extraction_display,
        report.analysis,
    )
}
