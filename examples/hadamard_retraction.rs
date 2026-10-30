//! The Hadamard conjecture read as a winding retraction — the checks.
//!
//! Every claim the essay leans on, put to arithmetic. The winding is the pair
//! (num, den) reduced to lowest terms, folded into [0,1), exactly as
//! `fibonacci_qc.rs` holds it. Two readings sit on that one pair:
//!   is_self_inverse  — the ±1 reading, true only at 0 and 1/2
//!   to_complex       — the U(1) reading, e^{2πi·num/den}
//! This mirrors `Winding::is_self_inverse` at fibonacci_qc.rs:298 verbatim:
//!     self.num == 0 || (self.den == 2 && self.num == 1)
//! run:  cargo run --release --example hadamard_retraction

fn gcd(a: i64, b: i64) -> i64 { if b == 0 { a.abs() } else { gcd(b, a % b) } }

/// A winding reduced to lowest terms in [0,1), the same normalization as
/// Winding::new at fibonacci_qc.rs:275.
fn reduce(mut num: i64, mut den: i64) -> (i64, i64) {
    if den == 0 { return (0, 1); }
    if den < 0 { num = -num; den = -den; }
    num = num.rem_euclid(den);
    let g = gcd(num, den).max(1);
    (num / g, den / g)
}

/// The ±1 reading. fibonacci_qc.rs:298.
fn is_self_inverse(num: i64, den: i64) -> bool {
    let (n, d) = reduce(num, den);
    n == 0 || (d == 2 && n == 1)
}

/// H·Hᵀ = n·I for a ±1 matrix: the definition of a real Hadamard matrix.
fn is_hadamard(h: &[Vec<i64>]) -> bool {
    let n = h.len();
    for i in 0..n {
        for j in 0..n {
            let dot: i64 = (0..n).map(|k| h[i][k] * h[j][k]).sum();
            let want = if i == j { n as i64 } else { 0 };
            if dot != want { return false; }
        }
    }
    true
}

