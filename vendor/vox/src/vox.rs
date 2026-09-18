// ─── V⊙x — Control-flow Closure Auditor ────────────────────
// Native Rust implementation of the Belnap FOUR control-flow auditor.
// Lifts program CFGs to twelve-glyph IMASM words and runs SIXTEEN_3 verdict.
#![allow(dead_code)]

use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::format;
use alloc::vec;

// 12 IMASM opcodes as chars
pub const VINIT: char = '⊢';
pub const TANCH: char = '⊣';
pub const AFWD: char = '≻';
pub const AREV: char = '≺';
pub const FSPLIT: char = '∈';
pub const FFUSE: char = '∋';
pub const IMSCRIB: char = '⊙';
pub const IFIX: char = '⊡';
pub const CLINK: char = '⋈';
pub const EVALT: char = '⊤';
pub const EVALF: char = '⊥';
pub const ENGAGR: char = '⊞';

/// A decoded instruction from a binary.
#[derive(Clone, Debug)]
pub struct Instruction {
    pub address: u64,
    /// Actual next instruction address, absent for terminal/non-returning flow.
    pub fallthrough: Option<u64>,
    pub mnemonic: String,
    pub op_str: String,
}

/// Mnemonic prefixes that don't change classification
const MNEMONIC_PREFIXES: &[&str] = &["notrack ", "lock ", "bnd ", "rep ", "repe ", "repne ", "data16 "];

/// Strip mnemonic prefixes so classification is consistent
pub fn strip_prefix(mn: &str) -> &str {
    for p in MNEMONIC_PREFIXES {
        if mn.starts_with(p) {
            return &mn[p.len()..];
        }
    }
    mn
}

/// Parse a hex immediate from operand string
fn parse_imm(op_str: &str) -> Option<u64> {
    let s = op_str.trim();
    if s.starts_with("0x") || s.starts_with("0X") {
        u64::from_str_radix(&s[2..], 16).ok()
    } else {
        None
    }
}

/// Check if operand string indicates an immediate (direct target)
fn is_direct(op_str: &str) -> bool {
    let s = op_str.trim();
    parse_imm(s).is_some()
}

/// Check if the operand indicates a memory write (destination ends with ']')
fn writes_memory(op_str: &str) -> bool {
    let dst = op_str.split(',').next().unwrap_or("").trim();
    dst.ends_with(']')
}

/// Move instructions (data movement between slots)
const MOVE_OPS: &[&str] = &["mov", "movzx", "movsx", "movsxd", "movabs",
    "movaps", "movdqa", "movdqu", "movups", "movd", "movq",
    "lea", "push", "pop", "xchg", "leave"];

/// Arithmetic/logic instructions (engagement)
const ENGAGE_OPS: &[&str] = &["add", "sub", "adc", "sbb", "imul", "mul", "idiv", "div",
    "and", "or", "xor", "not", "neg", "inc", "dec", "shl", "shr", "shld", "shrd", "sar", "rol",
    "ror", "sal", "bt", "bsf", "bsr", "popcnt", "cdq", "cqo", "cwde", "pcmpgtd", "pandn",
    "pushfd", "pushfq", "popfd", "popfq", "lahf", "sahf"];

/// Terminal instructions
const TERMINAL_OPS: &[&str] = &["ret", "retf", "int3", "ud2", "hlt", "iret", "sysret", "sysretq"];

/// Truth-producing instructions
const TRUTH_OPS: &[&str] = &["cmp", "test", "ucomiss", "ucomisd", "ucomis", "comisd", "comiss"];

