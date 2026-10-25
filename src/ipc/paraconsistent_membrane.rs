// momonad_os/src/ipc/paraconsistent_membrane.rs

use crate::carrier::{Sixteen3, Cut, engage, coreflect, info_mass};

/// The Memory Management Unit (MMU) extension for Paraconsistent IPC.
/// When a process requests memory, it doesn't just get a pointer; 
/// it gets a membrane tagged with a SIXTEEN_3 carrier state.
pub struct ParaconsistentPage {
    pub physical_addr: usize,
    pub carrier_state: Sixteen3,
    pub fuse_adjoint: Option<u64>, // The cryptographic key to read the B-state
}

/// The hardware trap handler. Called by the MMU when a classical 
/// coreflection is detected on a protected page.
pub fn beta_trap_handler(pid: u32, page: &ParaconsistentPage, leaked_mass: f32) {
    println!("[mOMonadOS] 🚨 β-TRAP SPRUNG IN KERNEL SPACE!");
    println!("[mOMonadOS] Process {} attempted classical read on page {:x}", pid, page.physical_addr);
    println!("[mOMonadOS] Informational mass leaked: {}", leaked_mass);
    println!("[mOMonadOS] The Frobenius invariant was violated. Terminating process.");
    
    // The OS doesn't just segfault; it vents the informational exhaust 
    // to the environment and securely kills the process.
    crate::scheduler::vent_and_kill(pid);
}

/// The native read function for processes that possess the fuse adjoint.
pub fn native_read(page: &ParaconsistentPage, key: u64) -> Result<Sixteen3, &'static str> {
    if page.carrier_state.has_constructive() && !page.carrier_state.has_informational() {
        // It's a classical page, read it normally.
        return Ok(page.carrier_state);
    }
    
    if page.fuse_adjoint == Some(key) {
        // Bob's native read. The fuse adjoint matches.
        // We read the constructive layer without dropping the informational mass.
        // Zero Landauer cost.
        Ok(page.carrier_state)
    } else {
        // Eve's classical read. The process lacks the fuse adjoint.
        // The MMU forces a coreflection.
        let collapsed = coreflect(page.carrier_state);
        let leak = info_mass(page.carrier_state) - info_mass(collapsed);
        
        if leak >= 0.25 {
            // The hardware trips the trap.
            beta_trap_handler(crate::scheduler::current_pid(), page, leak);
            Err("β-TRAP: Process terminated")
        } else {
            Ok(collapsed)
        }
    }
}