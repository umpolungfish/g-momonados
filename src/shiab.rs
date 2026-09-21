//! The SHIABO as an executable transform.
//!
//! SHIAB is a holographic scale-collapse. Give it a value and it acts: the ∈
//! mark is the split δ, the ∋ mark the stitch μ, and ⊙ is μ∘δ = id. On an
//! integer that is the even/odd bit-lane deinterleave — δ pulls the bulk apart
//! into its two boundary lanes and packs them as the boundary value, μ
//! reinterleaves them and returns the exact bulk. The ⊡ mark reads off the
//! integer winding. Give it an IMASM word instead and SHIAB wraps it as the
//! bulk inside the boundary ⊢⊙∈ … ⋈∋⊡⊣ and runs the collapse, returning the
//! transformed word and the register it settles in. Everything is computed at
//! call time.
use num_bigint::BigUint;
use num_traits::{Zero, One};
use crate::counterfactual::read;

/// The canonical SHIABO word.
pub const SHIAB_WORD: &str = "⊢⊙∈≻⊤≺⊥⊞⋈∋⊡⊣";

/// The twelve marks; presence of any of these routes input to the word arm.
const MARKS: &str = "⊢⊣≻≺⋈⊙∈∋⊤⊥⊞⊡";

/// Little-endian bit vector of n (length = max(1, bit length)).
fn bits_of(n: &BigUint) -> Vec<bool> {
    if n.is_zero() { return vec![false]; }
    let mut v = Vec::new();
    let mut m = n.clone();
    let two = BigUint::from(2u8);
    while !m.is_zero() {
        v.push((&m % &two) == BigUint::one());
        m /= &two;
    }
    v
}

/// Pack a little-endian bit slice back into a BigUint.
fn from_bits(bits: &[bool]) -> BigUint {
    let mut acc = BigUint::zero();
    for (i, b) in bits.iter().enumerate() {
        if *b { acc += BigUint::one() << i; }
    }
    acc
}

/// δ: split the bulk into even- and odd-indexed bit lanes and pack them, low
/// half the even lane, high half the odd lane. This is the boundary encoding.
fn delta(n: &BigUint) -> (BigUint, usize) {
    let bits = bits_of(n);
    let b = bits.len();
    let (mut even, mut odd) = (Vec::new(), Vec::new());
    for (i, bit) in bits.iter().enumerate() {
        if i % 2 == 0 { even.push(*bit) } else { odd.push(*bit) }
    }
    let half = even.len();
    let boundary = from_bits(&even) + (from_bits(&odd) << half);
    (boundary, b)
}

/// μ: reinterleave the two lanes of a boundary value back into the bulk. `b` is
/// the original bit length, so the split point is known.
fn mu(boundary: &BigUint, b: usize) -> BigUint {
    let half = b.div_ceil(2);                 // number of even positions
    let low_mask = (BigUint::one() << half) - BigUint::one();
    let e = boundary & &low_mask;
    let o = boundary >> half;
    let ebits = {
        let mut v = bits_of(&e); v.resize(half, false); v
    };
    let obits = {
        let mut v = bits_of(&o); v.resize(b - half, false); v
    };
    let mut out = vec![false; b];
    for i in 0..b {
        out[i] = if i % 2 == 0 { ebits[i / 2] } else { obits[i / 2] };
    }
    from_bits(&out)
}

/// The integer winding invariant: the Hamming weight, the number of set bits.
/// n ≠ 0 is a protected class, ∮_γ A = 2πn.
fn winding(n: &BigUint) -> usize {
    bits_of(n).iter().filter(|&&b| b).count()
}

fn transform_number(n: &BigUint) -> String {
    let (boundary, b) = delta(n);
    let recovered = mu(&boundary, b);
    let w = winding(n);
    let prot = if w != 0 { "protected (∮=2πn, n≠0)" } else { "trivial winding" };
    let mut s = String::new();
    s.push_str(&format!("SHIAB collapse of {}\n", n));
    s.push_str(&format!("  bulk N        {}\n", n));
    s.push_str(&format!("  δ boundary    {}   (even/odd bit lanes split and packed)\n", boundary));
    s.push_str(&format!("  μ(δ N)        {}   μ∘δ = id {}\n", recovered,
        if &recovered == n { "✓" } else { "✗ MISMATCH" }));
    s.push_str(&format!("  winding n     {}   {}\n", w, prot));
    s
}

/// Wrap an input word as the bulk inside the boundary ⊢⊙∈ … ⋈∋⊡⊣.
fn frame(input: &str) -> String {
    let mut s = String::from("⊢⊙∈");
    s.push_str(input);
    s.push_str("⋈∋⊡⊣");
    s
}

fn transform_word(input: &str) -> String {
    let framed = frame(input);
    let mut s = String::new();
    s.push_str(&format!("SHIAB collapse of word {}\n", input));
    s.push_str(&format!("  transformed   {}\n", framed));
    match (read(input), read(&framed)) {
        (Some(bi), Some(bo)) => {
            s.push_str(&format!("  input  reg {}  verdict {}  winding {}  holds {}\n",
                bi.register, bi.verdict, framed.matches('⊡').count() - 1, bi.holds));
            s.push_str(&format!("  output reg {}  verdict {} ({})  winding {}  holds {}\n",
                bo.register, bo.verdict, bo.verdict_why, framed.matches('⊡').count(), bo.holds));
        }
        _ => s.push_str("  (input word did not parse)\n"),
    }
    s
}

fn help() -> String {
    let mut s = String::new();
    s.push_str("SHIABO — holographic scale-collapse, run live\n");
    s.push_str("  shiab <N>            collapse an integer: δ boundary, μ∘δ=id recovery, winding\n");
    s.push_str("  shiab <imasm-word>   wrap a word as bulk in ⊢⊙∈…⋈∋⊡⊣ and run the collapse\n");
    s.push_str("  shiab run            the canonical operator on itself\n");
    s.push_str(&format!("  canonical word: {}\n", SHIAB_WORD));
    s
}

pub fn shiab_main(args: &[&str]) -> String {
    match args.first().copied() {
        None | Some("help") => help(),
        Some("run") => transform_word("≻⊤≺⊥⊞"),
        Some(tok) => {
            if let Ok(n) = tok.parse::<BigUint>() {
                transform_number(&n)
            } else if tok.chars().any(|c| MARKS.contains(c)) {
                transform_word(tok)
            } else {
                format!("shiab: '{}' is neither a number nor an IMASM word\n{}", tok, help())
            }
        }
    }
}