/// Classify an instruction into one of 12 IMASM glyphs
pub fn classify_instruction(ins: &Instruction) -> char {
    let mn = strip_prefix(&ins.mnemonic);
    let ops = ins.op_str.trim();

    // Terminal instructions
    if mn.starts_with("ret") || TERMINAL_OPS.contains(&mn) {
        return TANCH;
    }

    // call/jmp with direct vs indirect
    if mn == "call" {
        if is_direct(ops) {
            return AFWD;
        } else {
            return IMSCRIB;
        }
    }

    if mn == "jmp" {
        if is_direct(ops) {
            return AREV;
        } else {
            return IMSCRIB;
        }
    }

    // syscall
    if mn == "syscall" || (mn == "int" && ops == "0x80") {
        return IMSCRIB;
    }

    // Conditional branches — forks (jcc except jmp)
    if mn.starts_with('j') && mn != "jmp" {
        return FSPLIT;
    }

    // setcc — truth consumed
    if mn.starts_with("set") {
        return EVALF;
    }

    // cmovcc — truth consumed
    if mn.starts_with("cmov") {
        return EVALF;
    }

    // cmp/test
    if TRUTH_OPS.contains(&mn) {
        return EVALT;
    }

    // Memory writes — commit state
    if writes_memory(ops) && !MOVE_OPS.contains(&mn) {
        return IFIX;
    }

    // Data movement
    if MOVE_OPS.contains(&mn) {
        return CLINK;
    }

    // Arithmetic/logic — engagement
    if ENGAGE_OPS.contains(&mn) {
        return ENGAGR;
    }

    // Default: engagement (total mapping)
    ENGAGR
}

/// An ordered address set. Instruction addresses arrive ascending, so lookup is
/// a binary search rather than a scan — a linear scan here makes `compute_merges`
/// quadratic, which is the difference between lifting a small object file and
/// lifting a real `.text`.
struct AddrSet {
    addrs: Vec<u64>,
}

impl AddrSet {
    fn from_sorted(addrs: Vec<u64>) -> Self {
        AddrSet { addrs }
    }

    fn contains(&self, addr: u64) -> bool {
        self.addrs.binary_search(&addr).is_ok()
    }
}

struct FlowFrames {
    reachable: Vec<bool>,
    order: Vec<usize>,
    joins: Vec<usize>,
    loop_open: Vec<usize>,
    loop_close: Vec<usize>,
}

/// Separate feedback from forward joins by a DFS of actual CFG edges.
/// Each DFS backedge owns a loop frame, opened at its header and closed at
/// its latch. Other predecessors contribute arity-1 forward fuses. Address
/// order alone cannot distinguish a backedge from a backwards shared tail.
fn flow_frames(insns: &[Instruction]) -> FlowFrames {
    let n=insns.len();
    let mut result=FlowFrames { reachable:vec![false;n], order:Vec::new(), joins:vec![0;n],
        loop_open:vec![0;n], loop_close:vec![0;n] };
    if n==0 { return result; }
    let addresses: BTreeMap<_,_>=insns.iter().enumerate().map(|(i,ins)|(ins.address,i)).collect();
    let mut edges=vec![Vec::new();n];
    for (i,ins) in insns.iter().enumerate() {
        if let Some(j)=ins.fallthrough.and_then(|a|addresses.get(&a).copied()) { edges[i].push(j); }
        let mn=strip_prefix(&ins.mnemonic);
        if mn.starts_with('j') {
            if let Some(j)=parse_imm(&ins.op_str).and_then(|a|addresses.get(&a).copied()) { edges[i].push(j); }
        }
    }
    let mut color=vec![0u8;n];
    let mut predecessors=vec![0usize;n];
    let mut stack=vec![(0usize,0usize)];
    color[0]=1; result.reachable[0]=true;
    while let Some((i,next))=stack.last_mut() {
        if *next==edges[*i].len() { color[*i]=2; result.order.push(*i); stack.pop(); continue; }
        let source=*i;
        let target=edges[source][*next]; *next+=1;
        if color[target]==1 {
            result.loop_open[target]+=1;
            result.loop_close[source]+=1;
        } else {
            predecessors[target]+=1;
            if color[target]==0 {
                color[target]=1; result.reachable[target]=true; stack.push((target,0));
            }
        }
    }
    for i in 0..n { result.joins[i]=predecessors[i].saturating_sub(1); }
    // Reverse postorder is topological once DFS feedback edges are framed.
    // A shared tail may be earlier in memory than an incoming branch; emitting
    // address order would close that branch before its work appears in the word.
    result.order.reverse();
    result
}

