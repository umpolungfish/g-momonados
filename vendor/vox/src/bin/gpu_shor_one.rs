//! One-tower gpu_shor membrane: membrane_one.sh shororder <a> <N>
//! All three order-finding arms of the tower in ONE payload, baked as IMASM
//! numerals on the tape carrier, PLUS the closing step that produces FACTORS:
//! order_tape closes within phi(n) <= n steps always; order_bsgs_tape takes
//! one unbounded tape-native baby table (m0 = isqrt(n)+1) and
//! giants walk to the first maximal-j collision, which reconstructs exactly
//! the minimal exponent; fde_step_tape is the dual-lane CPU mirror. No caps,
//! no refusals: a and N take any decimal length.
//! THE FACTOR CLOSE (what makes this Shor and not just period-finding): when
//! the order r is even and a^(r/2) is not -1 mod n, gcd(a^(r/2)-1, n) and
//! gcd(a^(r/2)+1, n) yield a nontrivial factor with probability >= 1/2 -- the
//! same closing step the original shor_qft membrane performs (report():
//! "factors recovered via gcd: p * q = N (verified: ...)"). All tape ops in
//! the close are already in the lift-proven set of this payload.
//! Hash-free AND reverse-free by contract: SipHash codegen emits SSE (movaps)
//! and in-place byte reverse autovectorizes into punpcklbw/pshuflw once the
//! decimal string grows -- both mis-decode in the x86 lift. Baby table is a
//! sorted Vec compared with cmp; digits are emitted through a scalar .rev()
//! iterator. Every code path stays lift-proven.

use ::vox::morphism_factor::{add, cmp, decimal_to_tape, divmod, gcd, mul, modulo, one, parse_numeral, sub, tape_u64, trim, zero};

fn eq(a: &[char], b: &[char]) -> bool { cmp(a, b) == core::cmp::Ordering::Equal }
fn tape0() -> Vec<char> { decimal_to_tape("0").unwrap() }

/// dec_of without the in-place reverse: scalar emission, lift-proven.
fn dec(t: &[char]) -> String {
    let b = trim(t.to_vec());
    if zero(&b) { return "0".into(); }
    let ten = tape_u64(10);
    let mut digits: Vec<u8> = Vec::new();
    let mut b = b;
    while !zero(&b) {
        let (q, r) = divmod(&b, &ten);
        let mut dv = 0u8;
        for (i, &c) in trim(r).iter().enumerate() {
            if c == ::vox::vox::EVALF { dv |= 1 << i; }
        }
        digits.push(b'0' + dv);
        b = q;
    }
    digits.into_iter().rev().map(|d| d as char).collect()
}

/// Tape exponentiation by square-and-multiply, exponent a tape (any size).
fn pow_tape(base: &[char], e_in: &[char], n: &[char]) -> Vec<char> {
    let two = tape_u64(2);
    let mut r = one();
    let mut b = modulo(base, n);
    let mut e = trim(e_in.to_vec());
    while !zero(&e) {
        let (q, rem) = divmod(&e, &two);
        if eq(&rem, &one()) { r = modulo(&mul(&r, &b), n); }
        b = modulo(&mul(&b, &b), n);
        e = q;
    }
    r
}

/// Uncapped orbit: closes within phi(n) <= n steps, counter a tape.
fn order_tape(a: &[char], n: &[char]) -> Option<Vec<char>> {
    if zero(n) { return None; }
    if eq(n, &one()) { return Some(tape0()); }
    if !eq(&gcd(a.to_vec(), n.to_vec()), &one()) { return Some(tape0()); }
    let mut state = modulo(a, n);
    let mut i = one();
    loop {
        if eq(&state, &one()) { return Some(trim(i)); }
        state = modulo(&mul(&state, a), n);
        i = add(&i, &one());
    }
}

/// Uncapped BSGS: baby table of m0 entries, giants to the first maximal-j
/// collision, which reconstructs exactly the minimal exponent.
fn order_bsgs_tape(a_t: &[char], n: &[char]) -> Option<Vec<char>> {
    if zero(n) { return None; }
    if eq(n, &one()) { return Some(tape0()); }
    if !eq(&gcd(a_t.to_vec(), n.to_vec()), &one()) { return Some(tape0()); }
    let a = modulo(a_t, n);
    if eq(&a, &one()) { return Some(one()); }
    // Unbounded tape-native BSGS: m0 = isqrt(n)+1 as a tape, baby-j
    // indices as tapes, giant walk to closure. No budget, no u64 parse, no
    // memory ceiling, no width bound on N anywhere: arbitrarily large
    // integers ingest through the same lane.
    let m0 = trim(add(&::vox::morphism_factor::isqrt(n), &one()));
    let mut table: Vec<(Vec<char>, Vec<char>)> = Vec::new();
    let mut cur = one();
    let mut j = tape_u64(0);
    while cmp(&j, &m0) == core::cmp::Ordering::Less {
        table.push((trim(cur.clone()), trim(j.clone())));
        cur = modulo(&mul(&cur, &a), n);
        j = add(&j, &one());
    }
    table.sort_by(|x, y| cmp(&x.0, &y.0));
    let step = trim(cur); // a^m0 mod n
    let mut giant = step.clone();
    let mut i = one();
    loop {
        if let Ok(mut idx) = table.binary_search_by(|probe| cmp(&probe.0, &giant)) {
            while idx + 1 < table.len() && table[idx + 1].0 == giant { idx += 1; }
            let j_max = table[idx].1.clone();
            let mut k = mul(&m0, &i);
            if !zero(&j_max) { k = sub(&k, &j_max); }
            if eq(&pow_tape(a_t, &k, n), &one()) { return Some(trim(k)); }
        }
        giant = modulo(&mul(&giant, &step), n);
        i = add(&i, &one());
    }
}

