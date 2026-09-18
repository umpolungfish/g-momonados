//! gpu_shor.rs — classical period (multiplicative order) finding on the GPU.
//!
//! The order of a modulo n is the least r with a^r == 1 (mod n), the period
//! Shor's algorithm extracts. The sequential scan multiplies by a until it
//! returns to 1, a dependent chain. This splits the range into chunks: a thread
//! jumps to the start of its chunk with one modular exponentiation, walks the
//! chunk multiplying, and reports any r that hits 1 by an atomic minimum, so the
//! true order (the smallest such r) wins. The range is swept in windows so no
//! one launch runs long enough to trip the display-driver watchdog.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use cudarc::driver::{CudaContext, LaunchConfig, PushKernelArg};
use cudarc::nvrtc::{compile_ptx_with_opts, CompileOptions};

const KERNEL_SRC: &str = r#"
typedef unsigned long long u64;
// NVRTC ships no host headers: the stdint types are not defined. u64 above
// was already the pattern; u32 and u8 follow it.
typedef unsigned int u32;
typedef unsigned char u8;

// (a+b) mod n for a,b already < n -- overflow-safe without a wider integer
// type, the same "carry, don't widen" discipline bits_add/bits_subtract use.
__device__ u64 addmod(u64 a, u64 b, u64 n){
    u64 s = a + b;
    if (s < a || s >= n) s -= n;
    return s;
}

// a mod n by long division, walking a's bits from the top down: double the
// running remainder and bring in the next bit, exactly bits_divmod's own
// walk (there over whole limbs, here over the 64 bits of one). This is what
// replaces the hardware %/__int128 reduction the original kernel used.
__device__ u64 modn(u64 a, u64 n){
    u64 rem = 0;
    for (int i = 63; i >= 0; i--){
        rem = addmod(rem, rem, n);
        if ((a >> i) & 1) rem = addmod(rem, 1, n);
    }
    return rem;
}

// a*b mod n by shift-and-add: walk b's bits low to high, doubling a (mod n)
// each step and adding it in wherever that bit of b is set -- the same
// shape as bits_multiply, replacing the __int128 widen-then-reduce the
// original kernel used.
__device__ u64 mulm(u64 a, u64 b, u64 n){
    u64 result = 0;
    a = modn(a, n);
    while (b){
        if (b & 1) result = addmod(result, a, n);
        a = addmod(a, a, n);
        b >>= 1;
    }
    return result;
}
__device__ u64 powm(u64 a, u64 e, u64 n){ u64 r=modn(1,n); a=modn(a,n); while(e){ if(e&1) r=mulm(r,a,n); a=mulm(a,a,n); e>>=1; } return r; }
extern "C" __global__ void order_search(u64 a, u64 n, u64 base, u64 chunk, u64 window_end, u64* result){
    u64 t = (u64)blockIdx.x*blockDim.x + threadIdx.x;
    u64 start = base + t*chunk;
    if (start >= window_end) return;
    if (*result <= start) return;          // a smaller r already found
    u64 end = start + chunk; if (end > window_end) end = window_end;
    u64 val = powm(a, start - 1, n);       // a^(start-1); start>=1 so no underflow
    for (u64 r=start; r<end; r++){
        val = mulm(val, a, n);             // val = a^r
        if (val == 1){ atomicMin(result, r); return; }
    }
}

// The BSGS pair: every baby step a^j and every giant step g^i is its own
// independent modular exponentiation, not a link in a chain depending on
// the step before it -- unlike order_search's sequential walk, thread j
// here never needs thread j-1's result. That is the actual structural
// difference from brute force, not a faster multiply.
extern "C" __global__ void baby_step(u64 a, u64 n, u64 m, u64* out){
    u64 j = (u64)blockIdx.x*blockDim.x + threadIdx.x;
    if (j >= m) return;
    out[j] = powm(a, j, n);
}
extern "C" __global__ void giant_step(u64 g, u64 n, u64 m, u64* out){
    u64 i = (u64)blockIdx.x*blockDim.x + threadIdx.x + 1;
    if (i > m) return;
    out[i-1] = powm(g, i, n);
}

