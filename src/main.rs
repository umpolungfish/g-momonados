#![cfg_attr(not(feature = "hosted"), no_std)]
// The IG primitives are Shavian glyphs and the glyph is the name; spelling them
// in Latin would be a different notation, so the lint is answered rather than
// obeyed.
#![allow(uncommon_codepoints)]
// N is the modulus, B a bound, and in `a^x mod N` the capital is the name the
// mathematics uses. Renaming them to satisfy the lint would make the code
// disagree with every statement of the algorithm it implements, so this lint is
// answered on the same grounds as the one above rather than obeyed.
#![allow(non_snake_case)]
#![cfg_attr(not(feature = "hosted"), no_main)]
#![cfg_attr(not(feature = "hosted"), feature(abi_x86_interrupt))]
#![cfg_attr(not(feature = "hosted"), feature(alloc_error_handler))]
#![allow(dead_code)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::approx_constant)]
#![allow(clippy::eq_op)]

// Needed in both builds: the portable modules use alloc::vec::Vec. On a host
// this resolves against the std-provided alloc, which is why build-std must be
// off for the hosted target -- rebuilding alloc there duplicates its lang items.
extern crate alloc;

// Hosted diagnostics are terminal surfaces of the active IMASM vessel.  The
// local stderr macro routes unqualified diagnostics; qualified calls use the
// explicit runtime writer.
macro_rules! eprintln {
    ($($arg:tt)*) => {{
        crate::nested_eprintln!($($arg)*);
    }};
}

#[cfg(not(feature = "hosted"))]
use core::panic::PanicInfo;
#[cfg(not(feature = "hosted"))]
use core::sync::atomic::{AtomicUsize, Ordering};
#[cfg(not(feature = "hosted"))]
use core::alloc::Layout;

mod runtime_nesting;
mod serial;
#[macro_use]
mod style;
mod basin;
mod belnap;
mod tokens;
mod counterfactual;
mod shiab;
mod hyperstack;
mod prooflift;
mod crystal;
mod kernel;
mod vita;
#[cfg(not(feature = "hosted"))]
mod interrupts;
#[cfg(feature = "hosted")]
#[path = "interrupts_hosted.rs"]
mod interrupts;
#[cfg(feature = "hosted")]
mod gpu_sixteen3;
#[cfg(feature = "hosted")]
mod gpu_prime;
#[cfg(feature = "hosted")]
mod gpu_kernel;
#[cfg(feature = "hosted")]
mod gpu_os;
#[cfg(feature = "hosted")]
mod gpu_graph;
mod word_nesting;
mod token_refinement;
mod factor_relation;
#[cfg(feature = "hosted")]
mod gpu_factor;
#[cfg(feature = "hosted")]
mod gpu_vox;
#[cfg(feature = "hosted")]
mod gpu_fde;
#[cfg(feature = "hosted")]
mod gpu_rho;
mod gpu_rho_ml;
#[cfg(feature = "hosted")]
mod gpu_ecm;
#[cfg(feature = "hosted")]
mod gpu_trilattice;
#[cfg(feature = "hosted")]
mod gpu_abc;
#[cfg(feature = "hosted")]
mod gpu_gnfs_poly;
#[cfg(feature = "hosted")]
mod gpu_gnfs_sieve;
#[cfg(feature = "hosted")]
mod gpu_gnfs_linalg;
#[cfg(feature = "hosted")]
mod gpu_gnfs_linalg_gpu;
#[cfg(feature = "hosted")]
mod gpu_gnfs_cong_ml;
#[cfg(feature = "hosted")]
mod gpu_gnfs_fb;
#[cfg(feature = "hosted")]
mod gpu_gnfs;
#[cfg(feature = "hosted")]
mod gpu_dqi;
#[cfg(feature = "hosted")]
mod gpu_opi;
#[cfg(feature = "hosted")]
mod gpu_shor;
#[cfg(feature = "hosted")]
mod gpu_millennium;
#[cfg(feature = "hosted")]
mod gpu_native_protocol;
#[cfg(feature = "hosted")]
mod gpu_catalog_crystal;
#[cfg(feature = "hosted")]
mod gpu_imasm_cycle;
#[cfg(feature = "hosted")]
mod gpu_native_cycle;
#[cfg(feature = "hosted")]
mod gpu_crystal_full_space;
#[cfg(feature = "hosted")]
mod gpu_ipc_no_serialization;
#[cfg(feature = "hosted")]
mod gpu_sixteen3_tensor_kernel;
#[cfg(feature = "hosted")]
mod gpu_dqi_xorsat;
mod frob_verify;
mod imas_ig;
mod aleph;
mod manus;
mod frobenius_fuzzer;
mod oracle;
mod blackbox;
mod axis_values;
mod dialetheic_compiler;
mod dialetheic_fib_shor;
mod stark_geometer;
mod dialect_necromancer;
mod braid_apocrypha;
mod proof_braider;
mod universe_wormhole;
mod vox_ce;
mod consciousness_lath;
mod paradox_engine;
mod key_dissolver;
mod compiler;
mod catalogue;
mod museum;
mod phase;
mod pk2sk;
mod qft;
mod shors_btc_2;
mod secp256k1_unwinder;
mod baryon_asymmetry;
mod btc_secret_key_oneshot;
mod moDOT_alchemy;
mod pari_integration;
mod tower_polynomials;
mod parasm;
mod belnap_shor;
pub mod prime_winding;
pub mod trilattice_factor;
pub mod native_numeral;
pub mod word_tape;
pub mod gaussian_extract;
pub mod abc_iutt;
pub mod abc_certificate;
mod belnap_shor_factors;
mod fibonacci_shor;
mod fibonacci_shor_plan;
mod belnap_ring_shor;
mod belnap_phase_shor;
mod phase_unbraid;
mod para_rh;
mod para_ym;
mod para_temporal;
mod para_category;
mod algebra;
mod canonical_ig;
mod catalog;
mod cl8nk;
mod consciousness;
mod rebis;
mod demonstrator;
mod dialect;
mod menu;
mod sequence;
// The multiboot1 stub and 32->64 bit trampoline. A hosted process is entered
// by the host loader, which has already done all of it.
#[cfg(not(feature = "hosted"))]
mod boot;
mod cr3echrz;
mod canonical_ordinal;
mod clay_status;
mod sic_povm;
mod frobenius_unify;
mod clay_witness;
mod belnap_sic_bridge;
mod belnap_c4;
mod shadow;
mod sic_compute;
mod dialect_expansion;
mod divisor_ring;
mod mersenne_parallel;
mod bifurcation_test;
mod entropy;
mod d12_sic;
mod d2048_sic;
mod bip39_sic_grover;
mod d2048_sieve;
mod provenance;
mod quadratic;
mod dqi;
mod dqi_ambient;
mod yz;
mod yz_list;
mod shor_qft;
mod factor_phase;
mod opi;
mod weight_ladder;
mod multilattice;
mod fde;
mod rsa_decrypter;
mod combo;
mod millennium;
mod sic_moduli;
mod riemann_sic;
mod riemann_hilbert;
mod witness;
mod witness_vessel;
mod ask;
mod proof;
mod seals;
mod constant_closure;
mod repl;
mod fibonacci_qc;
#[cfg(feature = "hosted")]
pub mod anyon_braid_cnot;
#[cfg(feature = "hosted")]
mod anyon_device;
mod winding_period;
mod oneshot_prime_winder;
mod nested_oneshot;
mod doubly_nested_oneshot;
mod nested_prime_factorization;
mod dynamic_nesting_prime_finder;
mod closure_nested;
mod factor_operator;
mod factor_membrane;
use g_momonados::arbitrary_factor;
mod membrane_family;
mod shor_b4_membrane;
mod coupled_bridge;
mod tower_vox;
mod lattice_flow;
mod triple_frame;
mod iuft_qc;
mod iuft_teichmuller;
mod vox;
mod vox_decode;
mod imasm_exec;
mod circuit;

