//! Numeric payload reader shared by the hosted membrane executables.

pub fn numbers(word: Option<&str>) -> Result<Vec<u64>, String> {
    let word = word.ok_or("Build with membrane_one.sh to bake the numeral payload")?;
    let values: Result<Vec<_>, _> = word.split_whitespace().map(|word| {
        let tape = vox::morphism_factor::parse_numeral(word)?;
        tape.iter().rev().try_fold(0u64, |n, &mark| {
            n.checked_mul(2)
                .and_then(|n| n.checked_add(u64::from(mark == vox::vox::EVALF)))
                .ok_or_else(|| "Membrane parameter exceeds u64".to_string())
        })
    }).collect();
    let values = values?;
    if values.is_empty() { return Err("Empty membrane payload".into()); }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn payload_roundtrip_and_errors() {
        let words = [0, 1, 9, u64::MAX].map(|n|
            vox::morphism_factor::emit_numeral(&vox::morphism_factor::tape_u64(n)));
        assert_eq!(numbers(Some(&words.join(" "))).unwrap(), [0, 1, 9, u64::MAX]);
        assert!(numbers(None).is_err());
        assert!(numbers(Some("")).is_err());
        assert!(numbers(Some("broken")).is_err());
        let wide = vox::morphism_factor::decimal_to_tape("18446744073709551616").unwrap();
        assert!(numbers(Some(&vox::morphism_factor::emit_numeral(&wide))).is_err());
    }
}