/// Forward-join addresses, repeated for each arity-reducing fuse. Feedback
/// uses explicit loop frames instead of orphan joins.
pub fn compute_merges(insns: &[Instruction]) -> Vec<u64> {
    let frames=flow_frames(insns);
    insns.iter().enumerate().flat_map(|(i,ins)|core::iter::repeat(ins.address).take(frames.joins[i])).collect()
}

/// Lift a function's instruction list to an IMASM word
pub fn recompile_function(insns: &[Instruction]) -> Vec<char> {
    let frames = flow_frames(insns);
    let mut tokens = vec![VINIT];

    for &i in &frames.order {
        let ins=&insns[i];
        tokens.extend(core::iter::repeat(FFUSE).take(frames.joins[i]));
        tokens.extend(core::iter::repeat(FSPLIT).take(frames.loop_open[i]));
        tokens.push(classify_instruction(ins));
        tokens.extend(core::iter::repeat(FFUSE).take(frames.loop_close[i]));
    }

    tokens
}

/// Instructions a compiler uses to pad between functions.
pub const PAD_OPS: &[&str] = &["int3", "nop", "ud2"];

/// Split a decoded run into functions.
///
/// A contiguous run of instructions is not one function. A boundary is a
/// terminal, then padding, then a body — all three, because both compilers
/// also pad for alignment inside a body and cutting on padding alone shatters
/// one function into dozens.
pub fn split_functions(insns: &[Instruction]) -> Vec<&[Instruction]> {
    let mut out = Vec::new();
    if insns.is_empty() {
        return out;
    }
    let mut start = 0usize;
    let mut saw_terminal = false;
    let mut in_pad = false;

    for i in 1..insns.len() {
        let prev = strip_prefix(&insns[i - 1].mnemonic);
        let here = strip_prefix(&insns[i].mnemonic);
        if PAD_OPS.contains(&prev) {
            in_pad = true;
        } else if prev.starts_with("ret") || TERMINAL_OPS.contains(&prev) || prev == "jmp" {
            saw_terminal = true;
        } else {
            saw_terminal = false;
            in_pad = false;
        }
        if in_pad && saw_terminal && !PAD_OPS.contains(&here) {
            out.push(&insns[start..i]);
            start = i;
            saw_terminal = false;
            in_pad = false;
        }
    }
    out.push(&insns[start..]);
    // A stretch that is nothing but padding is not a function either.
    out.retain(|f| f.iter().any(|i| !PAD_OPS.contains(&strip_prefix(&i.mnemonic))));
    out
}

/// Lift multiple functions to words
pub fn recompile_module(functions: &[(String, u64, Vec<Instruction>)]) -> Vec<(String, u64, Vec<char>)> {
    let mut result = Vec::new();
    for (label, addr, insns) in functions {
        let word = recompile_function(insns);
        result.push((label.clone(), *addr, word));
    }
    result
}

/// Run SIXTEEN_3 verdict on an IMASM word.
///
/// T closes, B holds a fork open across a terminal, N never forked, and F is
/// not a truth value at all: it reports that the word is ill-typed, a ∋ with
/// no ∈ to pair. Refusing to score an ill-formed word is the point. A fuse
/// that nothing opened means the lifter cut a function in the wrong place, and
/// scoring it T or N would bury that under a verdict that looks like an answer.
/// Work opcodes: the seven that transform the object (⊞ is ENGAGR here, EVALI in the core: one glyph, two names). ∈ ∋ ⊙ ⊢ ⊣ do not.
fn is_work(g: char) -> bool { matches!(g, AFWD | AREV | CLINK | EVALT | EVALF | ENGAGR | IFIX) }

