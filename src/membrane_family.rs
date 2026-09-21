//! membrane_family.rs — every factorizer ob3ect word run AS a membrane.
//!
//! An ob3ect is a sequence of morphisms. Every token is a process, and what the
//! process IS depends on the register. This module holds ONE register, the math
//! register, as a token->process table over a factor carrier (the same table
//! `gpu_kernel::factor_phase_leaf` proved for the live aggregate-phase word).
//! Building "all of them as membranes" is not a bespoke algorithm per word: it
//! is mounting each ob3ect word on this one register and stepping it through the
//! nested fixed-point vessel with the value N built in. Different words drive the
//! same carrier in a different order; some close a factor pair, some do not, and
//! that difference is the reading.
//!
//! The tokens (classic discriminants): 0 VINIT reset, 1 TANCH emit, 2 AFWD
//! advance a, 3 AREV root of the gap, 4 CLINK form the pair (a-b,a+b), 5 IMSCRIB
//! gap delta=a^2-N, 6 FSPLIT, 7 FFUSE, 8 EVALT band-validate, 9 EVALF square
//! test, 10 ENGAGR, 11 IFIX fix. Arithmetic is `native_numeral` word ops on the
//! numeral's own marks; BigUint is only the host boundary in and out.
#![allow(dead_code)]
extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use num_bigint::BigUint;
use num_traits::{One, Zero};
use crate::native_numeral::{add_via_word, subtract_via_word, multiply_via_word};
use cudarc::driver::{CudaContext, LaunchConfig, PushKernelArg};
use cudarc::nvrtc::{compile_ptx_with_opts, CompileOptions};

/// The nested fixed-point emit as a device kernel. The complete enclosure acts
/// as μ∘δ=id on the mark carrier: its two arms are the same banked morphism, so
/// fusion and fixation return the mark exactly. One thread per token writes the
/// mark back and ticks the device counter once, so the emit runs on the card and
/// the pass tick count is read from the GPU, not assumed.
const EMIT_PTX: &str = r#"
extern "C" __global__ void nested_emit(const unsigned* marks, unsigned* out,
                                       unsigned long long* ticks, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n) return;
    out[i] = marks[i];                       // μ∘δ = id on this carrier
    atomicAdd(ticks, 1ULL);
}
"#;

/// Run the word's whole op stream through the nested fixed-point emit on the GPU,
/// once. Returns the emitted marks (identity on the carrier) and the tick count
/// the device accumulated for one pass. Self-contained in the gpu_rho style: its
/// own kernel, no reach into the bare-metal kernel tree. None when no CUDA context
/// is present, so a headless host still runs the membrane through the id fallback.
fn gpu_nested_emit(ops:&[u32], device:usize) -> Option<(Vec<u32>, u64)> {
    if ops.is_empty() { return Some((Vec::new(), 0)); }
    // The context, kernel and loaded function are all fixed, so they are built
    // once per process and every later pass reuses them. Rebuilding the context
    // and recompiling the kernel per call was the whole per-call cost.
    use alloc::sync::Arc;
    use cudarc::driver::CudaFunction;
    thread_local! {
        static GPU: std::cell::OnceCell<(Arc<CudaContext>, CudaFunction)> = std::cell::OnceCell::new();
    }
    let (ctx, func) = GPU.with(|c| c.get_or_init(|| {
        let ctx = CudaContext::new(device).expect("CUDA context");
        let ptx = compile_ptx_with_opts(EMIT_PTX, CompileOptions::default())
            .expect("membrane emit kernel compiles");
        let module = ctx.load_module(ptx).expect("load emit module");
        let func = module.load_function("nested_emit").expect("load nested_emit");
        (ctx, func)
    }).clone());
    let stream = ctx.default_stream();
    let n = ops.len() as i32;
    let d_marks = stream.clone_htod(&ops.to_vec()).ok()?;
    let mut d_out = stream.alloc_zeros::<u32>(ops.len()).ok()?;
    let mut d_ticks = stream.alloc_zeros::<u64>(1).ok()?;
    let cfg = LaunchConfig { grid_dim:(((n as u32)+127)/128,1,1), block_dim:(128,1,1), shared_mem_bytes:0 };
    let mut b = stream.launch_builder(&func);
    b.arg(&d_marks); b.arg(&mut d_out); b.arg(&mut d_ticks); b.arg(&n);
    if unsafe { b.launch(cfg) }.is_err() { return None; }
    let out = stream.clone_dtoh(&d_out).ok()?;
    let ticks = stream.clone_dtoh(&d_ticks).ok()?;
    Some((out, ticks[0]))
}

