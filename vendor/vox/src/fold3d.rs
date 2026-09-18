//! B4->Ramachandran->Cartesian backbone reconstruction and a real PDB writer.
//!
//! Port of red-hot_rebis's rhr_p4rky/serpent_rod_v2.py (the B4_RAMACHANDRAN
//! table, build_frame/place_atom/build_backbone) and rhr_p4rky/pdb_writer.py
//! (ATOM/HELIX/SHEET/TER formatting) — the piece `fold.rs` alone doesn't
//! give: actual 3D coordinates, not just a secondary-structure label and a
//! contact list.
//!
//! B4 here is the same plain-char value `genetic::nuc_b4` already produces
//! ('F','T','B','N'), matching this crate's own idiom instead of importing
//! an enum. No external crates (this crate's own rule): sin/cos/sqrt are
//! hand-rolled below rather than pulling in libm, same as red-hot_rebis's
//! own no_std sibling in mOMonadOS hand-rolls its sqrt.
//!
//! One deliberate departure from the Python source: it indexes its B4 path
//! per-nucleotide but reads it with the per-residue loop index, an off-by-
//! factor-of-3 mismatch. Here every residue already has its own real B4
//! (the first position of the codon that produced it, forward or
//! reconstructed), so the lookup is direct instead of carrying that forward.

use alloc::string::String;
use alloc::vec::Vec;

fn sqrt_f64(x: f64) -> f64 {
    if x <= 0.0 { return 0.0; }
    let mut guess = if x > 1.0 { x / 2.0 } else { 1.0 };
    for _ in 0..40 { guess = 0.5 * (guess + x / guess); }
    guess
}

fn sin_f64(x: f64) -> f64 {
    // Every call site here hands in |x| <= pi (Ramachandran angles in
    // degrees converted to radians, or the fixed trans dihedral pi itself),
    // so a plain Taylor series needs no range reduction to reach double
    // precision.
    let x2 = x * x;
    let mut term = x;
    let mut sum = x;
    for k in 1..14i32 {
        term *= -x2 / ((2 * k) as f64 * (2 * k + 1) as f64);
        sum += term;
    }
    sum
}

fn cos_f64(x: f64) -> f64 {
    let x2 = x * x;
    let mut term = 1.0;
    let mut sum = 1.0;
    for k in 1..14i32 {
        term *= -x2 / ((2 * k - 1) as f64 * (2 * k) as f64);
        sum += term;
    }
    sum
}

fn fabs(x: f64) -> f64 { if x < 0.0 { -x } else { x } }

pub type Vec3 = (f64, f64, f64);

fn vec_sub(a: Vec3, b: Vec3) -> Vec3 { (a.0 - b.0, a.1 - b.1, a.2 - b.2) }
fn vec_norm(v: Vec3) -> f64 { sqrt_f64(v.0 * v.0 + v.1 * v.1 + v.2 * v.2) }
fn vec_cross(a: Vec3, b: Vec3) -> Vec3 {
    (a.1 * b.2 - a.2 * b.1, a.2 * b.0 - a.0 * b.2, a.0 * b.1 - a.1 * b.0)
}

const DEG: f64 = core::f64::consts::PI / 180.0;
const BOND_N_CA: f64 = 1.458;
const BOND_CA_C: f64 = 1.525;
const BOND_C_N: f64 = 1.329;
const BOND_C_O: f64 = 1.231;
const ANGLE_N_CA_C: f64 = 111.0 * DEG;
const ANGLE_CA_C_N: f64 = 116.2 * DEG;
const ANGLE_C_N_CA: f64 = 121.7 * DEG;

/// One Ramachandran step: phi/psi angles (degrees) and a secondary-structure
/// label, for one B4->B4 transition. The exact 16-entry table from
/// serpent_rod_v2.py's B4_RAMACHANDRAN.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct RamaEntry { pub phi: f64, pub psi: f64, pub ss: &'static str, pub conf: f64 }

