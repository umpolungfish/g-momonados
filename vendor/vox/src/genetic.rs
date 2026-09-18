//! The genetics lane: a coding sequence read as a word in the twelve.
//!
//! Not an analogy. The twelve operations and the twelve axes are one alphabet,
//! read as an operation or as an axis according to where the glyph stands. The
//! chain from nucleotide to glyph is proved in Lean and parsed into
//! `genetic_table.rs` by its generator: G is B because guanine wobble-pairs
//! with both C and U, C is T because it pairs only with G, A is F, U is N;
//! codons carry to amino acids by the genetic code; and exactly twelve amino
//! acids are promoted, bijecting the twelve axes.
//!
//! So a gene is already a word, and the same SIXTEEN_3 engine that verdicts x86
//! verdicts the transcript.

use alloc::string::String;
use alloc::vec::Vec;
use crate::genetic_table::{AA_GLYPH, CODON, NUC_B4};

/// One promoted codon, as read.
pub struct Read { pub codon: String, pub aa: &'static str, pub glyph: char, pub axis: &'static str }

/// What a lifted sequence says.
pub struct Transcript {
    pub word: Vec<char>,
    pub reading: Vec<Read>,
    /// the stop codon that ended the reading frame, if one did
    pub stopped: Option<&'static str>,
    /// the frame start, as an offset into the cleaned sequence
    pub start: usize,
    /// true when no AUG was found and the frame starts at zero
    pub implicit_frame: bool,
}

/// Belnap value of a nucleotide.
pub fn nuc_b4(c: char) -> Option<char> {
    NUC_B4.iter().find(|(n, _)| *n == c).map(|(_, b)| *b)
}

pub(crate) fn codon_meaning(c: &str) -> Option<(&'static str, &'static str)> {
    CODON.iter().find(|(k, _, _)| *k == c).map(|(_, kind, val)| (*kind, *val))
}

fn aa_glyph(aa: &str) -> Option<(char, &'static str)> {
    AA_GLYPH.iter().find(|(a, _, _, _)| *a == aa).map(|(_, g, ax, _)| (*g, *ax))
}

/// An RNA or DNA sequence to a word in the twelve. Reads from the first AUG in
/// frame, stops at a stop codon, and emits a glyph only where the codon names a
/// promoted amino acid; the ground layer activates no axis and is silent, which
/// is a fact of the code and not a gap in the lift.
pub fn lift_rna(seq: &str) -> Transcript {
    let clean: Vec<char> = seq.chars()
        .map(|c| c.to_ascii_uppercase())
        .filter(|c| matches!(c, 'A' | 'C' | 'G' | 'T' | 'U'))
        .map(|c| if c == 'T' { 'U' } else { c })
        .collect();

    let mut start = 0usize;
    let mut implicit_frame = true;
    for k in 0..clean.len().saturating_sub(2) {
        if clean[k] == 'A' && clean[k + 1] == 'U' && clean[k + 2] == 'G' {
            start = k; implicit_frame = false; break;
        }
    }

    let mut t = Transcript { word: Vec::new(), reading: Vec::new(), stopped: None, start, implicit_frame };
    let mut k = start;
    while k + 3 <= clean.len() {
        let codon: String = clean[k..k + 3].iter().collect();
        if let Some((kind, val)) = codon_meaning(&codon) {
            if kind == "stop" { t.stopped = Some(val); break; }
            if let Some((glyph, axis)) = aa_glyph(val) {
                t.word.push(glyph);
                t.reading.push(Read { codon, aa: val, glyph, axis });
            }
        }
        k += 3;
    }
    t
}

/// The one-letter code of a promoted amino acid, or None for the ground layer.
/// A protein arrives as residues, one layer past the codons `lift_rna` reads —
/// the codon degeneracy is already collapsed, so this reads the sequence directly.
/// The eight ground-layer amino acids (A G P T S R L V) activate no axis and are
/// silent, exactly as their codons are in the transcript.
fn one_letter_aa(c: char) -> Option<&'static str> {
    Some(match c.to_ascii_uppercase() {
        'M' => "Met", 'W' => "Trp", 'C' => "Cys", 'Y' => "Tyr",
        'F' => "Phe", 'I' => "Ile", 'N' => "Asn", 'Q' => "Gln",
        'H' => "His", 'D' => "Asp", 'K' => "Lys", 'E' => "Glu",
        // ground layer — real residues, no promoted axis:
        'A' | 'G' | 'P' | 'T' | 'S' | 'R' | 'L' | 'V' => return None,
        _ => return None,
    })
}

