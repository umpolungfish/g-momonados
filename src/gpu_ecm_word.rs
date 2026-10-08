//! GPU ECM over IMASM Tape parity arms, with WordTape boundaries and setup.
//! No BigUint conversion or numeric-limb kernel participates in this path.
use alloc::{format, string::String, vec, vec::Vec};
use crate::word_tape::WordTape;
use cudarc::driver::{CudaContext, CudaModule, LaunchConfig, PushKernelArg};
use cudarc::nvrtc::{compile_ptx, Ptx};

const EVALT: u8 = 8;
const EVALF: u8 = 9;

fn arms(word: &WordTape, capacity: usize) -> Vec<u8> {
    let mut tape = vec![EVALT; capacity];
    let marks: Vec<char> = word.as_word().chars().collect();
    for i in 0..word.bit_len() { tape[i] = if marks[1+5*i+3]=='⊥' { EVALF } else { EVALT }; }
    tape
}

fn from_arms(tape: &[u8]) -> Result<WordTape, String> {
    let mut value = WordTape::zero();
    for (i, &arm) in tape.iter().enumerate() {
        match arm {
            EVALT => {},
            EVALF => value = value.add(&WordTape::one_at(i)),
            _ => return Err(format!("GPU returned an invalid IMASM parity arm: {arm}")),
        }
    }
    Ok(value)
}

fn ptx(width: usize) -> Result<Ptx, String> {
    let source = format!("#define W {width}\n{}", include_str!("gpu_ecm_word.cu"));
    let mut hash = 0xcbf29ce484222325u64;
    for byte in source.bytes() { hash = (hash ^ byte as u64).wrapping_mul(0x100000001b3); }
    let path = format!("target/gpu_ecm_word_{width}_{hash:016x}.ptx");
    if std::path::Path::new(&path).exists() { return Ok(Ptx::from_file(path)); }
    let compiled = compile_ptx(source).map_err(|e| format!("IMASM ECM NVRTC: {e}"))?;
    std::fs::write(path, compiled.to_src()).map_err(|e| format!("IMASM ECM PTX cache: {e}"))?;
    Ok(compiled)
}

type Backend = (std::sync::Arc<CudaContext>, std::sync::Arc<CudaModule>);
std::thread_local! {
    static BACKENDS: std::cell::RefCell<std::collections::BTreeMap<(usize,usize),Backend>> = std::cell::RefCell::new(std::collections::BTreeMap::new());
}
fn backend(device: usize,width: usize) -> Result<Backend,String> {
    BACKENDS.with(|cache| {
        if let Some(value)=cache.borrow().get(&(device,width)) { return Ok(value.clone()); }
        let ctx=CudaContext::new(device).map_err(|e| format!("GPU {device} unavailable: {e}"))?;
        ctx.set_limit(cudarc::driver::sys::CUlimit::CU_LIMIT_STACK_SIZE,4096).map_err(|e| e.to_string())?;
        let module=ctx.load_module(ptx(width)?).map_err(|e| format!("IMASM ECM module: {e}"))?;
        cache.borrow_mut().insert((device,width),(ctx.clone(),module.clone()));
        Ok((ctx,module))
    })
}

fn primes(bound: usize) -> Vec<u8> {
    let mut flags = vec![EVALF; bound+1];
    flags[0]=EVALT; if bound>0 { flags[1]=EVALT; }
    for p in 2..=bound {
        if flags[p]!=EVALF { continue; }
        if p>bound/p { break; }
        for multiple in (p*p..=bound).step_by(p) { flags[multiple]=EVALT; }
    }
    flags
}

fn stage_one_schedule(b1: usize, flags: &[u8]) -> (Vec<u8>, Vec<i32>, Vec<i32>) {
    let mut tape = Vec::new();
    let mut offsets = vec![0];
    let mut repetitions = Vec::new();
    for p in 2..=b1 {
        if flags[p]!=EVALF { continue; }
        let mut power=p;
        let mut repeats=1;
        while power<=b1/p { power*=p; repeats+=1; }
        repetitions.push(repeats);
        let word=WordTape::from_small(power as u64);
        tape.extend(arms(&word,word.bit_len()));
        offsets.push(tape.len() as i32);
    }
    (tape,offsets,repetitions)
}

