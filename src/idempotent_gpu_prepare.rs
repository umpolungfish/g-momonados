//! Cargo-side compilation of the source-bound device image and Lean Y tree.
use crate::word_tape::WordTape;
use std::{format, string::String, vec, vec::Vec};

pub struct Prepared {
    pub ptx: String,
    pub state: Vec<u8>,
    pub continuation: Vec<i32>,
    pub receipt: Vec<u64>,
    pub powerset_law_failures: u64,
    pub seconds: f64,
}

pub fn compile(source: &WordTape, project: &std::path::Path) -> Result<Prepared, String> {
    let plan = vox_core::dialectic_lean::ResidentY::certified_program();
    let line = |key: &str| -> Result<&str, String> {
        plan.lines()
            .find_map(|s| s.strip_prefix(key))
            .ok_or_else(|| format!("Lean Y export omitted {key}"))
    };
    let token = |s: &str| -> Result<i32, String> {
        match s {
            "I" => Ok(1),
            "K" => Ok(2),
            "S" => Ok(3),
            "OP" => Ok(4),
            "PAY" => Ok(5),
            "@" => Ok(8),
            _ => Err(format!("unsupported Lean Y token {s}")),
        }
    };
    let mut tags = vec![6];
    let mut left = vec![0];
    let mut right = vec![0];
    let mut stack = Vec::new();
    let initial = format!("{} OP @ PAY @", line("Y-postfix=")?);
    for t in initial.split_whitespace() {
        let tag = token(t)?;
        let (l, r) = if tag == 8 {
            let r = stack.pop().ok_or("Y tree missing argument")?;
            let l = stack.pop().ok_or("Y tree missing function")?;
            (l, r)
        } else {
            (0, 0)
        };
        stack.push(tags.len() as i32);
        tags.push(tag);
        left.push(l);
        right.push(r);
    }
    if stack.len() != 1 {
        return Err("Lean Y tree did not compile to one root".into());
    }
    if tags.len() > 2048 {
        return Err("certified Y continuation exceeds the resident node bank".into());
    }
    let prefix: usize = line("Y-prefix-contractions=")?
        .parse()
        .map_err(|_| "invalid Y prefix count")?;
    let mut offsets = vec![0];
    let mut tokens = Vec::new();
    for step in 0..=prefix {
        for t in line(&format!("Y-prefix-{step}="))?.split_whitespace() {
            tokens.push(token(t)?);
        }
        offsets.push(tokens.len() as i32);
    }
    let array = |values: &[i32]| {
        values
            .iter()
            .map(i32::to_string)
            .collect::<Vec<_>>()
            .join(",")
    };
    // Leave carry space for the exact SIC-Zeta numerator 80*c in the baked
    // resident register, in addition to the source's modular arithmetic.
    let width = source.bit_len() + 10;
    let marks: Vec<char> = source.as_word().chars().collect();
    let arms: Vec<i32> = (0..width)
        .map(|i| {
            if i < source.bit_len() && marks[1 + 5 * i + 3] == '⊥' {
                9
            } else {
                8
            }
        })
        .collect();
    let header=format!("#define W {width}\n#define Y_NODE_COUNT {}\n#define Y_ROOT {}\n#define Y_RESUME 0\n#define Y_PREFIX {prefix}\n__constant__ int Y_TAG[]={{{}}};\n__constant__ int Y_LEFT[]={{{}}};\n__constant__ int Y_RIGHT[]={{{}}};\n__constant__ int Y_OFFSETS[]={{{}}};\n__constant__ int Y_TOKENS[]={{{}}};\n__constant__ unsigned char SOURCE_ARMS[]={{{}}};\n",
        tags.len(),stack[0],array(&tags),array(&left),array(&right),array(&offsets),array(&tokens),array(&arms));
    // Reuse the arithmetic implementation, without its unrelated ECM kernels.
    let arithmetic = include_str!("gpu_ecm_word.cu");
    let before_support = arithmetic
        .split("// One warp evaluates")
        .next()
        .ok_or("missing GPU arithmetic")?;
    let gcd_start = arithmetic
        .find("// n is odd, so binary gcd")
        .ok_or("missing GPU gcd")?;
    let gcd_end = arithmetic[gcd_start..]
        .find("__device__ __noinline__ bool proper")
        .ok_or("missing GPU inverse end")?
        + gcd_start;
    let cuda=format!("{header}\n{before_support}\n{}\n__device__ bool proper(const Word &g,const Word &n){{return !zero(g)&&!one(g)&&compare(g,n)<0;}}\n{}",
        &arithmetic[gcd_start..gcd_end],include_str!("idempotent_gpu.cu"));
    std::fs::write(project.join("membrane.cu"), &cuda).map_err(|e| e.to_string())?;
    let device_debug = std::env::var_os("IDEMPOTENT_DEVICE_DEBUG").is_some();
    let options = cudarc::nvrtc::CompileOptions {
        options: if device_debug { vec!["--std=c++17".into(), "--device-debug".into()] }
            else { vec!["--std=c++17".into()] },
        ..Default::default()
    };
    std::eprintln!("preparation: compiling {}-bit CUDA image (device_debug={device_debug})", source.bit_len());
    let ptx = cudarc::nvrtc::compile_ptx_with_opts(cuda, options)
        .map_err(|e| format!("idempotent GPU compilation: {e}"))?;
    std::eprintln!("preparation: CUDA image compiled; preparing resident GPU banks");
    use cudarc::driver::{LaunchConfig, PushKernelArg};
    let clock = std::time::Instant::now();
    let ctx = crate::idempotent_gpu_device::select()?;
    let stream = ctx.default_stream();
    let module = ctx.load_module(ptx.clone()).map_err(|e| e.to_string())?;
    let laws_kernel = module
        .load_function("powerset_monad_laws")
        .map_err(|e| e.to_string())?;
    let mut law_failures = stream.clone_htod(&[u64::MAX]).map_err(|e| e.to_string())?;
    let mut laws_launch = stream.launch_builder(&laws_kernel);
    laws_launch.arg(&mut law_failures);
    unsafe {
        laws_launch.launch(LaunchConfig {
            grid_dim: (1, 1, 1),
            block_dim: (32, 1, 1),
            shared_mem_bytes: 0,
        })
    }
    .map_err(|e| format!("powerset monad-law probe launch: {e}"))?;
    stream.synchronize().map_err(|e| e.to_string())?;
    let law_failures = stream
        .clone_dtoh(&law_failures)
        .map_err(|e| e.to_string())?;
    if law_failures[0] != 0 {
        return Err(format!(
            "GPU powerset monad laws failed: {}",
            law_failures[0]
        ));
    }
    let kernel = module
        .load_function("idempotent_membrane")
        .map_err(|e| e.to_string())?;
    let mut continuation = vec![0i32; 64 * 6146];
    for i in 0..tags.len() {
        continuation[3 * i] = tags[i];
        continuation[3 * i + 1] = left[i];
        continuation[3 * i + 2] = right[i];
    }
    continuation[6144] = tags.len() as i32;
    continuation[6145] = stack[0];
    let mut continuation = stream
        .clone_htod(&continuation)
        .map_err(|e| e.to_string())?;
    let mut state = stream
        .clone_htod(&vec![8u8; 64 * 14 * width])
        .map_err(|e| e.to_string())?;
    let mut receipt = stream
        .clone_htod(&[0u64; 10 + 64 * 10])
        .map_err(|e| e.to_string())?;
    let mut launch = stream.launch_builder(&kernel);
    launch
        .arg(&mut state)
        .arg(&mut continuation)
        .arg(&mut receipt)
        .arg(&32u32)
        .arg(&1i32);
    unsafe {
        launch.launch(LaunchConfig {
            grid_dim: (1, 1, 1),
            block_dim: (32, 1, 1),
            shared_mem_bytes: 0,
        })
    }
    .map_err(|e| e.to_string())?;
    let mut state = stream.clone_dtoh(&state).map_err(|e| e.to_string())?;
    let mut continuation = stream
        .clone_dtoh(&continuation)
        .map_err(|e| e.to_string())?;
    let mut receipt = stream.clone_dtoh(&receipt).map_err(|e| e.to_string())?;
    let seconds = clock.elapsed().as_secs_f64();
    if receipt[3] > 1 {
        return Err(format!(
            "GPU state preparation failed, status {}",
            receipt[3]
        ));
    }
    // Preserve the prepared idempotent if reached, then execute extraction
    // afresh on GPU at trigger. Derived factor words are not baked as inputs.
    state[2 * width..5 * width].fill(8);
    receipt[3] = 0;
    receipt[10 + 3] = 0;
    // Persist the complete current bank; scratch successors are allocated at
    // trigger and never baked as additional prepared alternatives.
    state.truncate(32 * 14 * width);
    continuation.truncate(32 * 6146);
    receipt.truncate(10 + 32 * 10);
    Ok(Prepared {
        ptx: ptx.to_src(),
        state,
        continuation,
        receipt,
        powerset_law_failures: law_failures[0],
        seconds,
    })
}
