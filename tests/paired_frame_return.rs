extern crate alloc;

#[path = "../src/belnap.rs"]
mod belnap;
#[path = "../src/parasm.rs"]
mod parasm;

use belnap::B4;
use parasm::ParaVM;

const PROGRAM: &str = include_str!("../.imasm/paired_frame_return.imasm");
const LIBRARY: &str = include_str!("../.imasm/paired_frame_library.imasm");

fn execute(input: Vec<B4>) -> ParaVM {
    let mut vm = ParaVM::new();
    vm.load(&format!("{PROGRAM}\n{LIBRARY}"))
        .expect("assemble paired return");
    vm.set_reads(input);
    vm.run(None);
    assert!(vm.halted);
    assert!(vm.call_stack.is_empty());
    vm
}

fn readout(vm: &ParaVM) -> Vec<&str> {
    vm.emit_buffer
        .iter()
        .map(|line| line.rsplit_once(" = ").unwrap().1)
        .collect()
}

#[test]
fn paired_return_all_sixteen_states() {
    let cells = [
        ([B4::T, B4::T], B4::N),
        ([B4::F, B4::T], B4::T),
        ([B4::T, B4::F], B4::F),
        ([B4::F, B4::F], B4::B),
    ];
    for (left, packed_left) in cells {
        for (right, packed_right) in cells {
            let input = [left, right].concat();
            let expected: Vec<_> = input.iter().map(|cell| cell.name().to_owned()).collect();
            let vm = execute(input);
            assert_eq!(vm.belief_of(4), packed_left);
            assert_eq!(vm.belief_of(5), packed_right);
            assert_eq!(readout(&vm), expected);
        }
    }
}

#[test]
fn paired_return_wide_streams() {
    for groups in [0, 1, 65, 129, 257, 1025] {
        let input: Vec<_> = (0..groups)
            .flat_map(|group| {
                (0..4).map(move |lane| {
                    if (group >> lane) & 1 == 0 {
                        B4::T
                    } else {
                        B4::F
                    }
                })
            })
            .collect();
        let expected: Vec<_> = input.iter().map(|cell| cell.name().to_owned()).collect();
        assert_eq!(readout(&execute(input)), expected);
    }
}

#[test]
fn paired_return_unequal_factor_widths() {
    // 7 and 3, grouped in two-bit FOUR cells and padded at the high end.
    let vm = execute(vec![B4::F, B4::F, B4::F, B4::F, B4::F, B4::T, B4::T, B4::T]);
    assert_eq!(readout(&vm), ["F", "F", "F", "F", "F", "T", "T", "T"]);
}

#[test]
fn resident_phase_square_matches_encoded_arithmetic_across_observations() {
    for modulus in ["⊥⊤⊥⊤⊥", "⊥⊥⊤⊤⊤⊥"] {
        let mut square = parasm::PhaseSquare::new(modulus).unwrap();
        let mut residue = "⊤⊥".to_string();
        for _ in 0..5 {
            let product = parasm::multiply_encoded_lsb_first(&residue, &residue).unwrap();
            let expected = parasm::modulo_encoded_lsb_first(&product, modulus).unwrap();
            let observed = square.observe(&residue).unwrap();
            assert_eq!(
                observed,
                expected
                    .chars()
                    .take(modulus.chars().count())
                    .collect::<String>()
            );
            residue = observed;
        }
    }
}

#[test]
fn montgomery_phase_shifted_frame_commutes_with_encoded_squaring() {
    for modulus in ["⊥⊤⊥⊤⊥", "⊥⊥⊤⊤⊤⊥"] {
        let mut phase = parasm::MontgomeryPhase::new(modulus).unwrap();
        let mut ordinary = "⊤⊥".to_string();
        let mut shifted = phase.enter(&ordinary).unwrap();
        for _ in 0..5 {
            let product = parasm::multiply_encoded_lsb_first(&ordinary, &ordinary).unwrap();
            ordinary = parasm::modulo_encoded_lsb_first(&product, modulus)
                .unwrap()
                .chars()
                .take(modulus.chars().count())
                .collect();
            shifted = phase.observe(&shifted).unwrap();
            assert_eq!(shifted, phase.enter(&ordinary).unwrap());
        }
    }

    fn cells(mut value: usize) -> String {
        if value == 0 {
            return "⊤".into();
        }
        let mut result = String::new();
        while value > 0 {
            result.push(if value & 1 == 1 { '⊥' } else { '⊤' });
            value >>= 1;
        }
        result
    }
    for modulus in (3..64usize).step_by(2) {
        let n = cells(modulus);
        let mut phase = parasm::MontgomeryPhase::new(&n).unwrap();
        for residue in [0, 1, 2, modulus / 2, modulus - 1] {
            let shifted = phase.enter(&cells(residue)).unwrap();
            let squared = phase.observe(&shifted).unwrap();
            let expected = phase.enter(&cells((residue * residue) % modulus)).unwrap();
            assert_eq!(squared, expected, "N={modulus}, x={residue}");
        }
    }
    let wide_modulus = format!("⊥{}⊥", "⊤".repeat(127));
    let mut wide_phase = parasm::MontgomeryPhase::new(&wide_modulus).unwrap();
    let shifted_two = wide_phase.enter("⊤⊥").unwrap();
    assert_eq!(
        wide_phase.observe(&shifted_two).unwrap(),
        wide_phase.enter("⊤⊤⊥").unwrap()
    );
    let mut wrapped = parasm::MontgomeryPhase::new("⊥⊤⊥⊤⊥").unwrap();
    assert_eq!(
        wrapped.enter("⊥⊥⊤⊥⊤⊥").unwrap(),
        wrapped.enter("⊥").unwrap()
    );
}

