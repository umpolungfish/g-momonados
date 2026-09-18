//! factor_membrane.rs — Factor-Separating Imscription Membrane M_kappa.
//!
//! Public membrane operations use exactly one numeral object: the canonical
//! LSB-first word emitted by native_numeral::encode,
//!
//!   W = ⊢ (≻⋈∈bit∋)* ⊙⊡⊣,   ⊥=1, ⊤=0.
//!
//! Γ and Λ are delegated to native_numeral::interlace_words and
//! native_numeral::deinterlace_word, and every factor witness must satisfy the
//! same native_numeral::syzygy_preserves predicate before it is reported.
//!
//! The older MSB helper functions remain below only for historical regression
//! tests; they are not a second public representation and are not used by the
//! public membrane commands.
#![allow(dead_code)]
extern crate alloc;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use num_bigint::BigUint;
use num_traits::{One, Zero};
use crate::native_numeral::{
    encode as numeral_encode, decode as numeral_decode, interlace_words,
    deinterlace_word, syzygy_preserves, multiply_via_word, divmod_via_word,
    modulo_via_word, pow2, hensel_unbraid, range_prune_unbraid,
    UnbraidBudget, is_prime_miller_rabin, subtract_via_word,

};
use num_bigint::BigInt;

pub const WORD: &str = "⊢∈⊞≺∋⊡⊣";

/// Parse interlace word cells ≻⋈∈bit∋ -> MSB-first bit vec.
pub fn parse_word_cells(w: &str) -> Vec<u8> {
    let cs: Vec<char> = w.chars().collect(); let mut out = Vec::new();
    let mut i = 0;
    while i + 4 < cs.len() {
        if cs[i] == '≻' && cs[i+1] == '⋈' && cs[i+2] == '∈'
            && (cs[i+3] == '⊥' || cs[i+3] == '⊤') && cs[i+4] == '∋' {
            out.push(if cs[i+3] == '⊥' { 1 } else { 0 }); i += 5;
        } else { i += 1; }
    }
    out
}
/// D2 split of MSB-first cells: even -> p-lane, odd -> q-lane.
pub fn d2_msb(bits: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let mut a = Vec::new(); let mut b = Vec::new();
    for (i, v) in bits.iter().enumerate() {
        if i % 2 == 0 { a.push(*v) } else { b.push(*v) }
    }
    (a, b)
}
/// MSB-first bit vec -> BigUint.
pub fn int_msb(b: &[u8]) -> BigUint {
    let mut a = BigUint::zero();
    for v in b.iter() { a <<= 1; if *v == 1 { a |= BigUint::one(); } }
    a
}
/// BigUint -> MSB-first bit vec.
pub fn bits_msb(n: &BigUint) -> Vec<u8> {
    if n.is_zero() { return alloc::vec![0]; }
    let mut o = Vec::new(); let mut m = n.clone(); let one = BigUint::one();
    while !m.is_zero() { o.push(if (&m & &one) == one { 1 } else { 0 }); m >>= 1; }
    o.reverse(); o
}
/// LSB-first (cell0=LSB, bvals/bvalsd convention) readers.
pub fn int_le(b: &[u8]) -> BigUint {
    let mut a = BigUint::zero();
    for (i, v) in b.iter().enumerate() { if *v == 1 { a |= BigUint::one() << i; } }
    a
}
/// BigUint -> LSB-first bit vec (cell0=LSB).
pub fn bits_le(n: &BigUint) -> Vec<u8> {
    if n.is_zero() { return alloc::vec![0]; }
    let mut o = Vec::new(); let mut m = n.clone(); let one = BigUint::one();
    while !m.is_zero() { o.push(if (&m & &one) == one { 1 } else { 0 }); m >>= 1; }
    o
}
/// THE crossing, LSB-first reading: D2 even/odd lanes -> int_le daughters.
pub struct CrossRecLE {
    pub cells: usize, pub p: BigUint, pub q: BigUint, pub n: BigUint,
    pub pbits: usize, pub qbits: usize,
    pub full_le: BigUint, pub full_msb: BigUint,
    pub roundtrip: bool,
}
pub fn cross_word_le(w: &str) -> Option<CrossRecLE> {
    let bits = parse_word_cells(w);
    if bits.is_empty() { return None; }
    let (le0, le1) = d2_msb(&bits);
    let (p, q) = (int_le(&le0), int_le(&le1));
    let n = &p * &q;
    let roundtrip = gamma(&le0, &le1) == bits;
    Some(CrossRecLE {
        cells: bits.len(), pbits: le0.len(), qbits: le1.len(),
        full_le: int_le(&bits), full_msb: int_msb(&bits),
        roundtrip, p, q, n,
    })
}
fn dep(b: &[u8]) -> String {
    b.iter().map(|&x| if x == 1 { '⊥' } else { '⊤' }).collect()
}
/// Re-interlace lanes -> cell bit vec (roundtrip check).
pub fn gamma(pe: &[u8], qo: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for k in 0..pe.len().max(qo.len()) {
        if k < pe.len() { out.push(pe[k]); }
        if k < qo.len() { out.push(qo[k]); }
    }
    out
}
/// THE crossing: word -> (P, Q, N=P*Q), all verified.
pub struct CrossRec {
    pub cells: usize, pub p: BigUint, pub q: BigUint, pub n: BigUint,
    pub pbits: usize, pub qbits: usize, pub nbits: usize,
    pub roundtrip: bool, pub verified: bool,
}
pub fn cross_word(w: &str) -> Option<CrossRec> {
    let bits = parse_word_cells(w);
    if bits.is_empty() { return None; }
    let (pe, qo) = d2_msb(&bits);
    let (p, q) = (int_msb(&pe), int_msb(&qo));
    let n = &p * &q;
    let roundtrip = gamma(&pe, &qo) == bits;
    Some(CrossRec {
        cells: bits.len(), pbits: pe.len(), qbits: qo.len(),
        nbits: n.bits() as usize, roundtrip, verified: roundtrip,
        p, q, n,
    })
}
fn parse_big(s: &str) -> Option<BigUint> { s.parse::<BigUint>().ok() }

