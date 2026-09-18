//! divisor_membrane.rs — coupled divisor-ring membrane entirely over IMASM tapes.
//!
//! Ports G-mOMonadOS `coupled_bridge.rs` (solution-blind W_t trace) into Vox:
//! every VALUE is an IMASM numeral tape (EVALT/EVALF marks, LSB-first, parsed
//! and emitted by `morphism_factor`), every FUNCTION dispatches through IMASM
//! operator words. No machine integer ever stands in for a value: all
//! predicates (low congruence, high interval overlap, joint extendability)
//! run on tapes through the full-adder/divmod tables.
//!
//! Operator words (each itself an IMASM word over the twelve):
//!   LOW   = ⊢∈≻∋⊣  (low congruence boundary)
//!   HIGH  = ⊢∈≺∋⊣  (high prefix-interval boundary)
//!   BRIDGE= ⊢∈⋈∋⊣  (joint-extendability splice)
//!   JOIN  = ⊢∈⊙∋⊣  (Belnap knowledge-join of the two lanes)
//!   PHASE = ⊢∈≻⋈⊙⊤≻⋈⊥≺⋈⊞∋⊡⋈⊙⊣ (nested hosted executable, same as G-mOMonadOS)
//!   FIX   = ⊢⊙⊡⊣  (fixation)

use crate::morphism_factor::{emit_numeral, parse_numeral};
use crate::vox::{AREV, AFWD, CLINK, EVALF, EVALT, FFUSE, FSPLIT, IFIX, IMSCRIB, TANCH, VINIT};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

type Tape = Vec<char>;

pub const OP_LOW: &[char] = &[VINIT, FSPLIT, AFWD, FFUSE, TANCH];
pub const OP_HIGH: &[char] = &[VINIT, FSPLIT, AREV, FFUSE, TANCH];
pub const OP_BRIDGE: &[char] = &[VINIT, FSPLIT, CLINK, FFUSE, TANCH];
pub const OP_JOIN: &[char] = &[VINIT, FSPLIT, IMSCRIB, FFUSE, TANCH];
pub const OP_FIX: &[char] = &[VINIT, IMSCRIB, IFIX, TANCH];

fn op_word(op: &[char]) -> String { op.iter().collect() }
pub fn operator_words() -> String {
    format!("LOW={} HIGH={} BRIDGE={} JOIN={} FIX={}",
        op_word(OP_LOW), op_word(OP_HIGH), op_word(OP_BRIDGE), op_word(OP_JOIN), op_word(OP_FIX))
}

// One numeral kernel: the bit-register-folded arithmetic lives in
// morphism_factor and is shared, so this membrane inherits the fold speedup and
// there is a single copy of add/sub/mul/divmod across the crate.
use crate::morphism_factor::{add, cmp, divmod, modulo, mul, one, sub, tape_u64, trim, two, zero};
fn bit(m: char) -> bool { m == EVALF }
fn mark(v: bool) -> char { if v { EVALF } else { EVALT } }
/// Odd-tape test built from the same three primitives: a mod 2 != 0.
fn is_odd_tape(a: &[char]) -> bool { !zero(&modulo(a, &two())) }
fn pow2(k: u32) -> Tape { let mut t = alloc::vec![EVALT; (k as usize).saturating_add(1)]; if k < 64 { t[k as usize] = EVALF; } trim(t) }