// FDE Shor membrane primitive.
// One CUDA thread owns one resident transition. The forward and reverse
// sidearms each perform the same arbitrary-width modular multiplication from
// the opened state. The boundary flag is true only for equal sidearms whose
// fused value is strictly below N.
__device__ bool less_limbs(const u32 *a, const u32 *b, u32 L) {
    for (int i = (int)L - 1; i >= 0; --i) {
        if (a[i] != b[i]) return a[i] < b[i];
    }
    return false;
}

__device__ bool equal_limbs(const u32 *a, const u32 *b, u32 L) {
    for (u32 i = 0; i < L; ++i) if (a[i] != b[i]) return false;
    return true;
}

__device__ void copy_limbs(u32 *dst, const u32 *src, u32 L) {
    for (u32 i = 0; i < L; ++i) dst[i] = src[i];
}

// out = (a + b) mod n.  All inputs are already reduced and the carry is kept
// separately, so no machine-width widening assumption is made about N.
__device__ void add_mod(u32 *out, const u32 *a, const u32 *b,
                        const u32 *n, u32 L) {
    u64 carry = 0;
    for (u32 i = 0; i < L; ++i) {
        u64 s = (u64)a[i] + b[i] + carry;
        out[i] = (u32)s;
        carry = s >> 32;
    }
    if (carry || !less_limbs(out, n, L)) {
        u64 borrow = 0;
        for (u32 i = 0; i < L; ++i) {
            u64 sub = (u64)n[i] + borrow;
            u64 value = out[i];
            out[i] = (u32)(value - sub);
            borrow = value < sub;
        }
    }
}

// Deliberately structure the product as a bit-fold over the resident limb
// tape.  This is the same fold shape as the IMASM arithmetic membrane.
__device__ void mul_mod(u32 *out, const u32 *x, const u32 *y,
                        const u32 *n, u32 L, u32 *scratch) {
    u32 *acc = scratch;
    u32 *cur = scratch + L;
    u32 *tmp = scratch + 2 * L;
    for (u32 i = 0; i < L; ++i) { acc[i] = 0; cur[i] = x[i]; }
    for (u32 bit = 0; bit < L * 32; ++bit) {
        if ((y[bit / 32] >> (bit % 32)) & 1u) {
            add_mod(tmp, acc, cur, n, L);
            copy_limbs(acc, tmp, L);
        }
        add_mod(tmp, cur, cur, n, L);
        copy_limbs(cur, tmp, L);
    }
    copy_limbs(out, acc, L);
}

extern "C" __global__ void fde_modular_step(
    const u32 *opened, const u32 *base, const u32 *modulus,
    u32 *forward, u32 *reverse, u8 *closed, u32 L) {
    u32 lane = blockIdx.x * blockDim.x + threadIdx.x;
    const u32 *state = opened + lane * L;
    u32 *f = forward + lane * L;
    u32 *r = reverse + lane * L;
    // Boundary contract: the opened state must already be reduced mod n
    // (the host reduces it, the same discipline as the u64 order path).
    // An unreduced x >= n poisons mul_mod: add_mod subtracts the modulus
    // only once per call, so a+b < n is required of every input, and the
    // fused value would land at or above the boundary. Refuse it here,
    // visibly, rather than computing a fiction the flag must reject.
    if (!less_limbs(state, modulus, L)) { closed[lane] = 0; return; }
    u32 *scratch = (u32 *)malloc(3 * L * sizeof(u32));
    if (!scratch) { closed[lane] = 0; return; }
    mul_mod(f, state, base, modulus, L, scratch);
    mul_mod(r, state, base, modulus, L, scratch);
    closed[lane] = equal_limbs(f, r, L) && less_limbs(f, modulus, L);
    free(scratch);
}
"#;

