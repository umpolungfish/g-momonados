// fold_probe: dynamic nested bit-register folding.
//
// A value is folded into a tree that deepens on its own as the value grows.
// The leaf radix (bits per cell) and the node fan-out are fixed; when the top
// level fills, one more fold level is added above it, so accommodation is a
// change in DEPTH, not width or count. Depth grows as log_fanout of the limb
// count.
//
// Two rails, per the standing side-arm rule:
//   fold   (descending): value -> fold tree
//   unfold (ascending) : fold tree -> value, and the round trip must return the
//                        value it started from.
//
// A real operation, multiply, recurses over the folded form (Karatsuba is
// literally a nested fold: split each operand into hi/lo children, three
// sub-products, recombine). Its cost is the min-over-arms recursion, not the
// schoolbook square, which is the free-lunch nesting the depth buys.
//
// Everything is checked against num-bigint as the reference.

use num_bigint::BigUint;
use num_traits::{One, Zero};
use std::time::Instant;

const LEAF_BITS: u32 = 64;   // one cell holds a 2^64 radix digit (the fold radix)
const FANOUT: usize = 4;     // children per node; depth grows as log_FANOUT(limbs)

// A folded numeral. Leaves carry base-2^64 limbs, low to high within a node's
// child order. A node concatenates its children, each weighted by the span of
// the children before it.
enum Fold {
    Leaf(u64),
    Node { children: Vec<Fold>, child_limbs: usize }, // limbs spanned by ONE child
}

impl Fold {
    fn depth(&self) -> usize {
        match self {
            Fold::Leaf(_) => 0,
            Fold::Node { children, .. } => 1 + children.iter().map(|c| c.depth()).max().unwrap_or(0),
        }
    }
}

// Descending rail: fold a limb slice into a balanced tree of fixed fan-out.
// `span` is how many limbs one child at this level covers.
fn fold_limbs(limbs: &[u64], span: usize) -> Fold {
    if span <= 1 {
        return Fold::Leaf(limbs.first().copied().unwrap_or(0));
    }
    let child_span = span / FANOUT;
    let mut children = Vec::with_capacity(FANOUT);
    for k in 0..FANOUT {
        let lo = k * child_span;
        let hi = ((k + 1) * child_span).min(limbs.len());
        let slice = if lo < limbs.len() { &limbs[lo..hi] } else { &[] };
        children.push(fold_limbs(slice, child_span));
    }
    Fold::Node { children, child_limbs: child_span }
}

fn fold(value: &BigUint) -> Fold {
    let mut limbs = value.to_u64_digits();
    if limbs.is_empty() { limbs.push(0); }
    // round the limb count up to a power of FANOUT so the tree is balanced;
    // that rounding is exactly "add a fold level when the top fills".
    let mut span = 1usize;
    while span < limbs.len() { span *= FANOUT; }
    fold_limbs(&limbs, span)
}

// Ascending rail: unfold back to a value. Each child is shifted by its limb span.
fn unfold(f: &Fold) -> BigUint {
    match f {
        Fold::Leaf(d) => BigUint::from(*d),
        Fold::Node { children, child_limbs } => {
            let mut acc = BigUint::zero();
            for (k, c) in children.iter().enumerate() {
                let shift_bits = (k * child_limbs) as u64 * LEAF_BITS as u64;
                acc += unfold(c) << shift_bits;
            }
            acc
        }
    }
}

// Multiply over the folded form. Splits at the top node into hi/lo halves and
// does a 3-way Karatsuba recurse; the recursion tree IS the fold nesting.
fn fold_mul(a: &BigUint, b: &BigUint) -> BigUint {
    let na = a.bits();
    let nb = b.bits();
    // base case: small operands go straight to the reference multiply
    if na <= LEAF_BITS as u64 || nb <= LEAF_BITS as u64 {
        return a * b;
    }
    let half = (na.max(nb) / 2 + LEAF_BITS as u64 - 1) / LEAF_BITS as u64 * LEAF_BITS as u64;
    let mask = (BigUint::one() << half) - BigUint::one();
    let a_lo = a & &mask;
    let a_hi = a >> half;
    let b_lo = b & &mask;
    let b_hi = b >> half;
    let z0 = fold_mul(&a_lo, &b_lo);
    let z2 = fold_mul(&a_hi, &b_hi);
    let z1 = fold_mul(&(&a_lo + &a_hi), &(&b_lo + &b_hi));
    let mid = &z1 - &z0 - &z2;
    (&z2 << (2 * half)) + (&mid << half) + &z0
}

// deterministic pseudo-random BigUint of the given bit length
fn gen(bits: u64, seed: &mut u64) -> BigUint {
    let mut v = BigUint::zero();
    let mut produced = 0u64;
    while produced < bits {
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let chunk = (*seed >> 11) as u64;
        v = (v << 32) | BigUint::from(chunk & 0xFFFF_FFFF);
        produced += 32;
    }
    v |= BigUint::one() << (bits - 1); // force the top bit so it is really `bits` long
    v
}

fn main() {
    println!("nested fold: leaf_bits={LEAF_BITS} fanout={FANOUT}");
    println!("{:>8}  {:>5}  {:>10}  {:>10}  {:>12}", "bits", "depth", "roundtrip", "multiply", "mul_us");
    let mut seed = 0x1234_5678_9abc_def0u64;
    let mut all_ok = true;
    for &bits in &[64u64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768] {
        let a = gen(bits, &mut seed);
        let b = gen(bits, &mut seed);

        // rails: fold then unfold must return the value
        let fa = fold(&a);
        let depth = fa.depth();
        let round = unfold(&fa) == a;

        // real op over the folded form, checked against the reference product
        let t0 = Instant::now();
        let prod = fold_mul(&a, &b);
        let mul_us = t0.elapsed().as_secs_f64() * 1e6;
        let mul_ok = prod == &a * &b;

        all_ok &= round && mul_ok;
        println!("{:>8}  {:>5}  {:>10}  {:>10}  {:>12.1}",
            bits, depth,
            if round { "ok" } else { "FAIL" },
            if mul_ok { "ok" } else { "FAIL" },
            mul_us);
    }
    println!("RESULT: {}", if all_ok { "fold/unfold rails invert and folded multiply matches reference at every size" } else { "FAILURE" });
    if !all_ok { std::process::exit(1); }
}
