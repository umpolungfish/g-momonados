//! Chou-Fasman secondary structure and heuristic tertiary contacts.
//!
//! Port of red-hot_rebis's rhr_p4rky/gene_to_protein_pipeline.py stages 4-5
//! (the same source mOMonadOS's rebis::fold ports), here keyed on the
//! `&'static str` three-letter amino acid names this crate already uses
//! throughout (`genetic_table::CODON`), rather than introducing a foreign
//! enum. SerpentRod invariant: windingNumber <= contacts + 1.

use alloc::vec::Vec;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum SecondaryLabel { Helix, Sheet, Coil }

impl SecondaryLabel {
    pub fn symbol(self) -> &'static str {
        match self { Self::Helix => "H", Self::Sheet => "S", Self::Coil => "C" }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum ContactKind { Hydrophobic, Disulfide, Ionic }

impl ContactKind {
    pub fn name(self) -> &'static str {
        match self { Self::Hydrophobic => "hydrophobic", Self::Disulfide => "disulfide", Self::Ionic => "ionic" }
    }
}

pub struct TertiaryContact { pub i: usize, pub j: usize, pub kind: ContactKind, pub confidence: u8 }

pub struct FoldResidue {
    pub aa: &'static str,
    pub position: usize,
    pub secondary: SecondaryLabel,
    pub contacts: usize,
    pub winding_number: usize,
}

pub struct FoldResult {
    pub residues: Vec<FoldResidue>,
    pub contacts: Vec<TertiaryContact>,
    pub frobenius_ok: bool,
}

/// (name, helix, sheet, turn) x100 — Chou-Fasman propensities, and
/// (hydropathy x10, charge) for contact prediction. One table, not two,
/// since every amino acid needs both and there is exactly one of each.
const PROPS: [(&str, u32, u32, u32, i32, i8); 20] = [
    ("Ala", 142, 83,  66,  18, 0), ("Arg", 98,  93,  95,  -45, 1),
    ("Asn", 67,  89,  156, -35, 0), ("Asp", 101, 54,  146, -35, -1),
    ("Cys", 70,  119, 119, 25, 0), ("Gln", 111, 110, 98,  -35, 0),
    ("Glu", 151, 37,  74,  -35, -1), ("Gly", 57,  75,  156, -4, 0),
    ("His", 100, 87,  95,  -32, 0), ("Ile", 108, 160, 47,  45, 0),
    ("Leu", 121, 130, 59,  38, 0), ("Lys", 116, 74,  101, -39, 1),
    ("Met", 145, 105, 60,  19, 0), ("Phe", 113, 138, 60,  28, 0),
    ("Pro", 57,  55,  152, -16, 0), ("Ser", 77,  75,  143, -8, 0),
    ("Thr", 83,  119, 96,  -7, 0), ("Trp", 108, 137, 96,  -9, 0),
    ("Tyr", 69,  147, 114, -13, 0), ("Val", 106, 170, 50,  42, 0),
];

fn props(aa: &str) -> (u32, u32, u32, i32, i8) {
    PROPS.iter().find(|(n, ..)| *n == aa).map(|(_, h, s, t, hy, c)| (*h, *s, *t, *hy, *c))
        .unwrap_or((100, 100, 100, 0, 0))
}

fn chou_fasman(aa: &str) -> (u32, u32) { let (h, s, ..) = props(aa); (h, s) }
fn is_hydrophobic(aa: &str) -> bool { props(aa).3 > 0 }
fn signed_charge(aa: &str) -> i8 { props(aa).4 }

