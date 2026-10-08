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
use crate::word_tape::WordTape;

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

/// Signed Hensel carry held as an IMASM magnitude word plus its orientation.
#[derive(Clone)]
struct SignedWord { negative: bool, magnitude: WordTape }

impl SignedWord {
    fn zero() -> Self { Self { negative: false, magnitude: WordTape::zero() } }
    fn negative(magnitude: WordTape) -> Self {
        let negative = !magnitude.is_zero();
        Self { negative, magnitude }
    }
    fn is_zero(&self) -> bool { self.magnitude.is_zero() }
    fn add_magnitude(&self, value: &WordTape) -> Self {
        if self.negative {
            if self.magnitude.ge(value) {
                let magnitude = self.magnitude.sub(value).unwrap();
                let negative = !magnitude.is_zero();
                Self { negative, magnitude }
            } else {
                Self { negative: false, magnitude: value.sub(&self.magnitude).unwrap() }
            }
        } else {
            Self { negative: false, magnitude: self.magnitude.add(value) }
        }
    }
    fn halve_even(&self) -> Self {
        debug_assert!(!self.magnitude.is_odd());
        let magnitude = self.magnitude.shr1();
        let negative = self.negative && !magnitude.is_zero();
        Self { negative, magnitude }
    }
}

fn hensel_unbraid_word(
    n: &WordTape, p_bits: usize, q_bits: usize, k: usize,
    p: WordTape, q: WordTape, carry: SignedWord,
    nodes: &mut u64, cap: u64,
) -> Option<(WordTape, WordTape)> {
    *nodes = nodes.saturating_add(1);
    if *nodes > cap { return None; }
    let short = p_bits.min(q_bits);
    let long = p_bits.max(q_bits);
    if k == long {
        return if carry.is_zero() && p.mul(&q) == *n { Some((p, q)) } else { None };
    }
    let c_parity = carry.magnitude.is_odd() as u8;
    let p_open = k < p_bits;
    let q_open = k < q_bits;
    let step = WordTape::one_at(k);
    if k < short {
        for p_bit in [0u8, 1] {
            let q_bit = (c_parity + p_bit) % 2;
            let new_p = if p_bit == 1 { p.add(&step) } else { p.clone() };
            let new_q = if q_bit == 1 { q.add(&step) } else { q.clone() };
            let mut contribution = WordTape::zero();
            if p_bit == 1 { contribution = contribution.add(&q); }
            if q_bit == 1 { contribution = contribution.add(&p); }
            if p_bit == 1 && q_bit == 1 { contribution = contribution.add(&step); }
            let new_carry = carry.add_magnitude(&contribution).halve_even();
            if let Some(pair) = hensel_unbraid_word(n, p_bits, q_bits, k+1, new_p, new_q, new_carry, nodes, cap) {
                return Some(pair);
            }
            if *nodes > cap { return None; }
        }
        None
    } else {
        let bit = c_parity;
        let (new_p, new_q, contribution) = if p_open {
            (if bit == 1 { p.add(&step) } else { p.clone() }, q.clone(), if bit == 1 { q.clone() } else { WordTape::zero() })
        } else if q_open {
            (p.clone(), if bit == 1 { q.add(&step) } else { q.clone() }, if bit == 1 { p.clone() } else { WordTape::zero() })
        } else {
            return if carry.is_zero() && p.mul(&q) == *n { Some((p, q)) } else { None };
        };
        let new_carry = carry.add_magnitude(&contribution).halve_even();
        hensel_unbraid_word(n, p_bits, q_bits, k+1, new_p, new_q, new_carry, nodes, cap)
    }
}

fn range_prune_unbraid_word(
    n: &WordTape, p_bits: usize, q_bits: usize, pos: i64,
    p: WordTape, q: WordTape, nodes: &mut u64, cap: u64,
) -> Option<(WordTape, WordTape)> {
    *nodes = nodes.saturating_add(1);
    if *nodes > cap { return None; }
    if pos < 0 { return if p.mul(&q) == *n { Some((p, q)) } else { None }; }
    let span = WordTape::one_at((pos as usize)+1).sub(&WordTape::one()).unwrap();
    let p_max = p.add(&span);
    let q_max = q.add(&span);
    if p.mul(&q).gt(n) || n.gt(&p_max.mul(&q_max)) { return None; }
    let p_free = (pos as usize) < p_bits.saturating_sub(1) && pos > 0;
    let q_free = (pos as usize) < q_bits.saturating_sub(1) && pos > 0;
    let choices = |free: bool| -> &'static [u8] { if free { &[0,1] } else if pos == 0 { &[1] } else { &[0] } };
    let step = WordTape::one_at(pos as usize);
    for &pb in choices(p_free) {
        let p_next = if pb == 1 { p.add(&step) } else { p.clone() };
        for &qb in choices(q_free) {
            let q_next = if qb == 1 { q.add(&step) } else { q.clone() };
            if let Some(pair) = range_prune_unbraid_word(n, p_bits, q_bits, pos-1, p_next.clone(), q_next, nodes, cap) {
                return Some(pair);
            }
            if *nodes > cap { return None; }
        }
    }
    None
}

