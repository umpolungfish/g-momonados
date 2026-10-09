use vox::godel_calculus::{check, encode_cell_binary, Nat, Operator};
use vox::glut_system::GlutSieve;
use vox::morphism_factor::{dec_of, emit_numeral, parse_numeral};

fn main() {
    // Only source values are supplied. Factor words come from cell/carry closure.
    for source in ["21", "35", "56", "91", "127", "143", "8051"] {
        let value = Nat::from_decimal(source).unwrap();
        let word = encode_cell_binary(&value);
        let tape = parse_numeral(&word).unwrap();
        let mut previous = None;
        for width in [1, 2, 4] {
            let mut relation = GlutSieve::new(&tape);
            let mut peak = relation.states.len();
            while relation.states.iter().any(|state| !state.is_complete(tape.len())) {
                relation.frame_superpose(width);
                peak = peak.max(relation.states.len());
            }
            let pair = relation.readout();
            if let Some(prior) = &previous { assert_eq!(&pair, prior); }
            previous = Some(pair.clone());
            match pair {
                Some((left, right)) => {
                    let left_word = emit_numeral(&left);
                    let right_word = emit_numeral(&right);
                    assert!(check(&left_word, Operator::Mul, &right_word, &word).unwrap().valid);
                    let execution = relation.readout_execution().unwrap();
                    execution.verify(&tape).unwrap();
                    let mut corrupted = execution.clone();
                    corrupted.checkpoints.last_mut().unwrap().carry.push('⊥');
                    assert!(corrupted.verify(&tape).is_err());
                    println!("source={source} frame={width} factors={}*{} peak={peak} checkpoints={} return=PASS carry-control=REJECT", dec_of(&left), dec_of(&right), execution.checkpoints.len());
                    if width == 1 {
                        println!("left.word={left_word}\nright.word={right_word}");
                        for state in &execution.checkpoints {
                            println!("column={} carry={} left-prefix={} right-prefix={}", state.position, dec_of(&state.carry), dec_of(&state.p_prefix), dec_of(&state.q_prefix));
                        }
                    }
                }
                None => {
                    assert!(relation.glut_crystal().iter().all(|(p, q)| dec_of(p) == "1" || dec_of(q) == "1"));
                    println!("source={source} frame={width} proper-pair=ABSENT peak={peak} unit-control=PASS");
                }
            }
        }
    }
}
