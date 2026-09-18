//! membranes — one binary per factorizer membrane, nothing but the compute.
//!
//! A membrane is a word plus a register. Every token is a process; the math
//! register says what each process is over the numeral carrier. There is no
//! kernel, no BigUint, no host bignum here: the value is carried as its own
//! limbs (base 2^32, LSB first), which is the numeral's mark tape packed, and
//! every step is arithmetic on those limbs. Each membrane binary bakes in one
//! word and does exactly one thing: take N, run the word through this register,
//! emit the factors.
//!
//! Token discriminants (classic order): 0 ⊢ VINIT reset, 1 ⊣ TANCH emit,
//! 2 ≻ AFWD advance a, 3 ≺ AREV root of the gap, 4 ⋈ CLINK form (a-b,a+b),
//! 5 ⊙ IMSCRIB gap = a^2-N, 6 ∈ FSPLIT, 7 ∋ FFUSE, 8 ⊤ EVALT band-validate,
//! 9 ⊥ EVALF square test, 10 ⊞ ENGAGR, 11 ⊡ IFIX fix.

// ── the numeral: limbs base 2^32, LSB first, no trailing zeros ──────────────
pub type Big = Vec<u32>;

fn norm(v: &mut Big) { while v.last() == Some(&0) { v.pop(); } }
pub fn is_zero(a: &Big) -> bool { a.is_empty() }
pub fn from_u32(x: u32) -> Big { if x == 0 { vec![] } else { vec![x] } }

pub fn cmp(a: &Big, b: &Big) -> core::cmp::Ordering {
    use core::cmp::Ordering::*;
    if a.len() != b.len() { return if a.len() < b.len() { Less } else { Greater }; }
    for i in (0..a.len()).rev() {
        if a[i] != b[i] { return if a[i] < b[i] { Less } else { Greater }; }
    }
    Equal
}

pub fn add(a: &Big, b: &Big) -> Big {
    let mut out = Big::new();
    let mut carry = 0u64;
    for i in 0..a.len().max(b.len()) {
        let s = carry + *a.get(i).unwrap_or(&0) as u64 + *b.get(i).unwrap_or(&0) as u64;
        out.push(s as u32);
        carry = s >> 32;
    }
    if carry != 0 { out.push(carry as u32); }
    norm(&mut out); out
}

/// a - b, assuming a >= b.
pub fn sub(a: &Big, b: &Big) -> Big {
    let mut out = Big::new();
    let mut borrow = 0i64;
    for i in 0..a.len() {
        let d = a[i] as i64 - *b.get(i).unwrap_or(&0) as i64 - borrow;
        if d < 0 { out.push((d + (1i64 << 32)) as u32); borrow = 1; }
        else { out.push(d as u32); borrow = 0; }
    }
    norm(&mut out); out
}

pub fn mul(a: &Big, b: &Big) -> Big {
    if a.is_empty() || b.is_empty() { return Big::new(); }
    let mut out = vec![0u32; a.len() + b.len()];
    for i in 0..a.len() {
        let mut carry = 0u64;
        for j in 0..b.len() {
            let cur = out[i + j] as u64 + a[i] as u64 * b[j] as u64 + carry;
            out[i + j] = cur as u32;
            carry = cur >> 32;
        }
        out[i + b.len()] += carry as u32;
    }
    norm(&mut out); out
}

fn bit(a: &Big, i: usize) -> bool { (a.get(i >> 5).copied().unwrap_or(0) >> (i & 31)) & 1 == 1 }
fn bits(a: &Big) -> usize {
    if a.is_empty() { return 0; }
    (a.len() - 1) * 32 + (32 - a.last().unwrap().leading_zeros() as usize)
}
fn shl1_into(r: &mut Big, newbit: bool) {
    let mut carry = newbit as u64;
    for limb in r.iter_mut() {
        let cur = (*limb as u64) << 1 | carry;
        *limb = cur as u32;
        carry = cur >> 32;
    }
    if carry != 0 { r.push(carry as u32); }
    norm(r);
}