// ── m3iosis tool ports (native Rust implementations) ────────
mod stark;
mod hqe;
mod dyson;
mod afdmc;
mod d2048_exact_sic;
mod troq;
mod hop;
mod braid_grammar;
mod braid_render;
mod braid_protocol;
mod text;
mod loss;
mod manifold;
mod kernel_torus;
mod ouroboros;
mod ovm;
mod exotic_one_shots;
mod crystal_scope;
mod ctc;
mod ctc_loom;
mod nesting;
mod carriers;
mod collatz;
mod straus;
mod erdos_walks;
mod fold_walk;
mod substrate;
mod invariant;
mod lean_census;
mod redteam;
mod minimal;
mod repair;
mod ringspec;
mod sk_forge;

use tokens::{canonical_count, continuous_count, novel_count, shunted_count};
use crystal::TOTAL;
use kernel::Kernel;
// ─── Bump allocator (no external crates) ─────────────────────

#[cfg(not(feature = "hosted"))]
#[repr(C, align(4096))]
struct HeapStorage([u8; 48 * 1024 * 1024]);
#[cfg(not(feature = "hosted"))]
static mut HEAP_STORAGE: HeapStorage = HeapStorage([0; 48 * 1024 * 1024]);

#[cfg(not(feature = "hosted"))]
struct BumpAllocator {
    next: AtomicUsize,
    end:  AtomicUsize,
}

#[cfg(not(feature = "hosted"))]
impl BumpAllocator {
    const fn new() -> Self {
        Self { next: AtomicUsize::new(0), end: AtomicUsize::new(0) }
    }
    fn init(&self, start: usize, size: usize) {
        self.next.store(start, Ordering::Relaxed);
        self.end.store(start + size, Ordering::Relaxed);
    }
}

#[cfg(not(feature = "hosted"))]
unsafe impl core::alloc::GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let align = layout.align();
        let size  = layout.size();
        // A zero-size request must never fail. With the heap nearly full the
        // alignment round-up alone can push past `end`, and returning null for a
        // zero-byte allocation surfaced as "memory allocation of 0 bytes failed"
        // — a panic that named a size nobody had asked for. Hand back a dangling
        // but correctly aligned pointer, which is what a zero-size allocation is.
        if size == 0 {
            return align as *mut u8;
        }
        loop {
            let cur     = self.next.load(Ordering::Relaxed);
            let aligned = (cur + align - 1) & !(align - 1);
            let new     = aligned + size;
            // Null here reaches `alloc_error` below, which reports the real
            // layout. Nothing is printed at this point so a caller that
            // handles the null itself is not made to look like a crash.
            if new > self.end.load(Ordering::Relaxed) { return core::ptr::null_mut(); }
            if self.next.compare_exchange_weak(
                cur, new, Ordering::Relaxed, Ordering::Relaxed,
            ).is_ok() {
                return aligned as *mut u8;
            }
        }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // LIFO reclaim: if this block is the most recent allocation, roll the
        // bump pointer back. Catches the transient Vecs of a forward pass.
        let end = ptr as usize + layout.size();
        let _ = self.next.compare_exchange(
            end, ptr as usize, Ordering::Relaxed, Ordering::Relaxed,
        );
    }
}

#[cfg(not(feature = "hosted"))]
#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator::new();

