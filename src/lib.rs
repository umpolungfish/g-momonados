//! Host-testable surface of the kernel.
//!
//! The kernel is a bare-metal binary, which is why `braid_protocol`'s dual pair
//! was written correctly and then never checked: there was nowhere to run it.
//! A dual that has never been closed is a claim, not a verification, and the
//! Frobenius condition is the one thing this module exists to make checkable.
//!
//! Only the portable modules are carried here. Nothing that touches serial,
//! interrupts, the entry point or the halt is included. The kernel declares its
//! own module tree in main.rs and does not read this file, but cargo does build
//! this target alongside the binary, which is why it carries the same no_std
//! condition below.
//!
//! Author: Lando⊗⊙perator

// The bare-metal build compiles this target too, so it carries main.rs's own
// no_std condition. Without it the kernel build tries to link std for the lib
// and fails on the whole prelude.
#![cfg_attr(not(feature = "hosted"), no_std)]
#![allow(uncommon_codepoints)]
#![allow(dead_code)]
#![allow(non_snake_case)]

extern crate alloc;

pub use vox_core::godel_analyzer;
pub use vox_core::godel_calculus;

#[cfg(feature = "hosted")]
pub mod runtime_nesting;
pub mod word_tape;
pub mod gpu_rho;
pub mod gpu_rho_ml;
pub mod native_numeral;
pub mod phase_unbraid;
pub mod tokens;
pub mod word_nesting;
pub mod token_refinement;
pub mod factor_relation;
#[cfg(feature = "hosted")]
pub mod gpu_graph;
pub mod braid_protocol;
pub mod vox;
pub mod period_finding_ecdlp;
pub mod secp256k1_unwinder;
pub mod factor_membrane;

// The membrane family runs its nested fixed-point emit on the GPU, self-contained
// in the gpu_rho style: its own device kernel, native_numeral for host arithmetic,
// no reach into the bare-metal kernel tree. Hosted-only, like gpu_rho and gpu_graph.
#[cfg(feature = "hosted")]
pub mod membrane_family;

// The gate runs on a host; there is no test harness on bare metal.
#[cfg(all(test, feature = "hosted"))]
mod braid_frobenius_tests;
mod btc_key_deriver;

#[cfg(test)]
mod godel_calculus_tests {
    use super::{godel_analyzer, godel_calculus};

    #[test]
    fn vendored_godel_calculus_selftest() {
        assert!(godel_calculus::selftest_report().is_ok());
        assert!(godel_analyzer::selftest_report().is_ok());
    }
}