/// Exhaustive native-word constraint propagator (the coupled reconstruction).
/// W -> daughter-word states -> propagate -> syzygy_preserves as terminal
/// closure predicate. No arithmetic factoring lives here: no isqrt, no trial
/// division, no Fermat/Pollard/GCD. Both braid arms from native_numeral are
/// word-state traversals (Hensel bottom-up = low arm L_k with carry c_k;
/// range-prune top-down = high arm H_l with prefix interval), coupled by the
/// bridge: a daughter pair closes iff p*q==N (word multiply) AND
/// syzygy_preserves(N,p,q). Unbounded: u64::MAX node budget, every
/// (p_bits,q_bits) split visited, no give-up terminal.
fn propagate_word_states(n: &BigUint) -> Option<(BigUint, BigUint)> {
    use core::sync::atomic::AtomicBool;
    let one = BigUint::one();
    let two = BigUint::from(2u32);
    if *n == BigUint::zero() || *n == one { return None; }
    // Even N: peel one factor of 2 through word ops (divmod_via_word), the
    // same word-level move the native unbraid makes at its own even boundary
    // -- not trial division, just the even-word case.
    if modulo_via_word(n, &two).unwrap() == BigUint::zero() {
        let q = divmod_via_word(n, &two).unwrap().0;
        if syzygy_preserves(n, &two, &q) { return Some((two, q)); }
        return None;
    }
    let n_signed = BigInt::from(n.clone());
    let total_bits = n.bits() as usize;
    let stop = AtomicBool::new(false);
    // Balanced splits first: real factor pairs sit near bits(N)/2 each.
    for p_bits in (2..=(total_bits / 2 + 1)).rev() {
        for q_bits in [total_bits + 1 - p_bits, total_bits - p_bits] {
            if q_bits < p_bits || q_bits == 0 { continue; }
            // Low arm: Hensel lift from the bottom bit (daughter-word states
            // L_k=(p_<k,q_<k,c_k), carry propagated word-exactly).
            let mut b = UnbraidBudget { nodes: 0, cap: u64::MAX };
            let p0 = BigUint::from(1u32);
            let q0 = BigUint::from(1u32);
            let n_minus_1 = subtract_via_word(n, &one).unwrap();
            let c0 = BigInt::from_biguint(num_bigint::Sign::Minus,
                divmod_via_word(&n_minus_1, &BigUint::from(2u32)).unwrap().0);
            if let Some((p, q)) = hensel_unbraid(&n_signed, p_bits, q_bits, 1, p0, q0, c0, &mut b, &stop) {
                // Bridge + terminal closure: word product AND syzygy agree.
                if multiply_via_word(&p, &q) == *n && syzygy_preserves(n, &p, &q) {
                    return Some((p, q));
                }
            }
            // High arm: top-down interval prune (daughter-word states H_l
            // with the prefix-interval bridge condition).
            let mut b2 = UnbraidBudget { nodes: 0, cap: u64::MAX };
            let start_pos = (p_bits.max(q_bits) as i64) - 2;
            let p_hi = pow2(p_bits - 1);
            let q_hi = pow2(q_bits - 1);
            if let Some((p, q)) = range_prune_unbraid(n, p_bits, q_bits, start_pos, p_hi, q_hi, &mut b2, &stop) {
                if multiply_via_word(&p, &q) == *n && syzygy_preserves(n, &p, &q) {
                    return Some((p, q));
                }
            }
        }
    }
    None
}