/// The math-register factor carrier. The value N and its band are built in; the
/// live morphism state (a, gap, root, the candidate pair) rides the tokens.
struct MathCarrier<'a> {
    n:&'a BigUint, lo:&'a BigUint, hi:&'a BigUint,
    a:BigUint, delta:BigUint, b:BigUint, square:bool,
    candidate:Option<(BigUint,BigUint)>, fixed:Option<(BigUint,BigUint)>,
    emitted:Option<(BigUint,BigUint)>,
}

/// The one token->process table of the math register. Byte-identical in effect
/// to `gpu_kernel::factor_phase_leaf`; kept here so the family runner is
/// self-contained and every membrane in the census rides exactly this register.
fn math_leaf(op:u32, s:&mut MathCarrier<'_>) {
    match op {
        0 => { s.delta=BigUint::zero(); s.b=BigUint::zero(); s.square=false;
               s.candidate=None; s.fixed=None; s.emitted=None; }
        2 => s.a = add_via_word(&s.a, &BigUint::one()),
        5 => s.delta = subtract_via_word(&multiply_via_word(&s.a,&s.a), s.n)
                        .unwrap_or(BigUint::zero()),
        3 => s.b = isqrt_floor(&s.delta),
        9 => s.square = multiply_via_word(&s.b,&s.b) == s.delta,
        4 if s.square && s.candidate.is_none() => {
            s.candidate = Some((
                subtract_via_word(&s.a,&s.b).unwrap_or(BigUint::zero()),
                add_via_word(&s.a,&s.b)));
        }
        8 => if let Some((p,q))=s.candidate.take() {
                if p>=*s.lo && q<=*s.hi && multiply_via_word(&p,&q)==*s.n {
                    s.candidate=Some((p,q));
                }
             },
        11 => s.fixed = s.candidate.take(),
        1 => s.emitted = s.fixed.take(),
        _ => {}
    }
}

/// Newton floor-root, host arithmetic only for the descent bound. The membrane's
/// own gap test (op 9) reconfirms b*b via word multiply, so the root is only a
/// seed the register verifies for itself.
fn isqrt_floor(n:&BigUint)->BigUint {
    if n.is_zero() { return BigUint::zero(); }
    let mut x = BigUint::one() << ((n.bits() as usize + 1)/2);
    loop {
        let y = (&x + n/&x) >> 1;
        if y >= x { return x; }
        x = y;
    }
}

/// Parse a glyph word to its classic token discriminants, boundary marks kept.
pub fn word_ops(word:&str)->Vec<u32> {
    word.chars().filter_map(|g|
        imasm_core::classic::Token::parse(&g.to_string()).map(|t| t as u32)
    ).collect()
}

/// Run one ob3ect WORD as a membrane in the math register: mount the value N,
/// seed a at ceil(sqrt(N)), and step the whole word through the nested
/// fixed-point vessel once per frontier position until the word's own processes
/// fix and emit a pair, or the band is exhausted. Returns the pair and the
/// nested mark tick count.
pub fn run_membrane(word:&str, n:&BigUint, depth:u32)
    -> (Option<(BigUint,BigUint)>, u64)
{
    let ops = word_ops(word);
    let one = BigUint::one();
    if *n < BigUint::from(4u8) { return (None,0); }
    let lo = &one + &one;                 // 2
    let hi = n / (&one + &one);            // N/2, the whole legal band
    let mut a = isqrt_floor(n);
    if multiply_via_word(&a,&a) < *n { a = &a + &one; }
    let mut c = MathCarrier { n, lo:&lo, hi:&hi, a, delta:BigUint::zero(),
        b:BigUint::zero(), square:false, candidate:None, fixed:None, emitted:None };
    let _ = depth;
    // The nested fixed-point emit of the word's op stream, run once on the GPU.
    // The emit is μ∘δ=id, so the emitted marks equal the ops; the device also
    // returns the per-pass tick count. When no card is present, the id fallback
    // gives the same marks and per-pass ticks, so the membrane still runs headless.
    let (emitted_ops, pass_ticks) = gpu_nested_emit(&ops, 0)
        .unwrap_or_else(|| (ops.clone(), ops.len() as u64));
    let mut ticks=0u64;
    // Self-bounded: the frontier walks a up from ceil(sqrt(N)); the pair, if any,
    // fixes by the time a reaches (p+q)/2, which never exceeds N/2. The band a>N/2
    // is the whole legal range, so the walk bounds itself on N with no step cap.
    loop {
        for &op in &emitted_ops {
            math_leaf(op, &mut c);
        }
        ticks += pass_ticks;
        if let Some(pq) = c.emitted.take() { return (Some(pq), ticks); }
        if c.a > hi { break; }
    }
    (None, ticks)
}