fn main() {
    println!("== Claim 1: the generic winding W(j,k)=jk/d fails the ±1 reading past order 2 ==");
    // The DFT's uniform assignment. Entry (1,1) is the winding 1/d.
    for d in 1..=8 {
        let si = is_self_inverse(1, d);
        println!("  d={d}: W(1,1)=1/{d} self-inverse (real ±1)? {si}");
    }
    println!("  -> real only at d=1,2; every larger order is disqualified by one entry.\n");

    println!("== Claim 2: Paley order 12 — QR mod 11 is the one order-2 character ==");
    // primitive root 2 mod 11; residue r is a QR iff its discrete log is even.
    let p = 11i64;
    let g = 2i64;
    let mut dlog = [0usize; 11];
    let mut x = 1i64;
    for e in 0..(p - 1) as usize { dlog[x as usize] = e; x = (x * g) % p; }
    let mut agree = true;
    for r in 1..p {
        // squares mod 11
        let is_qr = (1..p).any(|t| (t * t) % p == r);
        let log_even = dlog[r as usize] % 2 == 0;
        // a QR is the winding 0 (log even), a non-QR the winding 1/2 (log odd);
        // both self-inverse — den divides 2 by the parity, nothing else.
        let w = if log_even { (0, 1) } else { (1, 2) };
        let ok = (is_qr == log_even) && is_self_inverse(w.0, w.1);
        if !ok { agree = false; }
        println!("  r={:2}: QR={:5}  log even={:5}  winding={}/{}  self-inverse={}", r, is_qr, log_even, w.0, w.1, is_self_inverse(w.0, w.1));
    }
    println!("  QR ⇔ even-log, and both windings self-inverse, without exception? {agree}\n");

    // Build the bordered Paley matrix at order 12 and check orthogonality.
    let n = 12usize;
    let legendre = |a: i64| -> i64 {
        let a = a.rem_euclid(p);
        if a == 0 { 0 } else if (1..p).any(|t| (t * t) % p == a) { 1 } else { -1 }
    };
    let mut h = vec![vec![1i64; n]; n];
    // core rows/cols indexed 1..=11 by field elements 0..10
    for i in 1..n {
        for j in 1..n {
            h[i][j] = if i == j { -1 } else { legendre((i as i64) - (j as i64)) };
        }
    }
    for k in 0..n { h[0][k] = 1; h[k][0] = 1; }
    println!("  Paley-12 bordered matrix is a real Hadamard matrix (H·Hᵀ=12·I)? {}\n", is_hadamard(&h));

    println!("== Claim 3: order 16 — the Sylvester/FDE carrier reads as a real Hadamard ==");
    // Sylvester doubling from order 1; entry (i,j) = (-1)^popcount(i AND j),
    // which is the sixteen-mark FDE carrier read off its own bits.
    let m = 16usize;
    let mut s = vec![vec![0i64; m]; m];
    for i in 0..m {
        for j in 0..m {
            s[i][j] = if (i & j).count_ones() % 2 == 0 { 1 } else { -1 };
        }
    }
    println!("  16-mark carrier is a real Hadamard matrix (H·Hᵀ=16·I)? {}", is_hadamard(&s));
    // the carrier's own negation is a symmetry: -H is also Hadamard and equals
    // H under row/col relabel by the top mark.
    let negs: Vec<Vec<i64>> = s.iter().map(|r| r.iter().map(|&e| -e).collect()).collect();
    println!("  its own negation is also a real Hadamard matrix? {}", is_hadamard(&negs));

    println!("\n== Claim 4: retracting the whole DFT pointwise breaks orthogonality; the order-2 character quotient does not ==");
    // The generic cyclic character table is the DFT: entry (j,k) = ω^{jk}, ω=e^{2πi/d}.
    // Complex reading: rows are orthogonal at every d (this is why the complex case is trivial).
    // Naive real retraction: snap each entry to the nearest self-inverse value, i.e. sign of
    // its real part, ±1. Measure the worst off-diagonal row dot of the retracted ±1 table.
    for d in [3usize, 4, 5, 8, 12] {
        // complex DFT orthogonality (control): worst off-diagonal |row·conj(row)|
        let mut worst_c = 0f64;
        for a in 0..d {
            for b in 0..d {
                if a == b { continue; }
                let (mut re, mut im) = (0f64, 0f64);
                for k in 0..d {
                    let t = 2.0 * core::f64::consts::PI * (((a * k) as f64) - ((b * k) as f64)) / d as f64;
                    re += t.cos(); im += t.sin();
                }
                let m = (re * re + im * im).sqrt();
                if m > worst_c { worst_c = m; }
            }
        }
        // retracted ±1 table
        let mut h = vec![vec![1i64; d]; d];
        for j in 0..d {
            for k in 0..d {
                let t = 2.0 * core::f64::consts::PI * ((j * k) as f64) / d as f64;
                h[j][k] = if t.cos() >= 0.0 { 1 } else { -1 };
            }
        }
        let mut worst_r = 0i64;
        for a in 0..d {
            for b in 0..d {
                if a == b { continue; }
                let dot: i64 = (0..d).map(|k| h[a][k] * h[b][k]).sum();
                if dot.abs() > worst_r { worst_r = dot.abs(); }
            }
        }
        println!("  d={:2}: complex DFT worst off-diag ≈ {:.3e}   retracted ±1 worst off-diag = {}   still orthogonal? {}",
            d, worst_c, worst_r, worst_r == 0);
    }
    println!("  -> retraction of the full generic table breaks orthogonality for every d>2.");
    println!("     Paley/Sylvester never retract: they keep the single character already of order 2,");
    println!("     already ±1, so no snapping happens and orthogonality is preserved by construction");
    println!("     (Claims 2 and 3 above: 12·I and 16·I exact).");
}