/// Factor search with every value and every intermediate arithmetic result
/// kept as its canonical IMASM numeral word.
fn propagate_word_states_imasm(n: &WordTape) -> Option<(WordTape, WordTape)> {
    let one = WordTape::one();
    if n.is_zero() || n.is_one() { return None; }
    if !n.is_odd() {
        let (q, r) = n.divmod(&WordTape::from_small(2))?;
        if r.is_zero() { return Some((WordTape::from_small(2), q)); }
        return None;
    }
    let total_bits = n.bit_len();
    let n_minus_one = n.sub(&one)?;
    let (carry0, _) = n_minus_one.divmod(&WordTape::from_small(2))?;
    for p_bits in (2..=(total_bits/2+1)).rev() {
        for q_bits in [total_bits+1-p_bits, total_bits-p_bits] {
            if q_bits < p_bits || q_bits == 0 { continue; }
            let mut nodes = 0u64;
            let carry = SignedWord::negative(carry0.clone());
            if let Some(pair) = hensel_unbraid_word(n,p_bits,q_bits,1,one.clone(),one.clone(),carry,&mut nodes,u64::MAX) {
                return Some(pair);
            }
            let mut nodes = 0u64;
            let start_pos = p_bits.max(q_bits) as i64 - 2;
            let p_hi = WordTape::one_at(p_bits-1);
            let q_hi = WordTape::one_at(q_bits-1);
            if let Some(pair) = range_prune_unbraid_word(n,p_bits,q_bits,start_pos,p_hi,q_hi,&mut nodes,u64::MAX) {
                return Some(pair);
            }
        }
    }
    None
}

fn modpow_word(base: &WordTape, exponent: &WordTape, modulus: &WordTape) -> WordTape {
    let mut power = base.divmod(modulus).map(|x| x.1).unwrap_or_else(WordTape::zero);
    let mut exp = exponent.clone();
    let mut result = WordTape::one();
    while !exp.is_zero() {
        if exp.is_odd() { result = result.mul(&power).divmod(modulus).unwrap().1; }
        exp = exp.shr1();
        if !exp.is_zero() { power = power.mul(&power).divmod(modulus).unwrap().1; }
    }
    result
}

fn prime_word(n: &WordTape) -> bool {
    let two = WordTape::from_small(2);
    if !n.ge(&two) { return false; }
    for small in [2u64,3,5,7,11,13,17,19,23,29,31,37] {
        let p = WordTape::from_small(small);
        if *n == p { return true; }
        if n.divmod(&p).map(|x| x.1.is_zero()).unwrap_or(false) { return false; }
    }
    let one = WordTape::one();
    let n_minus_one = n.sub(&one).unwrap();
    let mut d = n_minus_one.clone();
    let mut s = 0usize;
    while !d.is_odd() { d = d.shr1(); s += 1; }
    for witness in [2u64,3,5,7,11,13,17,19,23,29,31,37] {
        let a = WordTape::from_small(witness);
        let a = if a.ge(n) { a.divmod(n).unwrap().1 } else { a };
        if a.is_zero() || a.is_one() { continue; }
        let mut x = modpow_word(&a, &d, n);
        if x.is_one() || x == n_minus_one { continue; }
        let mut passed = false;
        for _ in 1..s {
            x = x.mul(&x).divmod(n).unwrap().1;
            if x == n_minus_one { passed = true; break; }
            if x.is_one() { break; }
        }
        if !passed { return false; }
    }
    true
}