/// Mark/reset scope for transient heavy work (the vita turn): everything
/// allocated after `heap_mark()` is reclaimed by `heap_reset(mark)`. Only
/// sound when nothing allocated inside the scope outlives it.
#[cfg(not(feature = "hosted"))]
pub fn heap_mark() -> usize {
    ALLOCATOR.next.load(Ordering::Relaxed)
}
#[cfg(not(feature = "hosted"))]
pub fn heap_reset(mark: usize) {
    ALLOCATOR.next.store(mark, Ordering::Relaxed);
}
/// Bytes currently handed out, and the arena total. A bump allocator that has
/// run out returns null and the allocation error path takes the kernel down
/// without a message, so anything heavy should report this rather than let
/// exhaustion look like a hang.
#[cfg(not(feature = "hosted"))]
pub fn heap_used() -> (usize, usize) {
    let next = ALLOCATOR.next.load(Ordering::Relaxed);
    let end = ALLOCATOR.end.load(Ordering::Relaxed);
    let start = unsafe { core::ptr::addr_of!(HEAP_STORAGE.0) as usize };
    (next.saturating_sub(start), end.saturating_sub(start))
}

// ─── Kernel stack + bare-metal entry ─────────────────────────

#[cfg(not(feature = "hosted"))]
#[repr(C, align(16))]
struct KernelStack([u8; 128 * 1024]);
#[cfg(not(feature = "hosted"))]
static BOOT_STACK: KernelStack = KernelStack([0; 128 * 1024]);

#[cfg(not(feature = "hosted"))]
#[no_mangle]
#[unsafe(naked)]
pub unsafe extern "C" fn _rust_start() -> ! {
    core::arch::naked_asm!(
        "lea rax, [{stack}]",
        "add rax, {size}",
        "and rax, -16",
        "mov rsp, rax",
        "xor rbp, rbp",
        "call {entry}",
        "2:",
        "hlt",
        "jmp 2b",
        stack = sym BOOT_STACK,
        size  = const core::mem::size_of::<KernelStack>(),
        entry = sym rust_start,
    );
}

#[cfg(not(feature = "hosted"))]
extern "C" fn rust_start() -> ! {
    unsafe {
        ALLOCATOR.init(
            core::ptr::addr_of_mut!(HEAP_STORAGE.0) as usize,
            core::mem::size_of::<HeapStorage>(),
        );
    }
    kmain()
}

// ── Hosted runtime ───────────────────────────────────────────────────
// On a host the OS supplies what the bare-metal half builds by hand: an
// allocator, a stack, a panic path, and an entry point. So there is nothing to
// port here, only to step aside for.

// The host allocator is wrapped so its use is measured rather than assumed.
// An earlier version returned a constant 48 MB here on the grounds that it kept
// size decisions identical across builds. That is a hardcode standing in for a
// measurement, which is the thing this project exists to not do: every value
// should be computed. `used` below is now real, counted allocation by
// allocation, and works anywhere Rust runs.
#[cfg(feature = "hosted")]
mod hosted_heap {
    use core::sync::atomic::{AtomicUsize, Ordering};
    use std::alloc::{GlobalAlloc, Layout, System};

    pub static USED: AtomicUsize = AtomicUsize::new(0);

    /// Not a measurement and not pretending to be. A host has no fixed arena,
    /// so there is no total to read; this is a declared ceiling that the
    /// heavy-job guards compare against, defaulting to the bare-metal arena so
    /// the two builds refuse the same work. Settable at runtime.
    pub static BUDGET: AtomicUsize = AtomicUsize::new(48 * 1024 * 1024);

    pub struct Counting;

    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, l: Layout) -> *mut u8 {
            let p = System.alloc(l);
            if !p.is_null() { USED.fetch_add(l.size(), Ordering::Relaxed); }
            p
        }
        unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
            System.dealloc(p, l);
            USED.fetch_sub(l.size(), Ordering::Relaxed);
        }
    }
}

/// Live corner HUD — background thread that displays register state in terminal corner.
#[cfg(feature = "hosted")]
mod live_hud {
    use core::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Mutex, OnceLock};
    use std::thread;

    // Shared kernel state for HUD
    #[derive(Default, Clone)]
    pub struct HudState {
        pub tick_count: u64,
        pub ip: usize,
        pub program_len: usize,
        pub tier: u8,
        pub frob_checks: u64,
        pub frob_open: u64,
        pub b_live_ticks: u64,
        pub gate_discriminations: u64,
        pub value_period: usize,
        pub sig: (usize, usize, usize, usize),
        pub token_diversity: usize,
        pub self_ref: bool,
        pub halted: bool,
    }

    static HUD_STATE: OnceLock<Mutex<HudState>> = OnceLock::new();
    static HUD_RUNNING: AtomicBool = AtomicBool::new(false);
    static HUD_THREAD: OnceLock<Mutex<Option<thread::JoinHandle<()>>>> = OnceLock::new();

    fn get_state() -> &'static Mutex<HudState> {
        HUD_STATE.get_or_init(|| Mutex::new(HudState::default()))
    }

    fn get_thread_handle() -> &'static Mutex<Option<thread::JoinHandle<()>>> {
        HUD_THREAD.get_or_init(|| Mutex::new(None))
    }

    pub fn update_hud_state(state: HudState) {
        if let Some(mut guard) = get_state().lock().ok() {
            *guard = state;
        }
    }

    fn draw_corner_hud(_state: &HudState) {
        // HUD fully disabled — no ANSI output
    }

    pub fn start_hud() {
        // HUD fully disabled — no background thread, no ANSI cursor movement
        if HUD_RUNNING.swap(true, Ordering::Relaxed) {
            return; // Already running
        }
    }

    pub fn stop_hud() {
        HUD_RUNNING.store(false, Ordering::Relaxed);
        if let Some(handle) = get_thread_handle().lock().unwrap().take() {
            handle.join().ok();
        }
    }

    pub fn update_from_kernel(k: &crate::kernel::Kernel) {
        let state = HudState {
            tick_count: k.tick_count,
            ip: k.ip,
            program_len: k.program.len(),
            tier: k.snapshot.map(|s| s.tier).unwrap_or(0),
            frob_checks: k.frob_checks,
            frob_open: k.frob_open,
            b_live_ticks: k.snapshot.map(|s| s.b_live_ticks).unwrap_or(0),
            gate_discriminations: k.snapshot.map(|s| s.gate_discriminations).unwrap_or(0),
            value_period: k.snapshot.map(|s| s.value_period).unwrap_or(0),
            sig: k.snapshot.map(|s| s.sig).unwrap_or((0,0,0,0)),
            token_diversity: k.snapshot.map(|s| s.token_diversity).unwrap_or(0),
            self_ref: k.snapshot.map(|s| s.self_ref).unwrap_or(false),
            halted: k.halted,
        };
        update_hud_state(state);
    }
}