#[test]
fn support_polynomial_extracts_a_pair_in_the_phase_frame() {
    let n = "⊥⊥⊥⊥"; // 15
    let mut phase = parasm::MontgomeryPhase::new(n).unwrap();
    let montgomery_one = phase.enter("⊥").unwrap();
    let base = phase.enter("⊥⊥").unwrap(); // 2
    let next = phase.observe(&base).unwrap(); // 4
    let support = phase.support_polynomial(&next, &montgomery_one, n).unwrap();
    let factor = parasm::gcd_encoded_lsb_first(&support, n).unwrap();
    assert_eq!(factor.trim_end_matches('⊤'), "⊥⊤⊥"); // 5
    let complement = parasm::complement_encoded_lsb_first(&factor, n).unwrap();
    assert_eq!(complement, Some("⊥⊥⊤⊤".into())); // 3, emitted after zero residual
}

#[test]
fn support_frame_evaluation_matches_encoded_horner_across_partial_frames() {
    fn trim(mut value: String) -> String {
        while value.len() > 1 && value.ends_with('⊤') {
            value.pop();
        }
        value
    }

    for n in ["⊥⊥⊥⊥", "⊥⊥⊥⊥⊥⊥⊥⊥", "⊥⊤⊤⊤⊤⊥⊥⊥⊥", "⊥⊤⊤⊤⊤⊤⊤⊥⊥⊥⊥"]
    {
        let width = n.chars().count();
        let base = "⊥⊥";
        let mut phase = parasm::MontgomeryPhase::new(n).unwrap();
        let montgomery_one = phase.enter("⊥").unwrap();
        let shifted_base = phase.enter(base).unwrap();
        let observed = phase
            .support_polynomial(&shifted_base, &montgomery_one, n)
            .unwrap();

        let mut expected = "⊤".to_string();
        for coefficient in n.chars().rev() {
            let product = parasm::multiply_encoded_lsb_first(&expected, base).unwrap();
            expected = parasm::modulo_encoded_lsb_first(&product, n)
                .unwrap()
                .chars()
                .take(width)
                .collect();
            if coefficient == '⊥' {
                let sum = parasm::add_encoded_lsb_first(&expected, "⊥").unwrap();
                expected = parasm::modulo_encoded_lsb_first(&sum, n)
                    .unwrap()
                    .chars()
                    .take(width)
                    .collect();
            }
        }
        assert_eq!(observed, phase.enter(&trim(expected)).unwrap(), "N={n}");
    }
}

#[test]
fn long_support_run_uses_exact_frame_geometric_sum() {
    use num_bigint::BigUint;
    let modulus = "⊥".repeat(20); // 2^20 - 1, one support run
    let width = modulus.chars().count();
    let mut phase = parasm::MontgomeryPhase::new(&modulus).unwrap();
    let one_montgomery = phase.enter("⊥").unwrap();
    let x = "⊥⊥"; // 3
    let x_montgomery = phase.enter(x).unwrap();
    let observed = phase
        .support_polynomial(&x_montgomery, &one_montgomery, &modulus)
        .unwrap();

    let modulus_value = (BigUint::from(1u8) << 20usize) - BigUint::from(1u8);
    let mut expected_value = BigUint::from(0u8);
    for _ in 0..width {
        expected_value = (expected_value * 3u8 + 1u8) % &modulus_value;
    }
    assert_eq!(expected_value, BigUint::from(660_550u32));
    let expected = (0..width)
        .map(|bit| {
            if ((&expected_value >> bit) & BigUint::from(1u8)) == BigUint::from(1u8) {
                '⊥'
            } else {
                '⊤'
            }
        })
        .collect::<String>();
    assert_eq!(observed, phase.enter(&expected).unwrap());
}

#[test]
fn encoded_complement_cancels_and_returns_both_signs_of_residual() {
    assert_eq!(
        parasm::complement_encoded_lsb_first("⊥⊥", "⊥⊤⊥⊤⊥").unwrap(),
        Some("⊥⊥⊥⊤⊤".into())
    );
    assert_eq!(
        parasm::complement_encoded_lsb_first("⊥⊥⊥", "⊥⊤⊥⊤⊥").unwrap(),
        Some("⊥⊥⊤⊤⊤".into())
    );
    assert_eq!(
        parasm::complement_encoded_lsb_first("⊥⊤⊥", "⊥⊤⊥⊤⊥").unwrap(),
        None
    );
}

#[test]
fn encoded_complement_scales_with_the_source_word() {
    let source = format!("⊥⊥{}⊥⊥", "⊤".repeat(127));
    let expected = format!("⊥{}⊥⊤", "⊤".repeat(128));
    assert_eq!(
        parasm::complement_encoded_lsb_first("⊥⊥", &source).unwrap(),
        Some(expected)
    );
}