pub fn fold_sequence(chain: &[&'static str]) -> FoldResult {
    let n = chain.len();
    if n == 0 { return FoldResult { residues: Vec::new(), contacts: Vec::new(), frobenius_ok: true }; }

    let mut pred = alloc::vec![SecondaryLabel::Coil; n];

    // Alpha-helix: window=4, sum > 412; extend while individual > 100.
    {
        let mut i = 0;
        while i + 4 <= n {
            let sum: u32 = chain[i..i + 4].iter().map(|&a| chou_fasman(a).0).sum();
            if sum > 412 {
                let mut j = i + 4;
                while j < n && chou_fasman(chain[j]).0 > 100 { j += 1; }
                for k in i..j { pred[k] = SecondaryLabel::Helix; }
                i = j;
            } else { i += 1; }
        }
    }
    // Beta-sheet: window=3, sum > 315; skip positions already helix.
    {
        let mut i = 0;
        while i + 3 <= n {
            let already_h = (i..i + 3).any(|k| pred[k] == SecondaryLabel::Helix);
            if !already_h {
                let sum: u32 = chain[i..i + 3].iter().map(|&a| chou_fasman(a).1).sum();
                if sum > 315 {
                    let mut j = i + 3;
                    while j < n && chou_fasman(chain[j]).1 > 100 && pred[j] != SecondaryLabel::Helix { j += 1; }
                    for k in i..j { pred[k] = SecondaryLabel::Sheet; }
                    i = j;
                } else { i += 1; }
            } else { i += 1; }
        }
    }

    let mut contacts: Vec<TertiaryContact> = Vec::new();
    let min_seq_dist = { let d = n / 4; if d < 2 { 2 } else if d > 4 { 4 } else { d } };

    {
        let hydro: Vec<usize> = (0..n).filter(|&k| is_hydrophobic(chain[k])).collect();
        for a in 0..hydro.len() {
            for b in a + 1..hydro.len() {
                let (pi, pj) = (hydro[a], hydro[b]);
                if pj - pi > min_seq_dist {
                    let diff = (pj - pi) as u32;
                    let nv = n as u32;
                    let conf = if diff >= nv { 20u8 } else { (90u32.saturating_sub(diff * 70 / nv.max(1))) as u8 };
                    contacts.push(TertiaryContact { i: pi, j: pj, kind: ContactKind::Hydrophobic, confidence: conf });
                }
            }
        }
    }
    {
        let cys: Vec<usize> = (0..n).filter(|&k| chain[k] == "Cys").collect();
        for a in 0..cys.len() {
            for b in a + 1..cys.len() {
                let d = cys[b] - cys[a];
                if d >= 4 && d <= 200 {
                    contacts.push(TertiaryContact { i: cys[a], j: cys[b], kind: ContactKind::Disulfide, confidence: 80 });
                }
            }
        }
    }
    {
        let pos_c: Vec<usize> = (0..n).filter(|&k| signed_charge(chain[k]) > 0).collect();
        let neg_c: Vec<usize> = (0..n).filter(|&k| signed_charge(chain[k]) < 0).collect();
        let lim = (n * 4) / 5;
        for &pi in &pos_c {
            for &pj in &neg_c {
                let sd = if pj > pi { pj - pi } else { pi - pj };
                if sd > 3 && sd < lim {
                    let conf = (70u32.saturating_sub(sd as u32 * 70 / (n as u32).max(1))) as u8;
                    contacts.push(TertiaryContact { i: pi.min(pj), j: pi.max(pj), kind: ContactKind::Ionic, confidence: conf });
                }
            }
        }
    }

    contacts.sort_unstable_by(|a, b| a.i.cmp(&b.i).then(a.j.cmp(&b.j)).then(b.confidence.cmp(&a.confidence)));
    contacts.dedup_by(|a, b| a.i == b.i && a.j == b.j);

    let mut contact_count = alloc::vec![0usize; n];
    for c in &contacts {
        if c.i < n { contact_count[c.i] += 1; }
        if c.j < n { contact_count[c.j] += 1; }
    }

    let residues = (0..n).map(|k| {
        let cc = contact_count[k];
        FoldResidue { aa: chain[k], position: k, secondary: pred[k], contacts: cc, winding_number: cc + 1 }
    }).collect();

    FoldResult { residues, contacts, frobenius_ok: true }
}