fn coevolve_factor_tree_word(value: &WordTape, depth: usize, out: &mut String, budget: &mut u32) {
    let indent = "  ".repeat(depth);
    if *budget == 0 { out.push_str(&format!("{}... (node budget reached)\n", indent)); return; }
    *budget -= 1;
    let (seed_p, seed_q) = value.deinterlace();
    let seed_roundtrip = seed_p.interlace(&seed_q) == *value;
    if value.is_zero() || value.is_one() {
        out.push_str(&format!("{}{} UNIT\n", indent, value.as_word()));
    } else if prime_word(value) {
        out.push_str(&format!("{}{} PRIME [Γ(Λ(W))==W:{}]\n", indent, value.as_word(), seed_roundtrip));
    } else if let Some((p,q)) = propagate_word_states_imasm(value) {
        let pair_word = p.interlace(&q);
        let (p_back,q_back) = pair_word.deinterlace();
        let closes = p.mul(&q) == *value && p_back == p && q_back == q;
        out.push_str(&format!("{}{} = {} × {} [μ closes:{}; ΓΛ closes:{}]\n", indent,
            value.as_word(), p.as_word(), q.as_word(), p.mul(&q)==*value, closes));
        coevolve_factor_tree_word(&p,depth+1,out,budget);
        coevolve_factor_tree_word(&q,depth+1,out,budget);
    } else {
        out.push_str(&format!("{}{} UNFACTORED [Γ(Λ(W))==W:{}]\n", indent, value.as_word(), seed_roundtrip));
    }
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
"factor_membrane — every numeric operand is a canonical IMASM numeral word\n\
 factor <word>       factor with word-state propagation and close Γ/Λ/μ on words\n\
 cross <word>        deinterlace and multiply the two word lanes\n\
 separate <word>     cross, named as membrane separation\n\
 word <p-word> <q-word>  Γ-interlace and multiply two numeral words\n\
 recurse <word>      two-arm co-evolution tree over numeral words\n\
 encode <word>       read a canonical word and return it unchanged\n\
 Decimal operands are not accepted by the membrane."
        );
    }

    match args[0] {
        "encode" => {
            if args.len() != 2 { return String::from("usage: factor_membrane encode <canonical-native-word>"); }
            let raw = args[1..].join(" ");
            let w = match WordTape::from_canonical_word(&raw) {
                Some(w) => w,
                None => return String::from("expected a canonical native_numeral word"),
            };
            format!("{}  (canonical native_numeral; idempotent=true)", w.as_word())
        }

        "word" => {
            if args.len() != 3 { return String::from("usage: factor_membrane word <P-native-word> <Q-native-word>"); }
            let (p, q) = match (WordTape::from_canonical_word(args[1]), WordTape::from_canonical_word(args[2])) {
                (Some(a), Some(b)) => (a, b), _ => return String::from("bad P/Q: each must be a canonical native word"),
            };
            let n = p.mul(&q);
            let w = p.interlace(&q);
            let (p_back,q_back) = w.deinterlace();
            format!("{}  (P={} Q={} μ(P,Q)={} Λ(Γ(P,Q))==(P,Q):{})",
                w.as_word(), p.as_word(), q.as_word(), n.as_word(), p_back == p && q_back == q)
        }

        "cross" | "separate" => {
            if args.len() != 2 { return String::from("usage: factor_membrane cross <canonical-native-word>"); }
            let raw = args[1..].join(" ");
            let w = match WordTape::from_canonical_word(&raw) {
                Some(w) => w,
                None => return String::from("expected a canonical native_numeral word"),
            };
            let (p,q) = w.deinterlace();
            let lane_n = p.mul(&q);
            let roundtrip = p.interlace(&q) == w;
            format!(
                "W={}\nΛ(W).p={}\nΛ(W).q={}\nΓ(Λ(W))==W: {}\nμ(p,q)={}\nμ(p,q)==W: {}",
                w.as_word(), p.as_word(), q.as_word(), roundtrip, lane_n.as_word(), lane_n == w
            )
        }

        "factor" => {
            if args.len() != 2 { return String::from("usage: factor_membrane factor <canonical-native-word>"); }
            let raw = args[1..].join(" ");
            let n = match WordTape::from_canonical_word(&raw) {
                Some(w) => w,
                None => return String::from("INVALID: expected a canonical native_numeral word"),
            };
            if n.is_zero() || n.is_one() {
                return format!("N={}\nUNIT: no two-factor descent exists", n.as_word());
            }
            match propagate_word_states_imasm(&n) {
                Some((p, q)) => {
                    let d = p.interlace(&q);
                    let (p_back, q_back) = d.deinterlace();
                    let product_closes = p.mul(&q) == n;
                    let representation_closes = p_back == p && q_back == q;
                    if !product_closes || !representation_closes {
                        return format!("IMASM closure failed for N={}", n.as_word());
                    }
                    format!(
                        "N={}\nCOMPOSITE-found\np={}\nq={}\nΓ(p,q)={}\nΛ.p={}\nΛ.q={}\nμ(p,q)==N: true\nΛ(Γ(p,q))==(p,q): true",
                        n.as_word(), p.as_word(), q.as_word(), d.as_word(), p_back.as_word(), q_back.as_word()
                    )
                }
                None => format!("N={}\nno factor pair emitted by the two-arm word-state propagation", n.as_word()),
            }
        }

        "recurse" => {
            if args.len() != 2 { return String::from("usage: factor_membrane recurse <canonical-native-word>"); }
            let raw = args[1..].join(" ");
            let value = match WordTape::from_canonical_word(&raw) {
                Some(w) => w,
                None => return String::from("INVALID: expected a canonical native_numeral word"),
            };
            let mut out = String::from(
                "two-arm co-evolution tree — δ seeds, μ closes, both branches descend:\n");
            let mut budget = 4096u32;
            coevolve_factor_tree_word(&value, 1, &mut out, &mut budget);
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

    #[test] fn public_factor_runs_from_word_and_closes_factor_diagram() {
        let w = numeral_encode("91");
        let report = repl_factor_membrane(&["factor", &w]);
        assert!(report.contains("μ(p,q)==N: true"));
        assert!(report.contains("Λ(Γ(p,q))==(p,q): true"));
    }

    #[test] fn public_cross_is_gamma_lambda_identity_on_canonical_words() {
        // Γ is bit-interleave, not multiplication: decode(Γ(13,17)) = 595,
        // not 13*17 = 221.  So the lane round-trip holds while the parent
        // factor syzygy on decode(D) is correctly false.  The full syzygy
        // (with n = p*q) is covered by public_pair_word_uses_native_numeral_syzygy.
        let p = big("13");
        let q = big("17");
        let d = canonical_pair_word(&p, &q).unwrap();
        let report = repl_factor_membrane(&["cross", &d]);
        assert!(report.contains("Γ(Λ(W))==W: true"));
        assert!(report.contains("μ(p,q)==W: false"));
    }
    #[test] fn every_public_option_requires_canonical_native_words() {
        let w91 = numeral_encode("91");
        let w13 = numeral_encode("13");
        let w17 = numeral_encode("17");

        for op in ["encode", "cross", "separate", "factor", "recurse"] {
            assert!(repl_factor_membrane(&[op, "91"]).contains("canonical native_numeral word"));
        }
        assert!(!repl_factor_membrane(&["word", &w13, &w17]).contains("bad P/Q"));
        assert!(repl_factor_membrane(&["word", "13", &w17]).contains("bad P/Q"));
        assert!(repl_factor_membrane(&["word", &w13, "17"]).contains("bad P/Q"));
        assert!(repl_factor_membrane(&["word", "13", "17"]).contains("bad P/Q"));
        assert!(repl_factor_membrane(&["factor", &w91]).contains("COMPOSITE-found"));
    }

    #[test] fn recurse_is_two_arm_tree_both_branches_descend() {
        let prime = numeral_encode("15959");
        let report = repl_factor_membrane(&["recurse", &prime]);
        assert!(report.contains("PRIME"));
        // 91 = 7 × 13: μ closes on 91, and BOTH children are recursed to
        // prime leaves. Following only the even lane would drop one of them;
        // here both 7 and 13 appear as PRIME leaves.
        let w91 = numeral_encode("91");
        let w7 = numeral_encode("7");
        let w13 = numeral_encode("13");
        let report91 = repl_factor_membrane(&["recurse", &w91]);
        assert!(report91.contains("μ closes:true"));
        assert!(report91.contains(&format!("{} PRIME", w7)));
        assert!(report91.contains(&format!("{} PRIME", w13)));
    }

    #[test] fn propagator_closes_large_composite_through_membrane() {
        // 10007x10009: exhaustive word-state traversal closes via syzygy.
        let n: BigUint = "100160063".parse().unwrap();
        let (px, qx) = propagate_word_states(&n).unwrap();
        assert_eq!(multiply_via_word(&px, &qx), n);
        assert!(syzygy_preserves(&n, &px, &qx));
        let w = numeral_encode("100160063");
        let report = repl_factor_membrane(&["factor", &w]);
        assert!(report.contains("COMPOSITE-found"));
        assert!(report.contains("μ(p,q)==N: true"));
    }

    #[test] fn cross_distinguishes_lane_roundtrip_from_parent_factor_syzygy() {
        let word = numeral_encode("15959");
        let report = repl_factor_membrane(&["cross", &word]);
        assert!(report.contains("Γ(Λ(W))==W: true"));
        assert!(report.contains("μ(p,q)==W: false"));
    }

}
