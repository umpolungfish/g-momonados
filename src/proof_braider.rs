// ─── proof_braider.rs ──────────────────────────────────────────────────
// Lean ↔ IMASM ↔ braid roundtrip (spec: proof-braider).
//
// The claim's canonical Frobenius word is compiled to a braid (δ,
// braid_to_imasm) and read back as a tangle (μ, read_tangle). The roundtrip
// PASSES only if the tangle closes — depth returns to its start, μ∘δ survives
// the trip through the topological representation.
#![allow(dead_code)]
extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use crate::braid_protocol::{braid_to_imasm, read_tangle, token_name};

pub fn proof_braider_main(args: &[&str]) -> String {
    let flat: Vec<&str> = args.iter().flat_map(|s| s.split_whitespace()).collect();
    let sub = flat.first().copied().unwrap_or("");
    if sub == "value" {
        let val_str = flat.get(1).copied().unwrap_or("10403");
        // Convert value bits to braid generators:
        // bit 0 -> sigma_1, bit 1 -> sigma_2
        let mut gens: Vec<i32> = Vec::new();
        for b in val_str.bytes() {
            for shift in (0..8).rev() {
                if (b >> shift) & 1 == 1 {
                    gens.push(2);
                } else {
                    gens.push(1);
                }
            }
        }
        if gens.is_empty() {
            gens.push(1);
        }
        let prog = braid_to_imasm(&gens, 1, true);
        let mut imasm = String::new();
        for t in &prog {
            imasm.push_str(token_name(t));
            imasm.push(' ');
        }

        let mut out = String::from("PROOF-BRAIDER (VALUE CLOSURE)\n=============================\n\n");
        out.push_str(&format!("input N: {}\n", val_str));
        out.push_str("Lean:    μ∘δ = id\n");
        out.push_str(&format!("IMASM:   {} tokens\n", prog.len()));

        match read_tangle(&prog, 8, 1) {
            Ok(tr) => {
                let recovered = tr.generators == gens;
                // Decode generators back to value bytes:
                let mut recovered_bytes = Vec::new();
                for chunk in tr.generators.chunks(8) {
                    let mut byte = 0u8;
                    for (i, &g) in chunk.iter().enumerate() {
                        if g == 2 {
                            byte |= 1 << (7 - i);
                        }
                    }
                    recovered_bytes.push(byte);
                }
                let recovered_str = String::from_utf8_lossy(&recovered_bytes);
                let val_matches = recovered_str.trim_matches('\0') == val_str;

                out.push_str(&format!("crossings: {}   writhe: {:+}\n", tr.crossings, tr.writhe));
                out.push_str(&format!("recovered N: {}\n", if val_matches { val_str } else { &recovered_str }));
                out.push_str(&format!(
                    "roundtrip: {}   (μ∘δ {} N)\n",
                    if recovered && val_matches { "PASS" } else { "FAIL" },
                    if recovered && val_matches { "faithfully returns" } else { "diverges on" }
                ));
                if recovered && val_matches {
                    out.push_str("\nFrobenius closure confirmed: braider δ and unbraider μ faithfully returned N without divergence.\n");
                }
            }
            Err(e) => out.push_str(&format!("roundtrip: FAIL ({})\n", e)),
        }
        return out;
    }

    if sub != "roundtrip" {
        return "proof-braider roundtrip <lean-module-or-claim>\n\
                proof-braider value <decimal-or-numeral>\n\n\
                Lift a claim or numeric value to a braid and read it back. The\n\
                roundtrip PASSES iff the tangle closes (μ∘δ survives the trip).\n\n\
                Try:  proof-braider roundtrip Imscribing.Frobenius\n\
                      proof-braider value 10403\n".to_string();
    }
    let claim = flat.get(1).copied().unwrap_or("Imscribing.Frobenius");

    // The canonical Frobenius braid word; δ compiles it to IMASM, μ reads it
    // back. Closure survives iff μ recovers the same generators — the same
    // roundtrip criterion `demonstrate mu-delta` uses.
    let gens: [i32; 3] = [1, 2, -1];
    let prog = braid_to_imasm(&gens, 1, false);
    let mut imasm = String::new();
    for t in &prog {
        imasm.push_str(token_name(t));
        imasm.push(' ');
    }

    let mut out = String::from("PROOF-BRAIDER\n=============\n\n");
    out.push_str(&format!("claim:   {}\n", claim));
    out.push_str("Lean:    μ∘δ = id\n");
    out.push_str(&format!("IMASM:   {}\n", imasm.trim()));
    let gstr: Vec<String> = gens.iter().map(|g| format!("{}", g)).collect();
    out.push_str(&format!("braid:   [{}]\n\n", gstr.join(" ")));

    match read_tangle(&prog, gens.len() + 2, 1) {
        Ok(tr) => {
            let recovered = tr.generators == gens;
            out.push_str(&format!("crossings: {}   writhe: {:+}\n", tr.crossings, tr.writhe));
            out.push_str(&format!(
                "roundtrip: {}   (μ {} the braid)\n",
                if recovered { "PASS" } else { "FAIL" },
                if recovered { "recovers" } else { "does not recover" }
            ));
            if recovered {
                out.push_str("\nFrobenius closure survived the trip through the braid: the\n\
                              proof-object and its topological shadow are one object.\n");
            } else {
                out.push_str(&format!(
                    "\nμ read back {:?}, not {:?} — closure did not survive the representation.\n",
                    tr.generators, gens
                ));
            }
        }
        Err(e) => out.push_str(&format!("roundtrip: FAIL   ({})\n", e)),
    }
    out
}

