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
    vm.load(&format!("{PROGRAM}\n{LIBRARY}")).expect("assemble paired return");
    vm.set_reads(input);
    vm.run(None);
    assert!(vm.halted);
    assert!(vm.call_stack.is_empty());
    vm
}

fn readout(vm: &ParaVM) -> Vec<&str> {
    vm.emit_buffer.iter().map(|line| line.rsplit_once(" = ").unwrap().1).collect()
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
            .flat_map(|group| (0..4).map(move |lane| {
                if (group >> lane) & 1 == 0 { B4::T } else { B4::F }
            }))
            .collect();
        let expected: Vec<_> = input.iter().map(|cell| cell.name().to_owned()).collect();
        assert_eq!(readout(&execute(input)), expected);
    }
}

#[test]
fn paired_return_unequal_factor_widths() {
    // 7 and 3, grouped in two-bit FOUR cells and padded at the high end.
    let vm = execute(vec![B4::F, B4::F, B4::F, B4::F,
        B4::F, B4::T, B4::T, B4::T]);
    assert_eq!(readout(&vm), ["F", "F", "F", "F", "F", "T", "T", "T"]);
}
