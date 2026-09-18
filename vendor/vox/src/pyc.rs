//! The CPython lane: a `.pyc` read as words.
//!
//! The other lanes read a container and decode machine code. This one reads a
//! container and decodes CPython bytecode, which is the same act: a fork is a
//! conditional jump, a merge is an offset two edges arrive at, a commit is a
//! store, work is a call, and a return is a terminal. The opcode numbers and the
//! inline-cache widths are properties of one interpreter version and are
//! generated from it rather than retyped.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use crate::pyc_table::{HAVE_ARGUMENT, OPS, PYC_MAGIC, PY_VERSION};
use crate::vox::{AFWD, FFUSE, FSPLIT, IFIX, TANCH, VINIT};

fn opname(op: u8) -> &'static str {
    OPS.iter().find(|o| o.0 == op).map(|o| o.1).unwrap_or("<unknown>")
}
fn caches(op: u8) -> usize {
    OPS.iter().find(|o| o.0 == op).map(|o| o.4 as usize).unwrap_or(0)
}
fn is_jrel(op: u8) -> bool { OPS.iter().any(|o| o.0 == op && o.2 == 1) }
fn is_jabs(op: u8) -> bool { OPS.iter().any(|o| o.0 == op && o.3 == 1) }

/// One decoded instruction: byte offset, name, and resolved jump target.
pub struct Ins { pub offset: usize, pub name: &'static str, pub target: Option<usize> }

/// Decode a code object's bytes.
///
/// From 3.11 the interpreter interleaves inline cache entries into `co_code`;
/// they are part of the stream and must be stepped over rather than decoded, or
/// every offset after the first cached opcode is wrong. Jump arguments are in
/// instruction units, counted from the instruction after the jump and after its
/// caches, and `JUMP_BACKWARD` counts the other way.
pub fn decode(code: &[u8]) -> Vec<Ins> {
    let mut out = Vec::new();
    let (mut i, mut ext) = (0usize, 0usize);
    // An instruction carrying an extended argument BEGINS at its first
    // EXTENDED_ARG, and that is the offset a jump to it targets. Recording the
    // offset of the opcode itself loses every merge whose arrival point needed
    // more than one argument byte.
    let mut start: Option<usize> = None;
    while i + 1 < code.len() {
        let (op, arg_byte) = (code[i], code[i + 1] as usize);
        let name = opname(op);
        if name == "CACHE" { i += 2; continue; }
        let arg = if op >= HAVE_ARGUMENT { (ext << 8) | arg_byte } else { 0 };
        if name == "EXTENDED_ARG" {
            if start.is_none() { start = Some(i); }
            ext = (ext << 8) | arg_byte; i += 2; continue;
        }
        let offset = start.take().unwrap_or(i);
        let next = i + 2 + 2 * caches(op);
        let target = if is_jabs(op) {
            Some(arg * 2)
        } else if is_jrel(op) {
            if name.starts_with("JUMP_BACKWARD") { Some(next.saturating_sub(arg * 2)) }
            else { Some(next + arg * 2) }
        } else { None };
        out.push(Ins { offset, name, target });
        ext = 0;
        i = next;
    }
    out
}

fn is_return(n: &str) -> bool { n.starts_with("RETURN_") }
fn is_uncond(n: &str) -> bool { n.starts_with("JUMP_FORWARD") || n.starts_with("JUMP_BACKWARD") || n == "JUMP_ABSOLUTE" }
fn is_fork(n: &str) -> bool { n.starts_with("POP_JUMP_IF") || n.starts_with("JUMP_IF") || n.starts_with("FOR_ITER") || n.starts_with("SEND") }
fn is_store(n: &str) -> bool {
    matches!(n, "STORE_FAST"|"STORE_GLOBAL"|"STORE_DEREF"|"STORE_NAME"|"STORE_ATTR"|"STORE_SUBSCR"|"STORE_SLICE")
        || n.starts_with("STORE_FAST_")
}
fn is_call(n: &str) -> bool { n.starts_with("CALL") }

/// Offsets where control converges from two or more predecessors.
///
/// A jump target is not a merge by itself: if the fall-through arm returned or
/// jumped away first, the target has one predecessor and the fork never rejoins.
/// Only two incoming edges is a real merge, and that is exactly what separates a
/// guard whose paths rejoin from a branch that commits and leaves.
pub fn merges(ins: &[Ins]) -> Vec<usize> {
    let mut succ: Vec<usize> = Vec::new();
    for (idx, i) in ins.iter().enumerate() {
        if !is_return(i.name) && !is_uncond(i.name) {
            if let Some(n) = ins.get(idx + 1) { succ.push(n.offset); }
        }
        if let Some(t) = i.target { succ.push(t); }
    }
    succ.sort_unstable();
    let mut out = Vec::new();
    let mut k = 0;
    while k < succ.len() {
        let mut j = k;
        while j < succ.len() && succ[j] == succ[k] { j += 1; }
        if j - k >= 2 { out.push(succ[k]); }
        k = j;
    }
    out
}

/// Lift one code object's bytes to a word in the twelve.
pub fn lift(code: &[u8]) -> Vec<char> {
    let ins = decode(code);
    let m = merges(&ins);
    let mut w = alloc::vec![VINIT];
    for i in &ins {
        if m.binary_search(&i.offset).is_ok() { w.push(FFUSE); }
        if is_fork(i.name) { w.push(FSPLIT); }
        else if is_store(i.name) { w.push(IFIX); }
        else if is_call(i.name) { w.push(AFWD); }
        else if is_return(i.name) { w.push(TANCH); }
    }
    w
}

// ── the container ────────────────────────────────────────────────────────────

/// A code object recovered from a .pyc: its name and its bytes.
pub struct CodeObj { pub name: String, pub code: Vec<u8> }

struct Un<'a> { b: &'a [u8], i: usize, refs: Vec<String> }