#[cfg(feature = "hosted")]
#[global_allocator]
static ALLOCATOR: hosted_heap::Counting = hosted_heap::Counting;

/// A mark is a real position in the measured total, so callers can see what a
/// scope cost. Reset cannot roll a system allocator back and does not pretend
/// to -- the allocator reclaims on drop, which is the same outcome by another
/// route.
#[cfg(feature = "hosted")]
pub fn heap_mark() -> usize {
    hosted_heap::USED.load(core::sync::atomic::Ordering::Relaxed)
}
#[cfg(feature = "hosted")]
pub fn heap_reset(_mark: usize) {}

#[cfg(feature = "hosted")]
pub fn heap_used() -> (usize, usize) {
    use core::sync::atomic::Ordering;
    (hosted_heap::USED.load(Ordering::Relaxed),
     hosted_heap::BUDGET.load(Ordering::Relaxed))
}

#[cfg(feature = "hosted")]
fn main() {
    if option_env!("PHASE_UNBRAID_BAKED_EXECUTE") == Some("1") {
        let base = option_env!("PHASE_UNBRAID_BAKED_BASE").unwrap_or("2");
        let shots = option_env!("PHASE_UNBRAID_BAKED_SHOTS").unwrap_or("8");
        let tape = alloc::format!("measurements/qpe-baked-{}", std::process::id());
        let report = phase_unbraid_gpu_baked(&[base, "4096", &tape, shots]);
        let closed = report.contains("verified word product = true");
        println!("{report}");
        std::process::exit(if closed { 0 } else { 2 });
    }
    let argv: Vec<String> = std::env::args().skip(1).collect();
    if !argv.is_empty() {
        // Direct CLI: word-valued factoring routes bypass the REPL line reader.
        let head = argv[0].as_str();
        if matches!(head, "gpu_gnfs" | "gpu-gnfs") {
            if argv.len() < 2 || argv[1] == "help" {
                println!("{}", gpu_gnfs::help());
                return;
            }
            if argv[1] == "blueprint" {
                println!("{}", gpu_gnfs::blueprint());
                return;
            }
            let bound = argv.get(2).and_then(|value| value.parse().ok()).unwrap_or(0);
            println!("{}", gpu_gnfs::run_factor(&argv[1], bound));
            return;
        }
        if matches!(head, "factor_membrane" | "membrane" | "fmembrane") {
            let refs: Vec<&str> = argv[1..].iter().map(|x| x.as_str()).collect();
            println!("{}", factor_membrane::repl_factor_membrane(&refs));
            return;
        }
        if matches!(head, "arbitrary_anyon_factor" | "arbitrary-anyon-factor") {
            let refs: Vec<&str> = argv[1..].iter().map(String::as_str).collect();
            if refs.is_empty() || refs[0] == "help" {
                println!("arbitrary_anyon_factor <natural|canonical-cell-binary-word> <base> <max_shots> <unix_socket>");
                return;
            }
            match arbitrary_anyon_factor(&refs) {
                Ok(report) => {
                    print!("{}", report.render());
                    if !report.verified { std::process::exit(2); }
                }
                Err(error) => {
                    eprintln!("arbitrary_anyon_factor: {error}");
                    std::process::exit(2);
                }
            }
            return;
        }
        if head == "sic" {
            let frame = g_momonados::sic::TetraSic::new();
            match g_momonados::sic::certificate::SicCertificate::measure(frame.frame()) {
                Ok(mut certificate) => {
                    certificate.exact = Some(g_momonados::sic::exact::certify_tetrahedron());
                    print!("{}", certificate.report(g_momonados::sic::certificate::EvidencePolicy::new(1e-12, 1e-8).unwrap()));
                }
                Err(error) => { eprintln!("sic: {error}"); std::process::exit(2); }
            }
            return;
        }
        if matches!(head, "arbitrary_factor" | "arbitrary-factor") {
            let input = argv.get(1).map(String::as_str).unwrap_or("");
            if input.is_empty() || input == "help" {
                println!("arbitrary_factor <natural-number|canonical-cell-binary-word>");
                return;
            }
            match arbitrary_factor::extract(input) {
                Ok(report) => {
                    println!("{}", report.render());
                    if !report.verified { std::process::exit(2); }
                }
                Err(error) => {
                    eprintln!("arbitrary_factor: {error}");
                    std::process::exit(2);
                }
            }
            return;
        }
        if matches!(head, "phase_unbraid" | "phase" | "unbraid") {
            let refs: Vec<&str> = argv[1..].iter().map(|x| x.as_str()).collect();
            println!("{}", dispatch_phase_unbraid(&refs));
            return;
        }
        if head == "anyon_ququart_word" {
            let refs: Vec<&str> = argv[1..].iter().map(|arg| arg.as_str()).collect();
            match anyon_braid_cnot::compile_ququart_fourier(&refs) {
                Ok(report) => println!("{report}"),
                Err(error) => { eprintln!("{error}"); std::process::exit(2); }
            }
            return;
        }
        if head == "anyon_cnot_word" {
            let refs: Vec<&str> = argv[1..].iter().map(|arg| arg.as_str()).collect();
            match anyon_braid_cnot::compile(&refs) {
                Ok(report) => println!("{report}"),
                Err(error) => {
                    eprintln!("{error}");
                    std::process::exit(2);
                }
            }
            return;
        }
        if head == "anyon_cnot_verify" {
            let refs: Vec<&str> = argv[1..].iter().map(|arg| arg.as_str()).collect();
            match anyon_braid_cnot::verify_file(&refs) {
                Ok(report) => println!("{report}"),
                Err(error) => {
                    eprintln!("{error}");
                    std::process::exit(2);
                }
            }
            return;
        }
        if head == "anyon_factor" {
            let refs: Vec<&str> = argv[1..].iter().map(|arg| arg.as_str()).collect();
            match anyon_factor(&refs) {
                Ok(report) => println!("{report}"),
                Err(error) => {
                    eprintln!("{error}");
                    std::process::exit(2);
                }
            }
            return;
        }
        if matches!(head, "factor_big" | "factorbig" | "bigfactor") {
            let refs: Vec<&str> = argv[1..].iter().map(|x| x.as_str()).collect();
            println!("{}", winding_period::repl_factor_big_wiring(&refs));
            return;
        }
        if matches!(head, "shor_b4" | "shor_b4_membrane" | "key_membrane") {
            let refs: Vec<&str> = argv[1..].iter().map(|s| s.as_str()).collect();
            println!("{}", shor_b4_membrane::repl_shor_b4_membrane(&refs));
            return;
        }
        if matches!(head, "membrane_family" | "mfam") {
            let refs: Vec<&str> = argv[1..].iter().map(|s| s.as_str()).collect();
            println!("{}", membrane_family::repl_membrane(&refs));
            return;
        }
        if head == "native_numeral" {
            let refs: Vec<&str> = argv[1..].iter().map(|s| s.as_str()).collect();
            println!("{}", native_numeral_cli(&refs));
            return;
        }
        if head == "closure_nested" {
            let refs: Vec<&str> = argv[1..].iter().map(|s| s.as_str()).collect();
            println!("{}", closure_nested::repl_closure_nested(&refs));
            return;
        }
        if head == "tower_vox" {
            let refs: Vec<&str> = argv[1..].iter().map(|s| s.as_str()).collect();
            println!("{}", tower_vox::repl_tower_vox(&refs));
            return;
        }
        if head == "dqi_ambient" {
            let refs: Vec<&str> = argv[1..].iter().map(|s| s.as_str()).collect();
            dqi_ambient::repl_dqi_ambient(&refs);
            return;
        }
        // Same minimal shape as factor_membrane: a factoring build transforms
        // the contained value into factors and does nothing else, so this
        // never touches Kernel/boot — just winding_period::factor itself.
        if matches!(head, "winding" | "wperiod") {
            let sub = argv.get(1).map(|s| s.as_str()).unwrap_or("");
            if sub == "factor" {
                let n: u64 = argv.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
                let tries: u32 = argv.get(3).and_then(|s| s.parse().ok()).unwrap_or(12);
                let seed: u64 = argv.get(4).and_then(|s| s.parse().ok()).unwrap_or(0x9E37_79B9_7F4A_7C15);
                match winding_period::factor(n, tries, seed) {
                    Some((a, r, p, q)) => println!("FACTORED {} = {} × {}  (a={}, r={})  p·q==N: {}", n, p, q, a, r, p * q == n),
                    None => println!("factor({}): no factor in {} tries", n, tries),
                }
                return;
            }
            if sub == "order" {
                let a: u64 = argv.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
                let n: u64 = argv.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);
                match winding_period::winding_order(a, n) {
                    Some(r) => println!("winding_order({}, {}) = {}  (a^r == 1 mod N, the closure winding)", a, n, r),
                    None => println!("winding_order({}, {}): no period (a not in the unit group)", a, n),
                }
                return;
            }
        }
        // The nested hosted executable vessel reads its own argv in admit();
        // route the relation flags to it rather than the REPL seed.
        if matches!(head, "--selector-relation" | "--selector-relation-stdin") {
            gpu_kernel::run_hosted_executable();
            return;
        }
        // Every other command is a REPL verb (`shor-qft run 7 15 8`,
        // `shor-qft run 7 15 8`, …) — none of them worth hand-adding a fast
        // path for one at a time. Run the real boot, then this one line as
        // if typed at the prompt, then quit: every command on every menu is
        // reachable this way, through argv, with no stdin pipe needed.
        let mut seed: alloc::collections::VecDeque<String> = alloc::collections::VecDeque::new();
        seed.push_back(argv.join(" "));
        seed.push_back("quit".to_string());
        kmain_seeded(seed);
        return;
    }
    kmain()
}