pub fn ramachandran(from: char, to: char) -> RamaEntry {
    match (from, to) {
        ('N', 'T') => RamaEntry { phi: -57.0,  psi: -47.0, ss: "helix",   conf: 0.88 },
        ('T', 'B') => RamaEntry { phi: -119.0, psi: 113.0, ss: "sheet",   conf: 0.85 },
        ('B', 'F') => RamaEntry { phi: 57.0,   psi: 45.0,  ss: "helix_l", conf: 0.72 },
        ('F', 'N') => RamaEntry { phi: -60.0,  psi: -30.0, ss: "turn",    conf: 0.75 },
        ('N', 'N') => RamaEntry { phi: -65.0,  psi: -15.0, ss: "loop",    conf: 0.42 },
        ('T', 'T') => RamaEntry { phi: -95.0,  psi: 5.0,   ss: "loop",    conf: 0.40 },
        ('F', 'F') => RamaEntry { phi: -70.0,  psi: 35.0,  ss: "loop",    conf: 0.38 },
        ('B', 'B') => RamaEntry { phi: -55.0,  psi: -45.0, ss: "loop",    conf: 0.36 },
        ('T', 'N') => RamaEntry { phi: -50.0,  psi: -55.0, ss: "helix",   conf: 0.55 },
        ('B', 'T') => RamaEntry { phi: -135.0, psi: 135.0, ss: "sheet",   conf: 0.52 },
        ('F', 'B') => RamaEntry { phi: 65.0,   psi: 50.0,  ss: "helix_l", conf: 0.48 },
        ('N', 'F') => RamaEntry { phi: -70.0,  psi: -25.0, ss: "turn",    conf: 0.52 },
        ('N', 'B') => RamaEntry { phi: -80.0,  psi: -10.0, ss: "loop",    conf: 0.30 },
        ('T', 'F') => RamaEntry { phi: -100.0, psi: 20.0,  ss: "loop",    conf: 0.28 },
        ('B', 'N') => RamaEntry { phi: -50.0,  psi: -35.0, ss: "loop",    conf: 0.30 },
        ('F', 'T') => RamaEntry { phi: -85.0,  psi: -5.0,  ss: "loop",    conf: 0.28 },
        _ => RamaEntry { phi: -60.0, psi: -40.0, ss: "loop", conf: 0.30 }, // unreached: from/to are always NUC_B4 values
    }
}

fn max_conf_for_ss(ss: &str) -> f64 {
    let all = ['N', 'T', 'F', 'B'];
    let mut m = 0.0f64;
    let mut found = false;
    for &a in &all {
        for &b in &all {
            let e = ramachandran(a, b);
            if e.ss == ss { found = true; if e.conf > m { m = e.conf; } }
        }
    }
    if found { m } else { 0.5 }
}

/// One Ramachandran step per residue: residue 0's "from" is the fictitious
/// N predecessor (matching serpent_rod_v2.py's i==0 special case); every
/// later residue's "from" is the previous residue's own B4.
pub fn rama_steps(b4_path: &[char]) -> Vec<RamaEntry> {
    let mut out = Vec::with_capacity(b4_path.len());
    for i in 0..b4_path.len() {
        let from = if i == 0 { 'N' } else { b4_path[i - 1] };
        out.push(ramachandran(from, b4_path[i]));
    }
    out
}

