#![allow(dead_code)]
//! ParaASM — Practical Paraconsistent Universal Engine
//! Port of priests-engine/para_vm.py + imscribing_grammar/para/para_vm.py
//! Belnap FOUR VM with 19-instruction ISA, assembler, dialetheic alignment,
//! measurement sequence algebra, and IG snapshot bridge.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use crate::belnap::B4;

// ── ParaASM instruction set ─────────────────────────────────────────

/// The 19-instruction ParaASM ISA.
/// Frobenius core (4), register ops (2), control flow (8), stack (2), I/O (2).
#[derive(Clone, Debug, PartialEq)]
pub enum ParaAsm {
    // ── Frobenius core (mirrors 4 of 12 IMASM tokens) ──
    ENGAGR(usize),              // Engage Both — register → paradox
    FSPLIT(usize, usize, usize),// Fork — src → (dst1, dst2); B bifurcates to T+F
    FFUSE(Vec<usize>, usize),   // Fuse — join many sources → dst
    IFIX(usize),                // Permanent brand — linear ! exponential

    // ── Register ops ──
    MOVE(usize, usize),      // Copy src → dst
    CLEAR(usize),            // Reset register to N

    // ── Control flow ──
    JMP(String),             // Unconditional jump to label
    JB(usize, String),       // Jump if B
    JT(usize, String),       // Jump if T
    JF(usize, String),       // Jump if F
    JN(usize, String),       // Jump if N
    CALL(String),            // Call subroutine (push return addr)
    RET,                     // Return from subroutine
    HALT,                    // Stop execution

    // ── Stack ──
    PUSH(usize),             // Push register to data stack
    POP(usize),              // Pop data stack → register

    // ── I/O (noop in kernel mode) ──
    EMIT(usize),             // Emit register value
    READ(usize),             // Read input → register (defaults N)
}

/// Dynamically sized ripple-carry addition. Bits use the numeral encoding
/// ⊥=1 and ⊤=0. The canonical source is a pure IMASM instruction stream;
/// Rust only includes it for assembly by ParaVM.
pub const BIT_REGISTER_ADD_ASM: &str = include_str!("../.imasm/bit_register_add.imasm");
/// Dynamically sized ripple-borrow subtraction over the same encoded tape.
pub const BIT_REGISTER_SUB_ASM: &str = include_str!("../.imasm/bit_register_sub.imasm");
const BIT_REGISTER_GATE_LIBRARY_ASM: &str = include_str!("../.imasm/bit_register_gate_library.imasm");
const PAIRED_FRAME_LIBRARY_ASM: &str = include_str!("../.imasm/paired_frame_library.imasm");

struct BitRegisterMulLayout {
    source: String,
    a_start: usize,
    b_start: usize,
    extra_start: usize,
    product_start: usize,
    product_width: usize,
    next_register: usize,
}

/// Wire one multiply circuit into an existing instruction stream. This is
/// shared by standalone multiplication, product closure, and modular phase
/// winding; the operands and result are all dynamically sized B4 tape banks.
fn append_bit_register_mul(
    source: &mut String,
    a_bits: &[usize],
    b_bits: &[usize],
    product_bits: &[usize],
    scratch_start: usize,
) -> usize {
    use core::fmt::Write as _;
    let partial = scratch_start;
    let carry = scratch_start + 1;
    for &bit in product_bits {
        writeln!(source, "MOVE %r11 %r{bit}").unwrap();
    }
    for (b_bit, &b_register) in b_bits.iter().enumerate() {
        writeln!(source, "MOVE %r11 %r{carry}").unwrap();
        for (a_bit, &a_register) in a_bits.iter().enumerate() {
            let column = a_bit + b_bit;
            writeln!(source, "MOVE %r{a_register} %r0").unwrap();
            writeln!(source, "MOVE %r{b_register} %r1").unwrap();
            source.push_str("CALL .and_gate\n");
            writeln!(source, "MOVE %r3 %r{partial}").unwrap();
            writeln!(source, "MOVE %r{} %r0", product_bits[column]).unwrap();
            writeln!(source, "MOVE %r{partial} %r1").unwrap();
            writeln!(source, "MOVE %r{carry} %r2").unwrap();
            source.push_str("CALL .full_adder\n");
            writeln!(source, "MOVE %r3 %r{}", product_bits[column]).unwrap();
            writeln!(source, "MOVE %r4 %r{carry}").unwrap();
        }
        for &product_bit in &product_bits[(b_bit + a_bits.len())..] {
            writeln!(source, "MOVE %r{product_bit} %r0").unwrap();
            source.push_str("MOVE %r11 %r1\n");
            writeln!(source, "MOVE %r{carry} %r2").unwrap();
            source.push_str("CALL .full_adder\n");
            writeln!(source, "MOVE %r3 %r{product_bit}").unwrap();
            writeln!(source, "MOVE %r4 %r{carry}").unwrap();
        }
    }
    scratch_start + 2
}

fn append_bit_register_add(
    source: &mut String,
    left: &[usize],
    right: &[usize],
    sum: &[usize],
    carry: usize,
) {
    use core::fmt::Write as _;
    source.push_str("MOVE %r11 %r");
    writeln!(source, "{carry}").unwrap();
    for (bit, &out) in sum.iter().enumerate() {
        if let Some(&input) = left.get(bit) { writeln!(source, "MOVE %r{input} %r0").unwrap(); }
        else { source.push_str("MOVE %r11 %r0\n"); }
        if let Some(&input) = right.get(bit) { writeln!(source, "MOVE %r{input} %r1").unwrap(); }
        else { source.push_str("MOVE %r11 %r1\n"); }
        writeln!(source, "MOVE %r{carry} %r2\nCALL .full_adder\nMOVE %r3 %r{out}\nMOVE %r4 %r{carry}").unwrap();
    }
}

fn append_bit_register_equality(
    source: &mut String,
    left: &[usize],
    right: &[usize],
    width: usize,
    verdict: usize,
    label: &str,
) {
    use core::fmt::Write as _;
    writeln!(source, "MOVE %r11 %r{verdict}").unwrap();
    for bit in 0..width {
        let a = left.get(bit).copied();
        let b = right.get(bit).copied();
        if let Some(a) = a { writeln!(source, "JT %r{a} .{label}_{bit}_left_zero").unwrap(); }
        else { writeln!(source, "JMP .{label}_{bit}_left_zero").unwrap(); }
        if let Some(a) = a { writeln!(source, "JF %r{a} .{label}_{bit}_left_one").unwrap(); }
        else { source.push_str("HALT\n"); }
        writeln!(source, ".{label}_{bit}_left_zero:").unwrap();
        if let Some(b) = b { writeln!(source, "JT %r{b} .{label}_{bit}_next\nJF %r{b} .{label}_{bit}_mismatch").unwrap(); }
        else { writeln!(source, "JMP .{label}_{bit}_next").unwrap(); }
        writeln!(source, ".{label}_{bit}_left_one:").unwrap();
        if let Some(b) = b { writeln!(source, "JF %r{b} .{label}_{bit}_next\nJT %r{b} .{label}_{bit}_mismatch").unwrap(); }
        else { writeln!(source, "JMP .{label}_{bit}_mismatch").unwrap(); }
        writeln!(source, ".{label}_{bit}_mismatch:\nMOVE %r12 %r{verdict}\n.{label}_{bit}_next:").unwrap();
    }
}

fn append_word_insertion_check(
    source: &mut String,
    from: &[[usize; 4]],
    to: &[[usize; 4]],
    insertion_index: usize,
    inserted_token: usize,
    verdict: usize,
    label: &str,
) {
    use core::fmt::Write as _;
    for (target_index, token_bits) in to.iter().enumerate() {
        let source_bits = if target_index == insertion_index {
            None
        } else {
            let source_index = if target_index < insertion_index { target_index } else { target_index - 1 };
            from.get(source_index).copied()
        };
        for bit in 0..4 {
            let target = token_bits[bit];
            if let Some(source_register) = source_bits.map(|bits| bits[bit]) {
                writeln!(source, "MOVE %r{source_register} %r0").unwrap();
            } else if target_index == insertion_index {
                source.push_str(if (inserted_token >> bit) & 1 == 1 { "MOVE %r12 %r0\n" } else { "MOVE %r11 %r0\n" });
            } else {
                source.push_str("MOVE %r11 %r0\n");
            }
            writeln!(source, "MOVE %r{target} %r1").unwrap();
            writeln!(source, "JT %r0 .{label}_{target_index}_{bit}_zero").unwrap();
            writeln!(source, "JF %r0 .{label}_{target_index}_{bit}_one").unwrap();
            source.push_str("HALT\n");
            writeln!(source, ".{label}_{target_index}_{bit}_zero:\nJT %r1 .{label}_{target_index}_{bit}_next\nJF %r1 .{label}_{target_index}_{bit}_bad\nHALT").unwrap();
            writeln!(source, ".{label}_{target_index}_{bit}_one:\nJF %r1 .{label}_{target_index}_{bit}_next\nJT %r1 .{label}_{target_index}_{bit}_bad\nHALT").unwrap();
            writeln!(source, ".{label}_{target_index}_{bit}_bad:\nMOVE %r12 %r{verdict}\n.{label}_{target_index}_{bit}_next:").unwrap();
        }
    }
}

const EDIT_SQUARE_D1: &str = "⊢⊤≻⋈≺⊙⊡⊣";
const EDIT_SQUARE_A: &str = "⊢⊤≻⋈⊥≺⊙⊡⊣";
const EDIT_SQUARE_D2: &str = "⊢⊤≻⋈≺⊞⊙⊡⊣";
const EDIT_SQUARE_B: &str = "⊢⊤≻⋈⊥≺⊞⊙⊡⊣";
const EDIT_SQUARE_C: &str = "⊢∈≻⋈⊥≺⋈∋⊙⊡⊣";
// Canonical closure witness, already encoded as LSB-first IMASM tapes.
// Layout: prime bases 2,3,5; each prime's additive (3,5,8) and
// multiplicative (2,4,8) exponent rows; then both radicals and support masks.
const EDIT_SQUARE_LANE_TAPES: &[&str] = &[
    "⊤⊥", "⊥⊥", "⊥⊤⊥",
    "⊤", "⊤", "⊥⊥", "⊥", "⊤⊥", "⊥⊥",
    "⊥", "⊤", "⊤", "⊤", "⊤", "⊤",
    "⊤", "⊥", "⊤", "⊤", "⊤", "⊤",
    "⊤⊥⊥⊥⊥", "⊤⊥", "⊥⊥⊥", "⊥⊤⊤",
];

struct EditSquareLayout {
    source: String,
    reads: Vec<B4>,
    width: usize,
    x: Vec<usize>,
    a: Vec<usize>,
    d2: Vec<usize>,
    b: Vec<usize>,
    sum: Vec<usize>,
    product: Vec<usize>,
    result_word: Vec<[usize; 4]>,
    lane_tapes: Vec<Vec<usize>>,
    edit_verdict: usize,
    relation_verdict: usize,
    x_positive: usize,
    s_positive: usize,
    closed: usize,
}

fn canonical_word_cells(word: &str) -> Result<(Vec<[usize; 4]>, Vec<B4>), String> {
    let mut registers = Vec::new();
    let mut data = Vec::new();
    for glyph in word.chars() {
        let ordinal = imasm_core::imasm16_3::ALL_TOKENS.iter()
            .position(|token| token.glyph() == glyph)
            .ok_or_else(|| alloc::format!("unknown IMASM token {glyph:?}"))?;
        let start = registers.len() * 4;
        registers.push([start, start + 1, start + 2, start + 3]);
        for bit in 0..4 {
            data.push(if (ordinal >> bit) & 1 == 1 { B4::F } else { B4::T });
        }
    }
    Ok((registers, data))
}

fn append_nonzero_check(source: &mut String, bits: &[usize], verdict: usize, label: &str) {
    use core::fmt::Write as _;
    writeln!(source, "MOVE %r12 %r{verdict}").unwrap();
    for (bit, &register) in bits.iter().enumerate() {
        writeln!(source, "JF %r{register} .{label}_nonzero\nJT %r{register} .{label}_next_{bit}").unwrap();
        writeln!(source, ".{label}_next_{bit}:").unwrap();
    }
    writeln!(source, "JMP .{label}_done\n.{label}_nonzero:\nMOVE %r11 %r{verdict}\n.{label}_done:").unwrap();
}