#[cfg(feature = "hosted")]
fn arbitrary_anyon_factor(args: &[&str]) -> Result<arbitrary_factor::Extraction, String> {
    use num_bigint::BigUint;
    let usage = "usage: arbitrary_anyon_factor <natural|canonical-cell-binary-word> <base> <max_shots> <unix_socket>";
    if args.len() != 4 {
        return Err(usage.into());
    }
    let base = args[1].parse::<BigUint>().map_err(|_| usage)?;
    let shots = args[2].parse::<u32>().map_err(|_| usage)?;
    if base < BigUint::from(2u32) || shots == 0 {
        return Err("arbitrary anyonic extraction requires base >= 2 and a positive shot budget".into());
    }
    arbitrary_factor::extract_with_anyons(args[0], |source| {
        let mut service = anyon_device::FibonacciGenerator::connect(args[3])?;
        let result = service.factor_source(source, &base, shots)?;
        Ok(result.map(|readout| arbitrary_factor::AnyonCandidate {
            factor: readout.p,
            detail: format!("base={} order={} shots={}", base, readout.order, readout.shots),
        }))
    })
}

#[cfg(feature = "hosted")]
fn anyon_factor(args: &[&str]) -> Result<String, String> {
    use num_bigint::BigUint;

    const MAX_TOTAL_SHOTS: u32 = 32;
    const DEFAULT_SOCKET: &str = "/tmp/g-momonados-fibonacci.sock";

    let usage = "usage: g-momonados anyon_factor <N>=128-bit-semiprime";
    if args.len() != 1 {
        return Err(usage.into());
    }

    let n = args[0].parse::<BigUint>().map_err(|_| usage)?;
    if n.bits() < 128 {
        return Err(usage.into());
    }

    let socket = std::env::var("G_MOMONADOS_ANYON_SOCKET")
        .unwrap_or_else(|_| DEFAULT_SOCKET.to_string());
    let mut service = anyon_device::FibonacciGenerator::connect(&socket)?;
    let result = service.factor_source(&n, &BigUint::from(2u8), MAX_TOTAL_SHOTS)?
        .ok_or("factor service exhausted its shot budget")?;

    Ok(format!(
        "Fibonacci anyon N-only closure\nsource={}\nbase={}\norder={}\nfactors={} x {}\nshots={}\nproduct_closed={}",
        n,
        2,
        result.order,
        result.p,
        result.q,
        result.shots,
        &result.p * &result.q == n,
    ))
}