/// (quotient, remainder) by binary long division. b must be nonzero.
pub fn divmod(a: &Big, b: &Big) -> (Big, Big) {
    let mut q = vec![0u32; a.len().max(1)];
    let mut r = Big::new();
    for i in (0..bits(a)).rev() {
        shl1_into(&mut r, bit(a, i));
        if cmp(&r, b) != core::cmp::Ordering::Less {
            r = sub(&r, b);
            q[i >> 5] |= 1u32 << (i & 31);
        }
    }
    norm(&mut q); (q, r)
}

pub fn isqrt(n: &Big) -> Big {
    if is_zero(n) { return Big::new(); }
    let mut x = { let mut v = vec![0u32; (bits(n) + 1) / 2 / 32 + 1]; let b = (bits(n) + 1) / 2; v[b >> 5] |= 1u32 << (b & 31); norm(&mut v); v };
    loop {
        let (nx, _) = divmod(n, &x);
        let sum = add(&x, &nx);
        let (y, _) = divmod(&sum, &from_u32(2));
        if cmp(&y, &x) != core::cmp::Ordering::Less { return x; }
        x = y;
    }
}

pub fn from_dec(s: &str) -> Big {
    let mut v = Big::new();
    let ten = from_u32(10);
    for c in s.trim().bytes() {
        if !c.is_ascii_digit() { continue; }
        v = add(&mul(&v, &ten), &from_u32((c - b'0') as u32));
    }
    v
}

pub fn to_dec(a: &Big) -> String {
    if a.is_empty() { return "0".into(); }
    let mut digits = Vec::new();
    let mut cur = a.clone();
    let bil = from_u32(1_000_000_000);
    while !is_zero(&cur) {
        let (q, r) = divmod(&cur, &bil);
        let chunk = if r.is_empty() { 0 } else { r[0] };
        digits.push(chunk);
        cur = q;
    }
    let mut out = digits.pop().unwrap().to_string();
    while let Some(d) = digits.pop() { out.push_str(&format!("{:09}", d)); }
    out
}

// ── the math register: token -> process over the carrier ────────────────────
struct Carrier {
    n: Big, lo: Big, hi: Big,
    a: Big, delta: Big, b: Big, square: bool,
    candidate: Option<(Big, Big)>, fixed: Option<(Big, Big)>, emitted: Option<(Big, Big)>,
}

fn leaf(op: u32, s: &mut Carrier) {
    match op {
        0 => { s.delta = Big::new(); s.b = Big::new(); s.square = false;
               s.candidate = None; s.fixed = None; s.emitted = None; }
        2 => s.a = add(&s.a, &from_u32(1)),
        5 => { let aa = mul(&s.a, &s.a);
               s.delta = if cmp(&aa, &s.n) != core::cmp::Ordering::Less { sub(&aa, &s.n) } else { Big::new() }; }
        3 => s.b = isqrt(&s.delta),
        9 => s.square = cmp(&mul(&s.b, &s.b), &s.delta) == core::cmp::Ordering::Equal,
        4 if s.square && s.candidate.is_none() => {
            let p = if cmp(&s.a, &s.b) != core::cmp::Ordering::Less { sub(&s.a, &s.b) } else { Big::new() };
            let q = add(&s.a, &s.b);
            s.candidate = Some((p, q));
        }
        8 => if let Some((p, q)) = s.candidate.take() {
                if cmp(&p, &s.lo) != core::cmp::Ordering::Less
                    && cmp(&q, &s.hi) != core::cmp::Ordering::Greater
                    && cmp(&mul(&p, &q), &s.n) == core::cmp::Ordering::Equal {
                    s.candidate = Some((p, q));
                }
             },
        11 => s.fixed = s.candidate.take(),
        1 => s.emitted = s.fixed.take(),
        _ => {}
    }
}

/// The one-composition zoom fixed point: at any nesting depth the mark comes
/// back unchanged, and one tick is spent. This is the collapse, in code.
#[inline(always)]
fn nested_emit(mark: u32, _depth: u32, ticks: &mut u64) -> u32 { *ticks += 1; mark }

