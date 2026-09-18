//! The module format: a word in the twelve, and the executable payload each
//! glyph carries. Ported from imasm_module.py. The glyph is the opcode (which of
//! the twelve axes the instruction is); the payload is normalised operands the
//! machine reads without ever parsing assembly.
//!
//! ```text
//! GLYPH \t field \t field ...      with  r:reg  i:imm  m:base:index:scale:disp:size
//! ```

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::format;

use crate::x86::{self, Op};
use crate::vox_decode;
use crate::loader;

fn bits_of(arch: &str) -> u8 { if arch == "x86-32" { 32 } else { 64 } }

const TERM: char = '⊣'; const SPLIT: char = '∈'; const FUSE: char = '∋';
const CALL: char = '≻'; const XFER: char = '≺'; const INDIRECT: char = '⊙'; const COMMIT: char = '⊡';
const LINK: char = '⋈'; const TRUTH: char = '⊤'; const CONSUME: char = '⊥'; const ENGAGE: char = '⊞';

fn is_move(mn: &str) -> bool {
    matches!(mn, "mov"|"movzx"|"movsx"|"movsxd"|"movabs"|"push"|"pop"|"xchg"|"leave")
}
fn is_truth(mn: &str) -> bool { matches!(mn, "cmp"|"test") }
fn is_terminal(mn: &str) -> bool { mn.starts_with("ret") || matches!(mn, "int3"|"ud2"|"hlt"|"iret"|"retf") }

/// The glyph — the same decision vox's classifier makes.
pub fn classify(i: &x86::Insn) -> char {
    let mn = i.mnemonic.as_str();
    if is_terminal(mn) { return TERM; }
    if mn == "call" { return if i.target.is_some() { CALL } else { INDIRECT }; }
    if mn == "jmp"  { return if i.target.is_some() { XFER } else { INDIRECT }; }
    if mn == "syscall" || mn == "sysenter" || mn == "int" { return INDIRECT; }
    if mn.starts_with('j') && mn != "jmp" { return SPLIT; }
    if mn.starts_with("set") { return CONSUME; }
    if mn.starts_with("cmov") { return CONSUME; }
    if is_truth(mn) { return TRUTH; }
    if i.writes_mem && !is_move(mn) { return COMMIT; }
    if is_move(mn) { return LINK; }
    ENGAGE
}

fn fields(ops: &[Op]) -> Vec<String> { ops.iter().map(|o| o.field()).collect() }

/// One instruction → its module lines. A merge emits a bare ∋ first.
fn encode(i: &x86::Insn, is_merge: bool) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    if is_merge { lines.push(FUSE.to_string()); }
    let mn = i.mnemonic.as_str();
    let g = classify(i);
    let f = fields(&i.ops);
    let join = |parts: &[String]| parts.join("\t");

    if g == TERM {
        // keep ret's imm16 (stdcall stack cleanup): ⊣  ret  i:0x4
        if mn == "ret" && !f.is_empty() { let mut parts=vec![TERM.to_string(), mn.to_string()]; parts.extend(f); lines.push(join(&parts)); }
        else { lines.push(format!("{}\t{}", TERM, mn)); }
    } else if g == INDIRECT && matches!(mn, "syscall"|"sysenter"|"int") {
        lines.push(format!("{}\tsyscall", INDIRECT));
    } else if mn == "call" || mn == "jmp" {
        let mut parts = vec![g.to_string(), mn.to_string()]; parts.extend(f);
        lines.push(join(&parts));
    } else if mn.starts_with('j') {
        let mut parts = vec![SPLIT.to_string(), mn[1..].to_string()]; parts.extend(f);
        lines.push(join(&parts));
    } else if mn.starts_with("set") {
        let mut parts = vec![CONSUME.to_string(), mn[3..].to_string(), "set".to_string()]; parts.extend(f);
        lines.push(join(&parts));
    } else if mn.starts_with("cmov") {
        let mut parts = vec![CONSUME.to_string(), mn[4..].to_string(), "cmov".to_string()]; parts.extend(f);
        lines.push(join(&parts));
    } else {
        let glyph = if g == TRUTH { TRUTH } else if g == COMMIT { COMMIT } else if g == LINK { LINK } else { ENGAGE };
        let mut parts = vec![glyph.to_string(), mn.to_string()]; parts.extend(f);
        lines.push(join(&parts));
    }
    lines
}

/// The whole binary as an executable IMASM module. Every executable byte is
/// decoded linearly, so an address reached only through an indirect jump — a
/// switch's jump-table arm, a call through a function pointer — is in the module
/// too, not just what recursive descent reached from direct edges.
pub fn emit(raw: &[u8]) -> String {
    let l = loader::load(raw);

    let bits = bits_of(l.arch);
    let mut out: Vec<String> = Vec::new();
    out.push(format!("; {} module ({} {})", INDIRECT, l.format, l.arch));
    out.push(format!("; entry 0x{:x}", l.entry));
    out.push(format!("; bits {}", bits));
    // The symbol table travels with the module, so a saved `.imasm` file is
    // self-contained: `vox run <symbol> <file>.imasm` resolves the name from
    // the file itself, with no second read of the original binary.
    for (name, addr) in &l.symbols {
        out.push(format!("; sym {} 0x{:x}", name, addr));
    }
    // IRELATIVE relocations ride with the module so a saved `.imasm` boots the
    // same as the binary: run each resolver, store its pointer in the slot.
    for (slot, resolver) in &l.irelative {
        out.push(format!("; irel 0x{:x} 0x{:x}", slot, resolver));
    }
    // RELATIVE relocations: the machine stores the value at the slot before the
    // process runs, the self-relocation a static-pie binary does at entry.
    for (slot, value) in &l.relative {
        out.push(format!("; rela 0x{:x} 0x{:x}", slot, value));
    }
    for (at, blob) in &l.data {
        let hex: String = blob.iter().map(|b| format!("{:02x}", b)).collect();
        out.push(format!("={:#x}\t{}", at, hex));
    }
    for (base, bytes) in &l.code {
        let mut pos = 0usize;
        while pos < bytes.len() {
            let addr = base + pos as u64;
            match x86::decode_mode(&bytes[pos..], addr, bits) {
                Some(d) if d.len > 0 => {
                    out.push(format!("@0x{:x}", addr));
                    for line in encode(&d, false) { out.push(line); }
                    pos += d.len;
                }
                _ => pos += 1, // alignment padding or data between functions
            }
        }
    }
    let mut s = out.join("\n"); s.push('\n'); s
}

/// Just the structure word (glyphs), per function.
pub fn words(raw: &[u8]) -> String {
    let l = loader::load(raw);
    let image = vox_decode::Image { segments: l.code };
    let mut seeds: Vec<u64> = l.symbols.values().copied().collect(); seeds.push(l.entry);
    let mut w = vox_decode::walk(&image, l.entry, &seeds);
    vox_decode::mark_noreturn(&mut w.functions, &l.symbols);
    let mut out: Vec<String> = Vec::new();
    for (start, f) in &w.functions {
        if f.is_empty() { continue; }
        // Audit and word export use the same CFG framing, including loop
        // regions and non-returning calls. The executable module is unchanged.
        let word = crate::vox::glyphs(&crate::vox::recompile_function(f));
        out.push(format!("0x{:x}\t{}", start, word));
    }
    out.join("\n")
}

/// Function symbol names → address, from the file's own tables.
pub fn symbols(raw: &[u8]) -> BTreeMap<String, u64> { loader::load(raw).symbols }
