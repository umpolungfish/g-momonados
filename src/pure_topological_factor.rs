//! Pure IMASM-word factorization entry point.
//!
//! This path accepts only a canonical native numeral word. The source,
//! factors, intermediate arithmetic, and closure checks remain WordTape /
//! EVALT-EVALF values. It delegates to Vox's resident NINE_ARM carrier, whose
//! arithmetic is performed by its packed Belnap-word operations.
//!
//! This is a word-native implementation, not a claim that the operator glyphs
//! alone factor an arbitrary integer in constant time. The carrier still
//! performs number-theoretic work on the word and is bounded here by an
//! explicit round budget. Budget exhaustion is not a primality result.

use crate::word_tape::WordTape;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// Default maximum number of carrier rounds for the dedicated CLI entry point.
pub const DEFAULT_ROUND_BUDGET: u64 = 100_000;

/// Factor a canonical IMASM numeral word without decoding it to a host bigint.
///
/// Returns a trace containing the factor words and two checked closures:
/// p * q == N and deinterlace(interlace(p, q)) == (p, q).
pub fn factor_word(source_word: &str, max_rounds: u64) -> Result<String, String> {
    let source_word = source_word.trim();
    if !source_word.starts_with('⊢') {
        return Err(
            "pure_factor accepts a canonical IMASM numeral word, not a decimal or scalar input"
                .into(),
        );
    }
    if max_rounds == 0 {
        return Err("carrier round budget must be positive".into());
    }

    let source = WordTape::from_canonical_word(source_word)
        .ok_or_else(|| "malformed or non-canonical IMASM numeral word".to_string())?;
    if source.as_word() != source_word {
        return Err("input is not the canonical spelling of its IMASM numeral".into());
    }
    if source.is_zero() || source.is_one() {
        return Err("unit inputs have no non-trivial two-factor decomposition".into());
    }

    // Cross into Vox through the native parity cells. No decimal conversion
    // or BigUint adapter is called on this path.
    let source_cells = source.to_imasm_tape();
    let _vox_word = vox_core::morphism_factor::emit_numeral(&source_cells);

    // Use the complete resident carrier, but impose a caller-visible bound.
    let tower = vox_core::morphism_factor::construct_carrier(
        vox_core::morphism_factor::NINE_ARM,
    )?;
    let stages: Vec<&str> = tower
        .iter()
        .map(|operator| vox_core::morphism_factor::morphism_name(operator))
        .collect();

    let factor_cells = vox_core::morphism_factor::run_carrier_rounds(
        &tower,
        &source_cells,
        max_rounds,
    )
    .ok_or_else(|| {
        format!(
            "no proper factor emitted within {max_rounds} NINE_ARM carrier rounds; budget exhaustion is not a primality certificate"
        )
    })?;

    let first = WordTape::from_imasm_tape(&factor_cells)
        .ok_or_else(|| "carrier returned a malformed factor tape".to_string())?;
    let (second, remainder) = source
        .divmod(&first)
        .ok_or_else(|| "carrier returned a zero or invalid divisor".to_string())?;

    let one = WordTape::one();
    if !remainder.is_zero()
        || !first.gt(&one)
        || !second.gt(&one)
        || first.mul(&second) != source
    {
        return Err(
            "carrier result failed strict factor bounds or exact product closure".into(),
        );
    }

    // Canonical ordering makes the report stable; it does not change the pair.
    let (p, q) = if first.gt(&second) {
        (second, first)
    } else {
        (first, second)
    };

    // Re-encode the pair as one interlaced braid, then independently unbraid it.
    let pair_braid = p.interlace(&q);
    let (p_back, q_back) = pair_braid.deinterlace();
    let braid_closes = p_back == p && q_back == q;
    if !braid_closes {
        return Err("factor-pair braid failed its interlace/deinterlace retraction".into());
    }

    Ok(format!(
        "PURE IMASM WORD FACTORIZATION\n\
         backend: Vox NINE_ARM over WordTape\n\
         source_word: {}\n\
         carrier_stages: {}\n\
         p_word: {}\n\
         q_word: {}\n\
         pair_braid: {}\n\
         product_closure: true\n\
         braid_retraction: true\n\
         carrier_round_budget: {}\n\
         primality_claim: none",
        source.as_word(),
        stages.join(" -> "),
        p.as_word(),
        q.as_word(),
        pair_braid.as_word(),
        max_rounds
    ))
}

/// Help text for the separate pure_factor command.
pub fn help() -> &'static str {
    "pure_factor <canonical-IMASM-word> [carrier-round-budget]\n\
     Word-only NINE_ARM factorization; decimal/scalar inputs are rejected.\n\
     A budget miss is inconclusive, not a primality proof."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pure_entry_rejects_decimal_scalar_input() {
        let error = factor_word("8051", 100).unwrap_err();
        assert!(error.contains("canonical IMASM numeral word"));
    }

    #[test]
    fn native_word_factorization_checks_both_retractions() {
        // 8051 = 83 * 97. Construct the input directly as a WordTape so the
        // test exercises the same no-scalar entry path as the CLI.
        let source = WordTape::from_small(8051);
        let report = factor_word(source.as_word(), DEFAULT_ROUND_BUDGET).unwrap();
        assert!(report.contains("product_closure: true"));
        assert!(report.contains("braid_retraction: true"));
        assert!(report.contains("backend: Vox NINE_ARM over WordTape"));
    }
}
