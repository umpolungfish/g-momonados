//! Exported, single-threaded resident-circuit ABI for Vox function calls.
#[path = "../membrane_complex.rs"] mod membrane_complex;
#[path = "../qft_circuit.rs"] mod qft_circuit;
use qft_circuit::{QftCircuit, PORTS};

static mut CIRCUIT: QftCircuit = QftCircuit::empty();
#[no_mangle] pub static mut CIRCUIT_OUTPUT: [f64; PORTS*2] = [0.0; PORTS*2];
#[no_mangle] pub static mut CIRCUIT_PREPARATIONS: u64 = 0;
#[no_mangle] pub static mut CIRCUIT_ACTIVATIONS: u64 = 0;

/// Safety: calls into this singleton must be serialized, as in Vox's VM.
#[no_mangle]
pub unsafe extern "C" fn membrane_prepare() -> i32 {
    (*core::ptr::addr_of_mut!(CIRCUIT)).prepare();
    CIRCUIT_OUTPUT = [0.0; PORTS*2];
    CIRCUIT_PREPARATIONS += 1;
    CIRCUIT_ACTIVATIONS = 0;
    0
}

/// Safety: calls into this singleton must be serialized.
#[no_mangle]
pub unsafe extern "C" fn membrane_activate(gates: u64, feedback: u64) -> i32 {
    if feedback > 1 { return 2; }
    let circuit = &mut *core::ptr::addr_of_mut!(CIRCUIT);
    if circuit.activate(gates, feedback == 1).is_err() { return 1; }
    for j in 0..PORTS {
        let signal = circuit.output(j);
        CIRCUIT_OUTPUT[j*2] = signal.re;
        CIRCUIT_OUTPUT[j*2+1] = signal.im;
    }
    CIRCUIT_ACTIVATIONS += 1;
    0
}

/// Safety: serialize reads with prepare/activate on the singleton.
#[no_mangle]
pub unsafe extern "C" fn membrane_info(which: u64) -> u64 {
    match which {
        0 => PORTS as u64,
        1 => core::ptr::addr_of!(CIRCUIT_OUTPUT) as u64,
        2 => CIRCUIT_PREPARATIONS,
        3 => CIRCUIT_ACTIVATIONS,
        _ => u64::MAX,
    }
}

fn main() {
    // Retain the exported entry points and data in the complete lifted ELF.
    unsafe {
        std::hint::black_box(membrane_prepare as unsafe extern "C" fn() -> i32)();
        std::hint::black_box(membrane_activate as unsafe extern "C" fn(u64, u64) -> i32)(1, 0);
        println!("prepared QFT circuit: output[0]={}", CIRCUIT_OUTPUT[0]);
        println!("ports={}", std::hint::black_box(membrane_info as unsafe extern "C" fn(u64) -> u64)(0));
    }
}