fn repl_phase_unbraid_tape(args: &[&str]) -> String {
    let Some(n) = args.first() else { return "phase_unbraid tape: missing N".into() };
    let base = args.get(1).copied().unwrap_or("2");
    let shots = args.get(2).copied().unwrap_or("8");
    let tile_size = args.get(3).copied().unwrap_or("4096");
    let default_tape = alloc::format!("measurements/g-momonados-qft-tape-{}", std::process::id());
    let tape_path = args.iter().position(|arg| *arg == "--tape")
        .and_then(|position| args.get(position + 1)).copied().unwrap_or(&default_tape);
    let gpu_args = [*n, base, tile_size, tape_path, shots];
    phase_unbraid_gpu(&gpu_args)
}

fn dispatch_phase_unbraid(args: &[&str]) -> String {
    match args.first().copied() {
        None | Some("help") => "phase_unbraid <N|word> [base=2] [shots=8] [batch_amplitudes=4096] [--tape <fresh_dir>]\nphase_unbraid gpu <N|word> [base=2] [batch_amplitudes=4096] [fresh_tape_dir] [shots=8]\nphase_unbraid gpu-baked [base=2] [batch_amplitudes=4096] [fresh_tape_dir] [shots=8]\nRecycled control QPE measures all phase bits with a retained complex residue register.\nA baked executable uses PHASE_UNBRAID_BAKED_EXECUTE=1 and FACTOR_PHASE_SOURCE_WORD at build time.".into(),
        Some("gpu-baked") => phase_unbraid_gpu_baked(&args[1..]),
        Some("gpu") => phase_unbraid_gpu(&args[1..]),
        _ => repl_phase_unbraid_tape(args),
    }
}

fn phase_unbraid_gpu(args: &[&str]) -> String {
    let Some(n_text) = args.first() else {
        return "usage: phase_unbraid gpu <N> [base=2] [tile_amplitudes=4096] [tape_dir] [shots=8]".into();
    };
    let parsed = n_text.parse::<num_bigint::BigUint>().ok().or_else(|| {
        let tape = vox_core::morphism_factor::parse_numeral(n_text).ok()?;
        Some(num_bigint::BigUint::new(vox_core::morphism_factor::tape_to_limbs(&tape, (tape.len() + 31) / 32)))
    });
    let n = match parsed {
        Some(value) if value.bits() >= 128 => value,
        _ => return "phase_unbraid gpu: N must be a semiprime of at least 128 bits".into(),
    };
    let mut base = args.get(1).and_then(|s| s.parse::<num_bigint::BigUint>().ok()).unwrap_or_else(|| num_bigint::BigUint::from(2u8));
    if base < num_bigint::BigUint::from(2u8) { base = num_bigint::BigUint::from(2u8); }
    let tile_amplitudes = args.get(2).and_then(|s| s.parse::<u64>().ok()).unwrap_or(4096);
    let tape_path = args.get(3).map(|path| (*path).to_string()).unwrap_or_else(|| {
        alloc::format!("measurements/g-momonados-qft-tape-{}", std::process::id())
    });
    let shots = args.get(4).and_then(|s| s.parse::<u32>().ok()).unwrap_or(8);
    let qft = match gpu_kernel::CudaQft::new(0) {
        Ok(qft) => qft,
        Err(error) => return format!("phase_unbraid gpu: {error}"),
    };
    match phase_unbraid::recycled_phase_report(n, base, tile_amplitudes, &tape_path, shots,
        |low, high, feedback, format| qft.recycled_phase_mix(low, high, feedback, format)) {
        Ok(report) => format!("CUDA phase execution on device 0\n{report}"),
        Err(error) => format!("phase_unbraid gpu: {error}"),
    }
}