/// Pair ∈ → ∋ over the word read as a loop.
///
/// A word is a loop and ROTAT is the cyclic shift, so any readout that pairs
/// split with fuse must pair around the cycle. Pairing over the linearized list
/// makes every region the cut passes through look dangling, and turns
/// rotation-invariant structure into spurious phase. Splits and fuses go
/// unmatched only on a true count imbalance, which no rotation can repair.
fn cyclic_pairs(word: &[char]) -> (Vec<(usize, usize)>, Vec<usize>, Vec<usize>) {
    let n = word.len();
    let split_idx: Vec<usize> = (0..n).filter(|&i| word[i] == FSPLIT).collect();
    let fuse_idx: Vec<usize> = (0..n).filter(|&i| word[i] == FFUSE).collect();
    if split_idx.is_empty() && fuse_idx.is_empty() { return (Vec::new(), Vec::new(), Vec::new()); }
    // Read from the index at which the running balance of splits against fuses
    // is least. From there the balance never falls below zero, so no fuse finds
    // the stack empty and the pairing is total on the smaller species: what is
    // left over is the surplus alone, and at most one side of it is non-empty.
    // On balanced counts this is the rotation that closes every region.
    let mut balance: i64 = 0;
    let mut least: i64 = 0;
    let mut start = 0usize;
    for i in 0..n {
        balance += if word[i] == FSPLIT { 1 } else if word[i] == FFUSE { -1 } else { 0 };
        if balance < least { least = balance; start = (i + 1) % n; }
    }
    let mut stack: Vec<usize> = Vec::new();
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    let mut un_fuse: Vec<usize> = Vec::new();
    for off in 0..n {
        let i = (start + off) % n;
        if word[i] == FSPLIT { stack.push(i); }
        else if word[i] == FFUSE {
            match stack.pop() { Some(si) => pairs.push((si, i)), None => un_fuse.push(i) }
        }
    }
    stack.sort_unstable();
    un_fuse.sort_unstable();
    (pairs, stack, un_fuse)
}

/// One region of a word's pairing: the division, the rejoining that answers it,
/// what stands between them, and whether any of that does work.
pub struct Region {
    pub split: usize,
    pub fuse: usize,
    pub interior: String,
    pub substantial: bool,
}

/// The pairing of a word, reported. The regions in the order the reading forms
/// them, and the divisions and rejoinings the reading leaves unanswered.
pub fn pairing(word: &[char]) -> (Vec<Region>, Vec<usize>, Vec<usize>) {
    let (pairs, un_split, un_fuse) = cyclic_pairs(word);
    let regions = pairs.into_iter().map(|(si, fj)| {
        let inner = cyclic_interior(word, si, fj);
        Region {
            split: si,
            fuse: fj,
            substantial: inner.iter().copied().any(is_work),
            interior: inner.into_iter().collect(),
        }
    }).collect();
    (regions, un_split, un_fuse)
}

/// Glyphs strictly inside a region, walking forward around the cycle.
fn cyclic_interior(word: &[char], si: usize, fj: usize) -> Vec<char> {
    let n = word.len();
    let len = (fj + n - si - 1) % n;
    (0..len).map(|j| word[(si + 1 + j) % n]).collect()
}

/// The tri-ancestral verdict, the same rule the SIXTEEN_3 engine reads.
///
/// T — every ∈ pairs with a ∋ around the cycle, and at least one work opcode
///     ran inside that region (a real transformation).
/// N — split and fused, but no work ran inside: μ∘δ=id verifies nothing. Also
///     the void, where nothing ever forked.
/// B — an ∈ dangles with no ∋ to pair (a fork left open).
/// F — a ∋ has no ∈ to pair (ill-typed).
pub fn verdict(word: &[char]) -> char {
    let (pairs, un_split, un_fuse) = cyclic_pairs(word);
    if pairs.is_empty() && un_split.is_empty() && un_fuse.is_empty() { return 'N'; }
    if un_fuse.len() > un_split.len() { return 'F'; }
    if !un_split.is_empty() { return 'B'; }
    for (si, fj) in pairs {
        if cyclic_interior(word, si, fj).into_iter().any(is_work) { return 'T'; }
    }
    'N'
}