fn bit_register_edit_square_program(x_width: usize, s_width: usize) -> Result<EditSquareLayout, String> {
    use core::fmt::Write as _;

    let words = [EDIT_SQUARE_D1, EDIT_SQUARE_A, EDIT_SQUARE_D2, EDIT_SQUARE_B, EDIT_SQUARE_C];
    let mut reads = Vec::new();
    let mut next = 32usize;
    let mut word_regs: Vec<Vec<[usize; 4]>> = Vec::new();
    for word in words {
        let (relative, encoded) = canonical_word_cells(word)?;
        let base = next;
        let cells: Vec<[usize; 4]> = relative.iter()
            .map(|bits| bits.map(|register| base + register))
            .collect();
        next += encoded.len();
        reads.extend(encoded);
        word_regs.push(cells);
    }

    let x_input: Vec<usize> = (next..next + x_width).collect();
    next += x_width;
    let s_input: Vec<usize> = (next..next + s_width).collect();
    next += s_width;
    let read_end = next;
    let width = core::cmp::max(x_width, s_width) + 2;
    let x: Vec<usize> = (next..next + width).collect(); next += width;
    let s: Vec<usize> = (next..next + width).collect(); next += width;
    let one: Vec<usize> = (next..next + width).collect(); next += width;
    let a: Vec<usize> = (next..next + width).collect(); next += width;
    let d2: Vec<usize> = (next..next + width).collect(); next += width;
    let b: Vec<usize> = (next..next + width).collect(); next += width;
    let sum: Vec<usize> = (next..next + width + 1).collect(); next += width + 1;
    let product: Vec<usize> = (next..next + width * 2).collect(); next += width * 2;
    let scratch = next; next += 4;
    let edit_verdict = next; next += 1;
    let relation_verdict = next; next += 1;
    let x_positive = next; next += 1;
    let s_positive = next; next += 1;
    let closed = next; next += 1;
    let mut source = String::from("ENGAGR %r10\nFSPLIT %r10 %r11 %r12\n");
    for reg in 32..read_end { writeln!(source, "READ %r{reg}").unwrap(); }
    let [d1_regs, a_regs, d2_regs, b_regs, c_regs] = word_regs.as_slice() else {
        return Err("edit-square word register layout failed".into());
    };
    let bot = imasm_core::imasm16_3::ALL_TOKENS.iter().position(|token| token.glyph() == '⊥').unwrap();
    let boxplus = imasm_core::imasm16_3::ALL_TOKENS.iter().position(|token| token.glyph() == '⊞').unwrap();
    writeln!(source, "MOVE %r11 %r{edit_verdict}").unwrap();
    append_word_insertion_check(&mut source, d1_regs, a_regs, 4, bot, edit_verdict, "edit_bot");
    append_word_insertion_check(&mut source, d1_regs, d2_regs, 5, boxplus, edit_verdict, "edit_boxplus");
    append_word_insertion_check(&mut source, a_regs, b_regs, 6, boxplus, edit_verdict, "edit_boxplus_after_bot");
    append_word_insertion_check(&mut source, d2_regs, b_regs, 4, bot, edit_verdict, "edit_bot_after_boxplus");

    for bit in 0..x_width { writeln!(source, "MOVE %r{} %r{}", x_input[bit], x[bit]).unwrap(); }
    for bit in x_width..width { writeln!(source, "MOVE %r11 %r{}", x[bit]).unwrap(); }
    for bit in 0..s_width { writeln!(source, "MOVE %r{} %r{}", s_input[bit], s[bit]).unwrap(); }
    for bit in s_width..width { writeln!(source, "MOVE %r11 %r{}", s[bit]).unwrap(); }
    for (bit, &reg) in one.iter().enumerate() {
        writeln!(source, "MOVE %r{} %r{reg}", if bit == 0 { 12 } else { 11 }).unwrap();
    }

    append_bit_register_add(&mut source, &x, &one, &a, scratch);
    append_bit_register_add(&mut source, &x, &s, &d2, scratch);
    append_bit_register_add(&mut source, &a, &s, &b, scratch);
    append_bit_register_add(&mut source, &a, &b, &sum, scratch);
    append_bit_register_mul(&mut source, &x, &d2, &product, scratch);
    // Sum's high product-width cells are literal zero; allocate explicit
    // zero registers so equality remains an ordinary IMASM bit comparison.
    let mut sum_compare = sum.clone();
    for _ in sum.len()..product.len() { sum_compare.push(next); next += 1; source.push_str("MOVE %r11 %r"); writeln!(source, "{}", next - 1).unwrap(); }
    append_bit_register_equality(&mut source, &sum_compare, &product, product.len(), relation_verdict, "square_relation");
    append_nonzero_check(&mut source, &x, x_positive, "square_x_positive");
    append_nonzero_check(&mut source, &s, s_positive, "square_s_positive");

    let mut lane_tapes = Vec::new();
    for encoded in EDIT_SQUARE_LANE_TAPES {
        let mut tape = Vec::new();
        for symbol in encoded.chars() {
            tape.push(next);
            writeln!(source, "MOVE %r{} %r{next}", if symbol == '⊥' { 12 } else { 11 }).unwrap();
            next += 1;
        }
        lane_tapes.push(tape);
    }

    writeln!(source, "MOVE %r11 %r{closed}").unwrap();
    for (index, flag) in [edit_verdict, relation_verdict, x_positive, s_positive].iter().enumerate() {
        writeln!(source, "JF %r{flag} .square_failed_{index}\nJT %r{flag} .square_passed_{index}\nHALT\n.square_failed_{index}:\nMOVE %r12 %r{closed}\n.square_passed_{index}:").unwrap();
    }
    for reg in &x { writeln!(source, "EMIT %r{reg}").unwrap(); }
    for reg in &a { writeln!(source, "EMIT %r{reg}").unwrap(); }
    for reg in &d2 { writeln!(source, "EMIT %r{reg}").unwrap(); }
    for reg in &b { writeln!(source, "EMIT %r{reg}").unwrap(); }
    for reg in &sum { writeln!(source, "EMIT %r{reg}").unwrap(); }
    for reg in &product { writeln!(source, "EMIT %r{reg}").unwrap(); }
    for token in c_regs { for reg in token { writeln!(source, "EMIT %r{reg}").unwrap(); } }
    for tape in &lane_tapes { for reg in tape { writeln!(source, "EMIT %r{reg}").unwrap(); } }
    for reg in [edit_verdict, relation_verdict, x_positive, s_positive, closed] { writeln!(source, "EMIT %r{reg}").unwrap(); }
    source.push_str("HALT\n");
    source.push_str(BIT_REGISTER_GATE_LIBRARY_ASM);

    Ok(EditSquareLayout { source, reads, width, x, a, d2, b, sum, product, result_word: c_regs.clone(), lane_tapes, edit_verdict, relation_verdict, x_positive, s_positive, closed })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrimeExponentRow {
    pub prime: String,
    pub additive_abc: [String; 3],
    pub multiplicative_d1_d2_c: [String; 3],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RadicalSupportLane {
    pub additive_radical: String,
    pub multiplicative_radical: String,
    /// Presence bits in prime order 2, 3, 5, encoded with ⊥=1.
    pub additive_support: String,
    pub multiplicative_support: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditSquareResult {
    pub d1: String,
    pub a: String,
    pub d2: String,
    pub b: String,
    pub g_t_sum: String,
    pub g_f_product: String,
    pub result_word: String,
    pub falsity_exponents: Vec<PrimeExponentRow>,
    pub information_support: RadicalSupportLane,
    /// The baked-in lane profile applies only when the dynamic closure is ⊤.
    pub lane_witness_applies: char,
    pub edit_closed: char,
    pub arithmetic_closed: char,
    pub positive_inputs: char,
    pub closed: char,
}

/// Run the edit/valuation commuting square as one ParaASM instruction stream.
/// The four edited words are tokenized through IMASM's canonical token set and
/// carried in four-bit B4 cells. The candidate seed and shift enter only as
/// LSB-first ⊤/⊥ tapes. Addition forms the truth lane; multiplication forms
/// the factor-structure lane; the stream closes only when both values agree
/// and all four literal edits match their encoded word witnesses.
pub fn edit_square_encoded_lsb_first(x_input: &str, s_input: &str) -> Result<EditSquareResult, String> {
    fn decode(stream: &str) -> Result<Vec<B4>, String> {
        if stream.is_empty() {
            return Err("expected a non-empty LSB-first ⊤/⊥ bitstream".into());
        }
        stream.chars().map(|symbol| match symbol {
            '⊤' => Ok(B4::T),
            '⊥' => Ok(B4::F),
            other => Err(alloc::format!("invalid numeral symbol {other:?}; expected ⊤ or ⊥")),
        }).collect()
    }
    fn emitted_bit(cell: &str) -> Result<char, String> {
        match cell.rsplit_once(" = ").map(|(_, value)| value) {
            Some("T") => Ok('⊤'),
            Some("F") => Ok('⊥'),
            Some(value) => Err(alloc::format!("edit-square emitted non-bit state {value}")),
            None => Err("edit-square emitted a malformed cell".into()),
        }
    }

    let x_tape = decode(x_input)?;
    let s_tape = decode(s_input)?;
    let mut layout = bit_register_edit_square_program(x_tape.len(), s_tape.len())?;
    layout.reads.extend_from_slice(&x_tape);
    layout.reads.extend_from_slice(&s_tape);
    let width = layout.width;
    let result_word_len = layout.result_word.len() * 4;
    let mut vm = ParaVM::new();
    vm.load(&layout.source)?;
    vm.set_reads(layout.reads);
    vm.run(None);
    let lane_emit_count: usize = layout.lane_tapes.iter().map(Vec::len).sum();
    let expected_emits = width * 4 + width + 1 + width * 2 + result_word_len + lane_emit_count + 5;
    if !vm.halted || vm.emit_buffer.len() != expected_emits {
        return Err(alloc::format!("edit-square membrane emitted {} of {expected_emits} cells", vm.emit_buffer.len()));
    }
    let mut cursor = 0usize;
    let mut take_tape = |length: usize| -> Result<String, String> {
        let end = cursor + length;
        let value = vm.emit_buffer[cursor..end].iter().map(|cell| emitted_bit(cell)).collect::<Result<String, _>>()?;
        cursor = end;
        Ok(value)
    };
    let d1 = take_tape(width)?;
    let a = take_tape(width)?;
    let d2 = take_tape(width)?;
    let b = take_tape(width)?;
    let g_t_sum = take_tape(width + 1)?;
    let g_f_product = take_tape(width * 2)?;
    drop(take_tape);
    let mut result_word = String::new();
    for _ in 0..layout.result_word.len() {
        let mut ordinal = 0usize;
        for bit in 0..4 {
            if emitted_bit(&vm.emit_buffer[cursor])? == '⊥' { ordinal |= 1 << bit; }
            cursor += 1;
        }
        let glyph = imasm_core::imasm16_3::ALL_TOKENS.get(ordinal)
            .ok_or_else(|| alloc::format!("result word contains unknown canonical token ordinal {ordinal}"))?;
        result_word.push(glyph.glyph());
    }
    let mut take_tape = |length: usize| -> Result<String, String> {
        let end = cursor + length;
        let value = vm.emit_buffer[cursor..end].iter().map(|cell| emitted_bit(cell)).collect::<Result<String, _>>()?;
        cursor = end;
        Ok(value)
    };
    let mut lane_values = Vec::with_capacity(layout.lane_tapes.len());
    for tape in &layout.lane_tapes { lane_values.push(take_tape(tape.len())?); }
    drop(take_tape);
    let mut take_verdict = || -> Result<char, String> {
        let value = emitted_bit(&vm.emit_buffer[cursor])?;
        cursor += 1;
        Ok(if value == '⊤' { '⊤' } else { '⊥' })
    };
    let edit_closed = take_verdict()?;
    let arithmetic_closed = take_verdict()?;
    let x_positive = take_verdict()?;
    let s_positive = take_verdict()?;
    let closed = take_verdict()?;
    let exponents = &lane_values[3..21];
    let falsity_exponents = (0..3).map(|prime| {
        let i = prime * 6;
        PrimeExponentRow {
            prime: lane_values[prime].clone(),
            additive_abc: [exponents[i].clone(), exponents[i + 1].clone(), exponents[i + 2].clone()],
            multiplicative_d1_d2_c: [exponents[i + 3].clone(), exponents[i + 4].clone(), exponents[i + 5].clone()],
        }
    }).collect();
    let information_support = RadicalSupportLane {
        additive_radical: lane_values[21].clone(),
        multiplicative_radical: lane_values[22].clone(),
        additive_support: lane_values[23].clone(),
        multiplicative_support: lane_values[24].clone(),
    };
    Ok(EditSquareResult {
        d1, a, d2, b, g_t_sum, g_f_product, result_word,
        falsity_exponents, information_support, lane_witness_applies: closed,
        edit_closed, arithmetic_closed,
        positive_inputs: if x_positive == '⊤' && s_positive == '⊤' { '⊤' } else { '⊥' },
        closed,
    })
}

/// Wire a restoring divider into a larger instruction stream. Its temporary
/// remainder and difference are B4 tape banks; a unique label prefix lets
/// several division closures coexist in one membrane.
fn append_bit_register_divmod(
    source: &mut String,
    dividend_bits: &[usize],
    divisor_bits: &[usize],
    quotient_bits: &[usize],
    remainder_bits: &[usize],
    scratch_start: usize,
    label_prefix: &str,
) -> usize {
    use core::fmt::Write as _;
    let remainder_width = remainder_bits.len();
    let difference_start = scratch_start;
    let borrow = difference_start + remainder_width;

    for (bit, &divisor) in divisor_bits.iter().enumerate() {
        writeln!(source, "JT %r{divisor} .{label_prefix}_zero_check_{}", bit + 1).unwrap();
        writeln!(source, "JF %r{divisor} .{label_prefix}_nonzero").unwrap();
        writeln!(source, ".{label_prefix}_zero_check_{}:", bit + 1).unwrap();
    }
    source.push_str("HALT\n");
    writeln!(source, ".{label_prefix}_nonzero:").unwrap();

    for &bit in quotient_bits { writeln!(source, "MOVE %r11 %r{bit}").unwrap(); }
    for &bit in remainder_bits { writeln!(source, "MOVE %r11 %r{bit}").unwrap(); }

    for (dividend_index, &dividend) in dividend_bits.iter().enumerate().rev() {
        for position in (1..remainder_width).rev() {
            writeln!(source, "MOVE %r{} %r{}", remainder_bits[position - 1], remainder_bits[position]).unwrap();
        }
        writeln!(source, "MOVE %r{dividend} %r{}", remainder_bits[0]).unwrap();
        writeln!(source, "MOVE %r11 %r{borrow}").unwrap();
        for bit in 0..remainder_width {
            writeln!(source, "MOVE %r{} %r0", remainder_bits[bit]).unwrap();
            if let Some(&divisor) = divisor_bits.get(bit) {
                writeln!(source, "MOVE %r{divisor} %r1").unwrap();
            } else {
                source.push_str("MOVE %r11 %r1\n");
            }
            writeln!(source, "MOVE %r{borrow} %r2").unwrap();
            source.push_str("CALL .full_subtractor\n");
            writeln!(source, "MOVE %r3 %r{}", difference_start + bit).unwrap();
            writeln!(source, "MOVE %r4 %r{borrow}").unwrap();
        }
        writeln!(source, "JT %r{borrow} .{label_prefix}_take_sub_{dividend_index}").unwrap();
        writeln!(source, "JF %r{borrow} .{label_prefix}_keep_rem_{dividend_index}").unwrap();
        writeln!(source, ".{label_prefix}_take_sub_{dividend_index}:").unwrap();
        writeln!(source, "MOVE %r12 %r{}", quotient_bits[dividend_index]).unwrap();
        for bit in 0..remainder_width {
            writeln!(source, "MOVE %r{} %r{}", difference_start + bit, remainder_bits[bit]).unwrap();
        }
        writeln!(source, "JMP .{label_prefix}_step_end_{dividend_index}").unwrap();
        writeln!(source, ".{label_prefix}_keep_rem_{dividend_index}:").unwrap();
        writeln!(source, "MOVE %r11 %r{}", quotient_bits[dividend_index]).unwrap();
        writeln!(source, ".{label_prefix}_step_end_{dividend_index}:").unwrap();
    }
    difference_start + remainder_width + 1
}

/// Emit the shared multiplicative arm. The compiler wires by widths only;
/// operand bits remain READ data and all partial products and carries are
/// computed by IMASM gates. `extra_width` reserves a third encoded tape for
/// the closure comparator nested around this product.
fn bit_register_mul_body(a_width: usize, b_width: usize, extra_width: usize) -> BitRegisterMulLayout {
    use core::fmt::Write as _;

    let a_start = 32usize;
    let b_start = a_start + a_width;
    let extra_start = b_start + b_width;
    let product_width = a_width + b_width;
    let product_start = extra_start + extra_width;
    let scratch_start = product_start + product_width;
    let mut source = String::new();

    source.push_str("ENGAGR %r10\nFSPLIT %r10 %r11 %r12\n");
    for bit in 0..a_width { writeln!(source, "READ %r{}", a_start + bit).unwrap(); }
    for bit in 0..b_width { writeln!(source, "READ %r{}", b_start + bit).unwrap(); }
    for bit in 0..extra_width { writeln!(source, "READ %r{}", extra_start + bit).unwrap(); }
    if extra_width > 0 {
        // The compiler supplies addresses only. Pairing and reverse transport
        // execute in the resident IMASM stream before product closure.
        for group in (0..a_width.max(b_width)).step_by(2) {
            for (lane, start, width, bit) in [
                (0, a_start, a_width, group), (1, a_start, a_width, group + 1),
                (2, b_start, b_width, group), (3, b_start, b_width, group + 1),
            ] {
                let from = if bit < width { start + bit } else { 11 };
                writeln!(source, "MOVE %r{from} %r{lane}").unwrap();
            }
            source.push_str("CALL .paired_transport\n");
            for (lane, start, width, bit) in [
                (0, a_start, a_width, group), (1, a_start, a_width, group + 1),
                (2, b_start, b_width, group), (3, b_start, b_width, group + 1),
            ] {
                if bit < width { writeln!(source, "MOVE %r{lane} %r{}", start + bit).unwrap(); }
            }
        }
    }
    let a_bits: Vec<usize> = (a_start..a_start + a_width).collect();
    let b_bits: Vec<usize> = (b_start..b_start + b_width).collect();
    let product_bits: Vec<usize> = (product_start..product_start + product_width).collect();
    let next_register = append_bit_register_mul(&mut source, &a_bits, &b_bits, &product_bits, scratch_start);
    BitRegisterMulLayout {
        source,
        a_start,
        b_start,
        extra_start,
        product_start,
        product_width,
        next_register,
    }
}

/// Compile a schoolbook multiplier into a complete IMASM instruction stream.
fn bit_register_mul_program(a_width: usize, b_width: usize) -> String {
    use core::fmt::Write as _;
    let mut layout = bit_register_mul_body(a_width, b_width, 0);
    for bit in 0..layout.product_width {
        writeln!(layout.source, "EMIT %r{}", layout.product_start + bit).unwrap();
    }
    layout.source.push_str("HALT\n");
    layout.source.push_str(BIT_REGISTER_GATE_LIBRARY_ASM);
    layout.source
}

/// Nest an exact product around an IMASM equality closure. The third operand
/// is another encoded tape; no host multiplication or comparison decides the
/// closure value.
fn bit_register_product_closure_program(a_width: usize, b_width: usize, n_width: usize) -> String {
    use core::fmt::Write as _;
    let mut layout = bit_register_mul_body(a_width, b_width, n_width);
    let closure = layout.next_register;
    let compare_width = core::cmp::max(layout.product_width, n_width);
    writeln!(layout.source, "MOVE %r11 %r{closure}").unwrap();
    for bit in 0..compare_width {
        writeln!(layout.source, "JT %r{closure} .cmp_active_{bit}").unwrap();
        writeln!(layout.source, "JF %r{closure} .cmp_after_{bit}").unwrap();
        writeln!(layout.source, ".cmp_active_{bit}:").unwrap();
        if bit < layout.product_width {
            writeln!(layout.source, "MOVE %r{} %r0", layout.product_start + bit).unwrap();
        } else {
            layout.source.push_str("MOVE %r11 %r0\n");
        }
        if bit < n_width {
            writeln!(layout.source, "MOVE %r{} %r1", layout.extra_start + bit).unwrap();
        } else {
            layout.source.push_str("MOVE %r11 %r1\n");
        }
        writeln!(layout.source, "JT %r0 .cmp_product_zero_{bit}").unwrap();
        writeln!(layout.source, "JF %r0 .cmp_product_one_{bit}").unwrap();
        layout.source.push_str("JMP .invalid\n");
        writeln!(layout.source, ".cmp_product_zero_{bit}:").unwrap();
        writeln!(layout.source, "JT %r1 .cmp_equal_{bit}").unwrap();
        writeln!(layout.source, "JF %r1 .cmp_mismatch_{bit}").unwrap();
        writeln!(layout.source, ".cmp_product_one_{bit}:").unwrap();
        writeln!(layout.source, "JF %r1 .cmp_equal_{bit}").unwrap();
        writeln!(layout.source, "JT %r1 .cmp_mismatch_{bit}").unwrap();
        layout.source.push_str("JMP .invalid\n");
        writeln!(layout.source, ".cmp_equal_{bit}:").unwrap();
        writeln!(layout.source, "JMP .cmp_after_{bit}").unwrap();
        writeln!(layout.source, ".cmp_mismatch_{bit}:").unwrap();
        writeln!(layout.source, "MOVE %r12 %r{closure}").unwrap();
        writeln!(layout.source, "JMP .cmp_after_{bit}").unwrap();
        writeln!(layout.source, ".cmp_after_{bit}:").unwrap();
    }
    // Pair transport has fused both factor frames. Fix only an exact product
    // closure, after every comparison has completed.
    writeln!(layout.source, "JT %r{closure} .closure_fix").unwrap();
    layout.source.push_str("JMP .closure_emit\n.closure_fix:\n");
    writeln!(layout.source, "IFIX %r{closure}").unwrap();
    layout.source.push_str(".closure_emit:\n");
    writeln!(layout.source, "EMIT %r{closure}\nHALT").unwrap();
    layout.source.push_str(PAIRED_FRAME_LIBRARY_ASM);
    layout.source.push_str(BIT_REGISTER_GATE_LIBRARY_ASM);
    layout.source
}

/// Compile restoring long division into IMASM cells. The generated circuit
/// depends on operand widths, while the two numerals remain encoded READ
/// values. Quotient decisions, comparisons, borrows, and remainder updates
/// all execute through the instruction stream.
fn bit_register_divmod_program(a_width: usize, b_width: usize) -> String {
    use core::fmt::Write as _;

    let a_start = 32usize;
    let b_start = a_start + a_width;
    let remainder_width = b_width + 1;
    let remainder_start = b_start + b_width;
    let difference_start = remainder_start + remainder_width;
    let quotient_start = difference_start + remainder_width;
    let borrow = quotient_start + a_width;
    let mut source = String::new();

    source.push_str("ENGAGR %r10\nFSPLIT %r10 %r11 %r12\n");
    for bit in 0..a_width { writeln!(source, "READ %r{}", a_start + bit).unwrap(); }
    for bit in 0..b_width { writeln!(source, "READ %r{}", b_start + bit).unwrap(); }

    // A zero divisor has no emitted quotient/remainder. T means 0, F means 1.
    for bit in 0..b_width {
        writeln!(source, "JT %r{} .divisor_zero_check_{}", b_start + bit, bit + 1).unwrap();
        source.push_str("JF %r");
        writeln!(source, "{} .divisor_nonzero", b_start + bit).unwrap();
        writeln!(source, ".divisor_zero_check_{}:", bit + 1).unwrap();
    }
    source.push_str("HALT\n.divisor_nonzero:\n");

    for bit in 0..a_width { writeln!(source, "MOVE %r11 %r{}", quotient_start + bit).unwrap(); }
    // Initialize each remainder cell, including its guard bit.
    for bit in 0..remainder_width {
        writeln!(source, "MOVE %r11 %r{}", remainder_start + bit).unwrap();
    }

    for dividend_bit in (0..a_width).rev() {
        for position in (1..remainder_width).rev() {
            writeln!(source, "MOVE %r{} %r{}", remainder_start + position - 1, remainder_start + position).unwrap();
        }
        writeln!(source, "MOVE %r{} %r{}", a_start + dividend_bit, remainder_start).unwrap();
        source.push_str("MOVE %r11 %r");
        writeln!(source, "{borrow}").unwrap();
        for bit in 0..remainder_width {
            writeln!(source, "MOVE %r{} %r0", remainder_start + bit).unwrap();
            if bit < b_width {
                writeln!(source, "MOVE %r{} %r1", b_start + bit).unwrap();
            } else {
                source.push_str("MOVE %r11 %r1\n");
            }
            writeln!(source, "MOVE %r{borrow} %r2").unwrap();
            source.push_str("CALL .full_subtractor\n");
            writeln!(source, "MOVE %r3 %r{}", difference_start + bit).unwrap();
            writeln!(source, "MOVE %r4 %r{borrow}").unwrap();
        }
        writeln!(source, "JT %r{borrow} .take_sub_{dividend_bit}").unwrap();
        writeln!(source, "JF %r{borrow} .keep_rem_{dividend_bit}").unwrap();
        writeln!(source, ".take_sub_{dividend_bit}:").unwrap();
        writeln!(source, "MOVE %r12 %r{}", quotient_start + dividend_bit).unwrap();
        for bit in 0..remainder_width {
            writeln!(source, "MOVE %r{} %r{}", difference_start + bit, remainder_start + bit).unwrap();
        }
        writeln!(source, "JMP .div_step_end_{dividend_bit}").unwrap();
        writeln!(source, ".keep_rem_{dividend_bit}:").unwrap();
        writeln!(source, "MOVE %r11 %r{}", quotient_start + dividend_bit).unwrap();
        writeln!(source, ".div_step_end_{dividend_bit}:").unwrap();
    }

    for bit in 0..a_width { writeln!(source, "EMIT %r{}", quotient_start + bit).unwrap(); }
    for bit in 0..remainder_width { writeln!(source, "EMIT %r{}", remainder_start + bit).unwrap(); }
    source.push_str("HALT\n");
    source.push_str(BIT_REGISTER_GATE_LIBRARY_ASM);
    source
}

/// Compose a bounded, width-specialized Euclidean chain. Each quotient and
/// remainder is produced by the same IMASM restoring divider, with early
/// closure when the encoded divisor reaches zero. The width controls only
/// circuit topology; both operands remain runtime B4 tapes.
fn bit_register_gcd_program(a_width: usize, b_width: usize) -> String {
    use core::fmt::Write as _;

    let width = core::cmp::max(a_width, b_width);
    let a_input = 32usize;
    let b_input = a_input + a_width;
    let a_bank = b_input + b_width;
    let b_bank = a_bank + width;
    let quotient = b_bank + width;
    let remainder = quotient + width;
    let work = remainder + width + 1;
    let a_bits: Vec<usize> = (a_bank..a_bank + width).collect();
    let b_bits: Vec<usize> = (b_bank..b_bank + width).collect();
    let q_bits: Vec<usize> = (quotient..quotient + width).collect();
    let r_bits: Vec<usize> = (remainder..remainder + width + 1).collect();
    let mut source = String::from("ENGAGR %r10\nFSPLIT %r10 %r11 %r12\n");
    for bit in 0..a_width { writeln!(source, "READ %r{}", a_input + bit).unwrap(); }
    for bit in 0..b_width { writeln!(source, "READ %r{}", b_input + bit).unwrap(); }
    for bit in 0..width {
        if bit < a_width { writeln!(source, "MOVE %r{} %r{}", a_input + bit, a_bits[bit]).unwrap(); }
        else { writeln!(source, "MOVE %r11 %r{}", a_bits[bit]).unwrap(); }
        if bit < b_width { writeln!(source, "MOVE %r{} %r{}", b_input + bit, b_bits[bit]).unwrap(); }
        else { writeln!(source, "MOVE %r11 %r{}", b_bits[bit]).unwrap(); }
    }

    // Euclid takes fewer than 2w divisions for w-bit nonnegative operands.
    // All iterations are present in the word; divisor-zero branches close
    // the chain as soon as the live remainder becomes zero.
    for iteration in 0..(2 * width + 1) {
        for (bit, &register) in b_bits.iter().enumerate() {
            writeln!(source, "JT %r{register} .gcd_zero_{iteration}_{bit}").unwrap();
            writeln!(source, "JF %r{register} .gcd_continue_{iteration}").unwrap();
            writeln!(source, ".gcd_zero_{iteration}_{bit}:").unwrap();
        }
        source.push_str("JMP .gcd_done\n");
        writeln!(source, ".gcd_continue_{iteration}:").unwrap();
        append_bit_register_divmod(
            &mut source, &a_bits, &b_bits, &q_bits, &r_bits, work,
            &alloc::format!("gcd_div_{iteration}"),
        );
        for bit in 0..width {
            writeln!(source, "MOVE %r{} %r{}", b_bits[bit], a_bits[bit]).unwrap();
            writeln!(source, "MOVE %r{} %r{}", r_bits[bit], b_bits[bit]).unwrap();
        }
    }
    // The final bound check prevents a truncated Euclidean chain from
    // emitting a plausible value if the width-bound invariant is violated.
    for (bit, &register) in b_bits.iter().enumerate() {
        writeln!(source, "JT %r{register} .gcd_final_zero_{bit}").unwrap();
        source.push_str("HALT\n");
        writeln!(source, ".gcd_final_zero_{bit}:").unwrap();
    }
    source.push_str(".gcd_done:\n");
    for register in &a_bits { writeln!(source, "EMIT %r{register}").unwrap(); }
    source.push_str("HALT\n");
    source.push_str(BIT_REGISTER_GATE_LIBRARY_ASM);
    source
}

/// Compose the multiplication and restoring-division membranes into a
/// width-specialized modular-exponentiation flow. The compiler wires only
/// the tape shapes; exponent branches inspect the encoded B4 bits at runtime.
fn bit_register_powmod_program(base_width: usize, exponent_width: usize, modulus_width: usize) -> String {
    use core::fmt::Write as _;

    let base_start = 32usize;
    let exponent_start = base_start + base_width;
    let modulus_start = exponent_start + exponent_width;
    let persistent_start = modulus_start + modulus_width;
    let result_bits: Vec<usize> = (persistent_start..persistent_start + modulus_width).collect();
    let base_bits: Vec<usize> = (persistent_start + modulus_width..persistent_start + 2 * modulus_width).collect();
    let product_bits: Vec<usize> = (persistent_start + 2 * modulus_width..persistent_start + 4 * modulus_width).collect();
    let quotient_width = core::cmp::max(base_width, 2 * modulus_width);
    let quotient_bits: Vec<usize> = (persistent_start + 4 * modulus_width..persistent_start + 4 * modulus_width + quotient_width).collect();
    let remainder_bits: Vec<usize> = (persistent_start + 4 * modulus_width + quotient_width
        ..persistent_start + 5 * modulus_width + quotient_width + 1).collect();
    let work_start = persistent_start + 5 * modulus_width + quotient_width + 1;
    let base_input: Vec<usize> = (base_start..base_start + base_width).collect();
    let exponent: Vec<usize> = (exponent_start..exponent_start + exponent_width).collect();
    let modulus: Vec<usize> = (modulus_start..modulus_start + modulus_width).collect();
    let one_bit = [12usize]; // FSPLIT's F arm is encoded one (⊥).

    let mut source = String::from("ENGAGR %r10\nFSPLIT %r10 %r11 %r12\n");
    for bit in 0..base_width { writeln!(source, "READ %r{}", base_start + bit).unwrap(); }
    for bit in 0..exponent_width { writeln!(source, "READ %r{}", exponent_start + bit).unwrap(); }
    for bit in 0..modulus_width { writeln!(source, "READ %r{}", modulus_start + bit).unwrap(); }

    // Set result to 1 mod N and the phase base to base mod N. Both reductions
    // share the same IMASM divider and return their residues on the tape.
    append_bit_register_divmod(&mut source, &one_bit, &modulus, &quotient_bits[..1], &remainder_bits, work_start, "pow_one_reduce");
    for bit in 0..modulus_width {
        writeln!(source, "MOVE %r{} %r{}", remainder_bits[bit], result_bits[bit]).unwrap();
    }
    append_bit_register_divmod(&mut source, &base_input, &modulus, &quotient_bits[..base_width], &remainder_bits, work_start, "pow_base_reduce");
    for bit in 0..modulus_width {
        writeln!(source, "MOVE %r{} %r{}", remainder_bits[bit], base_bits[bit]).unwrap();
    }

    for (index, &exponent_bit) in exponent.iter().enumerate() {
        writeln!(source, "JT %r{exponent_bit} .pow_skip_product_{index}").unwrap();
        writeln!(source, "JF %r{exponent_bit} .pow_product_{index}").unwrap();
        writeln!(source, ".pow_product_{index}:").unwrap();
        append_bit_register_mul(&mut source, &result_bits, &base_bits, &product_bits, work_start);
        append_bit_register_divmod(
            &mut source, &product_bits, &modulus, &quotient_bits[..2 * modulus_width],
            &remainder_bits, work_start, &alloc::format!("pow_result_reduce_{index}"),
        );
        for bit in 0..modulus_width {
            writeln!(source, "MOVE %r{} %r{}", remainder_bits[bit], result_bits[bit]).unwrap();
        }
        writeln!(source, ".pow_skip_product_{index}:").unwrap();

        append_bit_register_mul(&mut source, &base_bits, &base_bits, &product_bits, work_start);
        append_bit_register_divmod(
            &mut source, &product_bits, &modulus, &quotient_bits[..2 * modulus_width],
            &remainder_bits, work_start, &alloc::format!("pow_base_square_reduce_{index}"),
        );
        for bit in 0..modulus_width {
            writeln!(source, "MOVE %r{} %r{}", remainder_bits[bit], base_bits[bit]).unwrap();
        }
    }

    for &bit in &result_bits { writeln!(source, "EMIT %r{bit}").unwrap(); }
    source.push_str("HALT\n");
    source.push_str(BIT_REGISTER_GATE_LIBRARY_ASM);
    source
}

/// One resident dyadic phase step. The same assembled instruction stream is
/// reused as the phase residue changes; every square and reduction is in it.
pub struct PhaseSquare {
    vm: ParaVM,
    modulus: Vec<B4>,
    width: usize,
}

/// Build the odd-anchor inverse recurrence. At step k, the residual's low
/// cell selects q_k. A selected bit subtracts the candidate register, then
/// the signed residual shifts once. The remaining residual is the closure.
fn bit_register_complement_program(candidate_width: usize, source_width: usize) -> String {
    use core::fmt::Write as _;
    let width = candidate_width.max(source_width) + 2;
    let candidate_start = 32;
    let source_start = candidate_start + candidate_width;
    let residual_start = source_start + source_width;
    let difference_start = residual_start + width;
    let complement_start = difference_start + width;
    let borrow = complement_start + source_width;
    let mut source = String::from("ENGAGR %r10\nFSPLIT %r10 %r11 %r12\n");
    for reg in candidate_start..source_start + source_width {
        writeln!(source, "READ %r{reg}").unwrap();
    }
    writeln!(source, "JF %r{candidate_start} .complement_odd\nHALT\n.complement_odd:").unwrap();
    for bit in 0..width {
        let input = if bit < source_width { source_start + bit } else { 11 };
        writeln!(source, "MOVE %r{input} %r{}", residual_start + bit).unwrap();
    }
    for step in 0..source_width {
        writeln!(source, "MOVE %r{residual_start} %r{}", complement_start + step).unwrap();
        writeln!(source, "JT %r{} .complement_shift_{step}", complement_start + step).unwrap();
        writeln!(source, "MOVE %r11 %r{borrow}").unwrap();
        for bit in 0..width {
            writeln!(source, "MOVE %r{} %r0", residual_start + bit).unwrap();
            let input = if bit < candidate_width { candidate_start + bit } else { 11 };
            writeln!(source, "MOVE %r{input} %r1\nMOVE %r{borrow} %r2\nCALL .full_subtractor").unwrap();
            writeln!(source, "MOVE %r3 %r{}\nMOVE %r4 %r{borrow}", difference_start + bit).unwrap();
        }
        for bit in 0..width {
            writeln!(source, "MOVE %r{} %r{}", difference_start + bit, residual_start + bit).unwrap();
        }
        writeln!(source, ".complement_shift_{step}:").unwrap();
        for bit in 0..width - 1 {
            writeln!(source, "MOVE %r{} %r{}", residual_start + bit + 1, residual_start + bit).unwrap();
        }
    }
    for bit in 0..width {
        writeln!(source, "JT %r{} .complement_zero_{bit}\nHALT\n.complement_zero_{bit}:",
            residual_start + bit).unwrap();
    }
    for bit in 0..source_width {
        writeln!(source, "EMIT %r{}", complement_start + bit).unwrap();
    }
    source.push_str("HALT\n");
    source.push_str(BIT_REGISTER_GATE_LIBRARY_ASM);
    source
}

pub fn complement_encoded_lsb_first(candidate: &str, source: &str)
    -> Result<Option<String>, String> {
    let decode = |stream: &str| stream.chars().map(|symbol| match symbol {
        '⊤' => Ok(B4::T),
        '⊥' => Ok(B4::F),
        other => Err(alloc::format!("invalid complement cell {other:?}")),
    }).collect::<Result<Vec<_>, _>>();
    let candidate_bits = decode(candidate)?;
    let source_bits = decode(source)?;
    if candidate_bits.is_empty() || source_bits.is_empty() {
        return Err("complement requires two encoded words".into());
    }
    let mut reads = candidate_bits.clone();
    reads.extend_from_slice(&source_bits);
    let mut vm = ParaVM::new();
    vm.load(&bit_register_complement_program(candidate_bits.len(), source_bits.len()))?;
    vm.set_reads(reads);
    vm.run(None);
    if !vm.halted { return Err("complement did not halt".into()); }
    if vm.emit_buffer.is_empty() { return Ok(None); }
    if vm.emit_buffer.len() != source_bits.len() {
        return Err("complement emitted an incomplete word".into());
    }
    let mut complement = String::new();
    for cell in &vm.emit_buffer {
        let (_, value) = cell.rsplit_once(" = ")
            .ok_or_else(|| "complement emitted a malformed cell".to_string())?;
        match value {
            "T" => complement.push('⊤'),
            "F" => complement.push('⊥'),
            _ => return Err(alloc::format!("complement emitted {value}")),
        }
    }
    Ok(Some(complement))
}

impl PhaseSquare {
    pub fn new(modulus: &str) -> Result<Self, String> {
        use core::fmt::Write as _;
        let bits = modulus.chars().map(|symbol| match symbol {
            '⊤' => Ok(B4::T),
            '⊥' => Ok(B4::F),
            other => Err(alloc::format!("invalid phase modulus cell {other:?}")),
        }).collect::<Result<Vec<_>, _>>()?;
        if bits.is_empty() { return Err("empty phase modulus".into()); }
        let width = bits.len();
        let base_start = 32;
        let modulus_start = base_start + width;
        let product_start = modulus_start + width;
        let quotient_start = product_start + 2 * width;
        let remainder_start = quotient_start + 2 * width;
        let work_start = remainder_start + width + 1;
        let base: Vec<_> = (base_start..base_start + width).collect();
        let modulus_regs: Vec<_> = (modulus_start..modulus_start + width).collect();
        let product: Vec<_> = (product_start..product_start + 2 * width).collect();
        let quotient: Vec<_> = (quotient_start..quotient_start + 2 * width).collect();
        let remainder: Vec<_> = (remainder_start..remainder_start + width + 1).collect();
        let mut source = String::from("ENGAGR %r10\nFSPLIT %r10 %r11 %r12\n");
        for register in base.iter().chain(modulus_regs.iter()) {
            writeln!(source, "READ %r{register}").unwrap();
        }
        append_bit_register_mul(&mut source, &base, &base, &product, work_start);
        append_bit_register_divmod(&mut source, &product, &modulus_regs, &quotient,
            &remainder, work_start, "phase_square_reduce");
        for register in remainder.iter().take(width) {
            writeln!(source, "EMIT %r{register}").unwrap();
        }
        source.push_str("HALT\n");
        source.push_str(BIT_REGISTER_GATE_LIBRARY_ASM);
        let mut vm = ParaVM::new();
        vm.load(&source)?;
        Ok(Self { vm, modulus: bits, width })
    }

    pub fn observe(&mut self, residue: &str) -> Result<String, String> {
        let mut reads = residue.chars().map(|symbol| match symbol {
            '⊤' => Ok(B4::T),
            '⊥' => Ok(B4::F),
            other => Err(alloc::format!("invalid phase residue cell {other:?}")),
        }).collect::<Result<Vec<_>, _>>()?;
        if reads.len() > self.width { return Err("phase residue exceeds modulus width".into()); }
        reads.resize(self.width, B4::T);
        reads.extend_from_slice(&self.modulus);
        self.vm.registers.clear();
        self.vm.belief.clear();
        self.vm.call_stack.clear();
        self.vm.data_stack.clear();
        self.vm.emit_buffer.clear();
        self.vm.pc = 0;
        self.vm.halted = false;
        self.vm.set_reads(reads);
        self.vm.run(None);
        if !self.vm.halted || self.vm.emit_buffer.len() != self.width {
            return Err("resident phase square did not return a complete residue".into());
        }
        let mut result = String::new();
        for cell in &self.vm.emit_buffer {
            let (_, value) = cell.rsplit_once(" = ")
                .ok_or_else(|| "resident phase square emitted a malformed cell".to_string())?;
            match value {
                "T" => result.push('⊤'),
                "F" => result.push('⊥'),
                _ => return Err(alloc::format!("resident phase square emitted {value}")),
            }
        }
        Ok(result)
    }
}

/// Run the IMASM adder on two LSB-first numeral streams, returning the
/// emitted LSB-first sum including its final carry cell. This boundary only
/// translates the encoded alphabet to/from ParaVM's B4 cells; all addition,
/// carry propagation, and termination are performed by BIT_REGISTER_ADD_ASM.
pub fn add_encoded_lsb_first(a: &str, b: &str) -> Result<String, String> {
    fn decode(stream: &str) -> Result<Vec<B4>, String> {
        if stream.is_empty() {
            return Err("expected a non-empty LSB-first ⊤/⊥ bitstream".into());
        }
        stream.chars().map(|symbol| match symbol {
            '⊤' => Ok(B4::T),
            '⊥' => Ok(B4::F),
            other => Err(alloc::format!("invalid numeral symbol {other:?}; expected ⊤ or ⊥")),
        }).collect()
    }

    let left = decode(a)?;
    let right = decode(b)?;
    let width = core::cmp::max(left.len(), right.len());
    let mut reads = Vec::with_capacity(width * 2 + 1);
    for index in 0..width {
        reads.push(left.get(index).copied().unwrap_or(B4::T));
        reads.push(right.get(index).copied().unwrap_or(B4::T));
    }
    reads.push(B4::N);

    let mut vm = ParaVM::new();
    vm.load(BIT_REGISTER_ADD_ASM)?;
    vm.set_reads(reads);
    vm.run(None);
    if !vm.halted {
        return Err("IMASM adder did not close on its encoded terminator".into());
    }

    let mut result = String::new();
    for cell in &vm.emit_buffer {
        let (_, value) = cell.rsplit_once(" = ")
            .ok_or_else(|| "IMASM adder emitted a malformed cell".to_string())?;
        match value {
            "T" => result.push('⊤'),
            "F" => result.push('⊥'),
            _ => return Err(alloc::format!("IMASM adder emitted non-bit state {value}")),
        }
    }
    Ok(result)
}

/// Run the IMASM subtractor on two LSB-first numeral streams. The result
/// includes the final borrow cell: ⊥ means unsigned underflow, ⊤ means no
/// underflow. The interface performs encoding/decoding only.
pub fn subtract_encoded_lsb_first(a: &str, b: &str) -> Result<String, String> {
    fn decode(stream: &str) -> Result<Vec<B4>, String> {
        if stream.is_empty() {
            return Err("expected a non-empty LSB-first ⊤/⊥ bitstream".into());
        }
        stream.chars().map(|symbol| match symbol {
            '⊤' => Ok(B4::T),
            '⊥' => Ok(B4::F),
            other => Err(alloc::format!("invalid numeral symbol {other:?}; expected ⊤ or ⊥")),
        }).collect()
    }

    let left = decode(a)?;
    let right = decode(b)?;
    let width = core::cmp::max(left.len(), right.len());
    let mut reads = Vec::with_capacity(width * 2 + 1);
    for index in 0..width {
        reads.push(left.get(index).copied().unwrap_or(B4::T));
        reads.push(right.get(index).copied().unwrap_or(B4::T));
    }
    reads.push(B4::N);

    let mut vm = ParaVM::new();
    vm.load(BIT_REGISTER_SUB_ASM)?;
    vm.set_reads(reads);
    vm.run(None);
    if !vm.halted {
        return Err("IMASM subtractor did not close on its encoded terminator".into());
    }

    let mut result = String::new();
    for cell in &vm.emit_buffer {
        let (_, value) = cell.rsplit_once(" = ")
            .ok_or_else(|| "IMASM subtractor emitted a malformed cell".to_string())?;
        match value {
            "T" => result.push('⊤'),
            "F" => result.push('⊥'),
            _ => return Err(alloc::format!("IMASM subtractor emitted non-bit state {value}")),
        }
    }
    Ok(result)
}

/// Multiply two dynamically sized LSB-first encoded values. Only the source
/// topology is specialized to their widths; the values enter as encoded B4
/// cells and are multiplied by the generated IMASM instruction stream.
pub fn multiply_encoded_lsb_first(a: &str, b: &str) -> Result<String, String> {
    fn decode(stream: &str) -> Result<Vec<B4>, String> {
        if stream.is_empty() {
            return Err("expected a non-empty LSB-first ⊤/⊥ bitstream".into());
        }
        stream.chars().map(|symbol| match symbol {
            '⊤' => Ok(B4::T),
            '⊥' => Ok(B4::F),
            other => Err(alloc::format!("invalid numeral symbol {other:?}; expected ⊤ or ⊥")),
        }).collect()
    }

    let left = decode(a)?;
    let right = decode(b)?;
    let mut reads = Vec::with_capacity(left.len() + right.len());
    reads.extend_from_slice(&left);
    reads.extend_from_slice(&right);

    let mut vm = ParaVM::new();
    vm.load(&bit_register_mul_program(left.len(), right.len()))?;
    vm.set_reads(reads);
    vm.run(None);
    if !vm.halted {
        return Err("IMASM multiplier did not halt after its instruction stream".into());
    }

    let mut product = String::new();
    for cell in &vm.emit_buffer {
        let (_, value) = cell.rsplit_once(" = ")
            .ok_or_else(|| "IMASM multiplier emitted a malformed cell".to_string())?;
        match value {
            "T" => product.push('⊤'),
            "F" => product.push('⊥'),
            _ => return Err(alloc::format!("IMASM multiplier emitted non-bit state {value}")),
        }
    }
    if product.chars().count() != left.len() + right.len() {
        return Err("IMASM multiplier emitted an incomplete product tape".into());
    }
    Ok(product)
}

/// Verify `a*b == n` inside one nested IMASM stream: the schoolbook product
/// is composed first, then its output tape is closed against the encoded N
/// tape by in-stream B4 equality branches. Returns ⊤ for exact closure and ⊥
/// for a mismatch.
pub fn product_closure_encoded_lsb_first(a: &str, b: &str, n: &str) -> Result<char, String> {
    fn decode(stream: &str) -> Result<Vec<B4>, String> {
        if stream.is_empty() {
            return Err("expected a non-empty LSB-first ⊤/⊥ bitstream".into());
        }
        stream.chars().map(|symbol| match symbol {
            '⊤' => Ok(B4::T),
            '⊥' => Ok(B4::F),
            other => Err(alloc::format!("invalid numeral symbol {other:?}; expected ⊤ or ⊥")),
        }).collect()
    }

    let left = decode(a)?;
    let right = decode(b)?;
    let target = decode(n)?;
    let mut reads = Vec::with_capacity(left.len() + right.len() + target.len());
    reads.extend_from_slice(&left);
    reads.extend_from_slice(&right);
    reads.extend_from_slice(&target);

    let mut vm = ParaVM::new();
    vm.load(&bit_register_product_closure_program(left.len(), right.len(), target.len()))?;
    vm.set_reads(reads);
    vm.run(None);
    if !vm.halted || vm.emit_buffer.len() != 1 {
        return Err("product-closure membrane did not produce one closed verdict".into());
    }
    match vm.emit_buffer[0].rsplit_once(" = ").map(|(_, value)| value) {
        Some("T [FIXED]") => Ok('⊤'),
        Some("F") => Ok('⊥'),
        Some(value) => Err(alloc::format!("product-closure emitted non-classical state {value}")),
        None => Err("product-closure emitted a malformed verdict cell".into()),
    }
}

/// Divide two encoded values through the generated restoring-division
/// instruction stream. Quotient and remainder are returned LSB-first at
/// their working widths; an empty result means the IMASM zero-divisor arm
/// halted before emission.
pub fn divmod_encoded_lsb_first(a: &str, b: &str) -> Result<(String, String), String> {
    fn decode(stream: &str) -> Result<Vec<B4>, String> {
        if stream.is_empty() {
            return Err("expected a non-empty LSB-first ⊤/⊥ bitstream".into());
        }
        stream.chars().map(|symbol| match symbol {
            '⊤' => Ok(B4::T),
            '⊥' => Ok(B4::F),
            other => Err(alloc::format!("invalid numeral symbol {other:?}; expected ⊤ or ⊥")),
        }).collect()
    }

    let left = decode(a)?;
    let right = decode(b)?;
    let a_width = left.len();
    let b_width = right.len();
    let mut reads = Vec::with_capacity(a_width + b_width);
    reads.extend_from_slice(&left);
    reads.extend_from_slice(&right);

    let mut vm = ParaVM::new();
    vm.load(&bit_register_divmod_program(a_width, b_width))?;
    vm.set_reads(reads);
    vm.run(None);
    if !vm.halted {
        return Err("IMASM divider did not halt after its instruction stream".into());
    }
    if vm.emit_buffer.is_empty() {
        return Err(alloc::format!("division by zero (closed by the IMASM zero-divisor branch; divisor cells {:?}, read {}, pc {})",
            (0..b_width).map(|bit| vm.belief_of(32 + a_width + bit)).collect::<Vec<_>>(), vm.read_pos, vm.pc));
    }
    let mut cells = Vec::with_capacity(vm.emit_buffer.len());
    for cell in &vm.emit_buffer {
        let (_, value) = cell.rsplit_once(" = ")
            .ok_or_else(|| "IMASM divider emitted a malformed cell".to_string())?;
        cells.push(match value {
            "T" => '⊤',
            "F" => '⊥',
            _ => return Err(alloc::format!("IMASM divider emitted non-bit state {value}")),
        });
    }
    let remainder_width = b_width + 1;
    if cells.len() != a_width + remainder_width {
        return Err("IMASM divider emitted incomplete quotient/remainder tapes".into());
    }
    let quotient: String = cells[..a_width].iter().collect();
    let remainder: String = cells[a_width..].iter().collect();
    Ok((quotient, remainder))
}

/// Remainder-only projection of the same IMASM divmod circuit.
pub fn modulo_encoded_lsb_first(a: &str, b: &str) -> Result<String, String> {
    divmod_encoded_lsb_first(a, b).map(|(_, remainder)| remainder)
}

/// Run an unrolled Euclidean closure in one IMASM instruction stream and
/// return the greatest common divisor at the common input width.
pub fn gcd_encoded_lsb_first(a: &str, b: &str) -> Result<String, String> {
    fn decode(stream: &str) -> Result<Vec<B4>, String> {
        if stream.is_empty() {
            return Err("expected a non-empty LSB-first ⊤/⊥ bitstream".into());
        }
        stream.chars().map(|symbol| match symbol {
            '⊤' => Ok(B4::T),
            '⊥' => Ok(B4::F),
            other => Err(alloc::format!("invalid numeral symbol {other:?}; expected ⊤ or ⊥")),
        }).collect()
    }

    let left = decode(a)?;
    let right = decode(b)?;
    let width = core::cmp::max(left.len(), right.len());
    let mut vm = ParaVM::new();
    vm.load(&bit_register_gcd_program(left.len(), right.len()))?;
    let mut reads = left;
    reads.extend_from_slice(&right);
    vm.set_reads(reads);
    vm.run(None);
    if !vm.halted || vm.emit_buffer.len() != width {
        return Err("IMASM Euclidean membrane failed to close a complete gcd tape".into());
    }
    let mut result = String::new();
    for cell in &vm.emit_buffer {
        let (_, value) = cell.rsplit_once(" = ")
            .ok_or_else(|| "IMASM gcd emitted a malformed cell".to_string())?;
        match value {
            "T" => result.push('⊤'),
            "F" => result.push('⊥'),
            _ => return Err(alloc::format!("IMASM gcd emitted non-bit state {value}")),
        }
    }
    Ok(result)
}

/// Modular exponentiation as one composed IMASM membrane: encoded base,
/// exponent, and modulus tapes enter READ; phase-bit branches select product
/// closures; every modular reduction reuses the instruction-level divider.
pub fn powmod_encoded_lsb_first(base: &str, exponent: &str, modulus: &str) -> Result<String, String> {
    fn decode(stream: &str) -> Result<Vec<B4>, String> {
        if stream.is_empty() {
            return Err("expected a non-empty LSB-first ⊤/⊥ bitstream".into());
        }
        stream.chars().map(|symbol| match symbol {
            '⊤' => Ok(B4::T),
            '⊥' => Ok(B4::F),
            other => Err(alloc::format!("invalid numeral symbol {other:?}; expected ⊤ or ⊥")),
        }).collect()
    }

    let base_bits = decode(base)?;
    let exponent_bits = decode(exponent)?;
    let modulus_bits = decode(modulus)?;
    let mut reads = Vec::with_capacity(base_bits.len() + exponent_bits.len() + modulus_bits.len());
    reads.extend_from_slice(&base_bits);
    reads.extend_from_slice(&exponent_bits);
    reads.extend_from_slice(&modulus_bits);

    let mut vm = ParaVM::new();
    vm.load(&bit_register_powmod_program(base_bits.len(), exponent_bits.len(), modulus_bits.len()))?;
    vm.set_reads(reads);
    vm.run(None);
    if !vm.halted {
        return Err("IMASM modular-phase program did not halt at closure".into());
    }
    if vm.emit_buffer.is_empty() {
        return Err("modulus zero (closed by the IMASM zero-divisor arm)".into());
    }
    if vm.emit_buffer.len() != modulus_bits.len() {
        return Err("IMASM modular-phase program emitted an incomplete residue".into());
    }
    let mut residue = String::new();
    for cell in &vm.emit_buffer {
        let (_, value) = cell.rsplit_once(" = ")
            .ok_or_else(|| "IMASM modular-phase program emitted a malformed cell".to_string())?;
        match value {
            "T" => residue.push('⊤'),
            "F" => residue.push('⊥'),
            _ => return Err(alloc::format!("IMASM modular-phase program emitted non-bit state {value}")),
        }
    }
    Ok(residue)
}

impl ParaAsm {
    pub fn op_name(&self) -> &'static str {
        match self {
            ParaAsm::ENGAGR(_)  => "ENGAGR",
            ParaAsm::FSPLIT(..) => "FSPLIT",
            ParaAsm::FFUSE(..)  => "FFUSE",
            ParaAsm::IFIX(_)    => "IFIX",
            ParaAsm::MOVE(..)   => "MOVE",
            ParaAsm::CLEAR(_)   => "CLEAR",
            ParaAsm::JMP(_)     => "JMP",
            ParaAsm::JB(..)     => "JB",
            ParaAsm::JT(..)     => "JT",
            ParaAsm::JF(..)     => "JF",
            ParaAsm::JN(..)     => "JN",
            ParaAsm::CALL(_)    => "CALL",
            ParaAsm::RET        => "RET",
            ParaAsm::HALT       => "HALT",
            ParaAsm::PUSH(_)    => "PUSH",
            ParaAsm::POP(_)     => "POP",
            ParaAsm::EMIT(_)    => "EMIT",
            ParaAsm::READ(_)    => "READ",
        }
    }

    /// Arity classification for kernel verification.
    pub fn is_frobenius(&self) -> bool {
        matches!(self, ParaAsm::ENGAGR(_) | ParaAsm::FSPLIT(..) |
                      ParaAsm::FFUSE(..) | ParaAsm::IFIX(_))
    }
}

// ── ParaRegister ────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct ParaRegister {
    pub flux: B4,
    pub value: Option<&'static str>, // Some("FIXED") or None
    pub paradox_count: u32,
}

impl ParaRegister {
    pub fn new() -> Self {
        Self { flux: B4::N, value: None, paradox_count: 0 }
    }

    pub fn engage(&mut self) {
        self.flux = B4::B;
        self.paradox_count += 1;
    }

    pub fn is_fixed(&self) -> bool { self.value == Some("FIXED") }

    pub fn is_active(&self) -> bool {
        self.flux != B4::N || self.value.is_some()
    }

    pub fn clear(&mut self) {
        self.flux = B4::N;
        self.value = None;
    }
}

// ── Assembler ───────────────────────────────────────────────────────

pub struct AssembledProgram {
    pub instructions: Vec<ParaAsm>,
    pub label_map: BTreeMap<String, usize>,
}

/// Parse a register argument: "%r0" → 0, "%r15" → 15
fn parse_reg(arg: &str) -> Result<usize, String> {
    if arg.starts_with("%r") {
        arg[2..].parse::<usize>()
            .map_err(|_| format!("bad register: {}", arg))
    } else {
        Err(format!("expected %rN, got: {}", arg))
    }
}

/// Assemble ParaASM source text → (instructions, label_map).
/// Labels: `.name:` at start of line. Comments: `;` to end of line.
pub fn assemble(text: &str) -> Result<AssembledProgram, String> {
    let mut instrs: Vec<ParaAsm> = Vec::new();
    let mut labels: BTreeMap<String, usize> = BTreeMap::new();

    for (lineno, raw) in text.lines().enumerate() {
        // Strip comment
        let line = match raw.split(';').next() {
            Some(s) => String::from(s.trim()),
            None => continue,
        };
        if line.is_empty() { continue; }

        // Check for label: .name: [instruction]
        let (label, rest): (Option<String>, String) = if line.starts_with('.') {
            if let Some(colon) = line.find(':') {
                let lbl = String::from(&line[..colon]);
                let after = String::from(line[colon + 1..].trim());
                (Some(lbl), after)
            } else {
                (None, line)
            }
        } else {
            (None, line)
        };

        if let Some(lbl) = label {
            labels.insert(lbl, instrs.len());
        }

        if rest.is_empty() { continue; }

        let parts: Vec<&str> = rest.split_whitespace().collect();
        if parts.is_empty() { continue; }

        let op = parts[0].to_uppercase();
        let args = &parts[1..];

        let instr = match op.as_str() {
            "ENGAGR" => {
                if args.len() < 1 { return Err(format!("line {}: ENGAGR needs 1 arg", lineno)); }
                ParaAsm::ENGAGR(parse_reg(args[0])?)
            }
            "FSPLIT" => {
                if args.len() < 3 { return Err(format!("line {}: FSPLIT needs 3 args", lineno)); }
                ParaAsm::FSPLIT(parse_reg(args[0])?, parse_reg(args[1])?, parse_reg(args[2])?)
            }
            "FFUSE" => {
                if args.len() < 2 { return Err(format!("line {}: FFUSE needs ≥2 args", lineno)); }
                let sources: Result<Vec<usize>, _> = args[..args.len()-1].iter()
                    .map(|a| parse_reg(a)).collect();
                let dst = parse_reg(args[args.len()-1])?;
                ParaAsm::FFUSE(sources?, dst)
            }
            "IFIX" => {
                if args.len() < 1 { return Err(format!("line {}: IFIX needs 1 arg", lineno)); }
                ParaAsm::IFIX(parse_reg(args[0])?)
            }
            "MOVE" => {
                if args.len() < 2 { return Err(format!("line {}: MOVE needs 2 args", lineno)); }
                ParaAsm::MOVE(parse_reg(args[0])?, parse_reg(args[1])?)
            }
            "CLEAR" => {
                if args.len() < 1 { return Err(format!("line {}: CLEAR needs 1 arg", lineno)); }
                ParaAsm::CLEAR(parse_reg(args[0])?)
            }
            "JMP" => {
                if args.len() < 1 { return Err(format!("line {}: JMP needs label", lineno)); }
                ParaAsm::JMP(String::from(args[0]))
            }
            "JB" => {
                if args.len() < 2 { return Err(format!("line {}: JB needs reg + label", lineno)); }
                ParaAsm::JB(parse_reg(args[0])?, String::from(args[1]))
            }
            "JT" => {
                if args.len() < 2 { return Err(format!("line {}: JT needs reg + label", lineno)); }
                ParaAsm::JT(parse_reg(args[0])?, String::from(args[1]))
            }
            "JF" => {
                if args.len() < 2 { return Err(format!("line {}: JF needs reg + label", lineno)); }
                ParaAsm::JF(parse_reg(args[0])?, String::from(args[1]))
            }
            "JN" => {
                if args.len() < 2 { return Err(format!("line {}: JN needs reg + label", lineno)); }
                ParaAsm::JN(parse_reg(args[0])?, String::from(args[1]))
            }
            "CALL" => {
                if args.len() < 1 { return Err(format!("line {}: CALL needs label", lineno)); }
                ParaAsm::CALL(String::from(args[0]))
            }
            "RET"  => ParaAsm::RET,
            "HALT" => ParaAsm::HALT,
            "PUSH" => {
                if args.len() < 1 { return Err(format!("line {}: PUSH needs 1 arg", lineno)); }
                ParaAsm::PUSH(parse_reg(args[0])?)
            }
            "POP" => {
                if args.len() < 1 { return Err(format!("line {}: POP needs 1 arg", lineno)); }
                ParaAsm::POP(parse_reg(args[0])?)
            }
            "EMIT" => {
                if args.len() < 1 { return Err(format!("line {}: EMIT needs 1 arg", lineno)); }
                ParaAsm::EMIT(parse_reg(args[0])?)
            }
            "READ" => {
                if args.len() < 1 { return Err(format!("line {}: READ needs 1 arg", lineno)); }
                ParaAsm::READ(parse_reg(args[0])?)
            }
            _ => return Err(format!("line {}: unknown opcode '{}'", lineno, op)),
        };
        instrs.push(instr);
    }

    Ok(AssembledProgram { instructions: instrs, label_map: labels })
}

// ── ParaVM ──────────────────────────────────────────────────────────

/// Practical Paraconsistent Universal Engine VM.
/// Belnap foundation: src/belnap.rs.
pub struct ParaVM {
    pub registers: BTreeMap<usize, ParaRegister>,
    pub belief: BTreeMap<usize, B4>,
    pub program: Vec<ParaAsm>,
    pub label_map: BTreeMap<String, usize>,
    pub pc: usize,
    pub total_steps: u64,
    pub cycles: u64,
    pub call_stack: Vec<usize>,
    pub data_stack: Vec<B4>,
    pub halted: bool,
    /// I/O capture buffer (EMIT writes here, READ reads from here if set)
    pub emit_buffer: Vec<String>,
    pub read_buffer: Option<Vec<B4>>,
    pub read_pos: usize,
}

impl ParaVM {
    pub fn new() -> Self {
        Self {
            registers: BTreeMap::new(),
            belief: BTreeMap::new(),
            program: Vec::new(),
            label_map: BTreeMap::new(),
            pc: 0,
            total_steps: 0,
            cycles: 0,
            call_stack: Vec::new(),
            data_stack: Vec::new(),
            halted: false,
            emit_buffer: Vec::new(),
            read_buffer: None,
            read_pos: 0,
        }
    }

    /// Get or create a register, returning belief as B4.
    pub fn belief_of(&self, reg_id: usize) -> B4 {
        self.belief.get(&reg_id).copied().unwrap_or(B4::N)
    }

    /// Set belief and update register flux.
    pub fn set_belief(&mut self, reg_id: usize, val: B4) {
        self.belief.insert(reg_id, val);
        self.registers.entry(reg_id).or_insert_with(ParaRegister::new).flux = val;
    }

    /// Engage a register — set to Both, increment paradox counter.
    pub fn engage(&mut self, reg_id: usize) {
        self.registers.entry(reg_id).or_insert_with(ParaRegister::new).engage();
        self.belief.insert(reg_id, B4::B);
    }

    /// Resolve a label to a PC address.
    pub fn resolve(&self, label: &str) -> Result<usize, String> {
        self.label_map.get(label)
            .copied()
            .ok_or_else(|| format!("undefined label: {}", label))
    }

    /// Load assembled program.
    pub fn load_program(&mut self, prog: AssembledProgram) {
        self.program = prog.instructions;
        self.label_map = prog.label_map;
        self.pc = 0;
        self.halted = false;
        self.total_steps = 0;
        self.cycles = 0;
    }

    /// Assemble and load source text.
    pub fn load(&mut self, text: &str) -> Result<(), String> {
        let prog = assemble(text)?;
        self.load_program(prog);
        Ok(())
    }

    /// Pre-set read buffer (for deterministic testing).
    pub fn set_reads(&mut self, values: Vec<B4>) {
        self.read_buffer = Some(values);
        self.read_pos = 0;
    }
}

// ── Execution ───────────────────────────────────────────────────────

impl ParaVM {
    /// Execute one instruction (internal).
    fn _exec(&mut self, instr: &ParaAsm) {
        match instr {
            ParaAsm::ENGAGR(r) => {
                self.engage(*r);
            }
            ParaAsm::FSPLIT(src, d1, d2) => {
                let b = self.belief_of(*src);
                let reg = self.registers.entry(*src).or_insert_with(ParaRegister::new);
                let p = reg.paradox_count;
                if b == B4::B {
                    self.set_belief(*d1, B4::T);
                    self.set_belief(*d2, B4::F);
                    let bump = p + 1;
                    self.registers.entry(*d1).or_insert_with(ParaRegister::new).paradox_count = bump;
                    self.registers.entry(*d2).or_insert_with(ParaRegister::new).paradox_count = bump;
                } else {
                    self.set_belief(*d1, b);
                    self.set_belief(*d2, b);
                    self.registers.entry(*d1).or_insert_with(ParaRegister::new).paradox_count = p;
                    self.registers.entry(*d2).or_insert_with(ParaRegister::new).paradox_count = p;
                }
            }
            ParaAsm::FFUSE(sources, dst) => {
                let mut joined = self.belief_of(sources[0]);
                for src in &sources[1..] {
                    joined = joined.join(self.belief_of(*src));
                }
                self.set_belief(*dst, joined);
            }
            ParaAsm::IFIX(r) => {
                self.registers.entry(*r).or_insert_with(ParaRegister::new).value = Some("FIXED");
                self.set_belief(*r, B4::T);
            }
            ParaAsm::MOVE(src, dst) => {
                let v = self.belief_of(*src);
                self.set_belief(*dst, v);
            }
            ParaAsm::CLEAR(r) => {
                self.registers.entry(*r).or_insert_with(ParaRegister::new).clear();
                self.belief.insert(*r, B4::N);
            }
            ParaAsm::JMP(label) => {
                if let Ok(addr) = self.resolve(label) {
                    self.pc = addr;
                }
            }
            ParaAsm::JB(r, label) => {
                if self.belief_of(*r) == B4::B {
                    if let Ok(addr) = self.resolve(label) { self.pc = addr; }
                }
            }
            ParaAsm::JT(r, label) => {
                if self.belief_of(*r) == B4::T {
                    if let Ok(addr) = self.resolve(label) { self.pc = addr; }
                }
            }
            ParaAsm::JF(r, label) => {
                if self.belief_of(*r) == B4::F {
                    if let Ok(addr) = self.resolve(label) { self.pc = addr; }
                }
            }
            ParaAsm::JN(r, label) => {
                if self.belief_of(*r) == B4::N {
                    if let Ok(addr) = self.resolve(label) { self.pc = addr; }
                }
            }
            ParaAsm::CALL(label) => {
                if let Ok(addr) = self.resolve(label) {
                    self.call_stack.push(self.pc);
                    self.pc = addr;
                }
            }
            ParaAsm::RET => {
                if let Some(addr) = self.call_stack.pop() {
                    self.pc = addr;
                } else {
                    self.halted = true;
                }
            }
            ParaAsm::HALT => {
                self.halted = true;
            }
            ParaAsm::PUSH(r) => {
                self.data_stack.push(self.belief_of(*r));
            }
            ParaAsm::POP(r) => {
                let val = self.data_stack.pop().unwrap_or(B4::N);
                self.set_belief(*r, val);
            }
            ParaAsm::EMIT(r) => {
                let b = self.belief_of(*r);
                let fixed = if self.registers.get(r).map_or(false, |reg| reg.is_fixed()) {
                    " [FIXED]"
                } else { "" };
                self.emit_buffer.push(format!("%r{} = {}{}", r, b.name(), fixed));
            }
            ParaAsm::READ(r) => {
                let val = if let Some(ref buf) = self.read_buffer {
                    let v = buf.get(self.read_pos).copied().unwrap_or(B4::N);
                    self.read_pos += 1;
                    v
                } else {
                    B4::N // kernel-mode default
                };
                self.set_belief(*r, val);
            }
        }
    }

    /// Single step. Returns false if halted or program empty.
    pub fn step(&mut self) -> bool {
        if self.halted || self.program.is_empty() {
            return false;
        }
        if self.pc >= self.program.len() {
            self.pc = 0;
            self.cycles += 1;
        }
        let instr = self.program[self.pc].clone();
        self.pc += 1;
        self.total_steps += 1;
        self._exec(&instr);
        !self.halted
    }

    /// Run up to `steps` instructions (None = run until halt).
    pub fn run(&mut self, steps: Option<usize>) {
        let mut n = 0;
        while !self.halted {
            if let Some(max) = steps {
                if n >= max { break; }
            }
            if !self.step() { break; }
            n += 1;
        }
    }

    /// Execute a single instruction directly (no program needed).
    pub fn exec_one(&mut self, instr: &ParaAsm) {
        self.total_steps += 1;
        self._exec(instr);
    }

    /// Reset VM state.
    pub fn reset(&mut self) {
        *self = ParaVM::new();
    }
}

// ── Snapshot (for IG bridge) ────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct ParaVmSnapshot {
    pub steps: u64,
    pub cycles: u64,
    pub pc: usize,
    pub active: usize,
    pub fixed: usize,
    pub paradox: u32,
    pub dist_n: u32,
    pub dist_t: u32,
    pub dist_f: u32,
    pub dist_b: u32,
    pub halted: bool,
    pub data_stack_depth: usize,
    pub call_stack_depth: usize,
}