fn phase_unbraid_gpu_baked(args: &[&str]) -> String {
    let source = option_env!("FACTOR_PHASE_SOURCE_WORD")
        .unwrap_or("⊢≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋⊙⊡⊣");
    let tape = match vox_core::morphism_factor::parse_numeral(source) {
        Ok(tape) => tape,
        Err(_) => return "phase_unbraid gpu-baked: malformed FACTOR_PHASE_SOURCE_WORD".into(),
    };
    let limbs = vox_core::morphism_factor::tape_to_limbs(&tape, (tape.len() + 31) / 32);
    let n = num_bigint::BigUint::new(limbs).to_string();
    let base = args.first().copied().unwrap_or("2");
    let tile_size = args.get(1).copied().unwrap_or("4096");
    let tape_path = args.get(2).map(|path| (*path).to_string()).unwrap_or_else(|| {
        alloc::format!("measurements/g-momonados-qft-baked-{}", std::process::id())
    });
    let shots = args.get(3).copied().unwrap_or("8");
    let tape_args = vec![n, base.to_string(), tile_size.to_string(), tape_path, shots.to_string()];
    let tape_refs: Vec<&str> = tape_args.iter().map(String::as_str).collect();
    format!("baked source word: {source}\n{}", phase_unbraid_gpu(&tape_refs))
}

#[cfg(feature = "hosted")]
fn native_numeral_cli(args: &[&str]) -> String {
    if args.is_empty() { return native_numeral::help(); }
    match args[0] {
        "encode" if args.len() > 1 => native_numeral::encode(args[1]),
        "decode" if args.len() > 1 => native_numeral::decode_report(args[1]),
        "factor" | "factor_decimal" if args.len() > 1 => native_numeral::factor_decimal(args[1]),
        "unbraid" if args.len() > 1 => {
            let cap = args.get(2).and_then(|s| s.parse::<u64>().ok()).unwrap_or(u64::MAX);
            native_numeral::unbraid_report(args[1], cap)
        }
        "decompose" if args.len() > 1 => native_numeral::decompose_report(args[1]),
        _ => native_numeral::help(),
    }
}

fn kmain() {
    kmain_seeded(alloc::collections::VecDeque::new())
}

/// Same boot, with `seed` commands run first (see `repl::repl_seeded`) — every
/// REPL command reachable non-interactively, through the real boot sequence,
/// with no OS-level stdin pipe needed and no per-command dispatch to hand-add.
pub fn kmain_seeded(seed: alloc::collections::VecDeque<String>) {
    serial::init();

    // Hosted: the host owns the IDT, so there is no PIT and no PIC remap. The
    // boot line states only the hardware this build actually touches.
    interrupts::init(100);
    #[cfg(not(feature = "hosted"))]
    sprintln!("{}[boot]{} Interrupts online — PIT 100Hz, PIC remapped", style::muted(), style::reset());
    #[cfg(feature = "hosted")]
    sprintln!("{}[boot]{} Interrupts: none — the host owns the IDT, no periodic slot", style::muted(), style::reset());

    {
        let (used, total) = heap_used();
        #[cfg(not(feature = "hosted"))]
        sprintln!("{}[boot]{} Heap: {}MB static BSS", style::muted(), style::reset(), total / (1024 * 1024));
        #[cfg(feature = "hosted")]
        sprintln!("{}[boot]{} Heap: host allocator, {} bytes counted, {}MB declared budget", style::muted(), style::reset(),
                  used, total / (1024 * 1024));
        let _ = used;
    }

    let mut k = Kernel::new();
    k.boot();
    catalog::catalog_init();
    sprintln!("{}[boot]{} IG Catalog: {} entries loaded", style::muted(), style::reset(), catalog::catalog_size());
    sprintln!("{}[boot]{} Kernel online — graph execution, token-arity driven", style::muted(), style::reset());
    // ── ⊙-ordinal faithfulness guard (Track B) ──
    sprintln!("{}[boot]{} Canonical ordinal check...", style::muted(), style::reset());
    match canonical_ordinal::verify_canonical_ordinals() {
        (true, _) => sprintln!("{}[boot]{} Ordinal faithfulness: {}all 44 values match Lean canonical{}",
            style::muted(), style::reset(), style::verdict_t(), style::reset()),
        (false, why) => {
            sprintln!("{}[boot]{} ⚠ ORDINAL DRIFT DETECTED: {}", style::muted(), style::reset(), why);
            sprintln!("{}[boot]{} Kernel will NOT proceed — ordinal drift is a structural integrity violation.", style::muted(), style::reset());
            sprintln!("{}[boot]{} Regenerate canonical_ordinal.rs from CanonicalOrdinalFaithfulness.lean", style::muted(), style::reset());
            loop { unsafe { core::arch::asm!("hlt", options(nostack, nomem, preserves_flags)); } }
        }
    }
    // ── Clay closure/resistance status (Track C) ──
    sprintln!("{}[boot]{} Clay Millennium status: {} closed, {} one-bump-short, {} unclosed", style::muted(), style::reset(),
        clay_status::clay_summary().0, clay_status::clay_summary().1, clay_status::clay_summary().2);
    // Every figure on this line is read from the constants that define it, so
    // the banner cannot assert a structure the code no longer carries.
    sprintln!("{}[boot]{} SIC-POVM d={}: Crystal-forced (dual lattice), Shavian count {}={}², WH group |orbit|={}", style::muted(), style::reset(),
        sic_povm::TOTAL_PRIMS, sic_povm::SHAVIAN_COUNT,
        sic_povm::SHAVIAN_ROOT, sic_povm::WH_GROUP_ORDER);
    // ── Frobenius unification self-verification (Track E) ──
    sprintln!("{}[boot]{} Frobenius identity check...", style::muted(), style::reset());
    let (frob_ham, frob_dist) = frobenius_unify::boot_summary();
    if frob_ham == 0 {
        sprintln!("{}[boot]{} Frobenius identity: KERNEL IS FROBENIUS FIXED POINT — d=0 ✓", style::muted(), style::reset());
    } else {
        sprintln!("{}[boot]{} Frobenius identity: hamming={}, weighted={:.4} — kernel is grammar operationalized", style::muted(), style::reset(),
            frob_ham, frob_dist);
    }

    sprintln!("{}[boot]{} Bootstrap: IMSCRIB→AREV→FSPLIT→AFWD→FFUSE→CLINK→IFIX→IMSCRIB (cyclic)", style::muted(), style::reset());
    sprintln!("{}[boot]{} Fibonacci anyon QC: algebra verified = {}", style::muted(), style::reset(), fibonacci_qc::verify_all());
    // ── Kernel torus winding display (Track A) ──
    let torus_map = kernel_torus::TorusMap::new(&kernel_torus::agent_loop_program());
    kernel_torus::display_banner(&torus_map);
    sprintln!("{}[boot]{} Crystal FS: {} addresses", style::muted(), style::reset(), TOTAL);
    sprintln!("{}[boot]{} {} total programs (I–XXIX): 12 canonical + {} continuous + {} novel + {} shunted", style::muted(), style::reset(),
        canonical_count() + continuous_count() + novel_count() + shunted_count(),
        continuous_count(), novel_count(), shunted_count());
    sprintln!();

    print_banner();
    
    // Start live corner HUD (hosted build)
    #[cfg(feature = "hosted")]
    live_hud::start_hud();
    
    repl::repl_seeded(&mut k, seed);

    // ── Shutdown: write to QEMU isa-debug-exit port (0xf4).
    // Value 0x10 → QEMU exits with status 0.
    // On real hardware or without the device, falls through to HLT.
    sprintln!("[SHUTDOWN] μ∘δ=id. Goodbye.");

    // `out` and `hlt` are privileged. On bare metal they are the QEMU
    // debug-exit device and the idle halt; in a userspace process they are a
    // general protection fault, which is what made the hosted build segfault on
    // quit. A host does not need a debug-exit device emulated at it -- it
    // exits.
    #[cfg(not(feature = "hosted"))]
    unsafe {
        core::arch::asm!(
            "out dx, eax",
            in("dx") 0xf4_u16,
            in("eax") 0x10_u32,
            options(nomem, nostack, preserves_flags)
        );
    }
    #[cfg(not(feature = "hosted"))]
    loop { unsafe { core::arch::asm!("hlt", options(nostack, nomem, preserves_flags)); } }

    #[cfg(feature = "hosted")]
    return;
}

