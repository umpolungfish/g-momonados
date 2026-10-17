//! Trigger the static resident state prepared by Cargo.
#[path = "../../examples/resident_prepared.rs"]
mod prepared_entry;
fn main() -> Result<(), String> { prepared_entry::main() }