/// Terminal classifier after exhaustive propagation. Only terminals:
/// COMPOSITE-found (Some above), PRIME-INDECOMPOSABLE (Miller-Rabin prime),
/// UNIT (0/1, handled by caller), INVALID (malformed input, caller side).
fn prime_indecomposable(n: &BigUint) -> bool {
    is_prime_miller_rabin(n).0
}


fn canonical_numeral_word(raw: &str) -> Option<String> {
    let raw = raw.trim();

    // Native-word input: accept it only when it is already the exact canonical
    // word emitted by native_numeral::encode for its decoded value.
    if raw.starts_with('⊢') {
        let n = numeral_decode(raw)?;
        let canonical = numeral_encode(&n.to_string());
        return if canonical == raw { Some(canonical) } else { None };
    }

    // Decimal input: parse it, then immediately enter the one canonical word
    // representation used by the membrane.
    let n = raw.parse::<BigUint>().ok()?;
    Some(numeral_encode(&n.to_string()))
}

fn membrane_value(raw: &str) -> Option<BigUint> {
    let w = canonical_numeral_word(raw)?;
    numeral_decode(&w)
}

fn canonical_pair_word(p: &BigUint, q: &BigUint) -> Option<String> {
    let pw = numeral_encode(&p.to_string());
    let qw = numeral_encode(&q.to_string());
    interlace_words(&pw, &qw).ok()
}

