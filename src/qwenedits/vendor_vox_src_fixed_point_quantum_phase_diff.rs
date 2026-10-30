--- vendor/vox/src/fixed_point_quantum_phase.rs (原始)


+++ vendor/vox/src/fixed_point_quantum_phase.rs (修改后)
//! Fixed-point quantum phase — vendored shim reconstructing the execution
//! interface this workspace's `recycled_carrier` requires from the on-device
//! Vox tree.
//!
//! A prepared `Program` carries its sealed source tape, its phase base tape,
//! and the recycled-QPE phase width the carrier must measure: two bits per
//! source bit (the parent kernel's amplitude-orbit doubling), matching the
//! width the `phase_unbraid` fixed-point pipeline banks for a modulus of that
//! size. The `Executor` trait is the device seam: a carrier implements it to
//! actually apply the emitted gates and supply its readout tape.

use crate::morphism_factor::{limbs_to_tape, trim, Tape};
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// The execution face of the fixed-point quantum phase stack.
pub mod execution {
    use super::*;

    /// A prepared structural-execution program sealed by a membrane.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Program {
        n: Tape,
        base: Tape,
    }

    impl Program {
        pub fn n(&self) -> &Tape {
            &self.n
        }
        pub fn base(&self) -> &Tape {
            &self.base
        }
        /// Width of the recycled phase register this program measures into.
        pub fn phase_width(&self) -> usize {
            let limbs = crate::morphism_factor::tape_to_limbs(&self.n, (self.n.len() + 31) / 32);
            let significant = limbs.iter().rev().position(|&l| l != 0).unwrap_or(limbs.len());
            let bits = if significant == 0 {
                1
            } else {
                (limbs.len() - significant) * 32 + (32 - limbs[limbs.len() - significant].leading_zeros() as usize)
            };
            bits * 2
        }
    }

    /// The device seam: a carrier applies the program's gates and returns its
    /// measured phase tape, one EVALF/EVALT mark per phase bit.
    pub trait Executor {
        fn execute_and_measure(&mut self, program: &Program) -> Result<Vec<char>, &'static str>;
    }

    /// Build a prepared program from a sealed source and base tape. Used by
    /// the membrane face in `fixed_point_quantum_membrane`.
    pub(crate) fn prepare(n: &Tape, base: &Tape) -> Result<Program, String> {
        if n.is_empty() || base.is_empty() {
            return Err("malformed prepared program".to_string());
        }
        Ok(Program { n: trim(n.clone()), base: trim(base.clone()) })
    }

    /// Re-seal a program face from decoded limbs (used when a baked word
    /// round-trips through the limb reader).
    #[allow(dead_code)]
    pub(crate) fn program_from_limbs(limbs: &[u32], base: &Tape) -> Program {
        Program { n: limbs_to_tape(limbs), base: trim(base.clone()) }
    }
}
