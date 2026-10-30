//! tower_vox.rs — the towers, written in IMASM, run on VOX, run on the GPU.
//!
//! The Factor Resonance Tower and the Dialetheic Resolution Tower are glyph
//! words.  Their values are glyph words too: N is a WordTape, and the tower
//! carries N's own word as its product mark.  Nothing in this module converts
//! a value to a limb vector or a BigUint to "do arithmetic"; the tower word is
//! the structure and the value word is the data, and vox::verdict is the one
//! reader that closes both — on the CPU auditor and on the device through
//! gpu_vox.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use num_bigint::BigUint;

use crate::word_tape::WordTape;

/// The Factor Resonance Tower, flat payload.  The dissolution rule has already
/// been applied: the two internal ∈...∋ dyads (candidate decomposition and
/// factor-pair self-test) collapsed into the one surviving outer dyad.
pub const FACTOR_TOWER_WORD: &str = "⊢⊙≻⋈∈⊤≻⋈⊙⊥≺⋈⊙⊙≻⋈⊙≺⋈⊙⊡⋈⊙∋⊣";

/// The Dialetheic Resolution Tower, flat payload.  Here contradiction itself is
/// the matter being nested: truth evidence (⊤≻⋈⊙) and false evidence (⊥≺⋈⊙)
/// share one ambient mark and one surviving dyad.
pub const DIALETHIC_TOWER_WORD: &str = "⊢⊙∈⊤≻⋈⊙⊥≺⋈⊙∋⊡⊣";

/// Encode N as its own IMASM word — the value is the glyph word, no limbs.
pub fn encode_value(n: &BigUint) -> String {
    WordTape::from_biguint(n).0.clone()
}

/// Splice `value` in place of the first ⊙ that lies INSIDE the tower's own
/// fork/fuse span (after the first ∈), not merely the first ⊙ in the raw
/// text. `vox pairs` on the flat payloads shows the textually-first ⊙ sits
/// BEFORE the tower's ∈ ever opens — splicing there leaves N's own dyads
/// (one ∈/∋ pair per bit, from `encode_value`) as regions wholly outside the
/// tower's real work span, with no ancestry relationship to it at all: 8
/// disjoint closed regions instead of N genuinely nested inside the one
/// region that does the work. vox::verdict still reads T either way, since T
/// only certifies that everything opened is closed, not that N ever entered
/// the span — so the carrying was silently inert. Splicing at the first
/// interior ⊙ instead nests N's whole word inside the tower's fork/fuse
/// span, confirmed by `vox pairs`: the outer region's own span and interior
/// text now contain N's cells rather than sitting ahead of them.
fn carry_at_interior_mark(tower: &str, value: &str) -> String {
    let fork = tower.find('∈').expect("tower payload has no fork");
    let interior_mark = tower[fork..]
        .find('⊙')
        .map(|off| fork + off)
        .expect("tower payload has no ⊙ inside its fork/fuse span");
    let mark_len = '⊙'.len_utf8();
    let mut out = String::with_capacity(tower.len() + value.len());
    out.push_str(&tower[..interior_mark]);
    out.push_str(value);
    out.push_str(&tower[interior_mark + mark_len..]);
    out
}

/// The Factor Resonance Tower carrying N: splice N's word in place of the
/// product mark that sits inside the tower's own fork/fuse span. The tower
/// word is the structure; N's word is the value; nothing else is converted.
pub fn factor_tower_carrying(n: &BigUint) -> String {
    let value = encode_value(n);
    carry_at_interior_mark(FACTOR_TOWER_WORD, &value)
}

/// The Dialetheic Resolution Tower carrying N at its interior ambient mark.
pub fn dialetheic_tower_carrying(n: &BigUint) -> String {
    let value = encode_value(n);
    carry_at_interior_mark(DIALETHIC_TOWER_WORD, &value)
}

/// Verdict a word through the CPU auditor (vox::verdict), the same reader the
/// GPU kernel is checked against.
fn cpu_verdict(word: &str) -> char {
    let chars: Vec<char> = word.chars().filter(|c| !c.is_whitespace()).collect();
    crate::vox::verdict(&chars)
}

/// Run one tower word on the GPU and beside it the CPU auditor.
fn run_pair(label: &str, word: &str, device: usize) -> String {
    let cpu = cpu_verdict(word);
    let gpu = crate::gpu_vox::run(word, device);
    format!(
        "{label}:\n  word: {word}\n  CPU vox::verdict: {cpu}\n  {gpu}\n"
    )
}

