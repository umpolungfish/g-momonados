//! Host-testable surface of the kernel.
//!
//! The kernel is a bare-metal binary, which is why `braid_protocol`'s dual pair
//! was written correctly and then never checked: there was nowhere to run it.
//! A dual that has never been closed is a claim, not a verification, and the
//! Frobenius condition is the one thing this module exists to make checkable.
//!
//! Only the portable modules are carried here. Nothing that touches serial,
//! interrupts, the entry point or the halt is included, but cargo does build
//! this target alongside the binary.
//!
//! Author: Lando⊗⊙perator

#![cfg_attr(not(feature = "hosted"), no_std)]
#![allow(uncommon_codepoints)]
#![allow(dead_code)]
#![allow(non_snake_case)]

extern crate alloc;

pub use vox_core::godel_analyzer;
pub use vox_core::godel_calculus;
pub use vox_core::godel_product;

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

#[cfg(feature = "hosted")]
pub mod membrane_family;

#[cfg(all(test, feature = "hosted"))]
mod braid_frobenius_tests;
mod btc_key_deriver;

#[cfg(test)]
mod godel_calculus_tests {
    use super::{godel_analyzer, godel_calculus, godel_product};

    #[test]
    fn vendored_godel_calculus_selftest() {
        assert!(godel_calculus::selftest_report().is_ok());
        assert!(godel_analyzer::selftest_report().is_ok());
        assert!(godel_product::selftest_report().is_ok());
    }
}