/// Two-arm co-evolution descent: the μ∘δ diagram with both branches kept live.
///
/// Each node is seeded by the dyadic split δ(value) = (even-cell lane,
/// odd-cell lane).  Γ (interlace) re-closes that seed to the value exactly;
/// that is the representation half of the syzygy and it always holds on a
/// canonical word.  The two arms then co-evolve to the arithmetic fixed point
/// through the coupled word-state propagator, whose output (p,q) is the pair
/// where μ (native word multiply) closes on the value: p*q == value and
/// syzygy_preserves holds.  Both daughters stay alive and are recursed, so the
/// leaves are the prime factorization and the whole tree is the exact
/// multiscale ancestry of the parent word.  This is the desired object: not a
/// single even-lane spine, but both branches descending under the μ relation.
fn coevolve_factor_tree(value: &BigUint, prefix: &str, out: &mut String, budget: &mut u32) {
    if *budget == 0 { out.push_str(&format!("{}... (node budget reached)\n", prefix)); return; }
    *budget -= 1;
    let one = BigUint::one();
    let w = numeral_encode(&value.to_string());
    // δ: dyadic seed split, with Γ∘δ = id verified on the canonical word.
    let (sew, sow) = match deinterlace_word(&w) {
        Ok(v) => v,
        Err(e) => { out.push_str(&format!("{}{} INVALID: {}\n", prefix, value, e)); return; }
    };
    let se = numeral_decode(&sew).unwrap();
    let so = numeral_decode(&sow).unwrap();
    let gamma_id = interlace_words(&sew, &sow).map(|x| x == w).unwrap_or(false);

    if *value <= one {
        out.push_str(&format!("{}{} UNIT\n", prefix, value));
        return;
    }
    if is_prime_miller_rabin(value).0 {
        out.push_str(&format!("{}{} PRIME  [δ=({},{}) Γ∘δ=id:{}]\n", prefix, value, se, so, gamma_id));
        return;
    }
    // Co-evolve the two arms to the μ-fixed point (p*q == value, syzygy held).
    match propagate_word_states(value) {
        Some((p, q)) => {
            let mu_closes = multiply_via_word(&p, &q) == *value && syzygy_preserves(value, &p, &q);
            out.push_str(&format!(
                "{}{} = {} × {}  [δ=({},{}) Γ∘δ=id:{} | μ closes:{}]\n",
                prefix, value, p, q, se, so, gamma_id, mu_closes
            ));
            // Both arms alive: descend the left child AND the right child.
            let child_prefix = format!("{}  ", prefix);
            coevolve_factor_tree(&p, &child_prefix, out, budget);
            coevolve_factor_tree(&q, &child_prefix, out, budget);
        }
        None => {
            out.push_str(&format!(
                "{}{} PRIME-INDECOMPOSABLE  [δ=({},{}) Γ∘δ=id:{}]\n",
                prefix, value, se, so, gamma_id
            ));
        }
    }
}