/// tower_vox <n> [device]  — carry N through both towers, verdict on CPU and GPU.
pub fn repl_tower_vox(args: &[&str]) -> String {
    if args.is_empty() {
        return String::from(
            "tower_vox <n> [device]\n\n\
             Write the Factor Resonance Tower and the Dialetheic Resolution\n\
             Tower in IMASM, carry N as its own glyph word, and verdict both on\n\
             the CPU auditor (vox::verdict) and on the GPU (gpu_vox).\n\n\
             Try:  tower_vox 2147712859 0\n",
        );
    }
    let n: BigUint = match args[0].parse() {
        Ok(x) => x,
        Err(_) => return String::from("bad n — need a decimal integer\n"),
    };
    let device: usize = if args.len() > 1 { args[1].parse().unwrap_or(0) } else { 0 };

    let mut out = String::from("TOWER VOX\n=========\n\n");
    out.push_str(&format!("N encoded in IMASM: {}\n", encode_value(&n)));
    out.push_str(&format!(
        "factor tower flat payload (no value): {FACTOR_TOWER_WORD}\n"
    ));
    out.push_str(&format!(
        "dialetheic tower flat payload (no value): {DIALETHIC_TOWER_WORD}\n\n"
    ));
    out.push_str(&run_pair("Factor Resonance Tower", &factor_tower_carrying(&n), device));
    out.push_str(&run_pair("Dialetheic Resolution Tower", &dialetheic_tower_carrying(&n), device));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factor_flat_payload_is_the_surviving_dyad() {
        // Dissolution: the flat payload has exactly one surviving split/fuse
        // pair, the outer containment.
        assert_eq!(FACTOR_TOWER_WORD.matches('∈').count(), 1);
        assert_eq!(FACTOR_TOWER_WORD.matches('∋').count(), 1);
        // Fixation sits before the fuse but after the work: the factor pair is
        // fixed, then the enclosing fuse closes.
        let fix = FACTOR_TOWER_WORD.find('⊡').unwrap();
        let fuse = FACTOR_TOWER_WORD.rfind('∋').unwrap();
        assert!(fix < fuse);
    }

    #[test]
    fn dialetheic_flat_payload_has_one_dyad() {
        assert_eq!(DIALETHIC_TOWER_WORD.matches('∈').count(), 1);
        assert_eq!(DIALETHIC_TOWER_WORD.matches('∋').count(), 1);
    }

    #[test]
    fn value_word_is_canonical_imasm() {
        let w = encode_value(&BigUint::from(5u32));
        assert!(w.starts_with('⊢') && w.ends_with('⊣'));
        // 5 = 101₂ → two ⊤/⊥ parity cells: ⊥ (1), ⊤ (0), ⊥ (1)
        assert_eq!(w.matches('⊤').count() + w.matches('⊥').count(), 3);
    }

    #[test]
    fn carrying_splices_value_into_the_tower() {
        let n = BigUint::from(7u32);
        let carried = factor_tower_carrying(&n);
        let value = encode_value(&n);
        assert!(carried.contains(&value));
        // The tower's own remaining ⊙ marks are untouched.
        assert_eq!(carried.matches('⊙').count(), FACTOR_TOWER_WORD.matches('⊙').count() - 1 + value.matches('⊙').count());
    }

    #[test]
    fn carried_value_lands_inside_the_fork_fuse_span_not_ahead_of_it() {
        // Regression: splicing into the textually-first ⊙ (before the tower's
        // own ∈ ever opens) left N's dyads as regions wholly outside the
        // tower's real work span -- vox pairs showed 8 disjoint regions
        // instead of N nested inside the one region that does the work.
        // vox::verdict still read T on that broken carrying, since T only
        // certifies closure, not that N ever entered the span.
        for (tower, carrying) in [
            (FACTOR_TOWER_WORD, factor_tower_carrying as fn(&BigUint) -> String),
            (DIALETHIC_TOWER_WORD, dialetheic_tower_carrying as fn(&BigUint) -> String),
        ] {
            let fork = tower.find('∈').unwrap();
            let n = BigUint::from(91u32);
            let value = encode_value(&n);
            let carried = carrying(&n);
            let value_pos = carried.find(&value).expect("value not found in carried word");
            assert!(
                value_pos > fork,
                "value landed at byte {value_pos}, at or before the fork at byte {fork}"
            );
        }
    }

    #[test]
    fn both_flat_payloads_verdict_t_on_cpu() {
        assert_eq!(cpu_verdict(FACTOR_TOWER_WORD), 'T');
        assert_eq!(cpu_verdict(DIALETHIC_TOWER_WORD), 'T');
    }

    /// The towers actually run on the GPU: device 0 carries the word and the
    /// device verdict must match the CPU auditor's.  Skipped silently when no
    /// CUDA device is present, so the suite stays green off-device.
    #[test]
    fn factor_tower_runs_on_gpu() {
        let word = factor_tower_carrying(&BigUint::from(2147712859u32));
        let out = crate::gpu_vox::run(&word, 0);
        assert!(
            out.contains("matches: true") || out.contains("no CUDA context"),
            "gpu run: {out}"
        );
    }

    #[test]
    fn dialetheic_tower_runs_on_gpu() {
        let word = dialetheic_tower_carrying(&BigUint::from(2147712859u32));
        let out = crate::gpu_vox::run(&word, 0);
        assert!(
            out.contains("matches: true") || out.contains("no CUDA context"),
            "gpu run: {out}"
        );
    }
}
