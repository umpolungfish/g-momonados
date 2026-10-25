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
//! Each word is interpreted on the CUDA device as a state transition over a
//! multi-limb factor register. The device prepares the initial frontier, runs
//! every candidate through the word's opcode order, and checks the exact product
//! before returning a pair. The host loads N and the word, schedules bounded
//! batches, and reads the closed result; it has no factor-search fallback.
#![allow(dead_code)]
extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use num_bigint::BigUint;
use num_traits::One;
use imasm_core::{classic::Token, check::{self, ClosureState}};
use cudarc::driver::{CudaContext, CudaFunction, LaunchConfig, PushKernelArg};
use cudarc::nvrtc::compile_ptx;
use alloc::sync::Arc;
use core::cell::OnceCell;

/// Device source for the word interpreter and its multi-limb arithmetic.
const MF_GPU_SRC: &str = include_str!("membrane_family_gpu.cuh");

thread_local! {
    static MF_GPU: OnceCell<Result<(Arc<CudaContext>, CudaFunction, CudaFunction, usize), String>> = OnceCell::new();
}

fn membrane_gpu() -> Result<(Arc<CudaContext>, CudaFunction, CudaFunction, usize), String> {
    MF_GPU.with(|slot| slot.get_or_init(|| {
        let mut last = String::from("no CUDA device initialized");
        for device in [1usize, 0] {
            match CudaContext::new(device) {
                Ok(ctx) => {
                    let ptx = compile_ptx(MF_GPU_SRC).map_err(|e| format!("membrane NVRTC: {e}"))?;
                    let module = ctx.load_module(ptx).map_err(|e| format!("membrane module: {e}"))?;
                    let prepare = module.load_function("membrane_prepare")
                        .map_err(|e| format!("membrane prepare kernel: {e}"))?;
                    let func = module.load_function("membrane_word_factor")
                        .map_err(|e| format!("membrane kernel: {e}"))?;
                    return Ok((ctx, prepare, func, device));
                }
                Err(e) => last = format!("CUDA device {device}: {e}"),
            }
        }
        Err(format!("membrane requires CUDA on device 0 or 1: {last}"))
    }).clone())
}

/// Parse a glyph word to its classic token discriminants, boundary marks kept.
pub fn word_ops(word:&str)->Vec<u32> {
    word.chars().filter_map(|g|
        imasm_core::classic::Token::parse(&g.to_string()).map(|t| t as u32)
    ).collect()
}

/// Admit a linear spelling through the canonical IMASM protocol graph builder
/// and close-condition engine. A local stack check is insufficient: it can
/// approve a word whose actual protocol graph is open or ill-typed.
pub(crate) fn validate_protocol_word(word:&str)->Result<(char,ClosureState),String> {
    let mut tokens=Vec::new();
    for glyph in word.chars() {
        let token=Token::parse(&glyph.to_string())
            .ok_or_else(|| format!("protocol contains non-IMASM mark {glyph}"))?;
        tokens.push(token);
    }
    let verdict=check::word_verdict(&tokens);
    match verdict.1 {
        ClosureState::Closed(_) => Ok(verdict),
        ClosureState::Identity => Err(String::from("protocol reconnects without work")),
        ClosureState::Open => Err(String::from("protocol has an unreconnected fork or fuse")),
        ClosureState::None => Err(String::from("protocol has no fork/fuse closure")),
    }
}