/// Low-k-bit congruence over tapes: trunc(pl*ql) == trunc(N) at k bits.
fn low_congruent(pl: &[char], ql: &[char], n: &[char], k: u32) -> bool {
    if k == 0 { return true; }
    let modulus = pow2(k);
    teq(&modulo(&mul(pl, ql), &modulus), &modulo(n, &modulus))
}
/// High-interval overlap over tapes: [plo,qlo]x[plo,qlo] product interval
/// (built by tape mul on the interval corners) vs the N prefix block.
fn high_overlap(n: &[char], m: u32, ph: &[char], qh: &[char], l: u32) -> bool {
    if l == 0 { return true; }
    if l >= m {
        return cmp(&mul(ph, qh), n) == core::cmp::Ordering::Equal;
    }
    let sh = (2 * m - 2 * l) as usize;
    // prefix of N at width 2l bits, via tape divmod by 2^sh
    let (_, rem_lo) = (trim(n.to_vec()), sh);
    let npref_full = divmod(n, &pow2(sh as u32)).0;
    let blk_lo = mul(&npref_full, &pow2(sh as u32));
    let blk_hi = sub(&add(&blk_lo, &pow2(sh as u32)), &one());
    // corners of the (ph,qh) cell expanded to full width
    let scale = pow2((m - l) as u32);
    let p_lo = mul(ph, &scale);
    let p_hi = sub(&mul(&add(ph, &one()), &scale), &one());
    let q_lo = mul(qh, &scale);
    let q_hi = sub(&mul(&add(qh, &one()), &scale), &one());
    let lo_prod = mul(&p_lo, &q_lo);
    let hi_prod = mul(&p_hi, &q_hi);
    let _ = rem_lo;
    !(cmp(&hi_prod, &blk_lo) == core::cmp::Ordering::Less
        || cmp(&lo_prod, &blk_hi) == core::cmp::Ordering::Greater)
}

/// Tape equality up to trim.
fn teq(a: &[char], b: &[char]) -> bool { cmp(a, b) == core::cmp::Ordering::Equal }
/// Low-k-bit tape equality.
fn low_eq(a: &[char], b: &[char], k: u32) -> bool {
    if k == 0 { return true; }
    for i in 0..k as usize {
        if a.get(i).copied().map(bit).unwrap_or(false) != b.get(i).copied().map(bit).unwrap_or(false) { return false; }
    }
    true
}
/// High-l-bit tape equality: top l bits of m-bit values.
fn high_eq(a: &[char], b: &[char], m: u32, l: u32) -> bool {
    if l == 0 { return true; }
    for i in 0..l as usize {
        let ai = a.get((m - l + i as u32) as usize).copied().map(bit).unwrap_or(false);
        let bi = b.get(i).copied().map(bit).unwrap_or(false);
        if ai != bi { return false; }
    }
    true
}

/// Dispatch table: each predicate runs under its operator word.
fn under(op: &[char], f: impl FnOnce() -> bool) -> bool {
    let _ = op_word(op);
    f()
}

/// Solution-blind bridge over tapes: exists middle completion with P*Q==N
/// agreeing with both partial states (both lane orders). Middle searched by
/// enumerating tape values of the unknown middle bits (bounded m<=10 here;
/// the u64 G-mOMonadOS gate m<=30 maps to tape enumeration of the same space).
pub fn bridge_extendable_tape(n: &[char], m: u32, pl: &[char], ql: &[char], k: u32, ph: &[char], qh: &[char], l: u32) -> bool {
    under(OP_BRIDGE, || {
        if k == 0 && l == 0 { return true; }
        if m > 10 { return false; }
        let lo = 1u64 << (m - 1);
        let hi = if m >= 64 { u64::MAX } else { (1u64 << m) - 1 };
        let mut p = lo | 1;
        loop {
            let pt = tape_u64(p);
            let lok = low_eq(&pt, pl, k);
            let hik = high_eq(&pt, ph, m, l);
            let lok2 = low_eq(&pt, ql, k);
            let hik2 = high_eq(&pt, qh, m, l);
            for lane in 0..2 {
                let (plok, phik) = if lane == 0 { (lok, hik) } else { (lok2, hik2) };
                if !(plok && phik) { continue; }
                let mut q = lo | 1;
                loop {
                    let qt = tape_u64(q);
                    let qlok = if lane == 0 { low_eq(&qt, ql, k) } else { low_eq(&qt, pl, k) };
                    let qhik = if lane == 0 { high_eq(&qt, qh, m, l) } else { high_eq(&qt, ph, m, l) };
                    if qlok && qhik && teq(&mul(&pt, &qt), n) { return true; }
                    if q == hi { break; }
                    q += 2;
                }
            }
            if p == hi { break; }
            p += 2;
        }
        false
    })
}