/// Three-letter residue name (PDB ATOM records, some FASTA) to one-letter.
fn three_to_one(res: &str) -> Option<char> {
    Some(match res.to_ascii_uppercase().as_str() {
        "MET" => 'M', "TRP" => 'W', "CYS" => 'C', "TYR" => 'Y', "PHE" => 'F',
        "ILE" => 'I', "ASN" => 'N', "GLN" => 'Q', "HIS" => 'H', "ASP" => 'D',
        "LYS" => 'K', "GLU" => 'E', "ALA" => 'A', "GLY" => 'G', "PRO" => 'P',
        "THR" => 'T', "SER" => 'S', "ARG" => 'R', "LEU" => 'L', "VAL" => 'V',
        _ => return None,
    })
}

/// A protein — an amino-acid sequence — lifted to a word in the twelve. The same
/// act as `lift_rna` one layer up: each promoted residue emits its glyph, the
/// ground layer is silent. One-letter input; a residue that is not an amino acid
/// (a gap, an X, whitespace) is skipped.
pub fn lift_protein(seq: &str) -> Transcript {
    let mut t = Transcript { word: Vec::new(), reading: Vec::new(), stopped: None, start: 0, implicit_frame: true };
    for (i, c) in seq.chars().enumerate() {
        if c.is_whitespace() { continue; }
        if let Some(aa) = one_letter_aa(c) {
            if let Some((glyph, axis)) = aa_glyph(aa) {
                t.word.push(glyph);
                t.reading.push(Read { codon: format!("{}", i + 1), aa, glyph, axis });
            }
        }
    }
    t
}

/// Pull the residue sequence out of a PDB, one letter per residue, reading the
/// CA atom of each so an all-atom file yields one residue per position rather than
/// one per atom. Chain breaks are not marked — the fold is one word.
pub fn protein_from_pdb(text: &str) -> String {
    let mut seq = String::new();
    for line in text.lines() {
        if !(line.starts_with("ATOM") || line.starts_with("HETATM")) { continue; }
        if line.len() < 20 { continue; }
        let atom = line.get(12..16).map(|s| s.trim()).unwrap_or("");
        if atom != "CA" { continue; }
        let res = line.get(17..20).map(|s| s.trim()).unwrap_or("");
        // Most PDBs name residues in three letters (MET); the odot/DARPin designs
        // here name them in one (M). Take the three-letter reading when it maps,
        // else accept a lone alphabetic character as already a one-letter code.
        if let Some(c) = three_to_one(res) {
            seq.push(c);
        } else if res.len() == 1 {
            if let Some(ch) = res.chars().next() {
                if ch.is_ascii_alphabetic() { seq.push(ch.to_ascii_uppercase()); }
            }
        }
    }
    seq
}

/// FASTA to a bare sequence: drop the header lines, concatenate the rest.
pub fn protein_from_fasta(text: &str) -> String {
    text.lines().filter(|l| !l.starts_with('>')).collect::<Vec<_>>().join("")
}

/// A glycosylation site found on a peptide — the boundary interface where a glycan
/// attaches. N-linked sites are the sequon Asn-X-[Ser/Thr] with X not Proline;
/// O-linked sites are a Ser or Thr, reported separately because they carry no
/// promoted glyph (both are ground-layer) and so are invisible in the word itself.
pub struct GlycoSite {
    pub pos: usize,          // 1-based residue index of the anchor
    pub kind: &'static str,  // "N-linked" | "O-linked"
    pub motif: String,       // the residues that make the site
    pub glyph: Option<char>, // the anchor's mark, where it has one (Asn = ∈)
}