impl ParaVM {
    pub fn snapshot(&self) -> ParaVmSnapshot {
        let (mut dist_n, mut dist_t, mut dist_f, mut dist_b) = (0u32, 0u32, 0u32, 0u32);
        let mut paradox = 0u32;
        let mut active = 0usize;
        let mut fixed = 0usize;

        // Collect all register IDs
        let mut ids: Vec<usize> = self.registers.keys().chain(self.belief.keys()).copied().collect();
        ids.sort();
        ids.dedup();

        for rid in ids {
            let b = self.belief_of(rid);
            match b {
                B4::N => dist_n += 1,
                B4::T => dist_t += 1,
                B4::F => dist_f += 1,
                B4::B => dist_b += 1,
            }
            if let Some(reg) = self.registers.get(&rid) {
                paradox += reg.paradox_count;
                if reg.is_active() { active += 1; }
                if reg.is_fixed() { fixed += 1; }
            }
        }

        ParaVmSnapshot {
            steps: self.total_steps,
            cycles: self.cycles,
            pc: self.pc,
            active,
            fixed,
            paradox,
            dist_n, dist_t, dist_f, dist_b,
            halted: self.halted,
            data_stack_depth: self.data_stack.len(),
            call_stack_depth: self.call_stack.len(),
        }
    }