/// Multiplicative order of a modulo n on the GPU: least r>=1 with a^r==1 (mod
/// n), or 0 if none up to n (matching the CPU scan). Returns None if there is no
/// device.
pub fn order(a: u64, n: u64) -> Option<u64> {
    if n <= 1 { return Some(0); }
    let a = a % n;
    if a == 0 { return Some(0); }
    let ctx = CudaContext::new(0).ok()?;
    let stream = ctx.default_stream();
    let opts = CompileOptions::default();
    let ptx = compile_ptx_with_opts(KERNEL_SRC, opts).ok()?;
    let module = ctx.load_module(ptx).ok()?;
    let func = module.load_function("order_search").ok()?;

    let init: Vec<u64> = alloc::vec![u64::MAX];
    let mut d_res = stream.clone_htod(&init).ok()?;

    let chunk: u64 = 64;
    let threads: u32 = 256;
    let window: u64 = 1u64 << 26;                  // candidates per wave
    let blocks: u32 = ((window / chunk) as u32 + threads - 1) / threads;
    let cfg = LaunchConfig { grid_dim: (blocks,1,1), block_dim: (threads,1,1), shared_mem_bytes: 0 };

    let mut base: u64 = 1;
    while base < n.saturating_add(1) {
        let window_end = core::cmp::min(base.saturating_add(window), n.saturating_add(1));
        let mut b = stream.launch_builder(&func);
        b.arg(&a); b.arg(&n); b.arg(&base); b.arg(&chunk); b.arg(&window_end); b.arg(&mut d_res);
        unsafe { b.launch(cfg) }.ok()?;
        let r = stream.clone_dtoh(&d_res).ok()?;
        if r[0] != u64::MAX { return Some(r[0]); }
        base = window_end;
    }
    Some(0)
}

fn pow_mod_u64(a: u64, mut e: u64, n: u64) -> u64 {
    if n == 1 { return 0; }
    let mut r: u128 = 1 % n as u128;
    let mut ab: u128 = (a % n) as u128;
    while e > 0 {
        if e & 1 == 1 { r = (r * ab) % n as u128; }
        ab = (ab * ab) % n as u128;
        e >>= 1;
    }
    r as u64
}