pub fn factor(n: &WordTape, b1: u64, b2: u64, device: usize, sigma: u64, curves: u32) -> Result<Option<WordTape>, String> {
    if !n.is_odd() || n.bit_len()<4 { return Err("GPU IMASM ECM expects an odd canonical source greater than seven".into()); }
    if b1<2 || b2<b1 || curves==0 { return Err("ECM requires B1>=2, B2>=B1, and at least one curve".into()); }
    let clock=std::time::Instant::now();
    let trace=std::env::var_os("ECM_TIMING").is_some();
    let mark=|name: &str| { if trace { std::eprintln!("ECM timing {name} {:.6}",clock.elapsed().as_secs_f64()); } };
    if sigma<6 || sigma.checked_add(u64::from(curves)-1).is_none() { return Err("Suyama curves require sigma>=6 and a non-overflowing sigma range".into()); }
    let width=n.bit_len()+2;
    let limit=usize::try_from(b2).map_err(|_| "ECM bound exceeds addressable prime schedule")?;
    let flags=primes(limit);
    let (exponent_tape,offsets,repetitions)=stage_one_schedule(b1 as usize,&flags);
    let count=i32::try_from(offsets.len()-1).map_err(|_| "ECM schedule exceeds GPU index range")?;
    mark("schedule");
    let (ctx,module)=backend(device,width)?;
    mark("module");
    let stream=ctx.default_stream();
    let func=module.load_function("ecm_word").map_err(|e| format!("IMASM ECM kernel: {e}"))?;
    let input=stream.clone_htod(&arms(n,width)).map_err(|e| e.to_string())?;
    let unit_word=WordTape::one_at(n.bit_len()).divmod(n).ok_or("Invalid modulus")?.1;
    let unit=stream.clone_htod(&arms(&unit_word,width)).map_err(|e| e.to_string())?;
    let scalar=stream.clone_htod(&exponent_tape).map_err(|e| e.to_string())?;
    let steps=stream.clone_htod(&offsets).map_err(|e| e.to_string())?;
    let repeats=stream.clone_htod(&repetitions).map_err(|e| e.to_string())?;
    let schedule=stream.clone_htod(&flags).map_err(|e| e.to_string())?;
    let mut output=stream.clone_htod(&vec![EVALT;width]).map_err(|e| e.to_string())?;
    let mut found=stream.clone_htod(&[0i32]).map_err(|e| e.to_string())?;
    mark("transfer");
    let curves_i32=i32::try_from(curves).map_err(|_| "Curve count exceeds GPU index range")?;
    let cfg=LaunchConfig { grid_dim:(curves.div_ceil(4),1,1), block_dim:(128,1,1), shared_mem_bytes:0 };
    let mut launch=stream.launch_builder(&func);
    launch.arg(&input).arg(&unit).arg(&scalar).arg(&steps).arg(&repeats).arg(&count).arg(&schedule).arg(&b1).arg(&b2)
        .arg(&sigma).arg(&curves_i32).arg(&mut output).arg(&mut found);
    unsafe { launch.launch(cfg) }.map_err(|e| format!("IMASM ECM GPU launch: {e}"))?;
    let flag=stream.clone_dtoh(&found).map_err(|e| format!("IMASM ECM GPU completion: {e}"))?;
    mark("kernel_completed");
    if flag[0]==0 { return Ok(None); }
    let candidate=from_arms(&stream.clone_dtoh(&output).map_err(|e| e.to_string())?)?;
    if candidate.is_zero() || candidate.is_one() || !n.gt(&candidate) {
        return Err("GPU ECM returned an improper factor word".into());
    }
    let (_, remainder)=n.divmod(&candidate).ok_or("GPU ECM returned a zero divisor")?;
    if !remainder.is_zero() { return Err("GPU ECM factor does not divide the source word".into()); }
    mark("factor_closed");
    Ok(Some(candidate))
}

