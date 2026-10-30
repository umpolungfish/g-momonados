//! Semiprime Gödel-encoding pipeline, revised.  Encode -> Analyze -> Separate -> Verify -> Protocol.
//!
//! Changes from godel_semiprime2.rs (each is a real defect or a gap there):
//!  1. Factor separation returns typed `Nat`s; the text-scraping `extract_factors_from_report` is gone.
//!  2. Separation = exact bounded sieve, then winding with Belnap-valued bit-braiding
//!     (godel_winding) for factors above the sieve aperture, closed through `factor_pair_read`.
//!  3. `verify_protocol_conformance` only tested popcount >= 2, which is true of every number that is
//!     not a power of two. Conformance now means: product closed, cofactor prime within the aperture,
//!     and the glyph-level equation check passes.
//!  4. The report printed `frobenius.verdict T`, `topology.class flat_chain`, `period 12` as constants for
//!     every input. paraconsistent_spindle.py evaluates the protocol word to B (paradox held: ⊞ inside
//!     a closed fork/fuse), not T. Those lines are no longer asserted as computed results.
use crate::godel_analyzer::{analyze_with_sieve, parse_input, prime_sieve_read as analyzer_sieve,
                            render as render_analysis, DivisorBoundCertificate, PrimeSieveRead};
use crate::godel_calculus::{check, encode_cell_binary, Nat, Operator};
use crate::godel_winding::{braid, braid_word, render as render_sep, separate, unbraid, walk, Separation};
use alloc::format;
use alloc::string::{String, ToString};

pub const SEMIPRIME_PROTOCOL_WORD: &str = "⊢⊙∈⊤⊥⊞∋≻⋈≺⊡⊣";
/// Reference value, NOT computed in Rust: `paraconsistent_spindle.py` safe_verdict(word) = B
/// ("paradox held: ⊞ present in a transformed closure"). Re-check there if the word changes.
pub const SPINDLE_REFERENCE_VERDICT: &str = "B";

pub struct SemiprimeReport {
    pub source: Nat,
    pub word: String,
    pub analysis: String,
    pub separation: String,
    pub verification: String,
    pub conforms: bool,
}

pub fn process_semiprime(raw: &str) -> Result<SemiprimeReport, String> {
    process_semiprime_with(raw, 16, 200_000, 20_000)
}

pub fn process_semiprime_with(raw: &str, sieve_width: usize, plain_kmax: u64, fact_bound: u64)
    -> Result<SemiprimeReport, String>
{
    let value = parse_input(raw)?;
    let word = encode_cell_binary(&value);

    // structural analysis with the bounded-sieve certificate (unchanged)
    let aperture = 1usize << sieve_width;
    let sieve_read = analyzer_sieve(&word, aperture)?;
    let certificate = match &sieve_read {
        PrimeSieveRead::Bounded { aperture, tested_primes } => Some(DivisorBoundCertificate {
            bound: Nat::from_u64(*aperture as u64),
            tested_primes: Nat::from_u64(*tested_primes as u64),
            aperture_width: Nat::from_u64(sieve_width as u64),
        }),
        _ => None,
    };
    let analysis = render_analysis(&analyze_with_sieve(&value, certificate, Some(sieve_read))?);

    // separation: sieve, then braided winding
    let sep: Separation = separate(&value, sieve_width, plain_kmax, fact_bound)?;
    let separation = render_sep(&value, &sep);

    // verification
    let mut v = String::new();
    let mut conforms = false;
    if let Some(pair) = &sep.pair {
        let lhs = encode_cell_binary(&pair.factor);
        let rhs = encode_cell_binary(&pair.cofactor);
        let eq = check(&lhs, Operator::Mul, &rhs, &word).map_err(|e| e.to_string())?;
        let braided = braid(&pair.factor, &pair.cofactor);
        let (a, b) = unbraid(&braided);
        let braid_ok = a == pair.factor && b == pair.cofactor;
        v += &format!("glyph.equation-check     {} x {} = {}   {}\n", pair.factor, pair.cofactor, value,
                      if eq.valid { "PASS" } else { "FAIL" });
        v += &format!("braid.pair.word          {}\nbraid.pair.roundtrip     {}\n",
                      encode_cell_binary(&braided), if braid_ok { "PASS" } else { "FAIL" });
        conforms = pair.product_closed && pair.semiprime_closed == Some(true) && eq.valid && braid_ok;
    } else {
        v += "no factor pair recovered; no conformance claimed\n";
    }
    if let Some(f) = &sep.found {
        let w = walk(&value, &Nat::from_u64(f.base), f.step.max(2));
        if f.schedule == "plain" {
            v += &format!("braid.word               {}  (k = {})\nbelnap.trace             ", braid_word(f.step), f.step);
            for c in &w.crossings { v.push(c.kind); v.push(':'); v.push(c.state.glyph()); v.push(' '); }
            v.push('\n');
        }
    }
    v += &format!("protocol.word            {}\nspindle.reference.verdict {} (from paraconsistent_spindle.py; not computed here)\n",
                  SEMIPRIME_PROTOCOL_WORD, SPINDLE_REFERENCE_VERDICT);

    Ok(SemiprimeReport { source: value, word, analysis, separation, verification: v, conforms })
}

pub fn render_report(r: &SemiprimeReport) -> String {
    format!(
        "SEMIPRIME GÖDEL-ENCODING ANALYSIS\nvalue                    {}\ngödel.word               {}\nprotocol.conformance     {}\n\n\
         -- STRUCTURAL ANALYSIS --\n{}\n-- FACTOR SEPARATION --\n{}\n-- VERIFICATION --\n{}",
        r.source, r.word, if r.conforms { "PASS" } else { "FAIL" }, r.analysis, r.separation, r.verification)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn sieve_path_91() {
        let r = process_semiprime("91").unwrap();
        assert!(r.conforms, "{}", render_report(&r));
        assert!(r.separation.contains("separation.via           sieve"));
    }
    #[test] fn winding_path_when_factors_exceed_aperture() {
        // 293 * 3413 with a 2^6 aperture: only the winding can split it
        let r = process_semiprime_with("1000009", 6, 100_000, 5_000).unwrap();
        assert!(r.conforms, "{}", render_report(&r));
        assert!(r.separation.contains("winding"));
    }
    #[test] fn prime_is_not_reported_conforming() {
        let r = process_semiprime_with("1000003", 6, 20_000, 2_000).unwrap();
        assert!(!r.conforms);
    }
    #[test] fn power_of_two_is_not_reported_conforming() {
        // the old popcount>=2 test would have rejected this too, but 12 = 2*2*3 has popcount 2 and passed it
        let r = process_semiprime("12").unwrap();
        assert!(!r.conforms, "{}", render_report(&r));
    }
}