/// Multiplicative order via GPU BSGS. Baby steps (a^j, j=0..m-1) and giant
/// steps (g^i, i=1..=m) are each generated by their own kernel, and within
/// each kernel every thread computes its own exponentiation directly --
/// thread j never waits on thread j-1's result, unlike order_search's
/// sequential per-thread chain. The candidates touched total O(sqrt(order)),
/// not O(order): a search-space reduction, not a faster multiply, which is
/// the actual reason this reaches orders order_search's own wave-by-wave
/// brute force cannot in practical time. step_cap bounds the baby-step
/// table size (and so the order this can see), same contract as
/// winding_period::winding_order_big's own step_cap.
pub fn order_bsgs(a: u64, n: u64, step_cap: u64) -> Option<u64> {
    if n <= 1 { return Some(0); }
    let a = a % n;
    if a == 0 { return Some(0); }
    if crate::winding_period::gcd(a, n) != 1 { return None; }
    if a == 1 { return Some(1); }

    let m = core::cmp::min(crate::winding_period::isqrt(n) + 1, step_cap);
    if m == 0 { return None; }

    let ctx = CudaContext::new(0).ok()?;
    let stream = ctx.default_stream();
    let opts = CompileOptions::default();
    let ptx = compile_ptx_with_opts(KERNEL_SRC, opts).ok()?;
    let module = ctx.load_module(ptx).ok()?;
    let baby_fn = module.load_function("baby_step").ok()?;
    let giant_fn = module.load_function("giant_step").ok()?;

    let threads: u32 = 256;
    let blocks: u32 = ((m as u32).saturating_add(threads - 1)) / threads;
    let cfg = LaunchConfig { grid_dim: (blocks, 1, 1), block_dim: (threads, 1, 1), shared_mem_bytes: 0 };

    // Baby steps: a^j mod n for j=0..m-1, each thread its own exponentiation.
    let zeros: Vec<u64> = alloc::vec![0u64; m as usize];
    let mut d_baby = stream.clone_htod(&zeros).ok()?;
    let mut bb = stream.launch_builder(&baby_fn);
    bb.arg(&a); bb.arg(&n); bb.arg(&m); bb.arg(&mut d_baby);
    unsafe { bb.launch(cfg) }.ok()?;
    let baby: Vec<u64> = stream.clone_dtoh(&d_baby).ok()?;

    let mut indexed: Vec<(u64, u64)> = baby.iter().enumerate().map(|(j, &v)| (v, j as u64)).collect();
    indexed.sort_unstable();

    // Giant steps: g^i mod n for i=1..=m, g = a^{-m} mod n -- again every
    // thread its own exponentiation, no dependency on the step before it.
    let a_inv = crate::winding_period::modinv(a, n);
    let g = pow_mod_u64(a_inv, m, n);

    let mut d_giant = stream.clone_htod(&zeros).ok()?;
    let mut gb = stream.launch_builder(&giant_fn);
    gb.arg(&g); gb.arg(&n); gb.arg(&m); gb.arg(&mut d_giant);
    unsafe { gb.launch(cfg) }.ok()?;
    let giant: Vec<u64> = stream.clone_dtoh(&d_giant).ok()?;

    for (idx, &gv) in giant.iter().enumerate() {
        let i = (idx as u64) + 1;
        if let Ok(pos) = indexed.binary_search_by_key(&gv, |&(v, _)| v) {
            let j = indexed[pos].1;
            let cand = i.saturating_mul(m).saturating_add(j);
            if cand > 0 && pow_mod_u64(a, cand, n) == 1 {
                return Some(crate::winding_period::minimal_winding(a, n, cand));
            }
        }
    }
    None
}

/// Self-check: GPU order against the plain CPU scan on small n.
/// Uncapped arbitrary-size multiplicative order on the tape carrier. For
/// coprime (a, n) the orbit provably closes within phi(n) <= n steps, so
/// there is no step cap anywhere: the only ceiling is real compute, never
/// an input-format limit. Counter and result are tapes, so an order that
/// does not fit u64 still comes back whole. A 0-tape for n == 1 or
/// gcd(a, n) != 1 keeps the u64 order() contract.

/// Tape isqrt -> u64 for table sizing (the value fits far below u64::MAX
/// whenever the table itself is allocatable).
fn isqrt_to_u64(t: &[char]) -> u64 {
    use vox_core::morphism_factor::dec_of;
    let s = dec_of(t);
    s.parse::<u64>().unwrap_or(u64::MAX)
}

pub fn order_tape(a_tape: &[char], n_tape: &[char]) -> Option<Vec<char>> {
    use vox_core::morphism_factor::{add, cmp, decimal_to_tape, gcd, modulo, mul, one, trim, zero};

    if zero(n_tape) { return None; }
    if cmp(n_tape, &one()) == core::cmp::Ordering::Equal { return decimal_to_tape("0"); }
    let g = gcd(a_tape.to_vec(), n_tape.to_vec());
    if cmp(&g, &one()) != core::cmp::Ordering::Equal { return decimal_to_tape("0"); }

    let mut state = modulo(a_tape, n_tape);
    let mut i = one();
    loop {
        if cmp(&state, &one()) == core::cmp::Ordering::Equal {
            return Some(trim(i));
        }
        state = modulo(&mul(&state, a_tape), n_tape);
        i = add(&i, &one());
    }
}