fn print_banner() {
    // The mark, the name, then what it is. This is the one screen where the
    // reader has no context yet, so it leads with the object and not the
    // feature list.
    sprintln!();
    sprintln!("   {}⊙{}   {}G-mOMonadOS{}", style::glyph(), style::reset(),
              style::heading(), style::reset());
    #[cfg(not(feature = "hosted"))]
    sprintln!("   {}the self-imscribing bare-metal kernel{}", style::muted(), style::reset());
    #[cfg(feature = "hosted")]
    sprintln!("   {}GPU-native build, mOMonadOS ported to run via CUDA{}", style::muted(), style::reset());
    sprintln!("   {}μ∘δ = id{}", style::accent(), style::reset());
    sprintln!();
    sprintln!("   {}Frobenius core · Belnap FOUR · crystal FS · graph execution{}",
              style::muted(), style::reset());
    sprintln!();
    sprintln!("   {}help{} for commands, {}?{} for the menu, Tab completes.",
              style::key(), style::reset(), style::key(), style::reset());
    sprintln!();
}

// ─── Panic ────────────────────────────────────────────────────

#[cfg(not(feature = "hosted"))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial::write_str("\n[PANIC] ");
    sprint!("{}", info.message());
    sprintln!();
    loop { unsafe { core::arch::asm!("hlt", options(nostack, nomem, preserves_flags)); } }
}

/// Report heap exhaustion ourselves.
///
/// The default handler reaches the panic through `__rust_alloc_error_handler`,
/// an internal symbol that takes the size and align as loose integers. Under
/// this build — `build-std` with fat LTO — that call arrives with a size of
/// zero however large the failed request was: an allocation of 1264 bytes
/// against 552 free reported itself as "memory allocation of 0 bytes failed",
/// which sent every reader hunting for a zero-size bug that does not exist.
///
/// Taking the `Layout` here reads it directly instead of through that symbol,
/// and the writes below never allocate, which matters when the reason we are
/// here is that allocation just failed.
#[cfg(not(feature = "hosted"))]
#[alloc_error_handler]
fn alloc_error(layout: Layout) -> ! {
    let (used, total) = heap_used();
    serial::write_str("\n[PANIC] heap exhausted — wanted ");
    serial::write_dec(layout.size());
    serial::write_str(" bytes (align ");
    serial::write_dec(layout.align());
    serial::write_str("), ");
    serial::write_dec(total.saturating_sub(used));
    serial::write_str(" free of ");
    serial::write_dec(total);
    serial::write_str("\n");
    loop { unsafe { core::arch::asm!("hlt", options(nostack, nomem, preserves_flags)); } }
}