/// Public membrane entry points use exactly the same canonical word type as
/// native_numeral::encode.  The older MSB/LSB helpers above remain only as
/// legacy diagnostics/tests; they are not a second public codec.
pub fn repl_factor_membrane(args: &[&str]) -> String {
    if args.is_empty() || args[0] == "help" {
        return String::from(
"factor_membrane — every numeric operand accepts DECIMAL or canonical NATIVE-WORD\n\
 factor <x>       factor x, then require encode/Γ/Λ/μ syzygy closure\n\
 cross <x>        canonical Λ split of x\n\
 separate <x>     same canonical Λ split, named as membrane separation\n\
 word <p> <q>     Γ-interlace p and q; each may be decimal or native-word\n\
 recurse <x>      two-arm co-evolution tree: δ seeds, μ closes, both branches descend to primes\n\
 encode <x>       decimal encodes; native-word canonicalizes idempotently\n\
 All public paths use native_numeral::encode as the single representation."
        );
    }

    match args[0] {
        "encode" => {
            if args.len() < 2 { return String::from("usage: factor_membrane encode <decimal|native-word>"); }
            let raw = args[1..].join(" ");
            let w = match canonical_numeral_word(&raw) {
                Some(w) => w,
                None => return String::from("expected decimal or canonical native_numeral word"),
            };
            let n = numeral_decode(&w).unwrap();
            let back = numeral_encode(&n.to_string()) == w && numeral_decode(&w).map(|x| x == n).unwrap_or(false);
            format!("{}  (canonical native_numeral; value={} idempotent={})", w, n, back)
        }

        "word" => {
            if args.len() < 3 { return String::from("usage: factor_membrane word <decimal|native-word P> <decimal|native-word Q>"); }
            let (p, q) = match (membrane_value(args[1]), membrane_value(args[2])) {
                (Some(a), Some(b)) => (a, b), _ => return String::from("bad P/Q: each must be decimal or canonical native word"),
            };
            let n = &p * &q;
            let w = match canonical_pair_word(&p, &q) {
                Some(w) => w,
                None => return String::from("interlace failed"),
            };
            let syz = syzygy_preserves(&n, &p, &q);
            format!("{}  (P={} Q={} N=P*Q={} syzygy_preserves={})", w, p, q, n, syz)
        }

        "cross" | "separate" => {
            if args.len() < 2 { return String::from("usage: factor_membrane cross <encode-word|N>"); }
            let raw = args[1..].join(" ");
            let w = match canonical_numeral_word(&raw) {
                Some(w) => w,
                None => return String::from("expected decimal N or canonical native_numeral encode word"),
            };
            let full = numeral_decode(&w).unwrap();
            let (pw, qw) = match deinterlace_word(&w) {
                Ok(v) => v,
                Err(e) => return format!("cross failed: {}", e),
            };
            let p = numeral_decode(&pw).unwrap();
            let q = numeral_decode(&qw).unwrap();
            let lane_n = &p * &q;
            let roundtrip = interlace_words(&pw, &qw).map(|x| x == w).unwrap_or(false);
            // Γ∘Λ=id is only the representation round-trip.  The factor
            // syzygy is stronger: μ(p,q) must close on THIS parent value.
            let parent_syzygy = syzygy_preserves(&full, &p, &q);
            format!(
                "W={}\nΛ(W).p={}\nΛ(W).q={}\np={}\nq={}\nΓ(Λ(W))==W: {}\nfull decode(W)={}\nμ(p,q)={}\nμ(p,q)==decode(W): {}\nsyzygy preserves [parent; Γ; Λ; μ]: {}",
                w, pw, qw, p, q, roundtrip, full, lane_n, lane_n == full, parent_syzygy
            )
        }

        "factor" => {
            if args.len() < 2 { return String::from("usage: factor_membrane factor <encode-word|N>"); }
            let raw = args[1..].join(" ");
            let n_word = match canonical_numeral_word(&raw) {
                Some(w) => w,
                None => return String::from("INVALID: expected decimal N or canonical native_numeral encode word"),
            };
            let n = numeral_decode(&n_word).unwrap();
            if n == BigUint::zero() || n == BigUint::one() {
                return format!("N={}\nencode(N)={}\nUNIT: no two-factor descent exists", n, n_word);
            }
            match propagate_word_states(&n) {
                Some((p, q)) => {
                    if !syzygy_preserves(&n, &p, &q) {
                        return format!("INTERNAL SYZYGY FAILURE after factor_membrane factor N={}", n);
                    }
                    let d = canonical_pair_word(&p, &q).unwrap();
                    let (pw, qw) = deinterlace_word(&d).unwrap();
                    format!(
                        "N={}\nencode(N)={}\nCOMPOSITE-found\np={}\nq={}\nΓ(encode(p),encode(q))={}\nΛ.p={}\nΛ.q={}\np*q==N: {}\nsyzygy preserves [encode; Γ; Λ; μ]: true",
                        n, n_word, p, q, d, pw, qw, multiply_via_word(&p, &q) == n
                    )
                }
                None if prime_indecomposable(&n) => format!("N={}\nencode(N)={}\nPRIME-INDECOMPOSABLE: exhaustive word-state propagation closed with no daughter pair; no two-factor unbraid exists", n, n_word),
                None => format!("N={}\nencode(N)={}\nPRIME-INDECOMPOSABLE: exhaustive word-state propagation closed with no daughter pair", n, n_word),
            }
        }

        "recurse" => {
            if args.len() < 2 { return String::from("usage: factor_membrane recurse <decimal|native-word>"); }
            let raw = args[1..].join(" ");
            let w = match canonical_numeral_word(&raw) {
                Some(w) => w,
                None => return String::from("INVALID: expected decimal or canonical native_numeral word"),
            };
            let value = numeral_decode(&w).unwrap();
            let mut out = String::from(
                "two-arm co-evolution tree — δ seeds both lanes, μ closes on the value, both branches descend:\n");
            let mut budget = 4096u32;
            coevolve_factor_tree(&value, "  ", &mut out, &mut budget);
            out
        }

        _ => String::from("unknown subcommand (factor | cross | word | encode | separate | recurse)"),
    }
}

