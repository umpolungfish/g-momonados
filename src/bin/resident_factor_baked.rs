//! Trigger the static resident state prepared by Cargo.
#[path = "../../examples/resident_prepared.rs"]
mod prepared_entry;
fn main() -> Result<(), String> {
    #[cfg(feature = "hosted")]
    vox_core::ordering::cli::intercept(); prepared_entry::main() }