impl<'a> Un<'a> {
    fn u8(&mut self) -> Option<u8> { let v = *self.b.get(self.i)?; self.i += 1; Some(v) }
    fn u32(&mut self) -> Option<u32> {
        let mut v = 0u32;
        for k in 0..4 { v |= (*self.b.get(self.i + k)? as u32) << (8 * k); }
        self.i += 4; Some(v)
    }
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let s = self.b.get(self.i..self.i + n)?; self.i += n; Some(s)
    }

    /// Read one marshalled object, collecting any code objects met on the way.
    ///
    /// Only the shapes a code object is built from are decoded; anything else is
    /// stepped over by size. A type this does not know stops the walk rather
    /// than desynchronising it, because a marshal reader that guesses a width
    /// returns confident nonsense for everything after.
    fn obj(&mut self, out: &mut Vec<CodeObj>) -> Option<()> {
        let t = self.u8()?;
        let code = t & 0x7f;                       // high bit is the ref flag
        match code as char {
            '0' | 'N' | 'F' | 'T' | 'S' | '.' => Some(()),
            'i' => { self.take(4)?; Some(()) }
            'g' => { self.take(8)?; Some(()) }
            'y' => { self.take(16)?; Some(()) }
            'l' => { let n = self.u32()? as i32; self.take(n.unsigned_abs() as usize * 2)?; Some(()) }
            'z' | 'Z' | ')' => { let n = self.u8()? as usize;
                                 if code as char == ')' { for _ in 0..n { self.obj(out)?; } }
                                 else { self.take(n)?; } Some(()) }
            's' | 't' | 'u' | 'a' | 'A' => { let n = self.u32()? as usize; self.take(n)?; Some(()) }
            '(' | '[' | '<' | '>' => { let n = self.u32()? as usize;
                                       for _ in 0..n { self.obj(out)?; } Some(()) }
            '{' => { loop { let save = self.i; let t = self.u8()?;
                            if (t & 0x7f) as char == '0' { break; }
                            self.i = save; self.obj(out)?; self.obj(out)?; } Some(()) }
            'r' => { self.take(4)?; Some(()) }
            'c' => self.code_obj(out),
            _ => None,
        }
    }

    /// A code object, in the field order 3.11+ marshals it.
    fn code_obj(&mut self, out: &mut Vec<CodeObj>) -> Option<()> {
        self.take(4 * 5)?;                              // argcount..flags
        let save = self.i;
        let t = self.u8()?;
        let bytes = if (t & 0x7f) as char == 's' {
            let n = self.u32()? as usize; self.take(n)?.to_vec()
        } else { self.i = save; self.obj(out)?; Vec::new() };
        self.obj(out)?;                                  // consts: nested code objects land here
        self.obj(out)?;                                  // names
        self.obj(out)?;                                  // localsplusnames
        self.obj(out)?;                                  // localspluskinds
        self.obj(out)?;                                  // filename
        let save = self.i;
        let t = self.u8()?;
        let name = if matches!((t & 0x7f) as char, 'z' | 'Z') {
            let n = self.u8()? as usize;
            String::from_utf8_lossy(self.take(n)?).to_string()
        } else if matches!((t & 0x7f) as char, 'a' | 'A' | 'u' | 't') {
            let n = self.u32()? as usize;
            String::from_utf8_lossy(self.take(n)?).to_string()
        } else { self.i = save; self.obj(out)?; String::new() };
        self.obj(out)?;                                  // qualname
        self.take(4)?;                                   // firstlineno
        self.obj(out)?;                                  // linetable
        self.obj(out)?;                                  // exceptiontable
        let _ = &self.refs;
        out.push(CodeObj { name, code: bytes });
        Some(())
    }
}

/// Every code object in a .pyc, module first then the functions inside it.
///
/// The header is sixteen bytes from 3.7: magic, a flags word, and two more
/// words whose meaning depends on the flags. The magic is checked, because a
/// .pyc from another interpreter has different opcode numbers and lifting it
/// against this table would produce a word that reads cleanly and means nothing.
pub fn read_pyc(raw: &[u8]) -> Result<Vec<CodeObj>, String> {
    if raw.len() < 16 { return Err("not a .pyc: too short".to_string()); }
    if raw[..4] != PYC_MAGIC {
        return Err(alloc::format!(
            "magic {:02x}{:02x}{:02x}{:02x} is not this interpreter's ({}); \
             opcode numbers differ between versions, so V⊙x will not lift it",
            raw[0], raw[1], raw[2], raw[3], PY_VERSION));
    }
    let mut un = Un { b: raw, i: 16, refs: Vec::new() };
    let mut out = Vec::new();
    un.obj(&mut out).ok_or_else(|| "marshal: unhandled object type".to_string())?;
    if out.is_empty() { return Err("no code object found".to_string()); }
    out.reverse();                                       // module first
    Ok(out)
}