fn build_frame(z_dir: Vec3) -> (Vec3, Vec3, Vec3) {
    let z_len = vec_norm(z_dir);
    if z_len < 1e-10 { return ((1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (0.0, 0.0, 1.0)); }
    let z = (z_dir.0 / z_len, z_dir.1 / z_len, z_dir.2 / z_len);
    let reference = if fabs(z.0) < 0.9 { (1.0, 0.0, 0.0) } else if fabs(z.1) < 0.9 { (0.0, 1.0, 0.0) } else { (0.0, 0.0, 1.0) };
    let mut x = vec_cross(z, reference);
    let mut x_len = vec_norm(x);
    if x_len < 1e-10 {
        let reference2 = if reference == (1.0, 0.0, 0.0) { (0.0, 1.0, 0.0) } else { (1.0, 0.0, 0.0) };
        x = vec_cross(z, reference2);
        x_len = vec_norm(x);
    }
    let x = (x.0 / x_len, x.1 / x_len, x.2 / x_len);
    let y = vec_cross(z, x);
    (x, y, z)
}

fn place_atom(prev: Vec3, prev_prev: Vec3, bond_len: f64, bond_angle: f64, dihedral: f64) -> Vec3 {
    let mut v1 = vec_sub(prev, prev_prev);
    if vec_norm(v1) < 1e-10 { v1 = (0.0, 0.0, 1.0); }
    let (x, y, z) = build_frame(v1);
    let local = (
        bond_len * sin_f64(bond_angle) * cos_f64(dihedral),
        bond_len * sin_f64(bond_angle) * sin_f64(dihedral),
        -bond_len * cos_f64(bond_angle),
    );
    (
        prev.0 + local.0 * x.0 + local.1 * y.0 + local.2 * z.0,
        prev.1 + local.0 * x.1 + local.1 * y.1 + local.2 * z.1,
        prev.2 + local.0 * x.2 + local.1 * y.2 + local.2 * z.2,
    )
}

pub struct BackboneAtom { pub n: Vec3, pub ca: Vec3, pub c: Vec3, pub o: Vec3 }

/// Build the real 3D backbone (N, CA, C, O per residue) from a Ramachandran
/// step per residue, by the same NeRF internal->Cartesian construction
/// serpent_rod_v2.py's build_backbone uses.
pub fn build_backbone(steps: &[RamaEntry]) -> Vec<BackboneAtom> {
    let n_res = steps.len();
    let mut residues: Vec<BackboneAtom> = Vec::with_capacity(n_res);
    if n_res == 0 { return residues; }

    let n0: Vec3 = (0.0, 0.0, 0.0);
    let ca0: Vec3 = (BOND_N_CA, 0.0, 0.0);
    let c0 = place_atom(ca0, n0, BOND_CA_C, ANGLE_N_CA_C, 0.0);
    let o_dir = vec_sub(ca0, c0);
    let o_len = vec_norm(o_dir);
    let o0 = if o_len > 0.01 {
        (c0.0 + o_dir.0 * BOND_C_O / o_len, c0.1 + o_dir.1 * BOND_C_O / o_len, c0.2 + o_dir.2 * BOND_C_O / o_len)
    } else { (c0.0, c0.1 + BOND_C_O, c0.2) };
    residues.push(BackboneAtom { n: n0, ca: ca0, c: c0, o: o0 });

    for i in 1..n_res {
        let (pr_c, pr_ca) = { let pr = &residues[residues.len() - 1]; (pr.c, pr.ca) };
        let phi_i = steps[i].phi * DEG;
        let psi_i = steps[i].psi * DEG;
        let ni = place_atom(pr_c, pr_ca, BOND_C_N, ANGLE_CA_C_N, core::f64::consts::PI);
        let cai = place_atom(ni, pr_c, BOND_N_CA, ANGLE_C_N_CA, phi_i);
        let ci = place_atom(cai, ni, BOND_CA_C, ANGLE_N_CA_C, psi_i);
        let od = vec_sub(cai, ci);
        let ol = vec_norm(od);
        let oi = if ol > 0.01 {
            (ci.0 + od.0 * BOND_C_O / ol, ci.1 + od.1 * BOND_C_O / ol, ci.2 + od.2 * BOND_C_O / ol)
        } else { (ci.0, ci.1 + BOND_C_O, ci.2) };
        residues.push(BackboneAtom { n: ni, ca: cai, c: ci, o: oi });
    }
    residues
}

/// A contiguous run of one Ramachandran ss label, for HELIX/SHEET records.
pub struct SsElement { pub kind: &'static str, pub start: usize, pub end: usize, pub length: usize, pub confidence: f64 }

pub fn group_ss_elements(steps: &[RamaEntry]) -> Vec<SsElement> {
    let mut out = Vec::new();
    if steps.is_empty() { return out; }
    let mut kind = steps[0].ss;
    let mut start = 0usize;
    for i in 1..steps.len() {
        if steps[i].ss != kind {
            out.push(SsElement { kind, start, end: i - 1, length: i - start, confidence: max_conf_for_ss(kind) });
            kind = steps[i].ss;
            start = i;
        }
    }
    out.push(SsElement { kind, start, end: steps.len() - 1, length: steps.len() - start, confidence: max_conf_for_ss(kind) });
    out
}

fn pdb_res_name(aa: &str) -> String {
    let up = aa.to_ascii_uppercase();
    if up.len() == 3 { up } else { String::from("UNK") }
}

// Columns match the wwPDB v3.3 ATOM spec (same layout pdb_writer.py's
// ATOM_FMT uses): name at 12..16, res name at 17..20, chain at 21, res_seq
// at 22..26, x/y/z at 30..38/38..46/46..54 — the columns any real PDB
// reader (this crate's own `vox pdb`/`vox glyco`, or an external viewer)
// expects.
fn format_atom(serial: u32, name: &str, res_name: &str, chain_id: char, res_seq: u32,
               xyz: Vec3, temp: f64, element: &str) -> String {
    alloc::format!(
        "ATOM  {:>5} {} {:<3} {}{:>4}    {:>8.3}{:>8.3}{:>8.3}{:>6.2}{:>6.2}          {:<2}  ",
        serial, name, res_name, chain_id, res_seq, xyz.0, xyz.1, xyz.2, 1.0_f64, temp, element
    )
}

/// Write a full PDB structure file from a real backbone: HEADER/TITLE,
/// REMARK secondary-structure summary, HELIX/SHEET from the Ramachandran-
/// derived structure, ATOM records for every backbone N/CA/C/O, TER, END.
/// Port of pdb_writer.py's write_pdb_from_gen2.
pub fn write_pdb(
    chain: &[&str],
    backbone: &[BackboneAtom],
    elements: &[SsElement],
    frobenius_verified: bool,
    winding_number: usize,
    title: &str,
    chain_id: char,
) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(alloc::format!("HEADER    {}", title));
    lines.push(String::from("TITLE     COMPILED THROUGH VOX - B4-RAMACHANDRAN BACKBONE"));
    lines.push(alloc::format!("TITLE     FROBENIUS-CLOSED: {}", if frobenius_verified { "YES" } else { "NO" }));
    lines.push(alloc::format!("REMARK   1   WINDING NUMBER: {}", winding_number));
    lines.push(alloc::format!("REMARK   1   FROBENIUS VERIFIED: {}", if frobenius_verified { "YES" } else { "NO" }));

    if !elements.is_empty() {
        lines.push(alloc::format!("REMARK   2   SECONDARY STRUCTURE ELEMENTS: {}", elements.len()));
        for el in elements {
            let seq: String = (el.start..=el.end).map(|j| chain.get(j).copied().unwrap_or("?")).collect();
            lines.push(alloc::format!("REMARK   2     {:<8} [{:>3}-{:>3}] len={} conf={:.3} seq={}",
                el.kind, el.start + 1, el.end + 1, el.length, el.confidence, seq));
        }
    }

    let mut helix_num = 0u32;
    for el in elements {
        if el.kind == "helix" || el.kind == "helix_l" {
            helix_num += 1;
            let init_aa = pdb_res_name(chain.get(el.start).copied().unwrap_or("Ala"));
            let end_aa = pdb_res_name(chain.get(el.end).copied().unwrap_or("Ala"));
            let h_class = if el.kind == "helix" { 1 } else { 5 };
            let helix_id = alloc::format!("H{}", helix_num);
            lines.push(alloc::format!(
                "HELIX {:>3} {:<3} {:<3} {}{:>4}  {:<3} {}{:>4}{:>2}  {:>5}",
                helix_num, helix_id, init_aa, chain_id, el.start + 1, end_aa, chain_id, el.end + 1, h_class, el.length
            ));
        }
    }

    let sheet_elements: Vec<&SsElement> = elements.iter().filter(|e| e.kind == "sheet").collect();
    for (j, el) in sheet_elements.iter().enumerate() {
        let init_aa = pdb_res_name(chain.get(el.start).copied().unwrap_or("Ala"));
        let end_aa = pdb_res_name(chain.get(el.end).copied().unwrap_or("Ala"));
        let sense = if j == 0 { 0 } else if j % 2 == 1 { -1 } else { 1 };
        lines.push(alloc::format!(
            "SHEET {:>3} S1  {:>3} {:<3}{}{:>4}  {:<3} {}{:>4} {:>2}",
            j + 1, sheet_elements.len(), init_aa, chain_id, el.start + 1, end_aa, chain_id, el.end + 1, sense
        ));
    }

    let mut serial = 0u32;
    for (i, atom) in backbone.iter().enumerate() {
        let res_seq = (i + 1) as u32;
        let res3 = pdb_res_name(chain.get(i).copied().unwrap_or("Ala"));
        serial += 1; lines.push(format_atom(serial, " N  ", &res3, chain_id, res_seq, atom.n, 0.0, " N"));
        serial += 1; lines.push(format_atom(serial, " CA ", &res3, chain_id, res_seq, atom.ca, 0.0, " C"));
        serial += 1; lines.push(format_atom(serial, " C  ", &res3, chain_id, res_seq, atom.c, 0.0, " C"));
        serial += 1; lines.push(format_atom(serial, " O  ", &res3, chain_id, res_seq, atom.o, 0.0, " O"));
    }
    serial += 1;
    let last_res = pdb_res_name(chain.last().copied().unwrap_or("Ala"));
    lines.push(alloc::format!("TER   {:>5}      {:<3} {}{:>4} ", serial, last_res, chain_id, backbone.len()));
    lines.push(String::from("END"));

    let mut out = lines.join("\n");
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sin_cos_match_known_values() {
        let pi = core::f64::consts::PI;
        assert!(fabs(sin_f64(0.0)) < 1e-12);
        assert!(fabs(cos_f64(0.0) - 1.0) < 1e-12);
        assert!(fabs(sin_f64(pi / 2.0) - 1.0) < 1e-9);
        assert!(fabs(cos_f64(pi) - (-1.0)) < 1e-9);
        assert!(fabs(sqrt_f64(2.0) - core::f64::consts::SQRT_2) < 1e-12);
    }

    #[test]
    fn build_backbone_produces_one_atom_set_per_residue_at_real_bond_lengths() {
        let path = ['F', 'T', 'B', 'N', 'F'];
        let steps = rama_steps(&path);
        let backbone = build_backbone(&steps);
        assert_eq!(backbone.len(), 5);
        for i in 1..backbone.len() {
            let d = vec_norm(vec_sub(backbone[i].n, backbone[i - 1].c));
            assert!(fabs(d - BOND_C_N) < 1e-6, "N-C(prev) bond should be {} A, got {}", BOND_C_N, d);
        }
    }

    #[test]
    fn written_pdb_round_trips_through_vox_own_pdb_reader() {
        let chain: [&str; 5] = ["Met", "Ala", "Gly", "Leu", "Ser"];
        let path = ['F', 'T', 'B', 'N', 'T'];
        let steps = rama_steps(&path);
        let backbone = build_backbone(&steps);
        let elements = group_ss_elements(&steps);
        let pdb = write_pdb(&chain, &backbone, &elements, true, 2, "TEST", 'A');
        let seq = crate::genetic::protein_from_pdb(&pdb);
        assert_eq!(seq.len(), chain.len(), "one CA-derived residue per chain entry must come back out");
        assert_eq!(seq, "MAGLS");
    }
}
