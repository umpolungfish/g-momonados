//! Fixed-word and arithmetic membranes over canonical IMASM numeral words.
//! Each arithmetic primitive delegates to `WordTape`, whose operands and
//! results remain canonical glyph words throughout the operation.

use core::cmp::Ordering;
pub use g_momonados::word_tape::WordTape as Big;
pub mod ecm;
pub mod order_cycle;

pub fn from_word(raw: &str) -> Option<Big> { Big::from_canonical_word(raw) }
pub fn from_u32(value: u32) -> Big { Big::from_small(value as u64) }
pub fn from_u64(value: u64) -> Big { Big::from_small(value) }
pub fn from_u128(value: u128) -> Big {
    Big::from_small((value >> 64) as u64).shl(64).add(&Big::from_small(value as u64))
}
pub fn to_word(value: &Big) -> &str { value.as_word() }
pub fn is_zero(value: &Big) -> bool { value.is_zero() }
pub fn cmp(a: &Big, b: &Big) -> Ordering {
    if a.ge(b) { if b.ge(a) { Ordering::Equal } else { Ordering::Greater } }
    else { Ordering::Less }
}
pub fn add(a: &Big, b: &Big) -> Big { a.add(b) }
pub fn sub(a: &Big, b: &Big) -> Big { a.sub(b).expect("word subtraction requires a >= b") }
pub fn mul(a: &Big, b: &Big) -> Big { a.mul(b) }
pub fn divmod(a: &Big, b: &Big) -> (Big, Big) {
    a.divmod(b).expect("word division requires a nonzero divisor")
}
pub fn isqrt(n: &Big) -> Big { n.isqrt() }

fn token_of(glyph: char) -> Option<u32> {
    Some(match glyph {
        '⊢' => 0, '⊣' => 1, '≻' => 2, '≺' => 3, '⋈' => 4, '⊙' => 5,
        '∈' => 6, '∋' => 7, '⊤' => 8, '⊥' => 9, '⊞' => 10, '⊡' => 11,
        _ => return None,
    })
}

struct Carrier {
    n: Big, lo: Big, hi: Big, a: Big, delta: Big, delta_a: Big, b: Big, square: bool,
    candidate: Option<(Big, Big)>, candidate_valid: bool,
    fixed: Option<(Big, Big)>, emitted: Option<(Big, Big)>,
}

fn leaf(op: u32, state: &mut Carrier) {
    match op {
        0 => { state.delta = Big::zero(); state.b = Big::zero(); state.square = false;
            state.candidate = None; state.candidate_valid = false;
            state.fixed = None; state.emitted = None; }
        2 => state.a = add(&state.a, &from_u32(1)),
        5 => {
            let square = mul(&state.a, &state.a);
            state.delta = if cmp(&square, &state.n) != Ordering::Less { sub(&square, &state.n) }
                else { Big::zero() };
            state.delta_a = state.a.clone();
        }
        3 => state.b = isqrt(&state.delta),
        9 => state.square = cmp(&mul(&state.b, &state.b), &state.delta) == Ordering::Equal,
        4 if state.square && state.candidate.is_none() => {
            let p = if cmp(&state.delta_a, &state.b) != Ordering::Less { sub(&state.delta_a, &state.b) }
                else { Big::zero() };
            let q = add(&state.delta_a, &state.b);
            state.candidate = Some((p, q));
            state.candidate_valid = false;
        }
        8 => if let Some((p, q)) = state.candidate.take() {
            if cmp(&p, &state.lo) != Ordering::Less && cmp(&q, &state.hi) != Ordering::Greater
                && cmp(&mul(&p, &q), &state.n) == Ordering::Equal {
                state.candidate = Some((p, q));
                state.candidate_valid = true;
            } else {
                state.candidate_valid = false;
            }
        },
        11 if state.candidate_valid => {
            state.fixed = state.candidate.take();
            state.candidate_valid = false;
        }
        1 => state.emitted = state.fixed.take(),
        _ => {}
    }
}

pub fn run(word: &str, n: &Big, depth: u32, max_steps: u64) -> (Option<(Big, Big)>, u64) {
    let ops: Vec<u32> = word.chars().filter_map(token_of).collect();
    if cmp(n, &from_u32(4)) == Ordering::Less { return (None, 0); }
    let lo = from_u32(2);
    let hi = divmod(n, &from_u32(2)).0;
    let a = isqrt(n);
    let mut state = Carrier { n: n.clone(), lo, hi, a: a.clone(), delta: Big::zero(), delta_a: a, b: Big::zero(),
        square: false, candidate: None, candidate_valid: false, fixed: None, emitted: None };
    let mut ticks = 0u64;
    for frame in 0..max_steps {
        for &op in &ops {
            if frame != 0 && op == 0 { continue; }
            ticks += 1;
            leaf(op, &mut state);
        }
        if let Some(pair) = state.emitted.take() { return (Some(pair), ticks); }
        if cmp(&state.a, &state.hi) == Ordering::Greater { break; }
    }
    let _ = depth;
    (None, ticks)
}

