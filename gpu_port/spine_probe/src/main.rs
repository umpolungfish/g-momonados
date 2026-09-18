// spine_probe: prove the resident-megakernel mailbox.
//
// One kernel is launched once and never returns until told to shut down. It
// spins on a doorbell word in managed (unified) memory. The host writes a
// command into that same memory, rings the doorbell, and reads back the
// result. The host does no computation: it only shuttles bytes and checks the
// device answer against a CPU control. A heartbeat counter the kernel bumps
// every loop proves it stayed resident and looping while the host worked.
//
// Mailbox layout (i32 words), all in one managed buffer:
//   [0] doorbell : host sets 1 to request; device sets 2 when the answer is ready
//   [1] cmd      : 1 = increment, 2 = square
//   [2] arg
//   [3] result   : device writes the answer here
//   [4] heartbeat: device increments every spin
//   [5] shutdown : host sets 1 to end the resident kernel

use std::ptr::{read_volatile, write_volatile};

use cudarc::driver::{CudaContext, LaunchConfig, PushKernelArg};
use cudarc::driver::{result, sys};
use cudarc::nvrtc::compile_ptx;

const KERNEL: &str = r#"
extern "C" __global__ void mailbox_server(volatile int* mbox) {
    while (mbox[5] == 0) {
        if (mbox[0] == 1) {
            int cmd = mbox[1];
            int a   = mbox[2];
            int r;
            if (cmd == 1)      r = a + 1;
            else if (cmd == 2) r = a * a;
            else               r = -1;
            mbox[3] = r;
            __threadfence_system();
            mbox[0] = 2;              // answer ready
            __threadfence_system();
        }
        mbox[4] = mbox[4] + 1;       // heartbeat
    }
}
"#;

const DOORBELL: usize = 0;
const CMD: usize = 1;
const ARG: usize = 2;
const RESULT: usize = 3;
const HEARTBEAT: usize = 4;
const SHUTDOWN: usize = 5;
const MBOX_WORDS: usize = 8;

fn cpu_control(cmd: i32, a: i32) -> i32 {
    match cmd {
        1 => a + 1,
        2 => a * a,
        _ => -1,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let device: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let ctx = CudaContext::new(device)?;
    let stream = ctx.default_stream();

    let cma = ctx.attribute(sys::CUdevice_attribute::CU_DEVICE_ATTRIBUTE_CONCURRENT_MANAGED_ACCESS)?;
    let mm = ctx.attribute(sys::CUdevice_attribute::CU_DEVICE_ATTRIBUTE_MANAGED_MEMORY)?;
    println!("device {device}: managed_memory={mm}  concurrent_managed_access={cma}");

    // Mapped pinned mailbox: host owns pinned memory, the device sees it mapped
    // into its address space (zero-copy). This needs no concurrent managed
    // access, which this WSL2 setup does not have, so the host can poll freely
    // while the kernel is resident.
    let bytes = MBOX_WORDS * std::mem::size_of::<i32>();
    let flags = sys::CU_MEMHOSTALLOC_DEVICEMAP | sys::CU_MEMHOSTALLOC_PORTABLE;
    let host = unsafe { result::malloc_host(bytes, flags)? } as *mut i32;
    let mut dptr: sys::CUdeviceptr = 0;
    unsafe { sys::cuMemHostGetDevicePointer_v2(&mut dptr, host as *mut core::ffi::c_void, 0).result()?; }
    unsafe { for i in 0..MBOX_WORDS { write_volatile(host.add(i), 0); } }

    let module = ctx.load_module(compile_ptx(KERNEL)?)?;
    let func = module.load_function("mailbox_server")?;

    // Launch once, async, and never sync until shutdown: the host runs beside it.
    let cfg = LaunchConfig { grid_dim: (1, 1, 1), block_dim: (1, 1, 1), shared_mem_bytes: 0 };
    let ptr_val: u64 = dptr;
    let mut builder = stream.launch_builder(&func);
    builder.arg(&ptr_val);
    unsafe { builder.launch(cfg)?; }

    // Drive a battery of commands through the mailbox. Host only shuttles.
    let battery: [(i32, i32); 6] = [(1, 41), (2, 12), (1, 999), (2, 40), (1, 0), (2, 7)];
    let mut all_ok = true;
    for (n, &(cmd, a)) in battery.iter().enumerate() {
        unsafe {
            write_volatile(host.add(CMD), cmd);
            write_volatile(host.add(ARG), a);
            write_volatile(host.add(DOORBELL), 1); // ring
        }
        // spin until the device answers
        let mut spins: u64 = 0;
        loop {
            let db = unsafe { read_volatile(host.add(DOORBELL)) };
            if db == 2 { break; }
            spins += 1;
            if spins > 5_000_000_000 { println!("  TIMEOUT waiting on device"); all_ok = false; break; }
        }
        let got = unsafe { read_volatile(host.add(RESULT)) };
        let want = cpu_control(cmd, a);
        let ok = got == want;
        all_ok &= ok;
        println!("  cmd#{n}: cmd={cmd} arg={a} -> device={got} control={want}  {}", if ok { "match" } else { "DIVERGE" });
        unsafe { write_volatile(host.add(DOORBELL), 0); } // clear for next
    }

    // Read the heartbeat before shutdown: proves the kernel looped resident.
    let hb = unsafe { read_volatile(host.add(HEARTBEAT)) };
    println!("heartbeat spins while resident: {hb}");

    // Tell the resident kernel to exit, then it is safe to sync.
    unsafe { write_volatile(host.add(SHUTDOWN), 1); }
    stream.synchronize()?;
    unsafe { sys::cuMemFreeHost(host as *mut core::ffi::c_void).result()?; }

    println!("RESULT: {}", if all_ok { "all commands matched control; resident mailbox works" } else { "FAILURE" });
    if !all_ok { std::process::exit(1); }
    Ok(())
}
