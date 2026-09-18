//! Full-fidelity gene<->protein translation, both directions.
//!
//! `genetic::lift_rna`/`lift_protein` deliberately keep only the twelve
//! promoted residues — the eight ground-layer amino acids (A G P T S R L V)
//! are silent by design, because that lift answers one question: does the
//! glyph word close. That silence is correct for that question and wrong
//! for this one. A real reverse translation, or a real fold, needs every
//! residue a protein actually has — you cannot reconstruct or fold a chain
//! that is missing 40% of its residues. So this module keeps all twenty,
//! reusing `genetic_table::CODON` and `genetic::dialect_override` — the same
//! genetic code, read without throwing part of it away.

use crate::genetic::{codon_meaning, dialect_override};
use crate::genetic_table::CODON;
use alloc::string::String;
use alloc::vec::Vec;

/// Every codon (RNA form, e.g. "AUG") that encodes the given amino acid
/// under the named dialect. Empty dialect or "standard" is the standard
/// code; "mito"/"mitochondrial" applies the four vertebrate-mitochondrial
/// overrides from `genetic::dialect_override`.
pub fn codons_for_aa(aa: &str, dialect: &str) -> Vec<&'static str> {
    CODON.iter().filter_map(|(codon, _, _)| {
        let (kind, val) = dialect_override(dialect, codon).or_else(|| codon_meaning(codon))?;
        if kind == "aa" && val == aa { Some(*codon) } else { None }
    }).collect()
}

fn is_fourfold_box(p1p2: &str, aa: &str, dialect: &str) -> bool {
    ['A', 'C', 'G', 'U'].iter().all(|&p3| {
        let mut codon = String::from(p1p2);
        codon.push(p3);
        match dialect_override(dialect, &codon).or_else(|| codon_meaning(&codon)) {
            Some(("aa", val)) => val == aa,
            _ => false,
        }
    })
}

/// The single preferred codon for an amino acid, derived from the genetic
/// code alone — no external codon-usage table. Port of red-hot_rebis's
/// `_compute_preferred_codons` (serpentrod/protein_v5.py): within whichever
/// exact-stratum tier the AA's codons fall into, prefer the position-3 C
/// variant (highest fidelity); fall through tiers until one applies.
///
/// Tier 1: codons with middle base C — the exact-stratum box.
/// Tier 2: a 4-fold degenerate box (all four position-3 variants at this
///         p1,p2 code the same AA) even when the middle base isn't C.
/// Tier 3: any codon with position 3 == C.
/// Tier 4: the first codon in table order — only ever reached by a
///         singleton codon set (Met, Trp), where order can't matter.
pub fn preferred_codon_for_aa(aa: &str, dialect: &str) -> Option<&'static str> {
    let codons = codons_for_aa(aa, dialect);
    if codons.is_empty() { return None; }

    let exact: Vec<&'static str> = codons.iter().copied().filter(|c| c.as_bytes()[1] == b'C').collect();
    if !exact.is_empty() {
        return Some(exact.iter().copied().find(|c| c.as_bytes()[2] == b'C').unwrap_or(exact[0]));
    }

    let four_fold: Vec<&'static str> = codons.iter().copied()
        .filter(|c| is_fourfold_box(&c[0..2], aa, dialect))
        .collect();
    if !four_fold.is_empty() {
        return Some(four_fold.iter().copied().find(|c| c.as_bytes()[2] == b'C').unwrap_or(four_fold[0]));
    }

    let c3c = codons.iter().copied().find(|c| c.as_bytes()[2] == b'C');
    Some(c3c.unwrap_or(codons[0]))
}

/// The full forward pipeline: DNA/RNA -> every residue, promoted or not,
/// in translation order. Parallel to `genetic::lift_rna_dialect`, but where
/// that keeps only the promoted subset in `Transcript.word`, this keeps all
/// of them — the difference this module exists to fix.
pub struct FullTranslation {
    pub mrna: String,
    pub protein: Vec<&'static str>,
    pub start: usize,
    pub stopped: Option<&'static str>,
}

pub fn translate_full(seq: &str, dialect: &str) -> FullTranslation {
    let clean: Vec<char> = seq.chars()
        .map(|c| c.to_ascii_uppercase())
        .filter(|c| matches!(c, 'A' | 'C' | 'G' | 'T' | 'U'))
        .map(|c| if c == 'T' { 'U' } else { c })
        .collect();
    let mrna: String = clean.iter().collect();

    let mut start = 0usize;
    for k in 0..clean.len().saturating_sub(2) {
        if clean[k] == 'A' && clean[k + 1] == 'U' && clean[k + 2] == 'G' { start = k; break; }
    }

    let mut protein = Vec::new();
    let mut stopped = None;
    let mut k = start;
    while k + 3 <= clean.len() {
        let codon: String = clean[k..k + 3].iter().collect();
        match dialect_override(dialect, &codon).or_else(|| codon_meaning(&codon)) {
            Some(("stop", val)) => { stopped = Some(val); break; }
            Some(("aa", val)) => protein.push(val),
            _ => {}
        }
        k += 3;
    }
    FullTranslation { mrna, protein, start, stopped }
}

