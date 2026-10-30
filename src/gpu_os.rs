//! gpu_os.rs — the resident OS exec spine on the card.
//!
//! One kernel launched once that never returns until shut down. It holds a
//! command loop on the GPU and runs the twelve-mark engine, the same
//! `imasm_exec_one` the batch kernel calls, so the card carries one engine.
//! The host is a dumb pump: it writes a program into a mapped pinned mailbox,
//! rings a doorbell, and reads the result back. It computes nothing.
//!
//! The mailbox is mapped pinned host memory, not managed memory, because both
//! cards report concurrent_managed_access=0 under WSL2 (and on the native
//! Windows driver too), so managed memory cannot be touched by the host while a
//! kernel is resident. Mapped pinned memory the host owns and the device sees
//! zero-copy needs no such support.
//!
//! `gpu_os run <word>` drives one word through the resident kernel and checks
//! the device result against the CPU control `cpu_run`, exactly as
//! `gpu_kernel run` does, but through the resident mailbox rather than a fresh
//! launch.

use alloc::format;
use alloc::string::String;
use core::ffi::c_void;
use core::ptr::{read_volatile, write_volatile};

use cudarc::driver::{result, sys, CudaContext, LaunchConfig, PushKernelArg};
use cudarc::nvrtc::compile_ptx;

use crate::gpu_kernel::{b4_name, cpu_run, tok_id_pub, META, SDEPTH};

