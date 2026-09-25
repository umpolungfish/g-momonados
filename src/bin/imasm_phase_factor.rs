//! A contained encoded numeral enters the resident IMASM arithmetic circuits.
//! The host holds phase register addresses and reports only after closure.

extern crate alloc;

#[path = "../belnap.rs"]
mod belnap;
#[path = "../parasm.rs"]
mod parasm;

use std::collections::HashMap;
use std::time::Instant;
use vox_core::godel_calculus::{self, Nat, Structure};

const BAKED_N_WORD: Option<&str> = option_env!("IMASM_PHASE_N_WORD");
const BAKED_BASE_WORD: Option<&str> = option_env!("IMASM_PHASE_BASE_WORD");

fn trim(mut cells: String) -> String {
    while cells.ends_with('⊤') && cells.chars().count() > 1 {
        cells.pop();
    }
    cells
}

fn cells(word: &str) -> Result<String, String> {
    let reading =
        godel_calculus::decode(word).map_err(|error| format!("encoded word: {error:?}"))?;
    match reading.structure {
        Structure::CellBinary { bits_le } => Ok(bits_le
            .into_iter()
            .map(|bit| if bit { '⊥' } else { '⊤' })
            .collect()),
        _ => Err("expected a cell-binary IMASM numeral".into()),
    }
}

fn word(cells: &str) -> Result<String, String> {
    let bits = cells
        .chars()
        .map(|cell| match cell {
            '⊥' => Ok(true),
            '⊤' => Ok(false),
            _ => Err("phase produced a non-numeral cell".to_string()),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(godel_calculus::encode_cell_binary(&Nat::from_bits_le(bits)))
}

fn difference(left: &str, right: &str) -> Result<String, String> {
    let forward = parasm::subtract_encoded_lsb_first(left, right)?;
    let result = if forward.ends_with('⊥') {
        parasm::subtract_encoded_lsb_first(right, left)?
    } else {
        forward
    };
    Ok(trim(
        result.chars().take(result.chars().count() - 1).collect(),
    ))
}

fn proper_factor(
    n: &str,
    current_half: &str,
    earlier_half: &str,
) -> Result<Option<(String, String)>, String> {
    let delta = difference(current_half, earlier_half)?;
    let candidate = trim(parasm::gcd_encoded_lsb_first(&delta, n)?);
    factor_from_candidate(n, candidate)
}

fn factor_from_candidate(n: &str, candidate: String) -> Result<Option<(String, String)>, String> {
    if candidate == "⊥" || candidate == n || candidate == "⊤" {
        return Ok(None);
    }
    let Some(complement) = parasm::complement_encoded_lsb_first(&candidate, n)? else {
        return Ok(None);
    };
    let complement = trim(complement);
    // The complement circuit consumes N by the exact recurrence
    // r_{j+1} = (r_j - q_j P) / 2. Since P is odd, q_j is forced by r_j's
    // low bit. It emits Q only when the terminal residual is zero, so pair
    // extraction itself closes P*Q=N.
    Ok(Some((candidate, complement)))
}

fn should_read_support(phase_index: usize, base: &str) -> bool {
    (phase_index == 0 && base != "⊥⊤") || (phase_index > 8 && phase_index.is_power_of_two())
}

fn phase_return(n: &str, base: &str) -> Result<Option<(String, String)>, String> {
    let mut square = parasm::MontgomeryPhase::new(n)?;
    let unit = trim(square.enter("⊥")?);
    let one_montgomery = unit.clone();
    let mut seen: HashMap<String, String> = HashMap::new();
    seen.insert(unit.clone(), unit.clone());
    let mut current = trim(square.enter(base)?);
    let mut previous = unit;
    let mut phase_index = 0usize;
    loop {
        // At x = 2 the support polynomial is exactly N, so its gcd cannot
        // select a proper factor. Skip that tautological frame.
        if should_read_support(phase_index, base) {
            let candidate = trim(square.support_gcd(&current, &one_montgomery, n)?);
            if let Some(pair) = factor_from_candidate(n, candidate)? {
                return Ok(Some(pair));
            }
        }
        if let Some(earlier_half) = seen.get(&current) {
            return proper_factor(n, &previous, earlier_half);
        }
        seen.insert(current.clone(), previous);
        previous = current;
        current = trim(square.observe(&previous)?);
        phase_index += 1;
    }
}

fn run() -> Result<(), String> {
    if std::env::args_os().len() != 1 {
        return Err("the contained membrane accepts no runtime operands".into());
    }
    let started = Instant::now();
    let n = trim(cells(BAKED_N_WORD.ok_or("bake an IMASM N word first")?)?);
    let mut base = trim(cells(
        BAKED_BASE_WORD.ok_or("bake an IMASM base word first")?,
    )?);
    loop {
        if let Some((left, right)) = phase_return(&n, &base)? {
            let left_word = word(&left)?;
            let right_word = word(&right)?;
            println!(
                "P = {left_word}\nQ = {right_word}\nclosure = ⊤\nelapsed = {:?}",
                started.elapsed()
            );
            return Ok(());
        }
        base = trim(parasm::add_encoded_lsb_first(&base, "⊥")?);
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::{proper_factor, should_read_support};

    #[test]
    fn extracted_pair_closes_inside_the_complement_register() {
        let pair = proper_factor("⊥⊥⊥⊥", "⊥⊥⊤", "⊤").unwrap();
        assert_eq!(pair, Some(("⊥⊥".into(), "⊥⊤⊥".into())));
    }

    #[test]
    fn base_two_skips_the_trivial_support_frame() {
        assert!(!should_read_support(0, "⊥⊤"));
        assert!(should_read_support(0, "⊥⊥"));
        assert!(should_read_support(16, "⊥⊤"));
    }
}
