use g_momonados::{idempotent_closure, reentry_powerset, word_tape::WordTape};
use vox_core::dialectic_lean::ResidentY;

const SOURCE: &str = BAKED_SOURCE_LITERAL;
const INITIAL_RESIDUAL: &str = BAKED_INITIAL_RESIDUAL_LITERAL;

fn main() -> Result<(), String> {
    if std::env::args_os().len() != 1 {
        return Err("baked membrane accepts no instance arguments".into());
    }
    let source = WordTape::from_canonical_word(SOURCE).ok_or("invalid baked source")?;
    let candidate = WordTape::from_small(2);
    let residual =
        WordTape::from_canonical_word(INITIAL_RESIDUAL).ok_or("invalid baked residual")?;
    let mut y = ResidentY::from_certified_program(ResidentY::certified_program())?;
    let initial_state = (
        candidate.as_word().to_owned(),
        residual.as_word().to_owned(),
        0u8,
    );
    let mut held = reentry_powerset::state_unit(initial_state, 0usize);
    println!(
        "baked idempotent membrane; source bits={}; instance inputs=none",
        source.bit_len()
    );
    loop {
        let done = y.advance(|| {
            let input = std::mem::take(&mut held);
            if input.len() != 1 {
                return Err("resident membrane state must remain one complete branch".into());
            }
            let mut arrow = std::collections::BTreeMap::new();
            for &(coordinate, ref member_state) in &input {
                let successor_state = match member_state.2 {
                    0 => {
                        let candidate = WordTape::from_canonical_word(&member_state.0)
                            .ok_or("invalid candidate in resident state")?;
                        let residual = WordTape::from_canonical_word(&member_state.1)
                            .ok_or("invalid residual in resident state")?;
                        if !source.gt(&candidate) {
                            (member_state.0.clone(), member_state.1.clone(), 2)
                        } else if residual.is_zero() {
                            (member_state.0.clone(), member_state.1.clone(), 1)
                        } else {
                            // (e+1)e = e(e-1)+2e. Both coordinates remain source-bound.
                            let next_residual = residual
                                .add(&candidate.add(&candidate))
                                .divmod(&source)
                                .unwrap()
                                .1;
                            let next_candidate = candidate.add(&WordTape::one());
                            (
                                next_candidate.as_word().to_owned(),
                                next_residual.as_word().to_owned(),
                                0,
                            )
                        }
                    }
                    _ => member_state.clone(),
                };
                arrow.insert(
                    (coordinate, member_state.clone()),
                    reentry_powerset::state_unit(successor_state, coordinate),
                );
            }
            held = reentry_powerset::state_bind(input, &arrow);
            Ok(held.iter().all(|(_, state)| state.2 != 0))
        })?;
        if done {
            break;
        }
    }
    let (_, final_state) = held.iter().next().ok_or("resident powerset closed empty")?;
    if final_state.2 == 1 {
        let candidate = WordTape::from_canonical_word(&final_state.0)
            .ok_or("invalid closed candidate in resident state")?;
        let split = idempotent_closure::close(&source, &candidate)?;
        println!(
            "{}",
            idempotent_closure::report(&[
                "close".into(),
                g_momonados::native_numeral::decode(SOURCE)
                    .unwrap()
                    .to_string(),
                g_momonados::native_numeral::decode(split.idempotent.as_word())
                    .unwrap()
                    .to_string()
            ])?
        );
    } else {
        println!("complete idempotent search exhausted; no nontrivial idempotent");
    }
    println!("Y calls={} contractions={}", y.calls, y.contractions);
    Ok(())
}