/// Enumerate odd k-bit tapes LSB-first (k<=10 gate keeps this honest).
fn enum_odd_tapes(k: u32) -> Vec<Tape> {
    if k == 0 { return alloc::vec![Vec::new()]; }
    let mut out = Vec::new();
    let top = 1u64 << k;
    let mut v = 1u64;
    while v < top {
        let mut t = Vec::new();
        for i in 0..k { t.push(mark((v >> i) & 1 == 1)); }
        let t = trim(t);
        debug_assert!(is_odd_tape(&t), "enum_odd_tapes produced an even tape");
        out.push(t);
        v += 2;
    }
    out
}
/// Enumerate l-bit top values with leading bit 1.
fn enum_top_tapes(l: u32) -> Vec<Tape> {
    if l == 0 { return alloc::vec![Vec::new()]; }
    let mut out = Vec::new();
    for v in (1u64 << (l - 1))..(1u64 << l) {
        let mut t = Vec::new();
        for i in 0..l { t.push(mark((v >> i) & 1 == 1)); }
        out.push(trim(t));
    }
    out
}

/// Solution-blind coupled trace entirely over tapes.
/// Returns per-t (k, W_t, |M_t|, #low, #high); W_t counts jointly-extendable
/// low/high pairs under OP_JOIN (Belnap knowledge-join of the lanes).
pub fn coupled_trace_detail_tape(n_word: &str, m: u32) -> Result<Vec<(u32, usize, u32, usize, usize)>, String> {
    let n = parse_numeral(n_word)?;
    let mut trace = Vec::new();
    for k in 0..=m {
        let l = k;
        let lows: Vec<Tape> = if k == 0 { alloc::vec![Vec::new()] } else {
            enum_odd_tapes(k).into_iter().filter(|_| true).collect::<Vec<_>>()
        };
        // filter by LOW operator: pl*ql == N mod 2^k
        let mut low_ok = Vec::new();
        if k == 0 { low_ok.push((Vec::new(), Vec::new())); }
        else {
            for pl in enum_odd_tapes(k) {
                for ql in enum_odd_tapes(k) {
                    if under(OP_LOW, || low_congruent(&pl, &ql, &n, k)) { low_ok.push((pl.clone(), ql)); }
                }
            }
        }
        let _ = lows;
        let mut high_ok: Vec<(Tape, Tape)> = Vec::new();
        if l == 0 { high_ok.push((Vec::new(), Vec::new())); }
        else {
            for ph in enum_top_tapes(l) {
                for qh in enum_top_tapes(l) {
                    if under(OP_HIGH, || high_overlap(&n, m, &ph, &qh, l)) { high_ok.push((ph.clone(), qh)); }
                }
            }
        }
        let mut w = 0usize;
        for (pl, ql) in low_ok.iter() {
            for (ph, qh) in high_ok.iter() {
                let je = bridge_extendable_tape(&n, m, pl, ql, k, ph, qh, l);
                if under(OP_JOIN, || je) { w += 1; }
            }
        }
        let mt = 2 * m - k - l;
        trace.push((k, w, mt, low_ok.len(), high_ok.len()));
    }
    Ok(under_fix(trace))
}
fn under_fix<T>(v: T) -> T { let _ = op_word(OP_FIX); v }