/// Tape exponentiation by square-and-multiply, exponent a tape (any size).
fn pow_tape(base: &[char], e_in: &[char], n: &[char]) -> Vec<char> {
    use vox_core::morphism_factor::{cmp, divmod, modulo, mul, one, tape_u64, trim, zero};
    let two = tape_u64(2);
    let mut r = one();
    let mut b = modulo(base, n);
    let mut e = trim(e_in.to_vec());
    while !zero(&e) {
        let (q, rem) = divmod(&e, &two);
        if cmp(&rem, &one()) == core::cmp::Ordering::Equal {
            r = modulo(&mul(&r, &b), n);
        }
        b = modulo(&mul(&b, &b), n);
        e = q;
    }
    r
}

/// Arbitrary-size BSGS lane. One baby table of m0 entries (m0 scales down
/// with the modulus so the table stays in memory at ANY length -- a memory
/// ceiling, not an input-format limit); giant steps walk a^(m0*i) and the
/// first collision, taken at the largest matching baby index j, is exactly
/// the minimal exponent: every k with a^k = 1 first appears at
/// i = ceil(k/m0), where the maximal j reconstructs k = m0*i - j. A
/// complete round with no collision is unreachable for coprime (a, n)
/// (the order divides the group size), so no step cap exists anywhere.
pub fn order_bsgs_tape(a_tape: &[char], n_tape: &[char]) -> Option<Vec<char>> {
    use vox_core::morphism_factor::{add, cmp, decimal_to_tape, divmod, gcd, modulo, mul, one, sub, tape_u64, trim, zero};

    if zero(n_tape) { return None; }
    if cmp(n_tape, &one()) == core::cmp::Ordering::Equal { return decimal_to_tape("0"); }
    let g = gcd(a_tape.to_vec(), n_tape.to_vec());
    if cmp(&g, &one()) != core::cmp::Ordering::Equal { return decimal_to_tape("0"); }

    let a = modulo(a_tape, n_tape);
    if cmp(&a, &one()) == core::cmp::Ordering::Equal { return Some(one()); }

    let n_chars = (n_tape.len() as u64).max(1);
    // The budget is a memory ceiling, never a compute or format cap; and the
    // baby table never exceeds the classic BSGS balance point isqrt(n)+1 --
    // beyond it the table is pure waste (the orbit closes within phi(n)<=n).
    let budget: u64 = (2_000_000 / n_chars).clamp(1, 1 << 16);
    let isq = vox_core::morphism_factor::isqrt(n_tape);
    // Saturate: an isqrt beyond u64 means an astronomically large n, where
    // the budget is the binding constraint anyway; a wrapping +1 would hand
    // m0 = 0 and an empty baby table (found on the M501 membrane seal).
    let bal = crate::gpu_shor::isqrt_to_u64(&isq).saturating_add(1);
    let m0: u64 = budget.min(bal);

    let mut table: Vec<(Vec<char>, u64)> = Vec::with_capacity(m0 as usize);
    let mut cur = one();
    let mut j = 0u64;
    while j < m0 {
        table.push((trim(cur.clone()), j));
        cur = modulo(&mul(&cur, &a), n_tape);
        j += 1;
    }
    table.sort_by(|x, y| cmp(&x.0, &y.0));
    // cur is a^m0 mod n -- the giant increment.
    let step = trim(cur);

    let m0_tape = tape_u64(m0);
    let bump = add(n_tape, &sub(&m0_tape, &one()));
    let (i_max, _) = divmod(&bump, &m0_tape); // ceil(n / m0)

    let mut giant = step.clone();
    let mut i = one();
    loop {
        if let Ok(mut idx) = table.binary_search_by(|probe| cmp(&probe.0, &giant)) {
            while idx + 1 < table.len() && table[idx + 1].0 == giant { idx += 1; }
            let j_max = table[idx].1;
            let mut k = mul(&m0_tape, &i);
            if j_max > 0 { k = sub(&k, &tape_u64(j_max)); }
            if cmp(&pow_tape(a_tape, &k, n_tape), &one()) == core::cmp::Ordering::Equal {
                return Some(trim(k));
            }
        }
        if cmp(&i, &i_max) == core::cmp::Ordering::Greater { return None; }
        giant = modulo(&mul(&giant, &step), n_tape);
        i = add(&i, &one());
    }
}