    /// Active registers: (id, belief, paradox_count, is_fixed).
    pub fn active_regs(&self) -> Vec<(usize, B4, u32, bool)> {
        let mut ids: Vec<usize> = self.registers.keys().chain(self.belief.keys()).copied().collect();
        ids.sort();
        ids.dedup();
        ids.into_iter()
            .filter(|rid| {
                self.registers.get(rid).map_or(false, |r| r.is_active())
                || self.belief.contains_key(rid)
            })
            .map(|rid| {
                let reg = self.registers.get(&rid);
                (
                    rid,
                    self.belief_of(rid),
                    reg.map_or(0, |r| r.paradox_count),
                    reg.map_or(false, |r| r.is_fixed()),
                )
            })
            .collect()
    }
}

// ── Dialetheic Alignment ────────────────────────────────────────────

/// Dialetheic image: B → B, T/F → T, N → N.
pub fn dialetheic_image(r0: B4) -> B4 {
    match r0 {
        B4::B => B4::B,
        B4::T | B4::F => B4::T,
        B4::N => B4::N,
    }
}

/// B is the only bifurcation point under FSPLIT.
/// ∀r: if r=B then FSPLIT(r)→(T,F) with T≠F; if r≠B then both outputs equal.
pub fn b_is_only_bifurcation_point() -> bool {
    for &r in &[B4::N, B4::T, B4::F, B4::B] {
        // execute fsplit
        let (d1, d2) = if r == B4::B { (B4::T, B4::F) } else { (r, r) };
        if r == B4::B && d1 == d2 { return false; }
        if r != B4::B && d1 != d2 { return false; }
    }
    true
}

