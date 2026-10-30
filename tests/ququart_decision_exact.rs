//! Exercise the decision arena independently of archived width-control files.
//! The arena's tests check exact amplitudes, support, and reversible recovery.
extern crate alloc;

pub use g_momonados::phase_unbraid;

#[path = "../src/ququart_decision.rs"]
#[allow(dead_code)]
mod ququart_decision;
