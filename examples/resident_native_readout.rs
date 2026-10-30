//! Inspect the baked carrier and execute two retained Y/clock operations.
#[cfg(not(target_feature = "crt-static"))]
compile_error!("resident_native_readout requires a static image");
#[path = "../src/bin/ququart_support/mod.rs"]
#[allow(dead_code)]
mod support;
include!(concat!(env!("OUT_DIR"), "/resident_prepared.rs"));

fn main() -> Result<(), String> {
    let mut carrier = baked_membrane()?;
    let prepared = carrier.prepared_record()?;
    let source = g_momonados::word_tape::WordTape::from_canonical_word(BAKED_SOURCE_WORD)
        .ok_or("missing baked source")?;
    if source.bit_len() < 200
        || prepared["source_word"] != BAKED_SOURCE_WORD
        || prepared["phase"]["source_word"] != BAKED_SOURCE_WORD
    {
        return Err("prepared carrier lost its validation source".into());
    }
    let before = carrier
        .reentry_operations()
        .ok_or("missing baked continuation")?;
    for _ in 0..2 {
        if BAKED_COMPOSED_MODULAR {
            carrier.advance_contracted()?;
        } else {
            carrier.advance()?;
        }
    }
    let after = carrier
        .reentry_operations()
        .ok_or("returned carrier lost its continuation")?;
    if after.0 <= before.0 || after.1 <= before.1 || after.2 <= before.2 {
        return Err("native Y/clock did not transform the carried computation".into());
    }
    println!(
        "{}",
        serde_json::json!({"source_bits":source.bit_len(),
        "preparation_nanoseconds":BAKED_PREPARATION_NS,"before":before,"after":after,
        "monadic_binds":carrier.monadic_binds(),"execution_progress":carrier.execution_progress(),
        "execution":"resident-native"})
    );
    Ok(())
}