/// Three-arm dialetheic alignment check.
/// Returns (operational, logical, algebraic).
pub fn dialetheic_alignment_tri() -> (bool, bool, bool) {
    // operational: ffuse∘fsplit(B) = B ∧ B is only bifurcation
    let op = {
        let (d1, d2) = (B4::T, B4::F); // fsplit(B)
        d1.join(d2) == B4::B // ffuse
            && b_is_only_bifurcation_point()
    };
    // logical: B is dialetheic, nothing else is
    let log = B4::B.dialetheic()
        && ![B4::N, B4::T, B4::F].iter().any(|x| x.dialetheic());
    // algebraic: N not designated, T∨F=B, designated(B∧¬B)
    let alg = !B4::N.designated()
        && B4::T.join(B4::F) == B4::B
        && B4::B.band(B4::B.bnot()).designated();
    (op, log, alg)
}

// ── Measurement Sequence Algebra ────────────────────────────────────

/// Cost of measuring q with bias.
/// B→B: cost 2, B→T/F: cost 1, non-B: cost 0.
pub fn measure_cost(q: B4, bias: B4) -> u8 {
    if q != B4::B { return 0; }
    if bias == B4::B { 2 } else { 1 }
}

/// Single measurement step: collapse B under bias.
pub fn measure_step(q: B4, bias: B4) -> B4 {
    if q == B4::B {
        if bias == B4::B { B4::B } else { bias }
    } else {
        q
    }
}