/// The factorizer ob3ect family: name -> canonical glyph word, pulled from
/// ob3ect/digital. Each is run through the one math register above.
pub fn family()->Vec<(&'static str,&'static str)> {
    alloc::vec![
        ("aggregate_phase_morphism",      "⊢≻⋈⊙∈⊤≻⋈⊥≺⋈⊞∋⊡⋈⊙⊣"),
        ("closed_divisor_search",         "⊢⊣≻∈⊤⋈≺⊥⊞⊙∋⋈∈⊤≺⊥∋⊡⊣"),
        ("imscribing_factorizer",         "⊢⊣≻∈⊤⋈≺⊥⊞⊙⋈∈⊤≺⊥∋⊡⊣"),
        ("factor_separating_crossing",    "⊢∈≻⊤≺⊥⊞⋈∋⊙⊡⊣"),
        ("nested_instant_tower",          "⊢∈⊤≻⋈⊙⊥≺⊞∋⋈⊙⊡⊣"),
        ("instant_nested_phase",          "⊢∈≻⊤≺⊥⋈⊙⊞∋⊡⊣"),
        ("fully_nested_phase",            "⊢∈≻⊤≺⊥⋈⊙⊞∋⊡⋈⊙⊣"),
        ("phase_based",                   "⊢⊙∈≻⊤⋈≺⊥⊞∋⊡⋈⊙⊣"),
        ("frobenius_circuit",             "⊢≻⊙∈⊤⋈⊥≺⊞∋⊡⊣"),
        ("trilattice_native",            "⊢≻⊙∈⊤⋈⊥≺⊞⊡∋⋈⊣"),
        ("carry_fuse_closure",            "⊢⊙∈≻⊤≺⊥⊞⋈∋⊡⊣"),
        ("prime_inverse_braider",         "⊢∈≻⊤≺⊥⊞⋈∋⊙⊡⊣"),
        ("full_enfolding_nesting",        "⊢⊣≻⋈⊙∈⊤⊥⊞≺⋈∋⊡⊙⊣"),
    ]
}

/// REPL/CLI face: `membrane <run WORD N | family N | list>`.
pub fn repl_membrane(args:&[&str])->String {
    let depth:u32 = 64;
    match args.first().copied() {
        Some("list") => {
            let mut out=String::from("membrane family (name : word):\n");
            for (nm,w) in family() { out.push_str(&format!("  {nm:28} {w}\n")); }
            out
        }
        Some("run") if args.len()>=3 => {
            let word=args[1];
            let n = match args[2].parse::<BigUint>() { Ok(v)=>v, Err(_)=>return String::from("N must be decimal") };
            let (pq,ticks)=run_membrane(word,&n,depth);
            match pq {
                Some((p,q))=>format!("membrane {word}\n  N={n}\n  {n} = {p} x {q} (verified={})\n  nested mark ticks={ticks}",
                    &multiply_via_word(&p,&q)==&n),
                None=>format!("membrane {word}\n  N={n}\n  no pair fixed across the full band (this word does not close N in the math register)\n  nested mark ticks={ticks}"),
            }
        }
        Some("family") if args.len()>=2 => {
            let n = match args[1].parse::<BigUint>() { Ok(v)=>v, Err(_)=>return String::from("N must be decimal") };
            let mut out=format!("math-register membrane family on N={n}\n");
            out.push_str(&format!("{:28} {:>10} {:>12} {}\n","MEMBRANE","TICKS","FACTORED","PAIR"));
            for (nm,w) in family() {
                let (pq,ticks)=run_membrane(w,&n,depth);
                match pq {
                    Some((p,q))=>{
                        let ok = &multiply_via_word(&p,&q)==&n;
                        out.push_str(&format!("{nm:28} {ticks:>10} {:>12} {p} x {q}\n", if ok {"yes"} else {"BADVERIFY"}));
                    }
                    None=>out.push_str(&format!("{nm:28} {ticks:>10} {:>12} -\n","no")),
                }
            }
            out
        }
        _ => String::from("usage: membrane list | membrane run <WORD> <N> | membrane family <N>"),
    }
}