/// Run one word through the device factor register. CUDA prepares floor(sqrt(N))
/// and N/2, then each lane executes the complete word on one successive value
/// of the frontier until a product-closed pair returns or the legal band ends.
pub fn run_membrane(word:&str, n:&BigUint, depth:u32)
    -> Result<(Option<(BigUint,BigUint)>, u64, usize), String>
{
    let ops = word_ops(word);
    let one = BigUint::one();
    if ops.is_empty() { return Err(String::from("membrane word has no executable marks")); }
    validate_protocol_word(word)?;
    let (ctx, prepare, func, device) = membrane_gpu()?;
    if *n < BigUint::from(4u8) { return Ok((None,0,device)); }
    let nlimbs = n.to_u32_digits();
    if nlimbs.len() > 32 { return Err(format!("CUDA membrane supports up to 1024 source bits, received {}", n.bits())); }
    let nlimbs_len = nlimbs.len();
    let _ = depth;
    let stream = ctx.default_stream();
    let nlimbs_i32 = nlimbs_len as i32;
    let n_words = nlimbs.clone();
    let d_n = stream.clone_htod(&n_words).map_err(|e| format!("membrane upload N: {e}"))?;
    let mut d_start = stream.alloc_zeros::<u32>(nlimbs_len).map_err(|e| format!("membrane allocate start: {e}"))?;
    let mut d_hi = stream.alloc_zeros::<u32>(nlimbs_len).map_err(|e| format!("membrane allocate bound: {e}"))?;
    let mut prepare_launch = stream.launch_builder(&prepare);
    prepare_launch.arg(&d_n); prepare_launch.arg(&nlimbs_i32);
    prepare_launch.arg(&mut d_start); prepare_launch.arg(&mut d_hi);
    let prep_cfg = LaunchConfig { grid_dim:(1,1,1), block_dim:(1,1,1), shared_mem_bytes:0 };
    unsafe { prepare_launch.launch(prep_cfg) }.map_err(|e| format!("membrane GPU preparation: {e}"))?;
    let start_words = stream.clone_dtoh(&d_start).map_err(|e| format!("membrane read initial frontier: {e}"))?;
    let hi_words = stream.clone_dtoh(&d_hi).map_err(|e| format!("membrane read frontier bound: {e}"))?;
    let mut start = BigUint::new(start_words);
    let hi = BigUint::new(hi_words);
    let d_ops = stream.clone_htod(&ops).map_err(|e| format!("membrane upload word: {e}"))?;
    let nops_i32 = ops.len() as i32;
    const BATCH: u64 = 256;
    let mut ticks = 0u64;
    while start <= hi {
        let remaining = &hi - &start + &one;
        let count = if remaining > BigUint::from(BATCH) { BATCH }
                    else { remaining.to_u64_digits().first().copied().unwrap_or(BATCH) };
        let start_words_raw = start.to_u32_digits();
        let mut start_words = start_words_raw; start_words.resize(nlimbs_len, 0);
        let d_start_batch = stream.clone_htod(&start_words).map_err(|e| format!("membrane upload frontier: {e}"))?;
        let mut d_winner = stream.clone_htod(&alloc::vec![u64::MAX]).map_err(|e| format!("membrane allocate winner: {e}"))?;
        let mut d_ticks = stream.clone_htod(&alloc::vec![0u64]).map_err(|e| format!("membrane allocate ticks: {e}"))?;
        let output_words = count as usize * nlimbs_len;
        let mut d_p = stream.alloc_zeros::<u32>(output_words).map_err(|e| format!("membrane allocate p: {e}"))?;
        let mut d_q = stream.alloc_zeros::<u32>(output_words).map_err(|e| format!("membrane allocate q: {e}"))?;
        let cfg = LaunchConfig { grid_dim: (((count as u32)+127)/128,1,1), block_dim:(128,1,1), shared_mem_bytes:0 };
        let mut launch = stream.launch_builder(&func);
        launch.arg(&d_n); launch.arg(&d_hi); launch.arg(&nlimbs_i32);
        launch.arg(&d_ops); launch.arg(&nops_i32); launch.arg(&d_start_batch);
        launch.arg(&count); launch.arg(&mut d_winner); launch.arg(&mut d_ticks);
        launch.arg(&mut d_p); launch.arg(&mut d_q);
        unsafe { launch.launch(cfg) }.map_err(|e| format!("membrane GPU launch: {e}"))?;
        let batch_ticks = stream.clone_dtoh(&d_ticks).map_err(|e| format!("membrane read ticks: {e}"))?;
        ticks = ticks.saturating_add(batch_ticks[0]);
        let winner = stream.clone_dtoh(&d_winner).map_err(|e| format!("membrane read winner: {e}"))?[0];
        if winner != u64::MAX {
            let ps = stream.clone_dtoh(&d_p).map_err(|e| format!("membrane read p: {e}"))?;
            let qs = stream.clone_dtoh(&d_q).map_err(|e| format!("membrane read q: {e}"))?;
            let begin = winner as usize * nlimbs_len;
            let p = BigUint::new(ps[begin..begin+nlimbs_len].to_vec());
            let q = BigUint::new(qs[begin..begin+nlimbs_len].to_vec());
            return Ok((Some((p,q)), ticks, device));
        }
        start += BigUint::from(count);
    }
    Ok((None, ticks, device))
}