pub fn run(raw: &str, b1: u64, b2: u64, device: usize, sigma: u64, curves: u32) -> String {
    let Some(n)=WordTape::from_canonical_word(raw) else {
        return "gpu_ecm: expected a canonical IMASM numeral word".into();
    };
    let start=std::time::Instant::now();
    match factor(&n,b1,b2,device,sigma,curves) {
        Ok(Some(factor)) => format!("gpu_ecm IMASM GPU device={device} input_bits={} B1={b1} B2={b2} sigma={sigma} curves={curves} elapsed_seconds={:.6}\nfactor={}\nproduct_closes=true",n.bit_len(),start.elapsed().as_secs_f64(),factor.as_word()),
        Ok(None) => format!("gpu_ecm IMASM GPU device={device} input_bits={} B1={b1} B2={b2} sigma={sigma} curves={curves} elapsed_seconds={:.6}\nno factor in this curve batch",n.bit_len(),start.elapsed().as_secs_f64()),
        Err(error) => format!("gpu_ecm: {error}"),
    }
}

pub fn check(raw: &str, device: usize) -> String {
    let Some(n)=WordTape::from_canonical_word(raw) else { return "gpu_ecm check: expected canonical IMASM source".into(); };
    if !n.is_odd() || n.bit_len()<4 { return "gpu_ecm check: odd source greater than seven required".into(); }
    let test=|| -> Result<bool,String> {
        let width=n.bit_len()+2;
        let a=n.sub(&WordTape::from_small(2)).unwrap();
        let b=n.sub(&WordTape::from_small(3)).unwrap();
        let (ctx,module)=backend(device,width)?;
        let stream=ctx.default_stream();
        let func=module.load_function("word_arithmetic").map_err(|e| e.to_string())?;
        let dn=stream.clone_htod(&arms(&n,width)).map_err(|e| e.to_string())?;
        let unit_word=WordTape::one_at(n.bit_len()).divmod(&n).ok_or("Invalid modulus")?.1;
        let unit=stream.clone_htod(&arms(&unit_word,width)).map_err(|e| e.to_string())?;
        let da=stream.clone_htod(&arms(&a,width)).map_err(|e| e.to_string())?;
        let db=stream.clone_htod(&arms(&b,width)).map_err(|e| e.to_string())?;
        let mut product=stream.clone_htod(&vec![EVALT;width]).map_err(|e| e.to_string())?;
        let mut inverse=stream.clone_htod(&vec![EVALT;width]).map_err(|e| e.to_string())?;
        let mut gcd=stream.clone_htod(&vec![EVALT;width]).map_err(|e| e.to_string())?;
        let mut launch=stream.launch_builder(&func);
        launch.arg(&dn).arg(&unit).arg(&da).arg(&db).arg(&mut product).arg(&mut inverse).arg(&mut gcd);
        unsafe { launch.launch(LaunchConfig { grid_dim:(1,1,1), block_dim:(32,1,1), shared_mem_bytes:0 }) }.map_err(|e| e.to_string())?;
        let p=from_arms(&stream.clone_dtoh(&product).map_err(|e| e.to_string())?)?;
        let inv=from_arms(&stream.clone_dtoh(&inverse).map_err(|e| e.to_string())?)?;
        let g=from_arms(&stream.clone_dtoh(&gcd).map_err(|e| e.to_string())?)?;
        Ok(p==WordTape::from_small(6) && inv==n.sub(&WordTape::one()).unwrap().shr1() && g.is_one())
    };
    match test() { Ok(closes)=>format!("gpu_ecm IMASM GPU arithmetic input_bits={} device={device} closes={closes}",n.bit_len()), Err(e)=>format!("gpu_ecm check: {e}") }
}