// The resident kernel: the shared engine plus a doorbell loop. ctrl layout:
//   [0] doorbell : host sets 1 to request; device sets 2 when the result is ready
//   [1] shutdown : host sets 1 to end the resident kernel
//   [2] plen     : program length in marks
// ctrl[3] is the mode: 0 = run a word through the engine, 1 = boot self-imscription.
const RESIDENT_SRC: &str = concat!(
    include_str!("gpu_imasm_engine.cuh"),
    include_str!("gpu_boot.cuh"),
    r#"
extern "C" __global__ void resident_os(
    volatile int* ctrl, unsigned char* prog,
    unsigned char* out_stack, unsigned int* out_meta,
    unsigned long long max_ticks)
{
    while (ctrl[1] == 0) {
        if (ctrl[0] == 1) {
            unsigned int plen = (unsigned int)ctrl[2];
            if (plen > PROGRAM_CAP) plen = PROGRAM_CAP;
            unsigned char P[PROGRAM_CAP];
            for (unsigned int i = 0; i < plen; i++) P[i] = prog[i];
            if (ctrl[3] == 1) {
                boot_snapshot(P, plen, out_meta);
            } else {
                imasm_exec_one(P, plen, max_ticks, out_stack, out_meta);
            }
            __threadfence_system();
            ctrl[0] = 2;               // result ready
            __threadfence_system();
        }
    }
}
"#);

// Allocate a mapped pinned buffer of `bytes`, returning (host_ptr, device_ptr).
// The host uses host_ptr directly; the kernel gets device_ptr as its argument.
unsafe fn mapped(bytes: usize) -> Result<(*mut u8, u64), String> {
    let flags = sys::CU_MEMHOSTALLOC_DEVICEMAP | sys::CU_MEMHOSTALLOC_PORTABLE;
    let hp = result::malloc_host(bytes, flags).map_err(|e| format!("malloc_host: {e:?}"))? as *mut u8;
    let mut dp: sys::CUdeviceptr = 0;
    sys::cuMemHostGetDevicePointer_v2(&mut dp, hp as *mut c_void, 0)
        .result().map_err(|e| format!("host_get_device_pointer: {e:?}"))?;
    for i in 0..bytes { write_volatile(hp.add(i), 0u8); }
    Ok((hp, dp))
}

// Launch the resident kernel once, drive `ids` through the mailbox in the given
// mode (0 = exec a word, 1 = boot self-imscription), read back meta and stack,
// then shut the kernel down. The host only shuttles bytes.
fn drive(ids: &[u8], mode: i32, device: usize) -> Result<([u32; 32], [u8; 256], u64), String> {
    let cap = ids.len().next_power_of_two().max(64);
    let max_ticks: u64 = 65536;
    let ctx = CudaContext::new(device).map_err(|e| format!("no CUDA context: {e}"))?;
    let stream = ctx.default_stream();
    let source = format!("#define PROGRAM_CAP {cap}\n{RESIDENT_SRC}");
    let ptx = compile_ptx(source).map_err(|e| format!("NVRTC: {e}"))?;
    let module = ctx.load_module(ptx).map_err(|e| format!("module: {e}"))?;
    let func = module.load_function("resident_os").map_err(|e| format!("load: {e}"))?;

    let (ctrl_h, ctrl_d, prog_h, prog_d, stack_h, stack_d, meta_h, meta_d) = unsafe {
        let (ch, cd) = mapped(4 * 4)?;
        let (ph, pd) = mapped(cap)?;
        let (sh, sd) = mapped(SDEPTH)?;
        let (mh, md) = mapped(META * 4)?;
        (ch as *mut i32, cd, ph, pd, sh, sd, mh as *mut u32, md)
    };

    let cfg = LaunchConfig { grid_dim: (1, 1, 1), block_dim: (1, 1, 1), shared_mem_bytes: 0 };
    let mt = max_ticks;
    {
        let mut b = stream.launch_builder(&func);
        b.arg(&ctrl_d); b.arg(&prog_d); b.arg(&stack_d); b.arg(&meta_d); b.arg(&mt);
        unsafe { b.launch(cfg) }.map_err(|e| format!("launch: {e}"))?;
    }

    unsafe {
        for (i, &id) in ids.iter().enumerate() { write_volatile(prog_h.add(i), id); }
        write_volatile(ctrl_h.add(2), ids.len() as i32); // plen
        write_volatile(ctrl_h.add(3), mode);             // mode
        write_volatile(ctrl_h.add(0), 1);                // doorbell
    }
    let mut spins: u64 = 0;
    let mut timed_out = false;
    loop {
        if unsafe { read_volatile(ctrl_h.add(0)) } == 2 { break; }
        spins += 1;
        if spins > 20_000_000_000 { timed_out = true; break; }
    }

    let mut g_meta = [0u32; 32];
    let mut g_stack = [0u8; 256];
    unsafe {
        for i in 0..META { g_meta[i] = read_volatile(meta_h.add(i)); }
        for i in 0..SDEPTH { g_stack[i] = read_volatile(stack_h.add(i)); }
        write_volatile(ctrl_h.add(1), 1); // shutdown
    }
    let _ = stream.synchronize();
    unsafe {
        let _ = sys::cuMemFreeHost(ctrl_h as *mut c_void);
        let _ = sys::cuMemFreeHost(prog_h as *mut c_void);
        let _ = sys::cuMemFreeHost(stack_h as *mut c_void);
        let _ = sys::cuMemFreeHost(meta_h as *mut c_void);
    }
    if timed_out { return Err("timeout waiting on resident kernel".into()); }
    Ok((g_meta, g_stack, spins))
}

pub fn run(word: &str, device: usize) -> String {
    use crate::belnap_ring_shor::{glyph_to_token, Glyph};

    // Descending arm: parse the word to token ids, the same mapping the batch
    // kernel uses.
    let mut ids: alloc::vec::Vec<u8> = alloc::vec::Vec::new();
    for (pos, c) in word.chars().filter(|c| !c.is_whitespace()).enumerate() {
        match Glyph::from_char(c) {
            Some(g) => ids.push(tok_id_pub(glyph_to_token(g))),
            None => return format!("gpu_os run: char {pos} ('{c}') is not one of the twelve marks"),
        }
    }
    if ids.is_empty() { return "gpu_os run: empty word".into(); }

    let (g_meta, g_stack, spins) = match drive(&ids, 0, device) {
        Ok(v) => v, Err(e) => return format!("gpu_os run: {e}"),
    };

    let max_ticks: u64 = 65536;
    // Ascending arm: the CPU control, and parity across every field.
    let g_top = g_meta[2] as usize;
    let (c_halt, c_tick, c_depth, c_regs, c_engagr, c_mem, c_stack) = cpu_run(&ids, max_ticks);
    let mut parity = c_halt == g_meta[0] as u8 && c_tick == g_meta[1]
        && c_depth == g_meta[2] && c_engagr == g_meta[11] as u8;
    for i in 0..8 { if c_regs[i] as u32 != g_meta[3 + i] { parity = false; } }
    for a in 0..4 { if c_mem[a] as u32 != g_meta[12 + a] { parity = false; } }
    for i in 0..(c_depth as usize).min(SDEPTH) { if g_stack[i] != c_stack[i] { parity = false; } }

    let mut stack_str = String::new();
    for i in 0..g_top.min(SDEPTH) { if i > 0 { stack_str.push(' '); } stack_str.push_str(b4_name(g_stack[i])); }

    format!(
        "gpu_os resident_os on device {device}: {} marks (spins {spins})\n  halted: {}   ticks: {}\n  stack (bottom..top): [{}]  depth {}\n  matches CPU control: {}",
        ids.len(), g_meta[0] == 1, g_meta[1], stack_str, g_top, parity)
}