/// The factorizer ob3ect family: name -> canonical glyph word, pulled from
/// ob3ect/digital. Each is run through the one math register above.
pub fn family()->Vec<(&'static str,&'static str)> {
    alloc::vec![
        ("aggregate_phase_morphism",      "⊢≻⋈⊙∈⊤≻⋈⊥≺⋈⊞∋⊡⋈⊙⊣"),
        ("closed_divisor_search",         "⊢⊣≻∈⊤⋈≺⊥⊞⊙∋⋈∈⊤≺⊥∋⊡⊣"),
        ("imscribing_factorizer",         "⊢⊣≻∈⊤⋈≺⊥⊞⊙⋈∈⊤≺⊥∋∋⊡⊣"),
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
        Some("wiring") => {
            let mut out=String::from("protocol graph admission (name : wiring result):\n");
            for (name,word) in family() {
                let result=validate_protocol_word(word)
                    .map(|(verdict,state)| format!("kernel {verdict}: {state:?}"))
                    .unwrap_or_else(|error| format!("refused: {error}"));
                out.push_str(&format!("  {name:28} {result}\n"));
            }
            out
        }
        Some("run") if args.len()>=3 => {
            let word=args[1];
            let n = match args[2].parse::<BigUint>() { Ok(v)=>v, Err(_)=>return String::from("N must be decimal") };
            let (pq,ticks,device)=match run_membrane(word,&n,depth) { Ok(v)=>v, Err(e)=>return format!("membrane GPU error: {e}") };
            match pq {
                Some((p,q))=>format!("membrane {word}\n  N={n}\n  {n} = {p} x {q} (GPU product closure verified)\n  CUDA device={device}; nested mark ticks={ticks}"),
                None=>format!("membrane {word}\n  N={n}\n  no pair fixed across the full band (this word does not close N in the math register)\n  CUDA device={device}; nested mark ticks={ticks}"),
            }
        }
        Some("family") if args.len()>=2 => {
            let n = match args[1].parse::<BigUint>() { Ok(v)=>v, Err(_)=>return String::from("N must be decimal") };
            let mut out=format!("GPU math-register membrane family on N={n}\n");
            out.push_str(&format!("{:28} {:>10} {:>12} {}\n","MEMBRANE","TICKS","FACTORED","PAIR"));
            for (nm,w) in family() {
                let (pq,ticks,device)=match run_membrane(w,&n,depth) { Ok(v)=>v, Err(e)=>return format!("membrane GPU error: {e}") };
                match pq {
                    Some((p,q))=>{
                        out.push_str(&format!("{nm:28} GPU{device} {ticks:>10} {:>12} {p} x {q}\n","yes"));
                    }
                    None=>out.push_str(&format!("{nm:28} GPU{device} {ticks:>10} {:>12} -\n","no")),
                }
            }
            out
        }
        _ => String::from("usage: membrane list | membrane run <WORD> <N> | membrane family <N>"),
    }
}
