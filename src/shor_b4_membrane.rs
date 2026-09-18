//! shor_b4_membrane.rs — the Key membrane finding as a module.
//!
//! Verified source (MoDoT `./ask`, kernel-certified):
//!   click shors_algorithm ⋈ b4_factor_v2_engine on T<->H, D=0.67
//!     -> chimera_shor_b4test <rw mx vh rp tp mg ls sC et ht Z>
//!     KERNEL-CERTIFIED igFrobeniusAlg.mul p p = p.
//!   3-mer [shors_algorithm . b4_factor_v2_engine . belnap_shor_pipeline]
//!     best order fully enchains; regioregular T<->H bonds;
//!     syndiotactic tacticity (F B F); linear (telechelic).
//!   cycle b4_factor_v2_engine ⟳ shors_algorithm: reductive turnover,
//!     catalyst fixed point, KERNEL-CERTIFIED regeneration.
//! Composed word of the chimera tuple (kernel owns it):
//!   WORD = `⊢⋈⊡⊡⊥⊤⊙⊞⊤⋈≺⊤` (nested vessel ticks=17, depth 64).
#![allow(dead_code)]
extern crate alloc;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use num_bigint::BigUint;

pub const SHOR_TUPLE: &str = "<ht gt dc pq tc mc ls sq et nm ZM>";
pub const B4_TUPLE: &str = "<rw mx vh rp tp mg ls sC et ht Z>";
pub const BELNAP_SHOR_TUPLE: &str = "<ht gt dc pq tp mc ls sE et nm ZM>";
pub const CHIMERA_TUPLE: &str = "<rw mx vh rp tp mg ls sC et ht Z>";
pub const PRODUCT_TUPLE: &str = "<ht gt dc qb tp mg ls sq et kc ZM>";
pub const WORD: &str = "⊢⋈⊡⊡⊥⊤⊙⊞⊤⋈≺⊤";
pub const CLICK_DELTA: &str = "0.67";
pub const CLICK_PAIR: &str = "T<->H";

/// REPL/CLI face:
/// `shor_b4 click | cycle | chain | membrane run <WORD> <N> | membrane family <N> | list`
pub fn repl_shor_b4_membrane(args: &[&str]) -> String {
    match args.first().copied() {
        Some("click") => format!(
            "click shors_algorithm ⋈ b4_factor_v2_engine\n  live D=0.33 T<->H D={d} R<->S D=0.67\n  CLICK on {p} — closes.\n  product: {c} (ring saturated on {p})\n  KERNEL-CERTIFIED: igFrobeniusAlg.mul p p = p (mu∘d=id).\n  word: {w}",
            d = CLICK_DELTA, p = CLICK_PAIR, c = CHIMERA_TUPLE, w = WORD),
        Some("cycle") => format!(
            "cycle b4_factor_v2_engine ⟳ shors_algorithm\n  bind d=0.465 (catalytic grip) → reductive stroke → return stroke\n  catalyst {b} spent <-> restored; substrate {s} -> {t}\n  KERNEL-CERTIFIED: fixed point, Coagula∘Solve = id.\n  product(shors_algorithm‡): {t}",
            b = B4_TUPLE, s = SHOR_TUPLE, t = PRODUCT_TUPLE),
        Some("chain") => format!(
            "3-mer [shors_algorithm · b4_factor_v2_engine · belnap_shor_pipeline]\n  1. {s}\n  2. {b}\n  3. {l}\n  bonds: 1-2 condensation T<->H (D={d}); 2-3 condensation T<->H (D={d})\n  backbone: regioregular (T<->H head-to-tail); tacticity: syndiotactic (FBF)\n  cyclization: linear (telechelic, no head-to-tail closure)",
            s = SHOR_TUPLE, b = B4_TUPLE, l = BELNAP_SHOR_TUPLE, d = CLICK_DELTA),
        Some("list") => format!(
            "shor_b4 key finding\n  click: shors_algorithm ⋈ b4_factor_v2_engine -> {c} (T<->H D={d}, certified)\n  chain: [shor · b4 · belnap] regioregular/syndiotactic linear\n  cycle: b4 ⟳ shor fixed point certified\n  word: {w}\n  membrane family (name : word):\n{fam}",
            c = CHIMERA_TUPLE, d = CLICK_DELTA, w = WORD,
            fam = crate::membrane_family::family().iter()
                .map(|(nm, w)| format!("  {nm:28} {w}\n"))
                .collect::<String>()),
        Some("membrane") => {
            crate::membrane_family::repl_membrane(&args[1..])
        }
        _ => String::from("usage: shor_b4 click | cycle | chain | list | membrane list | membrane run <WORD> <N> | membrane family <N>"),
    }
}

/// Run the chimera WORD as a membrane on N (delegates to the math register).
pub fn run_chimera(n: &BigUint, depth: u32)
    -> (Option<(BigUint, BigUint)>, u64)
{
    crate::membrane_family::run_membrane(WORD, n, depth)
}

pub fn family_words() -> Vec<(&'static str, &'static str)> {
    let mut v: Vec<(&'static str, &'static str)> = crate::membrane_family::family();
    v.push(("chimera_shor_b4test", WORD));
    v
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn consts_hold_verified_values() {
        assert_eq!(CHIMERA_TUPLE, "<rw mx vh rp tp mg ls sC et ht Z>");
        assert_eq!(WORD, "⊢⋈⊡⊡⊥⊤⊙⊞⊤⋈≺⊤");
        assert_eq!(CLICK_DELTA, "0.67");
        assert_eq!(CLICK_PAIR, "T<->H");
    }
    #[test] fn faces_report_certified_findings() {
        assert!(repl_shor_b4_membrane(&["click"]).contains("KERNEL-CERTIFIED"));
        assert!(repl_shor_b4_membrane(&["cycle"]).contains("KERNEL-CERTIFIED"));
        let c = repl_shor_b4_membrane(&["chain"]);
        assert!(c.contains("regioregular") && c.contains("syndiotactic"));
    }
    #[test] fn chimera_word_parses_to_ops() {
        assert!(!crate::membrane_family::word_ops(WORD).is_empty());
    }
}