/// The full reverse pipeline: a full protein chain -> mRNA, one preferred
/// codon per residue, with the real degeneracy (every codon count, and the
/// product across the chain) reported alongside.
pub struct FullReverse {
    pub mrna: String,
    pub degeneracies: Vec<usize>,
    pub total_combinations: u64,
}

pub fn reverse_translate_full(chain: &[&str], dialect: &str) -> Option<FullReverse> {
    let mut mrna = String::with_capacity(chain.len() * 3);
    let mut degeneracies = Vec::with_capacity(chain.len());
    let mut total: u64 = 1;
    for &aa in chain {
        let all = codons_for_aa(aa, dialect);
        if all.is_empty() { return None; }
        degeneracies.push(all.len());
        total = total.saturating_mul(all.len() as u64);
        mrna.push_str(preferred_codon_for_aa(aa, dialect)?);
    }
    Some(FullReverse { mrna, degeneracies, total_combinations: total })
}

/// mRNA -> DNA coding strand: U->T, no complement (the coding strand IS
/// the mRNA with T in place of U).
pub fn reverse_transcribe(mrna: &str) -> String {
    mrna.chars().map(|c| if c == 'U' { 'T' } else { c }).collect()
}

const ONE_LETTER: [(char, &str); 20] = [
    ('A', "Ala"), ('R', "Arg"), ('N', "Asn"), ('D', "Asp"), ('C', "Cys"),
    ('Q', "Gln"), ('E', "Glu"), ('G', "Gly"), ('H', "His"), ('I', "Ile"),
    ('L', "Leu"), ('K', "Lys"), ('M', "Met"), ('F', "Phe"), ('P', "Pro"),
    ('S', "Ser"), ('T', "Thr"), ('W', "Trp"), ('Y', "Tyr"), ('V', "Val"),
];

/// One token — a 1-letter code, a 3-letter code, or a full name, any case —
/// to the canonical `&'static str` name this module and `genetic_table`
/// both use.
pub fn parse_aa_token(s: &str) -> Option<&'static str> {
    if s.chars().count() == 1 {
        let c = s.to_ascii_uppercase().chars().next()?;
        return ONE_LETTER.iter().find(|(l, _)| *l == c).map(|(_, n)| *n);
    }
    ONE_LETTER.iter().map(|(_, n)| *n).find(|n| n.eq_ignore_ascii_case(s))
}

/// A protein string to a full chain: 3-letter codes (dash/space/comma
/// separated) or 1-letter codes (a compact string, no separators).
pub fn parse_chain(input: &str) -> Option<Vec<&'static str>> {
    let input = input.trim();
    if input.is_empty() { return Some(Vec::new()); }
    if input.len() >= 3 && (input.contains('-') || input.contains(' ') || input.contains(',')) {
        let parts: Vec<&str> = input.split(&['-', ' ', ','][..]).filter(|s| !s.is_empty()).collect();
        let mut chain = Vec::with_capacity(parts.len());
        for part in parts { chain.push(parse_aa_token(part)?); }
        return Some(chain);
    }
    let mut chain = Vec::with_capacity(input.chars().count());
    for ch in input.chars() {
        let s = alloc::string::String::from(ch);
        chain.push(parse_aa_token(&s)?);
    }
    Some(chain)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferred_codon_ala_is_gcc() {
        assert_eq!(preferred_codon_for_aa("Ala", ""), Some("GCC"));
    }

    #[test]
    fn preferred_codon_leu_is_cuc_via_the_fourfold_tier() {
        // Leu's codons all have middle base U (tier 1 empty); CUN is a real
        // 4-fold box (CUU/CUC/CUA/CUG all -> Leu), so tier 2 picks CUC.
        assert_eq!(preferred_codon_for_aa("Leu", ""), Some("CUC"));
    }

    #[test]
    fn preferred_codon_met_is_its_only_codon() {
        assert_eq!(preferred_codon_for_aa("Met", ""), Some("AUG"));
    }

    #[test]
    fn full_translation_keeps_ground_layer_residues_lift_rna_drops() {
        // GCU = Ala, a ground-layer residue lift_rna would drop.
        let t = translate_full("AUGGCU", "");
        assert_eq!(t.protein, alloc::vec!["Met", "Ala"]);
    }

    #[test]
    fn reverse_of_full_translation_round_trips() {
        let forward = translate_full("AUGGCUUGUGGCAAGUAA", "");
        assert_eq!(forward.stopped, Some("UAA"));
        let back = reverse_translate_full(&forward.protein, "").unwrap();
        let round = translate_full(&back.mrna, "");
        assert_eq!(round.protein, forward.protein);
    }

    #[test]
    fn parse_chain_reads_one_letter_and_three_letter_forms_the_same() {
        assert_eq!(parse_chain("MAG"), Some(alloc::vec!["Met", "Ala", "Gly"]));
        assert_eq!(parse_chain("Met-Ala-Gly"), Some(alloc::vec!["Met", "Ala", "Gly"]));
    }

    #[test]
    fn mitochondrial_dialect_changes_degeneracy_and_choice() {
        // AGA/AGG are stop in mito, not Arg, so Arg's mito codon set shrinks.
        let std_codons = codons_for_aa("Arg", "");
        let mito_codons = codons_for_aa("Arg", "mitochondrial");
        assert!(mito_codons.len() < std_codons.len());
    }
}