/// Is q irreversible once collapsed? (T/F/N cannot return to B via single ops.)
pub fn collapse_irreversible(q: B4) -> bool {
    if q == B4::B { return true; }
    // Check: none of bnot, join(q,q), meet(q,q), band(q,q), bor(q,q) returns B
    ![q.bnot(), q.join(q), q.meet(q), q.band(q), q.bor(q)].contains(&B4::B)
}

/// Wigner's-friend-then-collapse cost: B→B measurement then collapse = 3.
pub fn wigner_then_collapse_cost(n: u32) -> u32 { 3 * n }

// ── Kernel-state bridge (mirrors p4ramill_py.kernel) ────────────────

#[derive(Clone, Debug)]
pub struct KernelState {
    pub r0: B4,
    pub r1: B4,
    pub r2: B4,
    pub paradox_count: u32,
    pub cycle_count: u32,
}

impl KernelState {
    pub fn new() -> Self {
        Self { r0: B4::B, r1: B4::B, r2: B4::B, paradox_count: 0, cycle_count: 0 }
    }

    /// Apply one frobenius kernel step: fsplit(r0)→(r1,r2), ffuse(r1,r2)→r0.
    pub fn kernel_step(&mut self) {
        let (d1, d2) = if self.r0 == B4::B {
            self.paradox_count += 1;
            (B4::T, B4::F)
        } else {
            (self.r0, self.r0)
        };
        self.r1 = d1;
        self.r2 = d2;
        self.r0 = d1.join(d2); // ffuse
        self.cycle_count += 1;
    }