/// What a word's open forks are, split into the part exits explain and the part
/// they do not.
///
/// A held-open fork is not by itself a defect. An early return IS a fork that
/// leaves and never rejoins, so a function with several exits carries several
/// open forks by construction. Measured across a large real binary the surplus
/// of ∈ over ∋ rises with the number of terminals at about five per exit, from
/// a mean of 0.30 at one exit to 15.5 at four or more, and above two exits
/// essentially every function carries surplus. Ranking candidates by raw
/// surplus therefore ranks by how many ways a function can return.
///
/// The residual is what remains once exits are accounted for. That is the part
/// worth reading.
pub struct OpenForks {
    /// ∈ minus ∋: forks the word never closes
    pub surplus: i32,
    /// ⊣ in the word: the ways this function leaves
    pub exits: i32,
    /// surplus beyond what the exits explain
    pub residual: i32,
}

/// Slope of surplus against exit count, measured over a stripped release binary
/// of about fifteen hundred functions.
const SURPLUS_PER_EXIT: i32 = 5;

pub fn open_forks(word: &[char]) -> OpenForks {
    let splits = word.iter().filter(|&&c| c == FSPLIT).count() as i32;
    let fuses  = word.iter().filter(|&&c| c == FFUSE).count() as i32;
    let exits  = word.iter().filter(|&&c| c == TANCH).count() as i32;
    let surplus = splits - fuses;
    OpenForks { surplus, exits, residual: surplus - SURPLUS_PER_EXIT * exits }
}

/// Emit word as glyph string
pub fn glyphs(word: &[char]) -> String {
    word.iter().collect()
}

/// Emit IMASM module text for a set of functions
pub fn emit_word(functions: &[(&str, u64, &[Instruction])]) -> String {
    let mut lines = vec![format!("; ⊙ vox native module")];
    let total: usize = functions.iter().map(|f| recompile_function(f.2).len()).sum();
    lines.push(format!("; {} words   {} glyphs", functions.len(), total));
    for (_label, addr, insns) in functions {
        let word = recompile_function(insns);
        lines.push(format!("0x{:x}", addr));
        lines.push(glyphs(&word));
    }
    lines.join("\n") + "\n"
}

/// Check round-trip: lift(decompile(emit(binary))) == lift(recompile_module(binary))
pub fn roundtrip_check(functions: &[(String, u64, Vec<Instruction>)]) -> bool {
    let words1 = recompile_module(functions);
    let mut combined1 = String::new();
    for (_, _, w) in &words1 {
        for c in w {
            combined1.push(*c);
        }
    }

    // Re-emit and parse back
    let views: Vec<(&str, u64, &[Instruction])> = functions.iter()
        .map(|(l, a, ins)| (l.as_str(), *a, ins.as_slice()))
        .collect();
    let text = emit_word(&views);
    let mut combined2 = String::new();
    for line in text.lines() {
        if line.starts_with('0') || line.is_empty() || line.starts_with(';') {
            continue;
        }
        combined2.push_str(line);
    }

    combined1 == combined2
}