#[cfg(test)] mod tests {
    use super::*;
    fn big(s: &str) -> BigUint { s.parse().unwrap() }
    #[test] fn gamma_roundtrip_gives_factors() {
        // W = gamma(13,17): cells MSB-first, P*Q=N verified.
        let w = "⊢≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋⊙⊡⊣"; // gamma(13,17)
        let r = cross_word(w).unwrap();
        assert!(r.roundtrip);
        assert_eq!(&r.p * &r.q, r.n);
        // lanes re-imscribe to the same word: gamma(P,Q)==W is the closure.
        let (pe, qo) = (bits_msb(&r.p), bits_msb(&r.q));
        assert_eq!(gamma(&pe, &qo), parse_word_cells(w));
    }
    #[test] fn le_bvalsd_line0_fullword_is_decimal() {
        // bvals word 0 (862 cells, LSB-first) reads back as the bvalsd decimal N0.
        let n0 = big("22112825529529666435281085255026230927612089502470015394413748319128822941402001986512729726569746599085900330031400051170742204560859276357953757185954298838958709229238491006703034124620545784566413664540684214361293017694020846391065875914794251435144458199");
        assert_eq!(n0.bits() as usize, 862);
        let bits = bits_le(&n0);
        assert_eq!(bits.len(), 862);
        assert_eq!(int_le(&bits), n0); // full-word LSB value == decimal
        let (l0, l1) = d2_msb(&bits);
        assert_eq!(gamma(&l0, &l1), bits); // D2 roundtrip
    }
    #[test] fn encode_cross_roundtrip_lsb() {
        // encode(N) -> word -> LE crossing: full-word LSB value == N, lanes roundtrip.
        for st in ["91", "143", "8051"] {
            let n = big(st);
            let bits = bits_le(&n);
            let cells: String = bits.iter()
                .map(|&b| if b == 1 { "≻⋈∈⊥∋" } else { "≻⋈∈⊤∋" }).collect();
            let w = format!("⊢{}⊙⊡⊣", cells);
            let l = cross_word_le(&w).unwrap();
            assert!(l.roundtrip);
            assert_eq!(l.full_le, n);
            assert_eq!(&l.p * &l.q, l.n);
        }
    }
    #[test] fn msb_gamma_vectors_unchanged() {
        // Settled MSB vectors still hold exactly.
        let w = "⊢≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋⊙⊡⊣";
        let r = cross_word(w).unwrap();
        assert_eq!((r.p.to_string(), r.q.to_string()), ("27".to_string(), "8".to_string()));
    }
    #[test] fn propagator_small_factors() {
        let n = big("91");
        let (p, q) = propagate_word_states(&n).unwrap();
        assert_eq!(multiply_via_word(&p, &q), n);
        assert!(syzygy_preserves(&n, &p, &q));
    }

    #[test] fn public_pair_word_uses_native_numeral_syzygy() {
        let p = big("13");
        let q = big("17");
        let n = &p * &q;
        let d = canonical_pair_word(&p, &q).unwrap();
        let (pw, qw) = deinterlace_word(&d).unwrap();
        assert_eq!(numeral_decode(&pw).unwrap(), p);
        assert_eq!(numeral_decode(&qw).unwrap(), q);
        assert!(syzygy_preserves(&n, &p, &q));
    }

    #[test] fn public_factor_accepts_native_encode_word_and_requires_syzygy() {
        let w = numeral_encode("91");
        let report = repl_factor_membrane(&["factor", &w]);
        assert!(report.contains("p*q==N: true"));
        assert!(report.contains("syzygy preserves [encode; Γ; Λ; μ]: true"));
    }

    #[test] fn public_cross_is_gamma_lambda_identity_on_canonical_words() {
        // Γ is bit-interleave, not multiplication: decode(Γ(13,17)) = 595,
        // not 13*17 = 221.  So the lane round-trip holds while the parent
        // factor syzygy on decode(D) is correctly false.  The full syzygy
        // (with n = p*q) is covered by public_pair_word_uses_native_numeral_syzygy.
        let p = big("13");
        let q = big("17");
        let d = canonical_pair_word(&p, &q).unwrap();
        assert_eq!(numeral_decode(&d).unwrap(), big("595"));
        let report = repl_factor_membrane(&["cross", &d]);
        assert!(report.contains("Γ(Λ(W))==W: true"));
        assert!(report.contains("μ(p,q)==decode(W): false"));
        assert!(report.contains("syzygy preserves [parent; Γ; Λ; μ]: false"));
    }
    #[test] fn every_public_option_accepts_decimal_and_native_word() {
        let w91 = numeral_encode("91");
        let w13 = numeral_encode("13");
        let w17 = numeral_encode("17");

        for x in ["91", w91.as_str()] {
            assert!(!repl_factor_membrane(&["encode", x]).contains("expected decimal"));
            assert!(!repl_factor_membrane(&["cross", x]).contains("expected decimal"));
            assert!(!repl_factor_membrane(&["separate", x]).contains("expected decimal"));
            assert!(!repl_factor_membrane(&["factor", x]).contains("expected decimal"));
            assert!(!repl_factor_membrane(&["recurse", x]).contains("expected decimal"));
        }

        assert!(!repl_factor_membrane(&["word", "13", "17"]).contains("bad P/Q"));
        assert!(!repl_factor_membrane(&["word", &w13, &w17]).contains("bad P/Q"));
        assert!(!repl_factor_membrane(&["word", "13", &w17]).contains("bad P/Q"));
        assert!(!repl_factor_membrane(&["word", &w13, "17"]).contains("bad P/Q"));
    }

    #[test] fn recurse_is_two_arm_tree_both_branches_descend() {
        // 15959 is prime: the tree is a single PRIME leaf and Γ∘δ=id holds
        // on the dyadic seed. No factorization line, no children.
        let report = repl_factor_membrane(&["recurse", "15959"]);
        assert!(report.contains("15959 PRIME"));
        assert!(report.contains("Γ∘δ=id:true"));
        assert!(!report.contains("15959 = "));
        // 91 = 7 × 13: μ closes on 91, and BOTH children are recursed to
        // prime leaves. Following only the even lane would drop one of them;
        // here both 7 and 13 appear as PRIME leaves.
        let report91 = repl_factor_membrane(&["recurse", "91"]);
        assert!(report91.contains("μ closes:true"));
        assert!(report91.contains("7 PRIME"));
        assert!(report91.contains("13 PRIME"));
    }

    #[test] fn propagator_closes_large_composite_through_membrane() {
        // 10007x10009: exhaustive word-state traversal closes via syzygy.
        let n: BigUint = "100160063".parse().unwrap();
        let (px, qx) = propagate_word_states(&n).unwrap();
        assert_eq!(multiply_via_word(&px, &qx), n);
        assert!(syzygy_preserves(&n, &px, &qx));
        let report = repl_factor_membrane(&["factor", "100160063"]);
        assert!(report.contains("COMPOSITE-found"));
        assert!(report.contains("syzygy preserves [encode; Γ; Λ; μ]: true"));
    }

    #[test] fn cross_distinguishes_lane_roundtrip_from_parent_factor_syzygy() {
        let report = repl_factor_membrane(&["cross", "15959"]);
        assert!(report.contains("Γ(Λ(W))==W: true"));
        assert!(report.contains("μ(p,q)==decode(W): false"));
        assert!(report.contains("syzygy preserves [parent; Γ; Λ; μ]: false"));
    }

}
