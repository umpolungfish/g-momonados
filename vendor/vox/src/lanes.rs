//! Bytecode lanes: EVM and WASM control flow lifted to the same twelve-glyph
//! word the x86 lane produces, and verdicted by the same closure law. Both are
//! `no_std`+`alloc` and carry no external crates. The x86 lane lives in `vox`;
//! the genetic (RNA) lane needs the genetic code and lives in the mOMonadOS
//! circuit; CPython needs a running interpreter's disassembler.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use crate::vox::{VINIT, TANCH, AFWD, FSPLIT, FFUSE, IFIX};

pub fn from_hex(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let mut hi: Option<u8> = None;
    for c in s.chars() {
        let v = match c { '0'..='9' => c as u8 - b'0', 'a'..='f' => c as u8 - b'a' + 10,
            'A'..='F' => c as u8 - b'A' + 10, _ => continue };
        match hi { None => hi = Some(v), Some(h) => { out.push((h << 4) | v); hi = None; } }
    }
    out
}

// ── EVM ──────────────────────────────────────────────────────────────────────

struct EvmIns { off: usize, name: String, target: Option<usize> }

fn evm_name(op: u8) -> &'static str {
    match op {
        0x00 => "STOP", 0x54 => "SLOAD", 0x55 => "SSTORE", 0x56 => "JUMP", 0x57 => "JUMPI",
        0x5b => "JUMPDEST", 0x35 => "CALLDATALOAD", 0x58 => "PC", 0x5a => "GAS",
        0xf1 => "CALL", 0xf2 => "CALLCODE", 0xf3 => "RETURN", 0xf4 => "DELEGATECALL",
        0xfa => "STATICCALL", 0xfd => "REVERT",
        0x01 => "ADD", 0x03 => "SUB", 0x10 => "LT", 0x11 => "GT", 0x14 => "EQ", 0x15 => "ISZERO",
        _ => "OP",
    }
}
fn evm_work(n: &str) -> bool {
    matches!(n, "SLOAD"|"CALLDATALOAD"|"ADD"|"SUB"|"LT"|"GT"|"EQ"|"ISZERO"|"GAS"|"PC"
        |"CALL"|"CALLCODE"|"DELEGATECALL"|"STATICCALL")
}
fn evm_halt(n: &str) -> bool { matches!(n, "STOP"|"RETURN"|"REVERT"|"JUMP") }

fn parse_evm(bytes: &[u8]) -> Vec<EvmIns> {
    let mut instrs = Vec::new();
    let (mut i, mut prev_push): (usize, Option<usize>) = (0, None);
    while i < bytes.len() {
        let op = bytes[i];
        if (0x60..=0x7f).contains(&op) {                 // PUSH1..PUSH32
            let n = (op - 0x5f) as usize;
            let mut val: usize = 0;
            for k in 0..n { if i + 1 + k < bytes.len() { val = (val << 8) | bytes[i + 1 + k] as usize; } }
            instrs.push(EvmIns { off: i, name: String::from("PUSH"), target: None });
            prev_push = Some(val);
            i += 1 + n;
        } else {
            let nm = evm_name(op);
            let target = if nm == "JUMP" || nm == "JUMPI" { prev_push } else { None };
            instrs.push(EvmIns { off: i, name: String::from(nm), target });
            prev_push = None;
            i += 1;
        }
    }
    instrs
}

fn lift_evm(instrs: &[EvmIns]) -> Vec<char> {
    let mut succ_count: BTreeMap<usize, u32> = BTreeMap::new();
    for (idx, ins) in instrs.iter().enumerate() {
        if !evm_halt(&ins.name) && idx + 1 < instrs.len() {
            *succ_count.entry(instrs[idx + 1].off).or_insert(0) += 1;
        }
        if ins.name == "JUMP" || ins.name == "JUMPI" {
            if let Some(t) = ins.target { *succ_count.entry(t).or_insert(0) += 1; }
        }
    }
    let mut word = alloc::vec![VINIT];
    for ins in instrs {
        let nm = ins.name.as_str();
        if nm == "JUMPDEST" && succ_count.get(&ins.off).copied().unwrap_or(0) >= 2 { word.push(FFUSE); }
        if nm == "JUMPI" { word.push(FSPLIT); }
        else if nm == "SSTORE" { word.push(IFIX); }
        else if evm_work(nm) { word.push(AFWD); }
        else if matches!(nm, "STOP"|"RETURN"|"REVERT") { word.push(TANCH); }
    }
    word
}

/// Lift an EVM bytecode hex string to a glyph word.
pub fn evm_word(hexstr: &str) -> Vec<char> { lift_evm(&parse_evm(&from_hex(hexstr))) }

// ── WASM ─────────────────────────────────────────────────────────────────────

fn leb_len(b: &[u8], i: usize) -> usize {
    let mut n = 0;
    while i + n < b.len() && (b[i + n] & 0x80) != 0 { n += 1; }
    n + 1
}

fn parse_wasm_body(bytes: &[u8]) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let op = bytes[i];
        let mut j = i + 1;
        let nm: &'static str = match op {
            0x00 => "unreachable", 0x01 => "nop", 0x02 => { j += 1; "block" }, 0x03 => { j += 1; "loop" },
            0x04 => { j += 1; "if" }, 0x05 => "else", 0x0b => "end",
            0x0c => { j += leb_len(bytes, j); "br" }, 0x0d => { j += leb_len(bytes, j); "br_if" },
            0x0f => "return", 0x10 => { j += leb_len(bytes, j); "call" },
            0x11 => { j += leb_len(bytes, j); j += leb_len(bytes, j); "call_indirect" },
            0x24 => { j += leb_len(bytes, j); "global.set" },
            0x36..=0x3e => { j += leb_len(bytes, j); j += leb_len(bytes, j); "store" },
            0x28..=0x35 => { j += leb_len(bytes, j); j += leb_len(bytes, j); "load" },
            0x20|0x21|0x22|0x23|0x41|0x42 => { j += leb_len(bytes, j); "op" },
            0x43 => { j += 4; "op" }, 0x44 => { j += 8; "op" },
            _ => "op",
        };
        out.push(nm);
        i = j;
    }
    out
}

fn lift_wasm(names: &[&str]) -> Vec<char> {
    let mut word = alloc::vec![VINIT];
    let mut ctrl: Vec<(&str, bool)> = Vec::new();  // (kind, escaped)
    for &nm in names {
        match nm {
            "block" | "loop" => ctrl.push((nm, false)),
            "if" => { ctrl.push(("if", false)); word.push(FSPLIT); }
            "return" | "br" => {
                word.push(if nm == "return" { TANCH } else { FSPLIT });
                for c in ctrl.iter_mut().rev() { if c.0 == "if" { c.1 = true; break; } }
            }
            "end" => {
                if let Some((k, escaped)) = ctrl.pop() { if k == "if" && !escaped { word.push(FFUSE); } }
            }
            "store" | "global.set" => word.push(IFIX),
            "call" | "call_indirect" => word.push(AFWD),
            _ => {}
        }
    }
    word
}

/// Lift a WASM function-body hex string to a glyph word.
pub fn wasm_word(hexstr: &str) -> Vec<char> { lift_wasm(&parse_wasm_body(&from_hex(hexstr))) }