fn radix4_digit_options(width: usize, digit: usize) -> Vec<u32> {
    let digits = width.div_ceil(2);
    if digit >= digits { return vec![0]; }
    if digit == 0 { return vec![1, 3]; }
    if digit + 1 == digits {
        return if width % 2 == 1 { vec![2, 3] } else { vec![1, 2, 3] };
    }
    vec![0, 1, 2, 3]
}

fn radix4_lift(
    n: &Big,
    p_bits: usize,
    q_bits: usize,
    digit: usize,
    p: Big,
    q: Big,
    steps: &mut u64,
    cap: u64,
) -> Option<(Big, Big)> {
    *steps = steps.saturating_add(1);
    if *steps > cap { return None; }
    let digits = p_bits.max(q_bits).div_ceil(2);
    if digit == digits {
        if p.bit_len() == p_bits && q.bit_len() == q_bits
            && cmp(&p, &q) != Ordering::Greater && mul(&p, &q) == *n {
            return Some((p, q));
        }
        return None;
    }

    let step = Big::one_at(2 * digit);
    let prefix_bits = 2 * (digit + 1);
    let target = n.truncate(prefix_bits);
    let p_digits = radix4_digit_options(p_bits, digit);
    let q_digits = radix4_digit_options(q_bits, digit);
    for pd in p_digits {
        let p_next = add(&p, &mul(&from_u32(pd), &step));
        for &qd in &q_digits {
            let q_next = add(&q, &mul(&from_u32(qd), &step));
            let residue = mul(&p_next, &q_next).truncate(prefix_bits);
            if residue == target {
                if let Some(pair) = radix4_lift(n,p_bits,q_bits,digit+1,p_next.clone(),q_next,steps,cap) {
                    return Some(pair);
                }
                if *steps > cap { return None; }
            }
        }
    }
    None
}

/// Lift both odd factor words in base four, closing their product modulo each
/// successive two-bit prefix before advancing to the next digit.
pub fn factor_radix4(n: &Big, node_cap: u64) -> (Option<(Big, Big)>, u64) {
    if cmp(n, &from_u32(4)) == Ordering::Less { return (None, 0); }
    if !n.is_odd() {
        let (q, r) = divmod(n, &from_u32(2));
        if is_zero(&r) { return (Some((from_u32(2), q)), 1); }
    }
    let bits = n.bit_len();
    let mut nodes = 0u64;
    let balanced_p_bits = bits / 2;
    let balanced_probe_cap = (node_cap / 64).max(1).min(node_cap);
    for q_bits in [bits - balanced_p_bits, bits + 1 - balanced_p_bits] {
        if q_bits < balanced_p_bits { continue; }
        if let Some(pair) = radix4_lift(
            n,
            balanced_p_bits,
            q_bits,
            0,
            Big::zero(),
            Big::zero(),
            &mut nodes,
            balanced_probe_cap,
        ) {
            return (Some(pair), nodes);
        }
    }
    for p_bits in 2..=(bits / 2 + 1) {
        if p_bits == balanced_p_bits { continue; }
        for q_bits in [bits + 1 - p_bits, bits - p_bits] {
            if q_bits < p_bits || q_bits == 0 { continue; }
            if let Some(pair) = radix4_lift(
                n,p_bits,q_bits,0,Big::zero(),Big::zero(),&mut nodes,node_cap,
            ) {
                return (Some(pair), nodes);
            }
            if nodes > node_cap { return (None, nodes); }
        }
    }
    (None, nodes)
}

pub fn main_membrane(name: &str, word: &str) {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let steps = args.iter().position(|arg| arg == "steps")
        .and_then(|i| args.get(i + 1)).and_then(|s| s.parse().ok()).unwrap_or(2_000_000u64);
    let Some(raw) = args.first() else {
        eprintln!("usage: {name} <canonical-IMASM-numeral-word> [steps K]");
        std::process::exit(2);
    };
    let Some(n) = from_word(raw) else {
        eprintln!("{name}: expected a canonical IMASM numeral word");
        std::process::exit(2);
    };
    let (pair, ticks) = run(word, &n, 1, steps);
    let (pair, producer) = match pair {
        Some(pair) => (Some(pair), "fixed-word frontier"),
        None => (ecm::factor(&n, 5_000, 50_000, 100), "ECM continuation"),
    };
    println!("membrane {name} input bits={} operator marks={}", n.bit_len(), word.chars().count());
    match pair {
        Some((p, q)) => println!("producer={producer}\nfactor={}\ncofactor={}\nproduct_closes={}\nIMASM ticks={ticks}",
            to_word(&p), to_word(&q), mul(&p, &q) == n),
        None => println!("  no pair fixed within {steps} frontier steps\n  IMASM ticks={ticks}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_keeps_canonical_imasm_words() {
        let a = from_word("⊢≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋⊙⊡⊣").unwrap(); // 5
        let b = from_word("⊢≻⋈∈⊥∋≻⋈∈⊥∋⊙⊡⊣").unwrap(); // 3
        let product = mul(&a, &b);
        assert_eq!(to_word(&product), "⊢≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋⊙⊡⊣"); // 15
        let (q, r) = divmod(&product, &b);
        assert_eq!(q, a);
        assert!(r.is_zero());
        assert_eq!(isqrt(&from_u32(225)), from_u32(15));
        assert!(from_word("15").is_none());
    }
}