pub fn token_of(g: char) -> Option<u32> {
    Some(match g {
        '⊢' => 0, '⊣' => 1, '≻' => 2, '≺' => 3, '⋈' => 4, '⊙' => 5,
        '∈' => 6, '∋' => 7, '⊤' => 8, '⊥' => 9, '⊞' => 10, '⊡' => 11,
        _ => return None,
    })
}

/// Run one membrane word on N in the math register at a nesting depth.
pub fn run(word: &str, n: &Big, depth: u32, max_steps: u64) -> (Option<(Big, Big)>, u64) {
    let ops: Vec<u32> = word.chars().filter_map(token_of).collect();
    if cmp(n, &from_u32(4)) == core::cmp::Ordering::Less { return (None, 0); }
    let lo = from_u32(2);
    let (hi, _) = divmod(n, &from_u32(2));
    let mut a = isqrt(n);
    if cmp(&mul(&a, &a), n) == core::cmp::Ordering::Less { a = add(&a, &from_u32(1)); }
    let mut c = Carrier { n: n.clone(), lo, hi, a, delta: Big::new(), b: Big::new(),
        square: false, candidate: None, fixed: None, emitted: None };
    let mut ticks = 0u64;
    for _ in 0..max_steps {
        for &op in &ops { let m = nested_emit(op, depth, &mut ticks); leaf(m, &mut c); }
        if let Some(pq) = c.emitted.take() { return (Some(pq), ticks); }
        if cmp(&c.a, &c.hi) == core::cmp::Ordering::Greater { break; }
    }
    (None, ticks)
}

/// The whole binary for one membrane: bake in NAME and WORD, read N from argv,
/// run the word, emit the factors. Nothing else.
pub fn main_membrane(name: &str, word: &str) {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let depth: u32 = args.iter().position(|a| a == "depth")
        .and_then(|i| args.get(i + 1)).and_then(|s| s.parse().ok()).unwrap_or(64);
    let steps: u64 = args.iter().position(|a| a == "steps")
        .and_then(|i| args.get(i + 1)).and_then(|s| s.parse().ok()).unwrap_or(2_000_000);
    let Some(nstr) = args.first() else {
        eprintln!("usage: {name} <N> [depth D] [steps K]"); std::process::exit(2);
    };
    let n = from_dec(nstr);
    let (pq, ticks) = run(word, &n, depth, steps);
    println!("membrane {name}  word {word}");
    println!("  N={}", to_dec(&n));
    match pq {
        Some((p, q)) => {
            let ok = cmp(&mul(&p, &q), &n) == core::cmp::Ordering::Equal;
            println!("  {} = {} x {}  (verified={ok})", to_dec(&n), to_dec(&p), to_dec(&q));
            println!("  IMASM nesting depth={depth}; nested mark ticks={ticks}");
        }
        None => {
            println!("  no pair fixed in {steps} frontier steps in the math register");
            println!("  IMASM nesting depth={depth}; nested mark ticks={ticks}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn arithmetic_roundtrip() {
        let a = from_dec("1329227995784918259451321596453652117");
        assert_eq!(to_dec(&a), "1329227995784918259451321596453652117");
        let r = isqrt(&a);
        assert!(cmp(&mul(&r, &r), &a) != core::cmp::Ordering::Greater);
        assert!(cmp(&mul(&add(&r, &from_u32(1)), &add(&r, &from_u32(1))), &a) == core::cmp::Ordering::Greater);
    }
    #[test] fn aggregate_factors_balanced() {
        // 1152921504606847009 x 1152921504606847067 close pair
        let n = mul(&from_dec("1152921504606847009"), &from_dec("1152921504606847067"));
        let (pq, _) = run("⊢≻⋈⊙∈⊤≻⋈⊥≺⋈⊞∋⊡⋈⊙⊣", &n, 64, 100_000);
        let (p, q) = pq.expect("balanced pair must close");
        assert_eq!(cmp(&mul(&p, &q), &n), core::cmp::Ordering::Equal);
    }
}
