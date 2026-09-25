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
    let reading = godel_calculus::decode(word).map_err(|error| format!("encoded word: {error:?}"))?;
    match reading.structure {
        Structure::CellBinary { bits_le } => Ok(bits_le.into_iter()
            .map(|bit| if bit { '⊥' } else { '⊤' }).collect()),
        _ => Err("expected a cell-binary IMASM numeral".into()),
    }
}

fn word(cells: &str) -> Result<String, String> {
    let bits = cells.chars().map(|cell| match cell {
        '⊥' => Ok(true),
        '⊤' => Ok(false),
        _ => Err("phase produced a non-numeral cell".to_string()),
    }).collect::<Result<Vec<_>, _>>()?;
    Ok(godel_calculus::encode_cell_binary(&Nat::from_bits_le(bits)))
}

fn difference(left: &str, right: &str) -> Result<String, String> {
    let forward = parasm::subtract_encoded_lsb_first(left, right)?;
    let result = if forward.ends_with('⊥') {
        parasm::subtract_encoded_lsb_first(right, left)?
    } else {
        forward
    };
    Ok(trim(result.chars().take(result.chars().count() - 1).collect()))
}

fn proper_factor(n: &str, current_half: &str, earlier_half: &str)
    -> Result<Option<(String, String)>, String> {
    let delta = difference(current_half, earlier_half)?;
    let candidate = trim(parasm::gcd_encoded_lsb_first(&delta, n)?);
    if candidate == "⊥" || candidate == n || candidate == "⊤" {
        return Ok(None);
    }
    let Some(complement) = parasm::complement_encoded_lsb_first(&candidate, n)? else {
        return Ok(None);
    };
    let complement = trim(complement);
    if parasm::product_closure_encoded_lsb_first(&candidate, &complement, n)? == '⊤' {
        Ok(Some((candidate, complement)))
    } else {
        Ok(None)
    }
}

fn phase_return(n: &str, base: &str) -> Result<Option<(String, String)>, String> {
    // The initial unit is the earlier phase register of the first residue.
    let mut seen: HashMap<String, String> = HashMap::new();
    seen.insert("⊥".into(), "⊥".into());
    let mut current = trim(parasm::modulo_encoded_lsb_first(base, n)?);
    let mut previous = "⊥".to_string();
    let mut square = parasm::PhaseSquare::new(n)?;
    loop {
        if let Some(earlier_half) = seen.get(&current) {
            return proper_factor(n, &previous, earlier_half);
        }
        seen.insert(current.clone(), previous);
        previous = current;
        current = trim(square.observe(&previous)?);
    }
}

fn run() -> Result<(), String> {
    if std::env::args_os().len() != 1 {
        return Err("the contained membrane accepts no runtime operands".into());
    }
    let started = Instant::now();
    let n = trim(cells(BAKED_N_WORD.ok_or("bake an IMASM N word first")?)?);
    let mut base = trim(cells(BAKED_BASE_WORD.ok_or("bake an IMASM base word first")?)?);
    loop {
        if let Some((left, right)) = phase_return(&n, &base)? {
            let left_word = word(&left)?;
            let right_word = word(&right)?;
            println!("P = {left_word}\nQ = {right_word}\nclosure = ⊤\nelapsed = {:?}", started.elapsed());
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