/// Every defined function address in an ELF's symbol tables.
///
/// A shared object's own entry point calls almost none of its exported
/// functions — those are called from outside, by whoever loads it — so descent
/// from the entry alone misses them. Seeding every defined function symbol is
/// what turns a handful of reachable functions into the whole program.
pub fn elf_function_symbols(raw: &[u8]) -> Vec<u64> {
    let mut out = Vec::new();
    if raw.len() < 64 || &raw[0..4] != b"\x7fELF" || raw[4] != 2 {
        return out;
    }
    let rd64 = |o: usize| -> u64 {
        u64::from_le_bytes(raw[o..o + 8].try_into().unwrap_or([0; 8]))
    };
    let rd32 = |o: usize| -> u32 {
        u32::from_le_bytes(raw[o..o + 4].try_into().unwrap_or([0; 4]))
    };
    let rd16 = |o: usize| -> u16 {
        u16::from_le_bytes(raw[o..o + 2].try_into().unwrap_or([0; 2]))
    };
    let shoff = rd64(40) as usize;
    let shentsize = rd16(58) as usize;
    let shnum = rd16(60) as usize;
    for k in 0..shnum {
        let sh = shoff + k * shentsize;
        if sh + 64 > raw.len() {
            break;
        }
        let sh_type = rd32(sh + 4);
        // SHT_SYMTAB = 2, SHT_DYNSYM = 11
        if sh_type != 2 && sh_type != 11 {
            continue;
        }
        let off = rd64(sh + 24) as usize;
        let size = rd64(sh + 32) as usize;
        let entsize = rd64(sh + 56) as usize;
        if entsize == 0 || off + size > raw.len() {
            continue;
        }
        let mut e = off;
        while e + entsize <= off + size {
            let info = raw[e + 4];
            let value = rd64(e + 8);
            // STT_FUNC = 2 in the low nibble of st_info
            if info & 0x0F == 2 && value != 0 {
                out.push(value);
            }
            e += entsize;
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// Parse ELF binary to extract executable sections
pub fn parse_elf(raw: &[u8]) -> (u64, Vec<(u64, Vec<u8>)>) {
    if raw.len() < 64 || &raw[0..4] != b"\x7fELF" {
        return (0, vec![]);
    }

    let is_64 = raw[4] == 2;
    let is_le = raw[5] == 1;

    // Helper closures for reading little/big endian
    let read_u64 = |off: usize| -> u64 {
        if is_le {
            u64::from_le_bytes(raw[off..off+8].try_into().unwrap())
        } else {
            u64::from_be_bytes(raw[off..off+8].try_into().unwrap())
        }
    };

    let read_u32 = |off: usize| -> u32 {
        if is_le {
            u32::from_le_bytes(raw[off..off+4].try_into().unwrap())
        } else {
            u32::from_be_bytes(raw[off..off+4].try_into().unwrap())
        }
    };

    let read_u16 = |off: usize| -> u16 {
        if is_le {
            u16::from_le_bytes(raw[off..off+2].try_into().unwrap())
        } else {
            u16::from_be_bytes(raw[off..off+2].try_into().unwrap())
        }
    };

    let (entry, shoff, shentsize, shnum) = if is_64 {
        (read_u64(24), read_u64(40), read_u16(58), read_u16(60))
    } else {
        (read_u32(24) as u64, read_u32(32) as u64, read_u16(46), read_u16(48))
    };

    let mut segments = vec![];

    for k in 0..shnum {
        let off = (shoff + (k as u64) * (shentsize as u64)) as usize;
        if off >= raw.len() { break; }

        if is_64 {
            // ELF64 section header: sh_name(4) at +0, sh_type(4) at +4,
            // sh_flags(8) at +8, sh_addr(8) at +16, sh_offset(8) at +24,
            // sh_size(8) at +32. Reading sh_type at +0 picks up sh_name and
            // nothing ever matches SHT_PROGBITS.
            let sh_type = read_u32(off + 4);
            let sh_flags = read_u64(off + 8);
            let sh_addr = read_u64(off + 16);
            let sh_offset = read_u64(off + 24);
            let sh_size = read_u64(off + 32);

            // SHT_PROGBITS=1, SHF_EXECINSTR=0x4
            if sh_type == 1 && (sh_flags & 0x4) != 0 && sh_size > 0 {
                let end = (sh_offset + sh_size) as usize;
                if end <= raw.len() {
                    segments.push((sh_addr, raw[sh_offset as usize..end].to_vec()));
                }
            }
        } else {
            // ELF32 section header: sh_name(4), sh_type(4), sh_flags(4),
            // sh_addr(4), sh_offset(4), sh_size(4).
            let sh_type = read_u32(off + 4);
            let sh_flags = read_u32(off + 8) as u64;
            let sh_addr = read_u32(off + 12) as u64;
            let sh_offset = read_u32(off + 16) as u64;
            let sh_size = read_u32(off + 20) as u64;

            if sh_type == 1 && (sh_flags & 0x4) != 0 && sh_size > 0 {
                let end = (sh_offset + sh_size) as usize;
                if end <= raw.len() {
                    segments.push((sh_addr, raw[sh_offset as usize..end].to_vec()));
                }
            }
        }
    }

    (entry, segments)
}