/// Locate every glycosylation boundary interface on a one-letter peptide. The
/// glycan tree that hangs off each site is a separate object — a branched word in
/// its own right — and needs the monosaccharides grounded before it can be lifted;
/// this names WHERE the boundary is, which is the half that reads off the peptide
/// with no new grounding. N-linked anchors are Asn, which is ∈ (the recognition
/// gate), so an N-site is an ∈ in the peptide word that opens a sequon.
pub fn glyco_sites(seq: &str) -> Vec<GlycoSite> {
    let res: Vec<char> = seq.chars()
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    let mut sites = Vec::new();
    for i in 0..res.len() {
        // N-linked sequon: N - X(≠P) - S|T
        if res[i] == 'N' && i + 2 < res.len() && res[i + 1] != 'P'
            && (res[i + 2] == 'S' || res[i + 2] == 'T') {
            sites.push(GlycoSite {
                pos: i + 1,
                kind: "N-linked",
                motif: res[i..=i + 2].iter().collect(),
                glyph: aa_glyph("Asn").map(|(g, _)| g),
            });
        }
    }
    // O-linked: Ser/Thr are candidates, reported but not every S/T is glycosylated;
    // flagged as sites the sequence ADMITS, distinct from the determinate N-sequon.
    for i in 0..res.len() {
        if res[i] == 'S' || res[i] == 'T' {
            sites.push(GlycoSite {
                pos: i + 1,
                kind: "O-linked",
                motif: String::from(res[i]),
                glyph: None,
            });
        }
    }
    sites.sort_by_key(|s| s.pos);
    sites
}

/// The vertebrate mitochondrial code differs from the standard one in four
/// codons. Reading a mitochondrial gene with the standard table terminates it
/// early, and one of the differences matters to the alphabet directly: UGA is a
/// terminator in the standard code and tryptophan here, which is the mark that
/// closes a reading.
pub fn dialect_override(dialect: &str, codon: &str) -> Option<(&'static str, &'static str)> {
    match dialect {
        "mitochondrial" | "mito" | "vertebrate_mitochondrial" => match codon {
            "AUA" => Some(("aa", "Met")),
            "UGA" => Some(("aa", "Trp")),
            "AGA" => Some(("stop", "AGA")),
            "AGG" => Some(("stop", "AGG")),
            _ => None,
        },
        _ => None,
    }
}

/// A residue sequence to a word. One-letter codes; anything unrecognised is
/// skipped, as the eight silent residues are.
pub fn lift_peptide(seq: &str) -> Transcript {
    const ONE: [(char, &str); 20] = [
        ('A', "Ala"), ('R', "Arg"), ('N', "Asn"), ('D', "Asp"), ('C', "Cys"),
        ('Q', "Gln"), ('E', "Glu"), ('G', "Gly"), ('H', "His"), ('I', "Ile"),
        ('L', "Leu"), ('K', "Lys"), ('M', "Met"), ('F', "Phe"), ('P', "Pro"),
        ('S', "Ser"), ('T', "Thr"), ('W', "Trp"), ('Y', "Tyr"), ('V', "Val"),
    ];
    let mut t = Transcript {
        word: Vec::new(), reading: Vec::new(), stopped: None,
        start: 0, implicit_frame: false,
    };
    for c in seq.chars().map(|c| c.to_ascii_uppercase()) {
        if let Some((_, aa)) = ONE.iter().find(|(l, _)| *l == c) {
            if let Some((glyph, axis)) = aa_glyph(aa) {
                t.word.push(glyph);
                let mut buf = [0u8; 4];
                t.reading.push(Read {
                    codon: String::from(c.encode_utf8(&mut buf)), aa, glyph, axis,
                });
            }
        }
    }
    t
}

/// An RNA or DNA sequence read in a named genetic code.
pub fn lift_rna_dialect(seq: &str, dialect: &str) -> Transcript {
    let clean: Vec<char> = seq.chars()
        .map(|c| c.to_ascii_uppercase())
        .filter(|c| matches!(c, 'A' | 'C' | 'G' | 'T' | 'U'))
        .map(|c| if c == 'T' { 'U' } else { c })
        .collect();

    let mut start = 0usize;
    let mut implicit_frame = true;
    for k in 0..clean.len().saturating_sub(2) {
        if clean[k] == 'A' && clean[k + 1] == 'U' && clean[k + 2] == 'G' {
            start = k; implicit_frame = false; break;
        }
    }

    let mut t = Transcript { word: Vec::new(), reading: Vec::new(), stopped: None, start, implicit_frame };
    let mut k = start;
    while k + 3 <= clean.len() {
        let codon: String = clean[k..k + 3].iter().collect();
        let meaning = dialect_override(dialect, &codon).or_else(|| codon_meaning(&codon));
        if let Some((kind, val)) = meaning {
            if kind == "stop" { t.stopped = Some(val); break; }
            if let Some((glyph, axis)) = aa_glyph(val) {
                t.word.push(glyph);
                t.reading.push(Read { codon, aa: val, glyph, axis });
            }
        }
        k += 3;
    }
    t
}