    /// Run n kernel steps.
    pub fn kernel_run(&mut self, n: u32) {
        for _ in 0..n { self.kernel_step(); }
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assemble_simple() {
        let src = "
            .start:
            ENGAGR %r0
            FSPLIT %r0 %r1 %r2
            FFUSE %r1 %r2 %r0
            IFIX %r0
            HALT
        ";
        let prog = assemble(src).unwrap();
        assert_eq!(prog.instructions.len(), 5);
        assert_eq!(prog.label_map.get(".start"), Some(&0));
        assert_eq!(prog.instructions[0], ParaAsm::ENGAGR(0));
        assert_eq!(prog.instructions[4], ParaAsm::HALT);
    }

    #[test]
    fn test_vm_basic_cycle() {
        let mut vm = ParaVM::new();
        vm.load("
            ENGAGR %r0
            FSPLIT %r0 %r1 %r2
            FFUSE %r1 %r2 %r0
            HALT
        ").unwrap();
        vm.run(None);
        let snap = vm.snapshot();
        // After ENGAGR: r0=B, paradox=1
        // After FSPLIT: r1=T, r2=F, paradox=2
        // After FFUSE: r0=T∨F=B
        assert!(snap.halted);
        assert_eq!(snap.paradox, 5);
        assert_eq!(vm.belief_of(0), B4::B);
    }

    #[test]
    fn imasm_ripple_adder_covers_full_adder_truth_table() {
        use crate::belnap::B4::{F, N, T};

        for a in [false, true] {
            for b in [false, true] {
                for carry_in in [false, true] {
                    // Prefix a bit pair that sets the desired carry, then let
                    // the candidate pair exercise that row of the truth table.
                    let (prefix_a, prefix_b) = if carry_in { (F, F) } else { (T, T) };
                    let mut vm = ParaVM::new();
                    vm.load(BIT_REGISTER_ADD_ASM).unwrap();
                    vm.set_reads(vec![
                        prefix_a, prefix_b,
                        if a { F } else { T },
                        if b { F } else { T },
                        N,
                    ]);
                    vm.run(None);

                    let emitted: Vec<B4> = vm.emit_buffer.iter().map(|line| {
                        if line.ends_with("= T") { T }
                        else if line.ends_with("= F") { F }
                        else { panic!("IMASM adder emitted a non-bit value: {line}") }
                    }).collect();
                    let sum = a ^ b ^ carry_in;
                    let carry_out = (a && b) || (a && carry_in) || (b && carry_in);
                    assert!(vm.halted, "stream terminator must close the IMASM loop");
                    assert_eq!(emitted, vec![
                        T, // The prefix always sums to zero.
                        if sum { F } else { T },
                        if carry_out { F } else { T },
                    ]);
                    assert_eq!(vm.read_pos, 5);
                }
            }
        }
    }

    #[test]
    fn imasm_ripple_adder_scales_past_machine_word_width() {
        use crate::belnap::B4::{F, N, T};

        // (2^300 - 1) + 1 = 2^300, supplied as an LSB-first stream.
        let width = 300usize;
        let mut reads = Vec::with_capacity(width * 2 + 1);
        for bit in 0..width {
            reads.push(F);
            reads.push(if bit == 0 { F } else { T });
        }
        reads.push(N);
        let mut vm = ParaVM::new();
        vm.load(BIT_REGISTER_ADD_ASM).unwrap();
        vm.set_reads(reads);
        vm.run(None);
        let emitted: Vec<B4> = vm.emit_buffer.iter().map(|line| {
            if line.ends_with("= T") { T }
            else if line.ends_with("= F") { F }
            else { panic!("IMASM adder emitted a non-bit value: {line}") }
        }).collect();
        assert!(vm.halted, "stream terminator must close the IMASM loop");
        assert_eq!(emitted, [vec![T; width], vec![F]].concat());
        assert_eq!(vm.read_pos, width * 2 + 1);
    }

    #[test]
    fn encoded_adder_boundary_keeps_the_defined_top_bottom_bit_mapping() {
        assert_eq!(add_encoded_lsb_first("⊥", "⊥").unwrap(), "⊤⊥");
        assert_eq!(add_encoded_lsb_first("⊥⊤", "⊤⊥").unwrap(), "⊥⊥⊤");
        assert!(add_encoded_lsb_first("⊤x", "⊥").is_err());
    }

    #[test]
    fn imasm_ripple_subtractor_covers_all_borrow_rows() {
        use crate::belnap::B4::{F, N, T};

        for a in [false, true] {
            for b in [false, true] {
                for borrow_in in [false, true] {
                    let (prefix_a, prefix_b) = if borrow_in { (T, F) } else { (T, T) };
                    let mut vm = ParaVM::new();
                    vm.load(BIT_REGISTER_SUB_ASM).unwrap();
                    vm.set_reads(vec![
                        prefix_a, prefix_b,
                        if a { F } else { T },
                        if b { F } else { T },
                        N,
                    ]);
                    vm.run(None);
                    let emitted: Vec<B4> = vm.emit_buffer.iter().map(|line| {
                        if line.ends_with("= T") { T }
                        else if line.ends_with("= F") { F }
                        else { panic!("IMASM subtractor emitted a non-bit value: {line}") }
                    }).collect();
                    let difference = a ^ b ^ borrow_in;
                    let borrow_out = (!a && (b || borrow_in)) || (b && borrow_in);
                    assert!(vm.halted, "stream terminator must close the IMASM loop");
                    assert_eq!(emitted, vec![
                        if borrow_in { F } else { T },
                        if difference { F } else { T },
                        if borrow_out { F } else { T },
                    ]);
                }
            }
        }
    }

    #[test]
    fn imasm_ripple_subtractor_scales_past_machine_word_width() {
        use crate::belnap::B4::{F, N, T};

        // 2^300 - 1 = 300 low one-bits, no final borrow.
        let width = 300usize;
        let mut reads = Vec::with_capacity((width + 1) * 2 + 1);
        reads.extend([T, F]); // low subtrahend bit is one
        for _ in 1..width { reads.extend([T, T]); }
        reads.extend([F, T, N]); // top bit of 2^300, then terminator
        let mut vm = ParaVM::new();
        vm.load(BIT_REGISTER_SUB_ASM).unwrap();
        vm.set_reads(reads);
        vm.run(None);
        let emitted: Vec<B4> = vm.emit_buffer.iter().map(|line| {
            if line.ends_with("= T") { T }
            else if line.ends_with("= F") { F }
            else { panic!("IMASM subtractor emitted a non-bit value: {line}") }
        }).collect();
        assert!(vm.halted);
        assert_eq!(emitted, [vec![F; width], vec![T, T]].concat());
        assert_eq!(vm.read_pos, (width + 1) * 2 + 1);
    }

    #[test]
    fn encoded_subtractor_reports_final_borrow_in_the_encoded_stream() {
        assert_eq!(subtract_encoded_lsb_first("⊤", "⊥").unwrap(), "⊥⊥");
        assert_eq!(subtract_encoded_lsb_first("⊥⊤", "⊤⊥").unwrap(), "⊥⊥⊥");
    }

    #[test]
    fn imasm_multiplier_matches_all_small_encoded_products() {
        fn stream(value: usize) -> String {
            if value == 0 { return String::from("⊤"); }
            let mut n = value;
            let mut bits = String::new();
            while n != 0 {
                bits.push(if n & 1 == 1 { '⊥' } else { '⊤' });
                n >>= 1;
            }
            bits
        }
        fn padded_product(value: usize, width: usize) -> String {
            (0..width).map(|bit| if (value >> bit) & 1 == 1 { '⊥' } else { '⊤' }).collect()
        }

        for a in 0..10usize {
            for b in 0..10usize {
                let a_stream = stream(a);
                let b_stream = stream(b);
                let got = multiply_encoded_lsb_first(&a_stream, &b_stream).unwrap();
                assert_eq!(got, padded_product(a * b, a_stream.chars().count() + b_stream.chars().count()),
                    "encoded product {a} × {b}");
            }
        }
    }

    #[test]
    fn imasm_multiplier_allocates_registers_past_byte_range() {
        let a = "⊥".repeat(260);
        let product = multiply_encoded_lsb_first(&a, "⊥").unwrap();
        assert_eq!(product, format!("{}⊤", a));
    }

    #[test]
    fn imasm_product_and_closure_are_nested_in_one_stream() {
        assert_eq!(product_closure_encoded_lsb_first("⊥⊥", "⊥⊤⊥", "⊥⊥⊥⊥").unwrap(), '⊤');
        assert_eq!(product_closure_encoded_lsb_first("⊥⊥", "⊥⊤⊥", "⊤⊤⊤⊤⊥").unwrap(), '⊥');
        assert_eq!(product_closure_encoded_lsb_first("⊥⊥", "⊥⊤⊥", "⊤⊤⊤⊤⊥⊤").unwrap(), '⊥');
    }

    #[test]
    fn paired_product_fuses_before_fixing_only_an_exact_closure() {
        let program = bit_register_product_closure_program(2, 2, 4);
        let fuse = program.find("CALL .paired_transport").unwrap();
        let fix = program.find("IFIX %r").unwrap();
        assert!(fuse < fix);

        for (target, expected) in [
            ([B4::F, B4::T, B4::T, B4::F], "T [FIXED]"), // 3 × 3 = 9
            ([B4::T, B4::T, B4::T, B4::F], "F"),         // 3 × 3 ≠ 8
        ] {
            let mut vm = ParaVM::new();
            vm.load(&program).unwrap();
            let mut reads = vec![B4::F; 4];
            reads.extend_from_slice(&target);
            vm.set_reads(reads);
            vm.run(None);
            assert!(vm.halted);
            assert_eq!(vm.emit_buffer.len(), 1);
            assert!(vm.emit_buffer[0].ends_with(expected));
        }
    }

    #[test]
    fn edit_square_checks_the_encoded_word_edits_and_both_arithmetic_arms() {
        let result = edit_square_encoded_lsb_first("⊤⊥", "⊤⊥").unwrap();
        assert_eq!(result.d1, "⊤⊥⊤⊤"); // 2
        assert_eq!(result.a, "⊥⊥⊤⊤"); // 3, zero-padded to circuit width
        assert_eq!(result.d2, "⊤⊤⊥⊤"); // 4
        assert_eq!(result.b, "⊥⊤⊥⊤"); // 5
        assert_eq!(result.g_t_sum, "⊤⊤⊤⊥⊤"); // 8
        assert_eq!(result.g_f_product, "⊤⊤⊤⊥⊤⊤⊤⊤"); // 8
        assert_eq!(result.result_word, EDIT_SQUARE_C);
        assert_eq!(result.edit_closed, '⊤');
        assert_eq!(result.arithmetic_closed, '⊤');
        assert_eq!(result.positive_inputs, '⊤');
        assert_eq!(result.closed, '⊤');
        assert_eq!(result.lane_witness_applies, '⊤');
        assert_eq!(result.falsity_exponents, vec![
            PrimeExponentRow { prime: "⊤⊥".into(), additive_abc: ["⊤".into(), "⊤".into(), "⊥⊥".into()], multiplicative_d1_d2_c: ["⊥".into(), "⊤⊥".into(), "⊥⊥".into()] },
            PrimeExponentRow { prime: "⊥⊥".into(), additive_abc: ["⊥".into(), "⊤".into(), "⊤".into()], multiplicative_d1_d2_c: ["⊤".into(), "⊤".into(), "⊤".into()] },
            PrimeExponentRow { prime: "⊥⊤⊥".into(), additive_abc: ["⊤".into(), "⊥".into(), "⊤".into()], multiplicative_d1_d2_c: ["⊤".into(), "⊤".into(), "⊤".into()] },
        ]);
        assert_eq!(result.information_support, RadicalSupportLane {
            additive_radical: "⊤⊥⊥⊥⊥".into(), multiplicative_radical: "⊤⊥".into(),
            additive_support: "⊥⊥⊥".into(), multiplicative_support: "⊥⊤⊤".into(),
        });
    }

    #[test]
    fn edit_square_rejects_nonclosing_positive_candidate() {
        let result = edit_square_encoded_lsb_first("⊤⊤⊥", "⊤⊥").unwrap();
        assert_eq!(result.edit_closed, '⊤');
        assert_eq!(result.arithmetic_closed, '⊥');
        assert_eq!(result.positive_inputs, '⊤');
        assert_eq!(result.closed, '⊥');
        assert_eq!(result.lane_witness_applies, '⊥');
    }

    #[test]
    fn imasm_product_closure_scales_past_machine_word_width() {
        let p = "⊥".repeat(260);
        let n = format!("{}⊤", p);
        assert_eq!(product_closure_encoded_lsb_first(&p, "⊥", &n).unwrap(), '⊤');
        let wrong = format!("⊤{}", &p["⊥".len()..]);
        assert_eq!(product_closure_encoded_lsb_first(&p, "⊥", &wrong).unwrap(), '⊥');
    }

    #[test]
    fn imasm_modular_phase_winding_composes_product_and_divisor_closure() {
        assert_eq!(powmod_encoded_lsb_first("⊤⊥", "⊤⊥⊤⊥", "⊥⊤⊤⊤⊥").unwrap(), "⊤⊤⊥⊤⊤");
        assert_eq!(powmod_encoded_lsb_first("⊥⊥", "⊥⊤⊥", "⊥⊥⊥").unwrap(), "⊥⊤⊥");
        assert_eq!(powmod_encoded_lsb_first("⊥⊤⊤⊥", "⊥⊥", "⊥⊤⊥").unwrap(), "⊤⊤⊥");
        assert_eq!(powmod_encoded_lsb_first("⊤", "⊤", "⊥⊥⊥").unwrap(), "⊥⊤⊤");
        assert_eq!(powmod_encoded_lsb_first("⊥", "⊤", "⊥").unwrap(), "⊤");
        assert!(powmod_encoded_lsb_first("⊥", "⊥", "⊤").is_err());
    }

    #[test]
    fn imasm_euclidean_closure_matches_small_gcds() {
        fn stream(value: usize) -> String {
            if value == 0 { return String::from("⊤"); }
            let mut n = value;
            let mut bits = String::new();
            while n != 0 {
                bits.push(if n & 1 == 1 { '⊥' } else { '⊤' });
                n >>= 1;
            }
            bits
        }
        fn expected(value: usize, width: usize) -> String {
            (0..width).map(|bit| if (value >> bit) & 1 == 1 { '⊥' } else { '⊤' }).collect()
        }
        fn gcd(mut a: usize, mut b: usize) -> usize {
            while b != 0 { (a, b) = (b, a % b); }
            a
        }

        for a in 0..20usize {
            for b in 0..20usize {
                let left = stream(a);
                let right = stream(b);
                let width = left.chars().count().max(right.chars().count());
                assert_eq!(gcd_encoded_lsb_first(&left, &right).unwrap(), expected(gcd(a, b), width), "gcd({a}, {b})");
            }
        }
        assert_eq!(gcd_encoded_lsb_first("⊥⊥⊤", "⊥⊤").unwrap(), "⊥⊤⊤");
        let wide = gcd_encoded_lsb_first(&"⊥".repeat(65), &"⊥".repeat(40)).unwrap();
        assert_eq!(wide, format!("{}{}", "⊥".repeat(5), "⊤".repeat(60)));
    }

    #[test]
    fn imasm_restoring_divider_matches_small_quotients_and_remainders() {
        fn stream(value: usize) -> String {
            if value == 0 { return String::from("⊤"); }
            let mut n = value;
            let mut bits = String::new();
            while n != 0 {
                bits.push(if n & 1 == 1 { '⊥' } else { '⊤' });
                n >>= 1;
            }
            bits
        }
        fn padded(value: usize, width: usize) -> String {
            (0..width).map(|bit| if (value >> bit) & 1 == 1 { '⊥' } else { '⊤' }).collect()
        }

        for dividend in 0..20usize {
            for divisor in 1..12usize {
                let a = stream(dividend);
                let b = stream(divisor);
                let (q, r) = divmod_encoded_lsb_first(&a, &b).unwrap();
                assert_eq!(q, padded(dividend / divisor, a.chars().count()),
                    "quotient {dividend} / {divisor}");
                assert_eq!(r, padded(dividend % divisor, b.chars().count() + 1),
                    "remainder {dividend} mod {divisor}");
            }
        }
        assert!(divmod_encoded_lsb_first("⊥", "⊤").is_err());
    }

    #[test]
    fn imasm_restoring_divider_scales_past_machine_word_width() {
        let a = "⊥".repeat(260);
        let (quotient, remainder) = divmod_encoded_lsb_first(&a, "⊥").unwrap();
        assert_eq!(quotient, a);
        assert_eq!(remainder, "⊤⊤");
    }

    #[test]
    fn test_imasm_native_decode_step() {
        // IMASM-NATIVE COMPUTE, seed of the self-disassembler.
        // The machine's native alphabet is B4 {N,T,F,B}; a byte is four cells.
        // This program reads a stream of B4 cells and, for each, DISPATCHES on its
        // value and emits a TRANSFORMED token — a nontrivial permutation
        //   N→T, T→F, F→B, B→N
        // proving data-dependent computation, not an echo. The decode "table" is
        // NOT data: it is the control-flow trie itself (JT/JF/JB dispatch = the
        // FSPLIT+EVALT structure), so the computation lives in the shape of the
        // word — compute inside the twelve. The subroutine call is CLINK; the
        // per-symbol branch is a fork that resolves to exactly one arm.
        let mut vm = ParaVM::new();
        // Constant cells the decode moves into the output register.
        vm.set_belief(10, B4::T);
        vm.set_belief(11, B4::F);
        vm.set_belief(12, B4::B);
        vm.set_belief(13, B4::N);
        // The input binary, re-expressed in the native B4 alphabet.
        vm.read_buffer = Some(vec![B4::T, B4::F, B4::B, B4::N]);
        vm.load("
            READ %r0
            CALL .decode
            READ %r0
            CALL .decode
            READ %r0
            CALL .decode
            READ %r0
            CALL .decode
            HALT
            .decode: JT %r0 .dT
            JF %r0 .dF
            JB %r0 .dB
            MOVE %r10 %r1   ; N -> T
            EMIT %r1
            RET
            .dT: MOVE %r11 %r1   ; T -> F
            EMIT %r1
            RET
            .dF: MOVE %r12 %r1   ; F -> B
            EMIT %r1
            RET
            .dB: MOVE %r13 %r1   ; B -> N
            EMIT %r1
            RET
        ").unwrap();
        vm.run(None);
        // The permutation applied to [T,F,B,N] is [F,B,N,T].
        let emitted: Vec<&str> = vm.emit_buffer.iter()
            .map(|s| s.rsplit(' ').next().unwrap_or(""))
            .collect();
        assert_eq!(emitted, vec!["F", "B", "N", "T"],
            "IMASM-native decode did not apply the permutation; emit = {:?}", vm.emit_buffer);
    }

    #[test]
    fn test_imasm_full_byte_decoder() {
        // PLANK 2: a full-byte instruction decoder, pure structure, no data table.
        // A byte is four B4 cells. Two carry the OPCODE field (a 16-leaf dispatch
        // trie, covering the twelve IMASM opcodes plus four spares) and two are the
        // OPERAND, passed through untouched. The wire opcode encoding is scrambled;
        // each leaf emits the CANONICAL IMASM token code, so the trie IS a real
        // 16-entry decode table realized as control flow. 16 opcodes × 16 operands
        // is the whole 256-byte space, decoded without a byte of data storage.
        const CELLS: [&str; 4] = ["N", "T", "F", "B"];      // cell index -> B4 name
        let perm = |w: usize| (w * 7 + 3) % 16;             // wire code -> opcode ordinal (a bijection)
        let canon = |ord: usize| -> (usize, usize) {        // ordinal -> canonical (hi,lo) cells
            if ord < 12 { (ord / 4, ord % 4) } else { (3, 3) } // >=12: (B,B) = unknown
        };
        // const cells live in r10..r13 (N,T,F,B); emit one canonical cell.
        let emit_cell = |idx: usize| format!("MOVE %r{} %r4\nEMIT %r4\n", 10 + idx);

        // Generate the decoder once: read 4 cells, two-level fork on the opcode
        // field, leaf emits canonical token cells then passes the operand through.
        let mut prog = String::from("READ %r0\nREAD %r1\nREAD %r2\nREAD %r3\n");
        prog += "JT %r0 .h1\nJF %r0 .h2\nJB %r0 .h3\n"; // r0==N falls through to .h0
        for i in 0..4 {
            prog += &format!(".h{}:\n", i);
            prog += &format!("JT %r1 .L{}_1\nJF %r1 .L{}_2\nJB %r1 .L{}_3\n", i, i, i);
            for j in 0..4 {
                let (hi, lo) = canon(perm(i * 4 + j));
                prog += &format!(".L{}_{}:\n", i, j);
                prog += &emit_cell(hi);          // canonical opcode hi
                prog += &emit_cell(lo);          // canonical opcode lo
                prog += "EMIT %r2\nEMIT %r3\n";  // operand, passed through
                prog += "JMP .done\n";
            }
        }
        prog += ".done:\nHALT\n";

        let decode_byte = |bytes: [usize; 4]| -> Vec<String> {
            let mut vm = ParaVM::new();
            vm.set_belief(10, B4::N);
            vm.set_belief(11, B4::T);
            vm.set_belief(12, B4::F);
            vm.set_belief(13, B4::B);
            let as_b4 = |v: usize| B4::from_u8(v as u8);
            vm.read_buffer = Some(bytes.iter().map(|&v| as_b4(v)).collect());
            vm.load(&prog).unwrap();
            vm.run(None);
            vm.emit_buffer.iter()
                .map(|s| String::from(s.rsplit(' ').next().unwrap_or("")))
                .collect()
        };

        // Check several bytes across different opcode leaves and operands. Expected
        // output is derived from the SAME table, so the assertion tests that the
        // control-flow trie faithfully realizes the decode, byte for byte.
        for bytes in [[1usize, 0, 2, 3], [0, 3, 1, 1], [2, 2, 3, 0], [3, 1, 0, 2]] {
            let (chi, clo) = canon(perm(bytes[0] * 4 + bytes[1]));
            let expected = vec![
                CELLS[chi].to_string(), CELLS[clo].to_string(),
                CELLS[bytes[2]].to_string(), CELLS[bytes[3]].to_string(),
            ];
            assert_eq!(decode_byte(bytes), expected,
                "full-byte decode mismatch for wire byte {:?}", bytes);
        }
    }

    #[test]
    fn test_imasm_instruction_stream_decoder() {
        // PLANK 3a: stream the byte decoder over a whole instruction stream.
        // The read/dispatch/emit block is wrapped in a ROTAT loop (.top). Every
        // leaf emits its token then loops; the reserved stop-opcode (wire (B,B))
        // HALTs. Feed a stream ending in a stop byte, collect the full IMASM word.
        let perm = |w: usize| (w * 7 + 3) % 16;
        let canon = |ord: usize| -> (usize, usize) {
            if ord < 12 { (ord / 4, ord % 4) } else { (3, 3) }
        };
        let emit_cell = |idx: usize| format!("MOVE %r{} %r4\nEMIT %r4\n", 10 + idx);
        let cells = ["N", "T", "F", "B"];

        let mut prog = String::from(".top:\nREAD %r0\nREAD %r1\nREAD %r2\nREAD %r3\n");
        prog += "JT %r0 .h1\nJF %r0 .h2\nJB %r0 .h3\n";
        for i in 0..4 {
            prog += &format!(".h{}:\n", i);
            prog += &format!("JT %r1 .L{}_1\nJF %r1 .L{}_2\nJB %r1 .L{}_3\n", i, i, i);
            for j in 0..4 {
                prog += &format!(".L{}_{}:\n", i, j);
                if i == 3 && j == 3 {
                    prog += "HALT\n";               // reserved stop-opcode ends the stream
                } else {
                    let (hi, lo) = canon(perm(i * 4 + j));
                    prog += &emit_cell(hi);
                    prog += &emit_cell(lo);
                    prog += "EMIT %r2\nEMIT %r3\n";
                    prog += "JMP .top\n";
                }
            }
        }

        let mut vm = ParaVM::new();
        vm.set_belief(10, B4::N);
        vm.set_belief(11, B4::T);
        vm.set_belief(12, B4::F);
        vm.set_belief(13, B4::B);
        let b = |v: usize| B4::from_u8(v as u8);
        // Two real instructions, then a stop byte (opcode (B,B)).
        let stream = [1usize, 0, 2, 3,   0, 3, 1, 1,   3, 3, 0, 0];
        vm.read_buffer = Some(stream.iter().map(|&v| b(v)).collect());
        vm.load(&prog).unwrap();
        vm.run(Some(10_000));
        let word: Vec<String> = vm.emit_buffer.iter()
            .map(|s| String::from(s.rsplit(' ').next().unwrap_or(""))).collect();

        // Expected: decode of the first two bytes (4 cells each), stop emits nothing.
        let mut expected = Vec::new();
        for byte in [[1usize, 0, 2, 3], [0, 3, 1, 1]] {
            let (chi, clo) = canon(perm(byte[0] * 4 + byte[1]));
            expected.push(cells[chi].to_string());
            expected.push(cells[clo].to_string());
            expected.push(cells[byte[2]].to_string());
            expected.push(cells[byte[3]].to_string());
        }
        assert_eq!(word, expected, "stream decode word mismatch");
        assert!(vm.halted, "stream did not halt on the stop-opcode");
    }

    #[test]
    fn test_evm_lift_reentrancy_verdict() {
        // PLANK 3b: lift real EVM control structure to an IMASM word and let the
        // kernel verdict it. The classic reentrancy bug is an ordering: a withdraw
        // that does its external CALL / commits state BEFORE the branch paths
        // rejoin leaves a window a re-entrant call slips through. In IMASM that is
        // a state commit (SSTORE = IFIX ⊡) landing inside a fork that has not fused
        // (JUMPDEST = FFUSE ∋). The engine, knowing nothing about Solidity, reports
        // the safe ordering CLOSED (T) and the vulnerable one OPEN (B).
        //
        // EVM opcode -> IMASM glyph (arm-splitting supplied by the CFG, as a full
        // lifter would; the per-opcode map is 1:1):
        let lift = |seq: &[&str]| -> String {
            seq.iter().map(|op| match *op {
                "ENTRY"                 => "⊢", // function entry (VINIT)
                "JUMPI"                 => "∈", // conditional branch = fork (FSPLIT)
                "THEN_BB"               => "≻", // taken basic block, work (AFWD)
                "THEN_TAG"              => "⊤", // the taken arm (EVALT)
                "ELSE_BB"               => "≺", // fall-through block, work (AREV)
                "ELSE_TAG"              => "⊥", // the else arm (EVALF)
                "JUMPDEST"              => "∋", // the merge point (FFUSE)
                "SSTORE"                => "⊡", // state commit, irreversible (IFIX)
                "STOP" | "RETURN"       => "⊣", // terminator (TANCH)
                _                       => "⊙", // unmodeled op = identity (IMSCRIB)
            }).collect()
        };

        // withdraw(), checks-effects-interactions: the guard's paths MERGE
        // (JUMPDEST) before the state write. Effects land after the fork resolves.
        let safe = lift(&["ENTRY", "JUMPI", "THEN_BB", "THEN_TAG",
                          "ELSE_BB", "ELSE_TAG", "JUMPDEST", "SSTORE", "STOP"]);
        // withdraw(), vulnerable: SSTORE commits state while the branch is still
        // open (no JUMPDEST merge before it) — the re-entrancy window.
        let vuln = lift(&["ENTRY", "JUMPI", "THEN_BB", "THEN_TAG",
                          "SSTORE", "ELSE_BB", "ELSE_TAG", "STOP"]);

        let verdict = |word: &str| -> char {
            let steps = imasm_core::imasm16_3::parse_glyph_word(word);
            imasm_core::imasm16_3::tri_ancestral_verdict(&steps).0
        };
        assert_eq!(verdict(&safe), 'T',
            "checks-effects-interactions should close; word = {safe}");
        assert_eq!(verdict(&vuln), 'B',
            "reentrant ordering should open (B); word = {vuln}");
    }

    // ── shared trie machinery for the round-trip planks (4, 5) ──────────────
    // Codes are 0..16, a code c encoded as two B4 cells (c/4, c%4). A trie reads
    // two cells and emits table(code) as two cells: disassemble and recompile are
    // the same generator with inverse tables.
    fn b4_idx(name: &str) -> usize {
        match name { "T" => 1, "F" => 2, "B" => 3, _ => 0 }
    }
    fn gen_code_trie(table: &dyn Fn(usize) -> usize) -> String {
        let emit_cell = |idx: usize| format!("MOVE %r{} %r4\nEMIT %r4\n", 10 + idx);
        let mut p = String::from("READ %r0\nREAD %r1\n");
        p += "JT %r0 .h1\nJF %r0 .h2\nJB %r0 .h3\n";
        for i in 0..4 {
            p += &format!(".h{}:\n", i);
            p += &format!("JT %r1 .L{}_1\nJF %r1 .L{}_2\nJB %r1 .L{}_3\n", i, i, i);
            for j in 0..4 {
                let out = table(i * 4 + j);
                p += &format!(".L{}_{}:\n", i, j);
                p += &emit_cell(out / 4);
                p += &emit_cell(out % 4);
                p += "JMP .done\n";
            }
        }
        p += ".done:\nHALT\n";
        p
    }
    fn run_code_trie(prog: &str, code: usize) -> usize {
        let mut vm = ParaVM::new();
        vm.set_belief(10, B4::N);
        vm.set_belief(11, B4::T);
        vm.set_belief(12, B4::F);
        vm.set_belief(13, B4::B);
        vm.read_buffer = Some(vec![
            B4::from_u8((code / 4) as u8), B4::from_u8((code % 4) as u8),
        ]);
        vm.load(prog).unwrap();
        vm.run(Some(10_000));
        let cells: Vec<usize> = vm.emit_buffer.iter()
            .map(|s| b4_idx(s.rsplit(' ').next().unwrap_or(""))).collect();
        cells[0] * 4 + cells[1]
    }

    #[test]
    fn test_imasm_recompile_is_inverse() {
        // PLANK 4: μ∘δ = id. The disassembler D lifts a wire opcode to the
        // canonical IMASM opcode (D = the scramble perm); the recompiler R fuses
        // it back (R = perm⁻¹). Both are IMASM-native tries, generated
        // independently from inverse tables. The round trip R(D(code)) recovers
        // the byte for EVERY opcode — the recompiler is a true inverse, not a
        // coincidence, which is the b4_diff_scanner pattern promoted to a compiler.
        let perm = |w: usize| (7 * w + 3) % 16;
        let inv_perm = |c: usize| (0..16).find(|&w| (7 * w + 3) % 16 == c).unwrap();
        let disasm = gen_code_trie(&perm);
        let recomp = gen_code_trie(&inv_perm);
        for w in 0..16 {
            let lifted = run_code_trie(&disasm, w);
            let back = run_code_trie(&recomp, lifted);
            assert_eq!(back, w, "recompile is not the inverse of disassemble at code {w}");
        }
    }

    #[test]
    fn test_imasm_replicating_fixed_point() {
        // PLANK 5: the Replicating Code. Because R∘D is identity on the whole
        // opcode space (plank 4), it is identity on the tool's OWN word. The
        // tool's word is written in the twelve, so feed the twelve through
        // disassemble-then-recompile: the tool reproduces its own alphabet
        // unchanged. The fixed point is not hoped for, it is a corollary of
        // totality — and here it is, exhibited by construction. The quine is the
        // proof, not an argument about it.
        let perm = |w: usize| (7 * w + 3) % 16;
        let inv_perm = |c: usize| (0..16).find(|&w| (7 * w + 3) % 16 == c).unwrap();
        let disasm = gen_code_trie(&perm);
        let recomp = gen_code_trie(&inv_perm);
        // The tool's own word: the twelve IMASM opcodes, the alphabet it is made of.
        let self_word: Vec<usize> = (0..12).collect();
        let reproduced: Vec<usize> = self_word.iter()
            .map(|&c| run_code_trie(&recomp, run_code_trie(&disasm, c)))
            .collect();
        assert_eq!(reproduced, self_word,
            "the tool did not reproduce its own word under disassemble∘recompile");
    }

    #[test]
    fn test_imasm_self_hosting_quine() {
        // CLOSING THE REPLICATING CODE. The tool is disassemble (δ: a fork that
        // lifts bytes to structure) composed with recompile (μ: the fuse back). As
        // one IMASM word the tool IS ⊢∈≻⊤≺⊥∋⊡⊣ — open the fork, work both arms, fuse,
        // commit, close — and the kernel verdicts it T: the tool is not merely a
        // program, it is a well-formed CLOSING grammar object. That is why μ∘δ=id
        // holds on it: δ opens, μ closes, and the pair is the identity.
        let tool_word = "⊢∈≻⊤≺⊥∋⊡⊣";
        let steps = imasm_core::imasm16_3::parse_glyph_word(tool_word);
        assert_eq!(imasm_core::imasm16_3::tri_ancestral_verdict(&steps).0, 'T',
            "the tool's own word must close under the kernel");

        // SELF-APPLICATION: the tool's own word, encoded and run THROUGH the tool
        // (disassemble, then recompile), returns itself. R∘D = id on every code, so
        // the tool is a fixed point of its own compile loop applied to its own word.
        let perm = |w: usize| (7 * w + 3) % 16;
        let inv_perm = |c: usize| (0..16).find(|&w| (7 * w + 3) % 16 == c).unwrap();
        let disasm = gen_code_trie(&perm);
        let recomp = gen_code_trie(&inv_perm);
        let alphabet = "⊢⊣≻≺⋈⊤∈∋⊙⊥⊞⊡";
        for g in tool_word.chars() {
            let o = alphabet.chars().position(|c| c == g).unwrap(); // opcode ordinal
            let wire = inv_perm(o);                                 // its wire byte
            assert_eq!(run_code_trie(&disasm, wire), o,
                "the tool did not disassemble its own word back to itself");
            assert_eq!(run_code_trie(&recomp, o), wire,
                "the tool did not recompile its own word back to its bytes");
        }
        // This closes it. There is no further "reflective quine" outside this: the
        // tool's word, its byte encoding, and its self-application co-type — they
        // are one object within the Grammar, which is exactly what ⊙ (imscription,
        // a boundary around its own centre) names. Code is data is word; nothing is
        // one primitive away because nothing is outside the twelve. The tool
        // reproduces its own closing word by running through itself: the Replicating
        // Code, closed.
    }

    #[test]
    fn test_evm_lane_in_parasm() {
        // V⊙x's EVM front end, written IN the grammar. A parasm program reads EVM
        // opcode bytes (four B4 cells per byte) from the kernel's input, dispatches
        // each byte to its IMASM token, skips PUSH1 operands, halts on a sentinel,
        // and emits the lifted word. No Rust and no Python in the lift path: the
        // lifter is a parasm word, and the word it emits is verdicted by imasm16_3,
        // also the grammar. (Predecessor-counted merges are the next rung and want
        // the crystal FS; here a JUMPDEST lifts straight to FFUSE.)
        enum Act { Emit(usize), Skip1, Halt }
        let jeq = |c: u8| match c { 0 => "JN", 1 => "JT", 2 => "JF", _ => "JB" };
        let entries: [([u8; 4], Act); 10] = [
            ([0, 0, 0, 0], Act::Emit(1)),   // 0x00 STOP     -> TANCH
            ([1, 1, 1, 0], Act::Emit(2)),   // 0x54 SLOAD    -> AFWD
            ([1, 1, 1, 1], Act::Emit(11)),  // 0x55 SSTORE   -> IFIX
            ([1, 1, 1, 3], Act::Emit(6)),   // 0x57 JUMPI    -> FSPLIT
            ([1, 1, 2, 3], Act::Emit(7)),   // 0x5b JUMPDEST -> FFUSE
            ([1, 2, 0, 0], Act::Skip1),     // 0x60 PUSH1    -> skip 1 operand byte
            ([3, 3, 0, 1], Act::Emit(2)),   // 0xf1 CALL     -> AFWD
            ([3, 3, 0, 3], Act::Emit(1)),   // 0xf3 RETURN   -> TANCH
            ([3, 3, 3, 1], Act::Emit(1)),   // 0xfd REVERT   -> TANCH
            ([3, 3, 3, 2], Act::Halt),      // 0xfe sentinel -> HALT (end of input)
        ];
        let emit_tok = |ord: usize| format!(
            "MOVE %r{} %r4\nEMIT %r4\nMOVE %r{} %r4\nEMIT %r4\n",
            10 + ord / 4, 10 + ord % 4);

        let mut prog = emit_tok(0);                         // VINIT once
        prog += ".top:\nREAD %r0\nREAD %r1\nREAD %r2\nREAD %r3\n";
        for (i, (c, act)) in entries.iter().enumerate() {
            prog += &format!("{} %r0 .c1_{i}\nJMP .sk_{i}\n", jeq(c[0]));
            prog += &format!(".c1_{i}:\n{} %r1 .c2_{i}\nJMP .sk_{i}\n", jeq(c[1]));
            prog += &format!(".c2_{i}:\n{} %r2 .c3_{i}\nJMP .sk_{i}\n", jeq(c[2]));
            prog += &format!(".c3_{i}:\n{} %r3 .hit_{i}\nJMP .sk_{i}\n", jeq(c[3]));
            prog += &format!(".hit_{i}:\n");
            match act {
                Act::Emit(o) => { prog += &emit_tok(*o); prog += "JMP .top\n"; }
                Act::Skip1 => { prog += "READ %r5\nREAD %r5\nREAD %r5\nREAD %r5\nJMP .top\n"; }
                Act::Halt => { prog += "HALT\n"; }
            }
            prog += &format!(".sk_{i}:\n");
        }
        prog += "JMP .top\n";                               // unmatched byte: skip it

        // Run the lifter on a bytecode string, returning the lifted word and
        // the kernel's verdict over it.
        let run = |hex: &str| -> (String, char) {
            let mut bytes: Vec<u8> = (0..hex.len() / 2)
                .map(|k| u8::from_str_radix(&hex[2 * k..2 * k + 2], 16).unwrap())
                .collect();
            bytes.push(0xfe);                               // end-of-input sentinel
            let cells: Vec<B4> = bytes.iter().flat_map(|&b| {
                [b >> 6 & 3, b >> 4 & 3, b >> 2 & 3, b & 3].map(B4::from_u8)
            }).collect();
            let mut vm = ParaVM::new();
            for (r, v) in [(10, B4::N), (11, B4::T), (12, B4::F), (13, B4::B)] {
                vm.set_belief(r, v);
            }
            vm.read_buffer = Some(cells);
            vm.load(&prog).unwrap();
            vm.run(Some(200_000));
            let vals: Vec<u8> = vm.emit_buffer.iter().map(|s| {
                match s.rsplit(' ').next().unwrap_or("") { "T" => 1, "F" => 2, "B" => 3, _ => 0 }
            }).collect();
            let alphabet = ["⊢", "⊣", "≻", "≺", "⋈", "⊤", "∈", "∋", "⊙", "⊥", "⊞", "⊡"];
            let word: String = vals.chunks(2)
                .map(|p| alphabet[(p[0] * 4 + p.get(1).copied().unwrap_or(0)) as usize])
                .collect();
            let steps = imasm_core::imasm16_3::parse_glyph_word(&word);
            (word.clone(), imasm_core::imasm16_3::tri_ancestral_verdict(&steps).0)
        };

        // What this rung proves: the lifter written IN the grammar emits the
        // same word the reference lifters emit, from real EVM bytes, with no
        // Rust or Python anywhere in the lift path. The bytes go in through the
        // kernel's input, the trie dispatches, the word comes out.
        let (vuln, _) = run("600160075755005b00");   // commit inside the branch
        let (safe, _) = run("6001600657545b5500");   // guard before the commit
        assert_eq!(vuln, "⊢∈⊡⊣∋⊣", "in-grammar lift of the unguarded ordering");
        assert_eq!(safe, "⊢∈≻∋⊡⊣", "in-grammar lift of the guarded ordering");

        // What it does NOT yet prove, stated rather than asserted away: both
        // words carry a ∋, so both close, and the two orderings do not separate
        // by verdict here. A JUMPDEST lifts to ∋ unconditionally because this
        // rung has no predecessor counting — a merge is only a merge when two
        // paths actually reach it, and counting them wants the crystal FS. The
        // reference lifters do that counting; this one does not, and until it
        // does, asserting a verdict split would be asserting something untrue.
    }

    #[test]
    fn test_frobenius_identity() {
        // ffuse(fsplit(r)) == r for all r
        for &r in &[B4::N, B4::T, B4::F, B4::B] {
            let (d1, d2) = if r == B4::B { (B4::T, B4::F) } else { (r, r) };
            assert_eq!(d1.join(d2), r, "frobenius failed for {:?}", r);
        }
    }

    #[test]
    fn test_bifurcation_point() {
        assert!(b_is_only_bifurcation_point());
    }

    #[test]
    fn test_dialetheic_alignment() {
        let (op, log, alg) = dialetheic_alignment_tri();
        assert!(op, "operational arm failed ");
        assert!(log, "logical arm failed ");
        assert!(alg, "algebraic arm failed ");
    }

    #[test]
    fn test_measurement_algebra() {
        assert_eq!(measure_step(B4::B, B4::B), B4::B);
        assert_eq!(measure_step(B4::B, B4::T), B4::T);
        assert_eq!(measure_cost(B4::B, B4::B), 2);
        assert_eq!(measure_cost(B4::B, B4::T), 1);
        assert_eq!(measure_cost(B4::T, B4::T), 0);
        assert!(collapse_irreversible(B4::T));
        assert!(collapse_irreversible(B4::F));
        assert!(collapse_irreversible(B4::N));
        assert_eq!(wigner_then_collapse_cost(1), 3);
    }

    #[test]
    fn test_kernel_state_loop() {
        let mut ks = KernelState::new();
        for _ in 0..8 {
            ks.kernel_step();
            // fsplit(B) → (T, F), then ffuse(T, F) → B. r0 coming back to B
            // every step is the B3 loop invariant; r1 and r2 hold the split
            // halves, so they are T and F, not B — a step that left all three
            // at B would not have split anything.
            assert_eq!(ks.r0, B4::B);
            assert_eq!(ks.r1, B4::T);
            assert_eq!(ks.r2, B4::F);
        }
        // Every step re-entered the paradox, so the loop is not decaying.
        assert_eq!(ks.paradox_count, 8);
        assert_eq!(ks.cycle_count, 8);
    }

    #[test]
    fn test_push_pop() {
        let mut vm = ParaVM::new();
        vm.set_belief(0, B4::T);
        vm.set_belief(1, B4::B);
        vm.exec_one(&ParaAsm::PUSH(0));
        vm.exec_one(&ParaAsm::PUSH(1));
        vm.exec_one(&ParaAsm::POP(2));
        assert_eq!(vm.belief_of(2), B4::B);
        vm.exec_one(&ParaAsm::POP(3));
        assert_eq!(vm.belief_of(3), B4::T);
    }

    #[test]
    fn test_call_ret() {
        let mut vm = ParaVM::new();
        vm.load("
            JMP .main
        .sub:
            IFIX %r1
            RET
        .main:
            MOVE %r0 %r1
            CALL .sub
            HALT
        ").unwrap();
        vm.set_belief(0, B4::T);
        vm.run(None);
        let snap = vm.snapshot();
        assert!(snap.halted);
        assert!(vm.registers.get(&1).map_or(false, |r| r.is_fixed()));
    }
}