/// CPU mirror of the FDE carrier step: both lanes compute state * a mod n,
/// the reverse lane from pre-reduced operands; the result is returned only
/// when the lanes agree below the modulus boundary. Runs on the tape
/// carrier at any size, no CUDA required.
pub fn fde_step_tape(state_tape: &[char], a_tape: &[char], n_tape: &[char]) -> Option<Vec<char>> {
    use vox_core::morphism_factor::{cmp, modulo, mul, zero};

    if zero(n_tape) { return None; }
    let s = modulo(state_tape, n_tape);
    let forward = modulo(&mul(&s, a_tape), n_tape);
    let ar = modulo(a_tape, n_tape);
    let reverse = modulo(&mul(&s, &ar), n_tape);
    if cmp(&forward, &reverse) == core::cmp::Ordering::Equal { Some(forward) } else { None }
}


/// The factor close (Shor's reduction, host side): given the order r of a
/// mod n, when r is even and a^(r/2) != -1 (mod n), one of
/// gcd(a^(r/2) -/+ 1, n) is a nontrivial factor with probability >= 1/2.
/// Returns Some((p, q)) with p * q = n, or None with the honest reason.
pub fn factor_close(a_tape: &[char], n_tape: &[char], r: &[char])
    -> (Option<(Vec<char>, Vec<char>)>, &'static str)
{
    use vox_core::morphism_factor::{add, cmp, divmod, gcd, one, sub, tape_u64, trim, zero};
    let one_v = one();
    if zero(r) || cmp(r, &one_v) == core::cmp::Ordering::Equal {
        return (None, "order is trivial (0 or 1) -- no close");
    }
    let two = tape_u64(2);
    let (half, rem) = divmod(r, &two);
    if cmp(&rem, &one_v) == core::cmp::Ordering::Equal {
        return (None, "order odd -- a^(r/2) undefined, retry with another base");
    }
    let h = pow_tape(a_tape, &half, n_tape);
    let minus_one = sub(n_tape, &one_v);
    if cmp(&h, &minus_one) == core::cmp::Ordering::Equal {
        return (None, "a^(r/2) = -1 (mod n) -- retry with another base");
    }
    let h_minus = trim(sub(&h, &one_v));
    let h_plus = trim(add(&h, &one_v));
    let f1 = gcd(h_minus, n_tape.to_vec());
    let f2 = gcd(h_plus, n_tape.to_vec());
    for f in [f1, f2] {
        let f = trim(f);
        if cmp(&f, &one_v) != core::cmp::Ordering::Equal
            && cmp(&f, n_tape) != core::cmp::Ordering::Equal {
            let (q, rem0) = divmod(n_tape, &f);
            if zero(&rem0) { return (Some((f, q)), "factors recovered"); }
        }
    }
    (None, "both closing gcds trivial for this base -- retry with another base")
}

pub fn verify() -> String {
    fn cpu_order(a: u64, n: u64) -> u64 {
        if n <= 1 { return 0; }
        let mut val = 1u64 % n;
        for r in 1..=n { val = (((val as u128) * (a as u128)) % n as u128) as u64; if val == 1 { return r; } }
        0
    }
    let cases: [(u64,u64); 8] = [(7,15),(5,21),(2,35),(2,77),(3,101),(10,1000003),(2,1048573),(6,999983)];
    let mut out = String::new();
    let mut all = true;
    for (a,n) in cases {
        let cpu = cpu_order(a,n);
        let gpu = order(a,n).unwrap_or(u64::MAX);
        let ok = cpu == gpu;
        all &= ok;
        out.push_str(&format!("  a={} n={}: CPU r={} GPU r={} -- match {}\n", a, n, cpu, gpu, ok));
    }
    format!("gpu_shor verify (order finding):\n{}  all match: {}", out, all)
}

/// FDE Shor membrane step: performs one modular multiplication step inside the
/// FDE carrier on the GPU. Both forward and reverse sidearms independently
/// compute `state * a mod n`, and the result is returned only if both lanes
/// produce identical output that remains below the modulus boundary.
pub fn fde_step(
    state_tape: &[char],
    a_tape: &[char],
    n_tape: &[char],
) -> Option<Vec<char>> {
    use vox_core::morphism_factor::{limb_count_for_bits, tape_to_limbs, limbs_to_tape};

    let max_bits = core::cmp::max(state_tape.len(), core::cmp::max(a_tape.len(), n_tape.len()));
    let L = limb_count_for_bits(max_bits);

    // Reduce the opened state mod n before upload -- the kernel refuses an
    // unreduced state by contract, exactly as the u64 path reduces a % n.
    // A zero modulus is refused outright: the boundary would be undefined.
    if n_tape.iter().all(|&c| c != vox_core::vox::EVALF) { return None; }
    let reduced_state = vox_core::morphism_factor::modulo(state_tape, n_tape);
    let opened_limbs = tape_to_limbs(&reduced_state, L);
    let base_limbs = tape_to_limbs(a_tape, L);
    let modulus_limbs = tape_to_limbs(n_tape, L);

    let ctx = CudaContext::new(0).ok()?;
    let stream = ctx.default_stream();
    let opts = CompileOptions::default();
    let ptx = compile_ptx_with_opts(KERNEL_SRC, opts).ok()?;
    let module = ctx.load_module(ptx).ok()?;
    let fde_fn = module.load_function("fde_modular_step").ok()?;

    let d_opened = stream.clone_htod(&opened_limbs).ok()?;
    let d_base = stream.clone_htod(&base_limbs).ok()?;
    let d_modulus = stream.clone_htod(&modulus_limbs).ok()?;
    let d_forward = stream.clone_htod(&vec![0u32; L]).ok()?;
    let d_reverse = stream.clone_htod(&vec![0u32; L]).ok()?;
    let d_closed = stream.clone_htod(&vec![0u8; 1]).ok()?;

    let threads: u32 = 1;
    let blocks: u32 = 1;
    let cfg = LaunchConfig { grid_dim: (blocks, 1, 1), block_dim: (threads, 1, 1), shared_mem_bytes: 0 };

    let mut b = stream.launch_builder(&fde_fn);
    b.arg(&d_opened); b.arg(&d_base); b.arg(&d_modulus);
    b.arg(&d_forward); b.arg(&d_reverse); b.arg(&d_closed); b.arg(&L);
    unsafe { b.launch(cfg) }.ok()?;

    let closed: Vec<u8> = stream.clone_dtoh(&d_closed).ok()?;
    if closed[0] == 0 {
        return None;
    }

    let forward: Vec<u32> = stream.clone_dtoh(&d_forward).ok()?;
    Some(limbs_to_tape(&forward))
}

/// Self-check: GPU FDE step against the CPU FDE carrier for small values.
pub fn verify_fde() -> String {
    use vox_core::morphism_factor::{dec_of, mul, modulo, one, tape_u64};
    let mut out = String::new();
    let mut all = true;
    for (a_val, n_val) in [(2u64, 15u64), (2, 21), (3, 35)] {
        let a = tape_u64(a_val);
        let n = tape_u64(n_val);
        let cpu = modulo(&mul(&one(), &a), &n);
        let gpu = fde_step(&one(), &a, &n);
        let ok = gpu.as_deref() == Some(&cpu[..]);
        all &= ok;
        out.push_str(&format!("  a={} n={}: CPU={} GPU={} -- match {}\n",
            a_val, n_val, dec_of(&cpu), gpu.as_ref().map(|g| dec_of(g)).unwrap_or_default(), ok));
    }
    format!("gpu_shor fde verify:\n{}  all match: {}", out, all)
}
