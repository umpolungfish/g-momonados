//! Execute the canonical Fibonacci-Shor constructor with separated target controls.
extern crate alloc;

#[path = "../../src/fibonacci_shor.rs"]
mod fibonacci_shor;

fn main() {
    let near = fibonacci_shor::controlled_phase_braid(0, 1);
    let far = fibonacci_shor::controlled_phase_braid(0, 4);
    let moved_control = fibonacci_shor::controlled_phase_braid(1, 4);
    println!("{{\"control\":0,\"target\":1,\"word\":{near:?}}}");
    println!("{{\"control\":0,\"target\":4,\"word\":{far:?}}}");
    println!("{{\"control\":1,\"target\":4,\"word\":{moved_control:?}}}");
    println!("{{\"target_change_preserved\":{},\"control_change_preserved\":{}}}",
        near != far, near != moved_control);

    // Keep N and the register widths fixed, change only the modular operand.
    for base in [2, 8] {
        let braid = fibonacci_shor::assemble_shor_braid(4, base, 15);
        println!("{{\"base\":{base},\"modulus\":15,\"modexp\":{:?},\"iqft\":{:?},\"period\":{}}}",
            braid.mod_exp_word, braid.iqft_word,
            braid.params.period.map(|x| x.to_string()).unwrap_or_else(|| "null".into()));
    }
    let a = fibonacci_shor::assemble_shor_braid(4, 2, 15);
    let b = fibonacci_shor::assemble_shor_braid(4, 8, 15);
    println!("{{\"base_change_preserved_in_modexp\":{},\"base_change_preserved_in_full_braid\":{}}}",
        a.mod_exp_word != b.mod_exp_word, a.total_word != b.total_word);
}