/// Dual-lane FDE step: forward and reverse lanes compute state*a mod n, the
/// reverse from pre-reduced operands; returned only when the lanes agree.
fn fde_step_tape(state: &[char], a: &[char], n: &[char]) -> Option<Vec<char>> {
    if zero(n) { return None; }
    let s = modulo(state, n);
    let forward = modulo(&mul(&s, a), n);
    let ar = modulo(a, n);
    let reverse = modulo(&mul(&s, &ar), n);
    if eq(&forward, &reverse) { Some(forward) } else { None }
}

/// THE FACTOR CLOSE. Given the order r of a mod n: when r is even and
/// a^(r/2) != -1 (mod n), one of gcd(a^(r/2) -/+ 1, n) is a nontrivial
/// factor with probability >= 1/2 (the standard Shor closing step; this is
/// the same step shor_qft::report performs on the statevector membrane).
/// Returns Some((p, q)) with p * q = n, or None with the honest reason.
fn factor_close(a: &[char], n: &[char], r: &[char]) -> (Option<(Vec<char>, Vec<char>)>, &'static str) {
    if zero(r) || eq(r, &one()) { return (None, "order is trivial (0 or 1) -- no close"); }
    let two = tape_u64(2);
    let (half, rem) = divmod(r, &two);
    if eq(&rem, &one()) { return (None, "order odd -- a^(r/2) undefined, retry with another base"); }
    let h = pow_tape(a, &half, n);
    let minus_one = sub(n, &one());
    if eq(&h, &minus_one) { return (None, "a^(r/2) = -1 (mod n) -- retry with another base"); }
    let h_minus = trim(sub(&h, &one()));
    let h_plus = trim(add(&h, &one()));
    let f1 = gcd(h_minus, n.to_vec());
    let f2 = gcd(h_plus, n.to_vec());
    for f in [f1, f2] {
        let f = trim(f);
        if !eq(&f, &one()) && !eq(&f, n) {
            let (q, rem0) = divmod(n, &f);
            if zero(&rem0) { return (Some((f, q)), "factors recovered"); }
        }
    }
    (None, "both closing gcds trivial for this base -- retry with another base")
}

fn run() -> Result<(), String> {
    let raw = option_env!("MEMBRANE_WORDS").unwrap_or("⊢⊙⊡⊣ ⊢⊙⊡⊣");
    let w: Vec<&str> = raw.split_whitespace().collect();
    if w.len() < 2 { return Err("gpu_shor membrane needs a N (any decimal length)".into()); }
    let a = parse_numeral(w[0])?;
    let n = parse_numeral(w[1])?;
    let (a_str, n_str) = (dec(&a), dec(&n));
    let o = order_tape(&a, &n);
    let b = order_bsgs_tape(&a, &n);
    println!("gpu_shor order: order of {} mod {} = {}", a_str, n_str,
        o.as_ref().map(|t| dec(t)).unwrap_or_else(|| "undefined (zero modulus)".into()));
    println!("gpu_shor bsgs: order of {} mod {} = {}", a_str, n_str,
        b.as_ref().map(|t| dec(t)).unwrap_or_else(|| "undefined (zero modulus)".into()));
    match (&o, &b) {
        (Some(x), Some(y)) if eq(x, y) => println!("tower cross-check: linear and bsgs lanes agree on {}", dec(x)),
        (Some(x), Some(y)) => println!("tower cross-check: DIVERGENCE linear={} bsgs={}", dec(x), dec(y)),
        _ => println!("tower cross-check: lanes incomplete (zero modulus)"),
    }
    // The closing step: order -> factors, the useful output of Shor's reduction.
    match &o {
        Some(r) => {
            let (factors, reason) = factor_close(&a, &n, r);
            match factors {
                Some((p, q)) => {
                    let prod = trim(mul(&p, &q));
                    println!("gpu_shor factor: factors recovered via gcd: {} * {} = {} (verified: {})",
                        dec(&p), dec(&q), dec(&prod), eq(&prod, &n));
                }
                None => println!("gpu_shor factor: no factors this base ({}); the order r = {} is still exact",
                    reason, dec(r)),
            }
        }
        None => println!("gpu_shor factor: zero modulus -- boundary undefined"),
    }
    match fde_step_tape(&a, &a, &n) {
        Some(r) => println!("gpu_shor fde_step: {}^2 mod {} = {} (forward == reverse)", a_str, n_str, dec(&r)),
        None => println!("gpu_shor fde_step: divergence or boundary failure"),
    }
    Ok(())
}

fn main() { if let Err(e) = run() { eprintln!("{e}"); std::process::exit(2); } }