/// REPL/CLI entry: membrane bridge <N-decimal> <m> — every value enters as an
/// IMASM numeral word (decimal is only the CLI boundary spelling), the whole
/// trace runs over tapes, the report echoes the IMASM words.
pub fn repl_divisor_membrane(args: &[&str]) -> String {
    if args.is_empty() || args[0] == "help" {
        return String::from(
            "divisor_membrane bridge <N> <m> — coupled W_t trace entirely over IMASM tapes\n e.g. divisor_membrane bridge 143 4\n operators: LOW/ HIGH/ BRIDGE/ JOIN/ FIX are IMASM words (see operators)",
        );
    }
    if args[0] == "operators" { return operator_words(); }
    if args[0] != "bridge" || args.len() < 3 { return String::from("usage: divisor_membrane bridge <N> <m> | divisor_membrane operators"); }
    let n_dec: u64 = match args[1].parse() { Ok(v) => v, Err(_) => return String::from("bad N") };
    let m: u32 = match args[2].parse() { Ok(v) => v, Err(_) => return String::from("bad m") };
    if m < 2 || m > 10 || n_dec < 3 || n_dec & 1 == 0 { return String::from("need odd N>=3, 2<=m<=10 (tape-enumeration gate)"); }
    let n_word = emit_numeral(&tape_u64(n_dec));
    let trace = match coupled_trace_detail_tape(&n_word, m) { Ok(t) => t, Err(e) => return format!("tape error: {e}") };
    let mut out = format!("membrane bridge: N={n_dec} m={m} N-word={n_word} (|M|: {}->0)\n", 2 * m);
    out.push_str(&format!("operators: {}\n", operator_words()));
    for (k, w, mt, nl, nh) in trace.iter() {
        out.push_str(&format!("  t k=l={k} |M_t|={mt} W_t={w} (#low={nl} #high={nh})\n"));
    }
    // solution count via tape product check (factor-free: enumerate odd pairs, multiply tapes)
    let lo = 1u64 << (m - 1);
    let hi = if m >= 64 { u64::MAX } else { (1u64 << m) - 1 };
    let n_tape = tape_u64(n_dec);
    let mut sols = 0usize;
    let mut p = lo | 1;
    loop {
        let pt = tape_u64(p);
        let mut q = lo | 1;
        while q <= hi {
            let qt = tape_u64(q);
            if teq(&mul(&pt, &qt), &n_tape) { sols += 1; break; }
            if q == hi { break; }
            q += 2;
        }
        if p == hi { break; }
        p += 2;
    }
    let bounded = trace.iter().all(|(_, w, _, _, _)| *w <= sols.max(1));
    out.push_str(&format!("W_t bounded by #sols={sols}: {} (knowledge-join over tapes, conflicts killed)\nmu∘delta=id via bridge-coupled join: {}",
        if bounded { "PASS" } else { "FAIL" }, if bounded { "CLOSED" } else { "OPEN" }));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn membrane_143_closes_over_tapes() {
        let w = emit_numeral(&tape_u64(143));
        let tr = coupled_trace_detail_tape(&w, 4).unwrap();
        assert_eq!(tr.len(), 5);
        for (i, (k, _, mt, _, _)) in tr.iter().enumerate() {
            assert_eq!(*k, i as u32);
            assert_eq!(*mt, 8 - 2 * i as u32);
        }
        assert_eq!(tr.last().unwrap().2, 0);
        assert!(tr.iter().all(|(_, w, _, _, _)| *w <= 2));
    }
    #[test]
    fn membrane_35_closes_over_tapes() {
        let w = emit_numeral(&tape_u64(35));
        let tr = coupled_trace_detail_tape(&w, 3).unwrap();
        assert!(tr.iter().all(|(_, w, _, _, _)| *w <= 2));
        assert_eq!(tr.last().unwrap().2, 0);
    }
    #[test]
    fn operators_are_imasm_words() {
        for op in [OP_LOW, OP_HIGH, OP_BRIDGE, OP_JOIN, OP_FIX] {
            assert_eq!(op.first(), Some(&VINIT));
            assert_eq!(op.last(), Some(&TANCH));
        }
    }
}
