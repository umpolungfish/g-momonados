//! V⊙x CLI — lift an x86-64 ELF to twelve-glyph IMASM words and verdict them.
//!
//! The native decoder and lifter carry no external crates, so this builds on
//! its own. Subcommands mirror the surface of the original vox.py.

use ::vox::vox;
mod circuit_cli;
use ::vox::vox_decode;
use ::vox::lanes;
use ::vox::genetic;
use ::vox::protein;
use ::vox::fold;
use ::vox::fold3d;
use ::vox::x86;
use ::vox::{imasm_module, imasm_vm, loader, safetensors, divisor_membrane};

fn usage() {
    eprintln!("V⊙x — control-flow closure auditor");
    eprintln!();
    eprintln!("  vox <file.so|.elf>        lift every function, tally verdicts");
    eprintln!("  vox lift <file>           same");
    eprintln!("  vox run <sym> --args a,b <file>   recompile and RUN a function");
    eprintln!("  vox imasm <file>          emit the executable IMASM module");
    eprintln!("  vox glyphs <module.imasm> <output.glyphs>   encode complete module as glyphs");
    eprintln!("  vox unglyphs <word.glyphs> <output.imasm>  restore exact executable module");
    eprintln!("  vox circuit <module.imasm> [--stdin | hex-mask[:feedback] ...]   prepare once, switch resident QFT gates");
    eprintln!("  vox word <file>           emit the structure word per function");
    eprintln!("  vox verdict <glyph-word>  verdict one word (T/B/N/F)");
    eprintln!("  vox morphism-factor <native-numeral-word>   factor entirely over IMASM tapes");
    eprintln!("  vox construct-carrier <operator-word>        decompose a word into its factoring-morphism tower");
    eprintln!("  vox factor-with <operator-word> <n-word>     factor N on a carrier built from the operator word");
    eprintln!("  vox factor-operator resolve|full <N>         the CL9NK moat resolver over folded tapes");
    eprintln!("  vox scout <N>                                read the shape of N and hand the factor");
    eprintln!("  vox factor <N>                               shape-routed full factorization");
    eprintln!("  vox membrane bridge <N> <m>   coupled divisor-ring W_t trace over IMASM tapes");
    eprintln!("  vox pairs <glyph-word>    the pairing: every region, what it holds, what is left open");
    eprintln!("  vox verdict --tsv <file>   verdict name<TAB>word lines in bulk");
    eprintln!("  vox evm <hex>             lift EVM bytecode, verdict its closure");
    eprintln!("  vox wasm <hex>            lift a WASM function body, verdict it");
    eprintln!("  vox hex <hex>             lift raw machine-code hex, verdict its closure");
    eprintln!("  vox rna <seq> [--dialect mito]   lift a coding sequence, verdict the transcript");
    eprintln!("  vox aa <seq>              lift a protein (one-letter residues), verdict the fold");
    eprintln!("  vox fasta <file>          lift a protein from FASTA");
    eprintln!("  vox pdb <file>            lift a protein from a PDB (CA per residue)");
    eprintln!("  vox glyco <seq|file>      locate the glycosylation boundary interfaces");
    eprintln!("  vox compile <seq> [--code standard|mitochondrial] [--pdb <path>]");
    eprintln!("                            the full pipeline, both ends: RNA/DNA in gives a");
    eprintln!("                            compiled protein with real fold info (Chou-Fasman");
    eprintln!("                            secondary structure, heuristic tertiary contacts, a");
    eprintln!("                            real 3D backbone via B4-Ramachandran-NeRF); protein");
    eprintln!("                            in gives RNA/DNA back out (Frobenius-preferred codon");
    eprintln!("                            per residue, full degeneracy) with the same fold on");
    eprintln!("                            the input. Direction auto-detects from the alphabet.");
    eprintln!("                            --pdb writes a real PDB file, readable by vox pdb.");
    eprintln!("  vox self                  lift V⊙x's own image and read it back");
    eprintln!("  vox pyc <file.pyc>        lift every code object in a .pyc, verdict each");
    eprintln!("  vox safetensors <file>  lift a HuggingFace safetensors file, verdict each tensor");
    eprintln!("  vox classify <mn> [ops]   the glyph an instruction lifts to");
    eprintln!("  vox tables <file> <symbol>   shift a function one nibble, verdict a random");
    eprintln!("                            baseline of the same length, and flag any resync run");
    eprintln!("                            far longer than chance -- an embedded constant table");
    eprintln!("                            (witness sets, factor bases, curve parameters), located");
    eprintln!("                            purely from the binary and shown at its real alignment");
    eprintln!("  vox --selftest            planted open/closed forks");
    eprintln!();
    eprintln!("T closes · B holds a fork open across a terminal · N never forked ·");
    eprintln!("F is ill-typed (a ∋ with no ∈ to pair).");
}


/// A mode-aware linear-sweep audit: decode every executable byte at the given
/// width, split into functions at each terminal, verdict each. Used where
/// recursive descent's length decoder does not apply (32-bit x86).
/// Read a coding sequence as a word in the twelve and verdict it. The dialect
/// names the code table: the standard one, or a mitochondrial gene, where UGA
/// delivers tryptophan rather than terminating.
fn rna(seq: &str, dialect: &str) -> i32 {
    let t = if dialect.is_empty() {
        genetic::lift_rna(seq)
    } else {
        genetic::lift_rna_dialect(seq, dialect)
    };
    if t.word.is_empty() {
        eprintln!("no promoted codon in that sequence");
        return 1;
    }
    println!("{:<8}{:<6}{:<16}{}", "CODON", "AA", "AXIS", "GLYPH");
    for r in &t.reading {
        println!("{:<8}{:<6}{:<16}{}", r.codon, r.aa, r.axis, r.glyph);
    }
    println!();
    println!("dialect  {}", if dialect.is_empty() { "standard" } else { dialect });
    println!("codons   {} promoted of {} read", t.word.len(), t.reading.len());
    println!("frame    {} at offset {}", if t.implicit_frame { "no AUG, read from the start" } else { "AUG" }, t.start);
    println!("word     {}", vox::glyphs(&t.word));
    match t.stopped {
        Some(s) => println!("stop     {}", s),
        None => println!("stop     (none: the sequence ran out before a stop)"),
    }
    println!("verdict  {}", vox::verdict(&t.word));
    0
}

fn protein(seq: &str, source: &str) -> i32 {
    let t = genetic::lift_protein(seq);
    if t.word.is_empty() {
        eprintln!("no promoted residue in that sequence (the eight ground-layer amino acids are silent)");
        return 1;
    }
    println!("{:<8}{:<6}{:<16}{}", "POS", "AA", "AXIS", "GLYPH");
    for r in &t.reading {
        println!("{:<8}{:<6}{:<16}{}", r.codon, r.aa, r.axis, r.glyph);
    }
    println!();
    println!("source   {}", source);
    println!("residues {} promoted of the sequence", t.reading.len());
    println!("word     {}", vox::glyphs(&t.word));
    println!("verdict  {}", vox::verdict(&t.word));
    0
}

/// `vox compile <seq> [--code standard|mitochondrial] [--pdb <path>]`
///
/// One entry point, both directions of the same pipeline. RNA/DNA in:
/// `protein::translate_full` (every residue, not just the twelve promoted
/// ones `genetic::lift_rna` keeps), then fold (`fold::fold_sequence` for
/// secondary/tertiary structure, `fold3d` for a real 3D backbone). Protein
/// in: reverse-translate with the Frobenius-preferred codon per residue
/// (`protein::preferred_codon_for_aa`) and run the SAME fold on the input,
/// so the "vice versa" direction carries fold info too. Direction
/// auto-detects from the input alphabet: pure A/C/G/T/U reads as nucleic
/// acid, anything else as protein codes.
fn compile(args: &[String]) -> i32 {
    let mut dialect = String::new();
    let mut pdb_path: Option<String> = None;
    let mut seq_parts: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--code" if i + 1 < args.len() => {
                dialect = if args[i + 1] == "mitochondrial" || args[i + 1] == "mito" {
                    String::from("mitochondrial")
                } else { String::new() };
                i += 2;
            }
            "--pdb" if i + 1 < args.len() => { pdb_path = Some(args[i + 1].clone()); i += 2; }
            other => { seq_parts.push(other.to_string()); i += 1; }
        }
    }
    let seq = seq_parts.join(" ");
    let dialect_name = if dialect.is_empty() { "standard" } else { "mitochondrial" };

    let compact: String = seq.chars().filter(|c| !c.is_whitespace() && *c != '-' && *c != ',').collect();
    let is_nucleic = !compact.is_empty()
        && compact.chars().all(|c| matches!(c.to_ascii_uppercase(), 'A' | 'C' | 'G' | 'T' | 'U'));

    fn report_fold(chain: &[&'static str], b4_path: &[char], pdb_path: Option<&str>) {
        let f = fold::fold_sequence(chain);
        let n_h = f.residues.iter().filter(|r| r.secondary == fold::SecondaryLabel::Helix).count();
        let n_s = f.residues.iter().filter(|r| r.secondary == fold::SecondaryLabel::Sheet).count();
        let n_c = f.residues.len() - n_h - n_s;
        println!();
        println!("fold     helix {}  sheet {}  coil {}   ({} contacts, SerpentRod invariant {})",
            n_h, n_s, n_c, f.contacts.len(), if f.frobenius_ok { "PASS" } else { "FAIL" });
        let steps = fold3d::rama_steps(b4_path);
        let backbone = fold3d::build_backbone(&steps);
        println!("backbone {} residues placed (B4-Ramachandran-NeRF)", backbone.len());
        if let Some(path) = pdb_path {
            let elements = fold3d::group_ss_elements(&steps);
            let winding = f.residues.iter().map(|r| r.winding_number).max().unwrap_or(0);
            let pdb = fold3d::write_pdb(chain, &backbone, &elements, f.frobenius_ok, winding, "COMPILED THROUGH VOX", 'A');
            match std::fs::write(path, pdb.as_bytes()) {
                Ok(()) => println!("pdb      written to {}", path),
                Err(e) => println!("pdb      could not write to {}: {}", path, e),
            }
        }
    }

    if is_nucleic {
        let t = protein::translate_full(&compact, &dialect);
        if t.protein.is_empty() {
            eprintln!("no protein translated from '{}'; needs an ATG/AUG start codon", seq);
            return 1;
        }
        let bytes: Vec<char> = t.mrna.chars().collect();
        let b4_path: Vec<char> = (0..t.protein.len()).map(|k| {
            let p = t.start + k * 3;
            bytes.get(p).and_then(|&c| genetic::nuc_b4(c)).unwrap_or('N')
        }).collect();

        println!("== vox compile: RNA/DNA -> protein ({}) ==", dialect_name);
        println!("input    {}", seq);
        println!("mrna     {}", t.mrna);
        println!("protein  {}", t.protein.join("-"));
        match t.stopped {
            Some(s) => println!("stop     {}", s),
            None => println!("stop     (none: the sequence ran out before a stop)"),
        }
        report_fold(&t.protein, &b4_path, pdb_path.as_deref());
        0
    } else {
        let chain = match protein::parse_chain(&seq) {
            Some(c) if !c.is_empty() => c,
            _ => {
                eprintln!("could not parse '{}' as protein or nucleic acid", seq);
                eprintln!("use 3-letter (Met-Ala) or 1-letter (MA) amino acid codes, or A/C/G/T/U");
                return 1;
            }
        };
        let reverse = match protein::reverse_translate_full(&chain, &dialect) {
            Some(r) => r,
            None => { eprintln!("no codon exists for some residue under the {} table", dialect_name); return 1; }
        };
        let dna = protein::reverse_transcribe(&reverse.mrna);
        let bytes: Vec<char> = reverse.mrna.chars().collect();
        let b4_path: Vec<char> = (0..chain.len()).map(|k| {
            genetic::nuc_b4(bytes[k * 3]).unwrap_or('N')
        }).collect();

        println!("== vox compile: protein -> RNA/DNA ({}) ==", dialect_name);
        println!("input    {}", chain.join("-"));
        println!("mrna     {}  (Frobenius-preferred codon per residue)", reverse.mrna);
        println!("dna      {}", dna);
        println!("degeneracy  {} total possible mRNA sequences (product of per-residue codon counts)",
            reverse.total_combinations);
        report_fold(&chain, &b4_path, pdb_path.as_deref());
        0
    }
}

fn glyco(seq: &str, source: &str) -> i32 {
    let t = genetic::lift_protein(seq);
    let sites = genetic::glyco_sites(seq);
    let n_linked: Vec<_> = sites.iter().filter(|s| s.kind == "N-linked").collect();
    let o_linked: Vec<_> = sites.iter().filter(|s| s.kind == "O-linked").collect();

    println!("source   {}", source);
    println!("peptide  {}   ({} promoted residues, the bulk)", vox::glyphs(&t.word), t.reading.len());
    println!("verdict  {}   (of the peptide backbone)", vox::verdict(&t.word));
    println!();
    println!("BOUNDARY INTERFACES — where a glycan meets the peptide:");
    println!("  N-linked sequons (Asn-X-[Ser/Thr], X≠Pro) — determinate:");
    if n_linked.is_empty() {
        println!("    none");
    } else {
        for s in &n_linked {
            println!("    pos {:<5} {}   anchor ∈ (recognition gate) opens the sequon", s.pos, s.motif);
        }
    }
    println!("  O-linked candidates (Ser/Thr) — admitted, not determinate, and");
    println!("  GROUND-LAYER so they carry no mark in the peptide word: {}", o_linked.len());
    println!();
    println!("  the glycan tree at each site is a branched word of its own; lifting it");
    println!("  needs the monosaccharides grounded through the chem pipeline, not here.");
    0
}

/// The organism reads itself.
///
/// V⊙x lifts every substrate it is pointed at; pointed at its own image it
/// lifts the lifter. What comes back is not decoration: the self-image is the
/// only binary whose source is here to check the reading against.
/// Lift every code object in a .pyc and verdict each.
fn pyc_file(path: &str) -> i32 {
    let raw = match std::fs::read(path) {
        Ok(r) => r,
        Err(e) => { eprintln!("cannot read {}: {}", path, e); return 2; }
    };
    let objs = match ::vox::pyc::read_pyc(&raw) {
        Ok(o) => o,
        Err(e) => { eprintln!("{}: {}", path, e); return 3; }
    };
    println!("{}  cpython {}  {} code object(s)", path, ::vox::pyc_table::PY_VERSION, objs.len());
    let mut tally = [0usize; 4];
    for o in &objs {
        let w = ::vox::pyc::lift(&o.code);
        let v = vox::verdict(&w);
        match v { 'T'=>tally[0]+=1,'B'=>tally[1]+=1,'N'=>tally[2]+=1,_=>tally[3]+=1 }
        let f = vox::open_forks(&w);
        println!("  {:<28} {}  {}   surplus {:>3} exits {}", o.name, v, vox::glyphs(&w), f.surplus, f.exits);
    }
    println!("  verdicts  T {}   B {}   N {}   F {}", tally[0], tally[1], tally[2], tally[3]);
    0
}

/// Lift a raw hex value stream: decode the hex to bytes, load them as a code
/// artifact, walk the control flow, recompile each function to its glyph word,
/// and read the closure verdict of each. The delta half of the vox pair pointed
/// at inline hex, the same lift `vox self` runs on a file.
fn hexlift(hexstr: &str) -> i32 {
    let raw = lanes::from_hex(hexstr);
    if raw.is_empty() { eprintln!("vox hex: no hex bytes in input"); return 2; }
    let l = loader::load(&raw);
    let image = vox_decode::Image { segments: l.code.clone() };
    let mut seeds: Vec<u64> = l.symbols.values().copied().collect();
    seeds.push(l.entry);
    let mut w = vox_decode::walk(&image, l.entry, &seeds);
    vox_decode::mark_noreturn(&mut w.functions, &l.symbols);

    println!("HEX    {} bytes  {}  {}  {} function(s) by descent",
             raw.len(), l.format, l.arch, w.functions.len());
    if w.functions.is_empty() {
        eprintln!("  no function reachable from entry 0x{:x}", l.entry);
        return 1;
    }
    let mut tally = [0usize; 4];
    for (addr, f) in &w.functions {
        let word = vox::recompile_function(f);
        let v = vox::verdict(&word);
        match v { 'T'=>tally[0]+=1, 'B'=>tally[1]+=1, 'N'=>tally[2]+=1, _=>tally[3]+=1 }
        let mark = if v == 'B' { "   <-- FINDING (fork open across commit)" } else { "" };
        println!("  0x{:08x}  {}  {}{}", addr, v, vox::glyphs(&word), mark);
    }
    println!("  verdicts  T {}   B {}   N {}   F {}", tally[0], tally[1], tally[2], tally[3]);
    0
}

/// Linear sweep of `bytes` as x86-64, splitting into fragments at each
/// terminal instruction (ret/int3/ud2/hlt/jmp) or at the first undecodable
/// byte. Returns each fragment's (start_offset, length_in_bytes) within
/// `bytes` -- the same terminal predicate the fallback sweep in `lift_file`
/// uses, applied here to a standalone buffer rather than a whole image.
fn linear_fragments(bytes: &[u8]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    let mut frag_start = 0usize;
    let mut frag_len = 0usize;
    while pos < bytes.len() {
        match x86::decode(&bytes[pos..], pos as u64) {
            Some(d) if d.len > 0 => {
                let terminal = d.mnemonic.starts_with("ret")
                    || matches!(d.mnemonic.as_str(), "int3" | "ud2" | "hlt" | "jmp");
                frag_len += d.len;
                pos += d.len;
                if terminal { out.push((frag_start, frag_len)); frag_start = pos; frag_len = 0; }
            }
            _ => {
                if frag_len > 0 { out.push((frag_start, frag_len)); }
                pos += 1; frag_start = pos; frag_len = 0;
            }
        }
    }
    if frag_len > 0 { out.push((frag_start, frag_len)); }
    out
}

/// Shift a byte buffer left by one nibble (4 bits): output byte i is built
/// from the low nibble of input byte i and the high nibble of input byte
/// i+1, so the same underlying bits decode from a half-byte-off starting
/// point -- real content, read out of alignment, not a different function.
fn nibble_shift(bytes: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; bytes.len()];
    for i in 0..bytes.len() {
        let hi = bytes[i] << 4;
        let lo = if i + 1 < bytes.len() { bytes[i + 1] >> 4 } else { 0 };
        out[i] = hi | lo;
    }
    out
}

/// xorshift64* -- a small deterministic PRNG so the random baseline is
/// reproducible from a fixed seed, with no external crate.
fn xorshift_bytes(state: &mut u64, n: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(n);
    while out.len() < n {
        *state ^= *state << 13;
        *state ^= *state >> 7;
        *state ^= *state << 17;
        for b in state.to_le_bytes() { if out.len() < n { out.push(b); } }
    }
    out
}

/// Locate embedded constant/data tables inside one named function by nibble-
/// phase resync anomaly: shift the function's real bytes by one nibble,
/// linear-sweep the result into fragments at each terminal instruction, and
/// compare the longest fragment against a random-byte baseline of the same
/// length. Ordinary code and pure noise both desync within a bounded range;
/// a run of near-identical instruction templates (a witness set, a factor
/// base, curve parameters) survives misalignment far longer than chance,
/// because its repetition is what the wrong phase accidentally resyncs
/// against. Flags the anomaly and prints the REAL, correctly aligned
/// instructions there, since the wrong-phase reading is a locator, not the
/// content.
fn find_tables(path: &str, sym: &str) -> i32 {
    let raw = match std::fs::read(path) {
        Ok(r) => r,
        Err(e) => { eprintln!("cannot read {}: {}", path, e); return 2; }
    };
    let l = loader::load(&raw);
    let mut addrs: Vec<(u64, String)> = l.symbols.iter().map(|(k, v)| (*v, k.clone())).collect();
    addrs.sort();
    // Symbol table names are Rust's raw mangled form (e.g. "_RNvNtCs...21is_prime_miller_rabin"),
    // which still carries the real identifier as a literal substring -- so match on
    // that instead of demangling, and prefer the shortest hit (the function itself,
    // not one of its closures or monomorphizations, which mangle longer).
    let candidates: Vec<usize> = addrs.iter().enumerate()
        .filter(|(_, (_, name))| name.contains(sym))
        .map(|(i, _)| i).collect();
    if candidates.is_empty() { eprintln!("vox tables: symbol '{}' not found", sym); return 1; }
    let idx = *candidates.iter().min_by_key(|&&i| addrs[i].1.len()).unwrap();
    if candidates.len() > 1 {
        eprintln!("  ({} symbols contain '{}', using the shortest: {})", candidates.len(), sym, addrs[idx].1);
    }
    let start = addrs[idx].0;
    let end = if idx + 1 < addrs.len() { addrs[idx + 1].0 } else { start + 0x10000 };
    let seg = match l.code.iter().find(|(base, bytes)| start >= *base && start < base + bytes.len() as u64) {
        Some(s) => s,
        None => { eprintln!("vox tables: 0x{:x} is not in any executable segment", start); return 1; }
    };
    let off = (start - seg.0) as usize;
    let len = ((end - start) as usize).min(seg.1.len().saturating_sub(off));
    if len == 0 { eprintln!("vox tables: empty range for '{}'", sym); return 1; }
    let bytes = &seg.1[off..off + len];

    println!("{}  0x{:x}..0x{:x}  {} bytes", sym, start, end, len);

    let shifted = nibble_shift(bytes);
    let real_frags = linear_fragments(&shifted);
    let (real_off, real_len) = real_frags.iter().copied().max_by_key(|&(_, flen)| flen).unwrap_or((0, 0));

    let trials = 20usize;
    let mut state: u64 = 0x9E3779B97F4A7C15 ^ (len as u64);
    let mut baseline: Vec<f64> = Vec::with_capacity(trials);
    for _ in 0..trials {
        let rnd = xorshift_bytes(&mut state, len);
        let m = linear_fragments(&rnd).iter().map(|&(_, flen)| flen).max().unwrap_or(0);
        baseline.push(m as f64);
    }
    let mean = baseline.iter().sum::<f64>() / trials as f64;
    let var = baseline.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / trials as f64;
    let sd = var.sqrt();
    let z = if sd > 0.0 { (real_len as f64 - mean) / sd } else if (real_len as f64) > mean { f64::INFINITY } else { 0.0 };

    println!("  phase-1 longest resync run: {} bytes at offset 0x{:x}  (random baseline, {} trials: mean {:.1}, sd {:.1})",
        real_len, real_off, trials, mean, sd);

    if z > 2.0 {
        println!("  FLAG: {:.1} standard deviations above the random baseline -- probable embedded constant/table", z);
        let show_end = (real_off + real_len + 16).min(bytes.len());
        // Walk forward from the function's real start so every printed line
        // lands on a genuine instruction boundary, rather than decoding raw
        // from `real_off` itself, which is a byte offset found in the SHIFTED
        // stream and is not guaranteed to be a boundary in the real one.
        println!("  real (correctly aligned) bytes near 0x{:x}:", start + real_off as u64);
        let mut pos = 0usize;
        let mut printing = false;
        while pos < show_end {
            match x86::decode(&bytes[pos..], start + pos as u64) {
                Some(d) if d.len > 0 => {
                    if !printing && pos + d.len > real_off.saturating_sub(8) { printing = true; }
                    if printing {
                        let op_str = if let (true, Some(t)) = (d.ops.len() == 1, d.target) { format!("0x{:x}", t) }
                            else { d.ops.iter().map(|o| o.intel()).collect::<Vec<_>>().join(", ") };
                        println!("    0x{:x}  {} {}", d.addr, d.mnemonic, op_str);
                    }
                    pos += d.len;
                }
                _ => {
                    if printing { println!("    0x{:x}  .byte 0x{:02x}", start + pos as u64, bytes[pos]); }
                    pos += 1;
                }
            }
        }
    } else {
        println!("  no anomaly -- consistent with ordinary code, nothing flagged");
    }
    0
}

fn selfread(path: &str) -> i32 {
    let raw = match std::fs::read(path) {
        Ok(r) => r,
        Err(e) => { eprintln!("cannot read {}: {}", path, e); return 2; }
    };
    let l = loader::load(&raw);
    let image = vox_decode::Image { segments: l.code.clone() };
    let mut seeds: Vec<u64> = l.symbols.values().copied().collect();
    seeds.push(l.entry);
    let mut w = vox_decode::walk(&image, l.entry, &seeds);
    vox_decode::mark_noreturn(&mut w.functions, &l.symbols);

    let mut tally = [0usize; 4];
    let mut by_exit: std::collections::BTreeMap<i32, (usize, i64)> = std::collections::BTreeMap::new();
    let mut worst: Vec<(i32, u64, i32, i32)> = Vec::new();   // residual, addr, surplus, exits
    for (addr, f) in &w.functions {
        let word = vox::recompile_function(f);
        match vox::verdict(&word) { 'T'=>tally[0]+=1, 'B'=>tally[1]+=1, 'N'=>tally[2]+=1, _=>tally[3]+=1 }
        let o = vox::open_forks(&word);
        let e = by_exit.entry(o.exits.min(4)).or_insert((0, 0));
        e.0 += 1; e.1 += o.surplus as i64;
        if o.residual > 0 { worst.push((o.residual, *addr, o.surplus, o.exits)); }
    }
    worst.sort_by(|a, b| b.0.cmp(&a.0));

    println!("{}  {}  {} function(s) by descent", path, l.format, w.functions.len());
    println!("  verdicts  T {}   B {}   N {}   F {}", tally[0], tally[1], tally[2], tally[3]);
    println!("  F is zero when the decoder is in phase with the image.\n");
    println!("  open forks against exits — an early return is a fork that never rejoins:");
    println!("    {:>6}  {:>7}  {:>14}", "exits", "funcs", "mean surplus");
    for (k, (n, sum)) in &by_exit {
        println!("    {:>5}{}  {:>7}  {:>14.2}", k, if *k == 4 { "+" } else { " " }, n, *sum as f64 / *n as f64);
    }
    println!("\n  {} function(s) carry surplus the exits do not explain; deepest first:", worst.len());
    for (res, addr, sur, ex) in worst.iter().take(10) {
        println!("    0x{:<8x}  residual {:>3}   surplus {:>3}   exits {}", addr, res, sur, ex);
    }
    println!("\n  Ranking by raw surplus ranks by how many ways a function can return.");
    println!("  The residual is what is left once that is paid for.");
    0
}

fn audit_linear(path: &str, l: &loader::Loaded, bits: u8) -> i32 {
    let total: usize = l.code.iter().map(|(_, b)| b.len()).sum();
    println!("{}  {} {}  entry 0x{:x}  {} byte(s) of code", path, l.format, l.arch, l.entry, total);
    let mut tally = [0usize; 4]; let mut funcs = 0usize; let mut covered = 0usize;
    let mut b_findings: Vec<(u64, String)> = Vec::new();
    for (base, bytes) in &l.code {
        let mut pos = 0usize; let mut cur: Vec<x86::Insn> = Vec::new(); let mut fstart = *base;
        let flush = |cur: &mut Vec<x86::Insn>, fstart: u64, tally: &mut [usize;4], funcs: &mut usize, bf: &mut Vec<(u64,String)>| {
            if cur.is_empty() { return; }
            let word = alloc_word(cur);
            let v = vox::verdict(&word);
            match v { 'T'=>tally[0]+=1,'B'=>tally[1]+=1,'N'=>tally[2]+=1,_=>tally[3]+=1 }
            if v == 'B' && bf.len() < 40 { bf.push((fstart, vox::glyphs(&word))); }
            *funcs += 1; cur.clear();
        };
        while pos < bytes.len() {
            let addr = base + pos as u64;
            match x86::decode_mode(&bytes[pos..], addr, bits) {
                Some(d) if d.len > 0 => {
                    if cur.is_empty() { fstart = addr; }
                    let mn = d.mnemonic.clone(); covered += d.len; pos += d.len;
                    let term = mn.starts_with("ret") || matches!(mn.as_str(), "int3"|"ud2"|"hlt"|"jmp");
                    cur.push(d);
                    if term { flush(&mut cur, fstart, &mut tally, &mut funcs, &mut b_findings); }
                }
                _ => { flush(&mut cur, fstart, &mut tally, &mut funcs, &mut b_findings); pos += 1; }
            }
        }
        flush(&mut cur, fstart, &mut tally, &mut funcs, &mut b_findings);
    }
    println!("  {} function(s) by linear sweep, {}% decoded ({} of {} bytes)",
        funcs, (covered*100/total.max(1)).min(100), covered.min(total), total);
    println!("  verdicts  T {}   B {}   N {}   F {}", tally[0], tally[1], tally[2], tally[3]);
    for (a, w) in b_findings.iter().take(12) { println!("    0x{:x}  {}", a, w); }
    0
}

/// Lift a decoded run to a word. This goes through `vox::recompile_function`
/// so the merge pass runs: without it no ∋ is ever emitted, every fork reads as
/// dangling, and the lane cannot return T at all.
fn alloc_word(insns: &[x86::Insn]) -> Vec<char> {
    let lifted: Vec<vox::Instruction> = insns.iter().map(|i| vox::Instruction {
        address: i.addr,
        fallthrough: if i.mnemonic=="jmp" || i.mnemonic.starts_with("ret") || matches!(i.mnemonic.as_str(),"int3"|"ud2"|"hlt"|"iret") {None} else {Some(i.addr+i.len as u64)},
        mnemonic: i.mnemonic.clone(),
        op_str: if let (true, Some(t)) = (i.ops.len() == 1, i.target) { format!("{:#x}", t) }
                else { i.ops.iter().map(|o| o.intel()).collect::<Vec<_>>().join(", ") },
    }).collect();
    vox::recompile_function(&lifted)
}


fn lift_file(path: &str) -> i32 {
    let raw = match std::fs::read(path) {
        Ok(r) => r,
        Err(e) => { eprintln!("cannot read {}: {}", path, e); return 2; }
    };
    let l = loader::load(&raw);
    if l.code.is_empty() {
        eprintln!("{}: no executable sections found", path);
        return 1;
    }
    if l.arch != "x86-64" && l.arch != "x86-32" {
        eprintln!("{}: {} {} code. V⊙x decodes x86; it will not misread another", path, l.format, l.arch);
        eprintln!("architecture and hand back a confident, wrong word.");
        return 3;
    }
    if l.arch == "x86-32" {
        return audit_linear(path, &l, 32);
    }
    let image = vox_decode::Image { segments: l.code };
    println!("{}  {}  entry 0x{:x}  {} byte(s) of code", path, l.format, l.entry, image.total_bytes());
    let mut seeds: Vec<u64> = l.symbols.values().copied().collect(); seeds.push(l.entry);
    let mut w = vox_decode::walk(&image, l.entry, &seeds);
    vox_decode::mark_noreturn(&mut w.functions, &l.symbols);
    let decoded: usize = w.functions.iter().map(|f| f.1.len()).sum();
    println!("  {} function(s), {} instruction(s)", w.functions.len(), decoded);
    println!("  claimed {}% of the image ({} of {} bytes)",
        w.claimed_percent(), w.claimed_bytes, w.total_bytes);

    let mut tally = [0usize; 4]; // T B N F
    let mut illtyped: Vec<(u64, String)> = Vec::new();
    let mut descent_f = 0usize;
    // recursive-descent functions: the trustworthy set
    let mut claimed: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
    for (start, f) in &w.functions {
        for ins in f { claimed.insert(ins.address); }
        let word = vox::recompile_function(f);
        match vox::verdict(&word) {
            'T' => tally[0] += 1, 'B' => tally[1] += 1, 'N' => tally[2] += 1,
            _ => { tally[3] += 1; descent_f += 1; if illtyped.len() < 8 { illtyped.push((*start, vox::glyphs(&word))); } }
        }
    }
    let descended = w.functions.len();
    // fallback sweep: linear-decode what descent never reached, group into
    // functions of their own at each terminal, and verdict those too.
    let mut swept = 0usize; let mut swept_bytes = 0usize;
    let mut sweep_f = 0usize;
    let mut illtyped_sweep: Vec<(u64, String)> = Vec::new();
    for (base, bytes) in &image.segments {
        let mut pos = 0usize;
        let mut cur: Vec<x86::Insn> = Vec::new();
        let flush = |cur: &mut Vec<x86::Insn>, tally: &mut [usize;4], swept: &mut usize, sweep_f: &mut usize,
                     illtyped_sweep: &mut Vec<(u64, String)>| {
            if cur.is_empty() { return; }
            let start_addr = cur[0].addr;
            let mut word = alloc_word(cur);
            match vox::verdict(&word) {
                'T'=>tally[0]+=1,'B'=>tally[1]+=1,'N'=>tally[2]+=1,
                _=>{ tally[3]+=1; *sweep_f+=1;
                     if illtyped_sweep.len() < 8 { illtyped_sweep.push((start_addr, vox::glyphs(&word))); } }
            }
            *swept += 1; word.clear(); cur.clear();
        };
        while pos < bytes.len() {
            let addr = base + pos as u64;
            if claimed.contains(&addr) { flush(&mut cur, &mut tally, &mut swept, &mut sweep_f, &mut illtyped_sweep);
                // skip the claimed instruction
                if let Some(d) = x86::decode(&bytes[pos..], addr) { pos += d.len.max(1); } else { pos += 1; }
                continue;
            }
            match x86::decode(&bytes[pos..], addr) {
                Some(d) if d.len > 0 => {
                    let mn = d.mnemonic.clone(); swept_bytes += d.len; pos += d.len;
                    let terminal = mn.starts_with("ret") || matches!(mn.as_str(), "int3"|"ud2"|"hlt"|"jmp");
                    cur.push(d);
                    if terminal { flush(&mut cur, &mut tally, &mut swept, &mut sweep_f, &mut illtyped_sweep); }
                }
                _ => { flush(&mut cur, &mut tally, &mut swept, &mut sweep_f, &mut illtyped_sweep); pos += 1; }
            }
        }
        flush(&mut cur, &mut tally, &mut swept, &mut sweep_f, &mut illtyped_sweep);
    }
    let total_cov = w.claimed_bytes + swept_bytes;
    println!();
    if swept > 0 {
        println!("  {} function(s) by descent, {} more by fallback sweep", descended, swept);
        println!("  covered {}% of the image ({} of {} bytes)",
            (total_cov*100/w.total_bytes.max(1)).min(100), total_cov.min(w.total_bytes), w.total_bytes);
    }
    println!("  verdicts  T {}   B {}   N {}   F {}  (F: {} from descent, {} from fallback sweep)",
        tally[0], tally[1], tally[2], tally[3], descent_f, sweep_f);
    for (a, g) in &illtyped {
        let head: String = g.chars().take(90).collect();
        println!("    0x{:x}  descent   {}", a, head);
    }
    for (a, g) in &illtyped_sweep {
        let head: String = g.chars().take(90).collect();
        println!("    0x{:x}  sweep     {}", a, head);
    }
    0
}

fn lane(isa: &str, word: &[char]) -> i32 {
    let v = vox::verdict(word);
    let mark = if v == 'B' { "   <-- FINDING (fork open across commit)" } else { "" };
    println!("{:<6} {}  {}{}", isa, v, vox::glyphs(word), mark);
    0
}

fn selftest() -> i32 {
    // Planted glyph words: the auditor's own law, independent of any decoder.
    // ⊢ open ∈ fork ⊡ commit ⊣ terminal ∋ merge
    let cases: &[(&str, &str, char)] = &[
        ("linear routine, never forks",        "⊢⊡⊣",   'N'),
        ("fork that merges before terminal",   "⊢∈⊡∋⊣", 'T'),
        ("fork held open across the terminal", "⊢∈⊡⊣",  'B'),
        ("merge with nothing to pair",         "⊢∋⊣",   'F'),
    ];
    let mut ok = true;
    for (name, w, want) in cases {
        let word: Vec<char> = w.chars().collect();
        let got = vox::verdict(&word);
        let mark = if got == *want { "ok" } else { ok = false; "FAIL" };
        println!("  {:<38} {}  {}  (expect {})  {}", name, w, got, want, mark);
    }
    // EVM and WASM bytecode: a state commit inside an unmerged branch (B) vs
    // paths that rejoin before the commit (T). Same law, real bytes.
    let bc: &[(&str, char, Vec<char>)] = &[
        ("EVM reentrant (commit in unmerged branch)", 'B', lanes::evm_word("600160075755005b00")),
        ("EVM guarded  (paths merge before commit)",  'T', lanes::evm_word("6001600657545b5500")),
        ("WASM reentrant (commit + return in branch)", 'B', lanes::wasm_word("20000440410141003602000f0b0b")),
        ("WASM guarded  (if merges before commit)",    'T', lanes::wasm_word("2000044010000b410041003602000b")),
    ];
    for (name, want, word) in bc {
        let got = vox::verdict(word);
        let mark = if got == *want { "ok" } else { ok = false; "FAIL" };
        println!("  {:<42} {}  (expect {})  {}", name, got, want, mark);
    }
    if ok { println!("selftest OK: the closure law holds on x86, EVM and WASM."); 0 }
    else { eprintln!("selftest FAILED"); 1 }
}


/// Read a file, or say which one and why, and leave without a backtrace.
///
/// `.expect("read")` printed `read: Os { code: 2 }` and a panic notice, naming
/// neither the path nor the verb. Fed a list of a few hundred binaries — which is
/// how this is actually used — that is a wall of identical panics with nothing in
/// them to act on, and the one path that was wrong stays hidden.
fn read_or_exit(path: &str) -> Vec<u8> {
    match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("vox: cannot read {}: {}", path, e);
            std::process::exit(2);
        }
    }
}

/// Real file and console I/O for `imasm_vm::Machine::run_process`, backed by
/// actual `std::fs`/stdio. A guest that opens, reads, or writes a file under
/// this does so for real, under whatever permission this process already
/// has — the same access the shell that launched `vox` already had.
struct StdHost { files: std::collections::BTreeMap<i32, std::fs::File>, next_fd: i32 }
impl StdHost { fn new() -> Self { StdHost { files: std::collections::BTreeMap::new(), next_fd: 100 } } }
impl imasm_vm::Host for StdHost {
    fn open(&mut self, path: &str, flags: i32, mode: i32) -> i32 {
        let mut opts = std::fs::OpenOptions::new();
        let acc = flags & 0b11;
        opts.read(acc == 0 || acc == 2).write(acc == 1 || acc == 2);
        if flags & 0o100 != 0 { opts.create(true); }
        if flags & 0o1000 != 0 { opts.truncate(true); }
        if flags & 0o2000 != 0 { opts.append(true); }
        #[cfg(unix)] { use std::os::unix::fs::OpenOptionsExt; opts.mode(mode as u32); }
        #[cfg(not(unix))] { let _ = mode; }
        match opts.open(path) {
            Ok(f) => { let fd = self.next_fd; self.next_fd += 1; self.files.insert(fd, f); fd }
            Err(_) => -2, // ENOENT: a precise errno map is a further rung
        }
    }
    fn read(&mut self, fd: i32, buf: &mut [u8]) -> i64 {
        use std::io::Read;
        match fd {
            0 => std::io::stdin().read(buf).map(|n| n as i64).unwrap_or(-5),
            _ => match self.files.get_mut(&fd) { Some(f) => f.read(buf).map(|n| n as i64).unwrap_or(-5), None => -9 },
        }
    }
    fn write(&mut self, fd: i32, buf: &[u8]) -> i64 {
        use std::io::Write;
        match fd {
            1 => std::io::stdout().write_all(buf).map(|_| buf.len() as i64).unwrap_or(-5),
            2 => std::io::stderr().write_all(buf).map(|_| buf.len() as i64).unwrap_or(-5),
            _ => match self.files.get_mut(&fd) { Some(f) => f.write(buf).map(|n| n as i64).unwrap_or(-5), None => -9 },
        }
    }
    fn close(&mut self, fd: i32) -> i32 {
        if matches!(fd, 0|1|2) { return 0; }
        if self.files.remove(&fd).is_some() { 0 } else { -9 }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(|s| s.as_str()) {
        None | Some("-h") | Some("--help") | Some("help") => { usage(); 0 }
        Some("--selftest") | Some("--self-test") | Some("selftest") | Some("self-test") => selftest(),
        Some("verdict") => {
            // A word given as an ARGUMENT is capped by the OS's argv limit, and a
            // lifted proof term runs to tens of thousands of glyphs — the words this
            // is most wanted for are exactly the ones that will not fit. `-` reads
            // the word from stdin instead, and prints only the verdict, so a sweep
            // can pipe thousands through without the shell in the way.
            if args.len() < 2 { eprintln!("vox verdict <glyph-word> | vox verdict - (word on stdin) | vox verdict --tsv <file>"); 1 }
            else if args[1] == "--tsv" {
                // A sweep hands over thousands of lifted words at once. Spawning a
                // process per word makes the shell the bottleneck and, worse, makes
                // it tempting to reimplement the verdict on the caller's side. One
                // process, one authority: `name<TAB>word` in, `name<TAB>verdict<TAB>len`
                // out, in input order. A blank word is not verdicted — it is
                // reported as `-`, since a word nobody lifted is not a word that
                // failed to close.
                if args.len() < 3 { eprintln!("vox verdict --tsv <file>"); 1 }
                else {
                    match std::fs::read_to_string(&args[2]) {
                        Err(e) => { eprintln!("vox verdict --tsv: {}: {}", args[2], e); 1 }
                        Ok(txt) => {
                            let mut counts = std::collections::BTreeMap::new();
                            for line in txt.lines() {
                                if line.trim().is_empty() { continue; }
                                let mut it = line.splitn(2, '\t');
                                let name = it.next().unwrap_or("");
                                let w = it.next().unwrap_or("").trim();
                                if w.is_empty() {
                                    println!("{}\t-\t0", name);
                                    *counts.entry('-').or_insert(0usize) += 1;
                                    continue;
                                }
                                let word: Vec<char> = w.chars().collect();
                                let v = vox::verdict(&word);
                                *counts.entry(v).or_insert(0usize) += 1;
                                println!("{}\t{}\t{}", name, v, word.len());
                            }
                            let total: usize = counts.values().sum();
                            let tally: Vec<String> = counts.iter().map(|(k, n)| format!("{} {}", k, n)).collect();
                            eprintln!("verdicted {} words: {}", total, tally.join("  "));
                            0
                        }
                    }
                }
            }
            else if args[1] == "-" {
                use std::io::Read;
                let mut buf = String::new();
                match std::io::stdin().read_to_string(&mut buf) {
                    Err(_) => { eprintln!("vox verdict -: could not read stdin"); 1 }
                    Ok(_) => {
                        let word: Vec<char> = buf.trim().chars().collect();
                        println!("verdict {}", vox::verdict(&word));
                        0
                    }
                }
            }
            else {
                let word: Vec<char> = args[1..].join("").chars().collect();
                println!("{}", vox::glyphs(&word));
                println!("verdict {}", vox::verdict(&word));
                0
            }
        }
        Some("pairs") | Some("pairing") => {
            if args.len() < 2 { eprintln!("vox pairs <glyph-word>"); 1 }
            else {
                let word: Vec<char> = args[1..].join("").chars().collect();
                let (regions, un_split, un_fuse) = vox::pairing(&word);
                println!("{:<7}{:<7}{:<7}{:<6}{}", "OPEN", "CLOSE", "SPAN", "WORK", "INTERIOR");
                for r in &regions {
                    let span = (r.fuse + word.len() - r.split) % word.len();
                    println!("{:<7}{:<7}{:<7}{:<6}{}", r.split, r.fuse, span,
                        if r.substantial { "yes" } else { "no" }, r.interior);
                }
                println!();
                println!("letters      {}", word.len());
                println!("regions      {} paired, {} substantial", regions.len(),
                    regions.iter().filter(|r| r.substantial).count());
                let ds: Vec<String> = un_split.iter().map(|i| i.to_string()).collect();
                let fs: Vec<String> = un_fuse.iter().map(|i| i.to_string()).collect();
                println!("unanswered   {} division(s) at {}", un_split.len(),
                    if ds.is_empty() { "-".to_string() } else { ds.join(",") });
                println!("unopened     {} rejoining(s) at {}", un_fuse.len(),
                    if fs.is_empty() { "-".to_string() } else { fs.join(",") });
                println!("verdict      {}", vox::verdict(&word));
                0
            }
        }
        Some("classify") => {
            if args.len() < 2 { eprintln!("vox classify <mnemonic> [operands]"); 1 }
            else {
                let ins = vox::Instruction { address: 0, fallthrough: None, mnemonic: args[1].to_lowercase(), op_str: args[2..].join(" ") };
                println!("{} {}", ins.mnemonic, vox::classify_instruction(&ins));
                0
            }
        }
        Some("tables") | Some("--tables") => {
            if args.len() < 3 { eprintln!("vox tables <file> <symbol>"); 1 }
            else { find_tables(&args[1], &args[2]) }
        }
        Some("evm") | Some("--evm") => { if args.len() < 2 { eprintln!("vox evm <hex>"); 1 } else { lane("EVM", &lanes::evm_word(&args[1])) } }
        Some("wasm") | Some("--wasm") => { if args.len() < 2 { eprintln!("vox wasm <hex>"); 1 } else { lane("WASM", &lanes::wasm_word(&args[1])) } }
        Some("hex") | Some("--hex") => { if args.len() < 2 { eprintln!("vox hex <hex>   lift raw machine-code hex, verdict its closure"); 1 } else { hexlift(&args[1..].join("")) } }
        Some("rna") | Some("--rna") => {
            if args.len() < 2 { eprintln!("vox rna <sequence> [--dialect mito]"); 1 }
            else {
                let mut dialect = String::new();
                let mut seq = String::new();
                let mut it = args[1..].iter();
                while let Some(a) = it.next() {
                    if a == "--dialect" { dialect = it.next().cloned().unwrap_or_default(); }
                    else { seq.push_str(a); }
                }
                rna(&seq, &dialect)
            }
        }
        Some("aa") | Some("protein") => {
            if args.len() < 2 { eprintln!("vox aa <one-letter amino-acid sequence>"); 1 }
            else { protein(&args[1..].join(""), "sequence") }
        }
        Some("fasta") => {
            if args.len() < 2 { eprintln!("vox fasta <file.fasta>"); 1 }
            else { match std::fs::read_to_string(&args[1]) {
                Ok(txt) => protein(&genetic::protein_from_fasta(&txt), &args[1]),
                Err(e) => { eprintln!("vox fasta: {}: {}", args[1], e); 1 }
            } }
        }
        Some("pdb") => {
            if args.len() < 2 { eprintln!("vox pdb <file.pdb>"); 1 }
            else { match std::fs::read_to_string(&args[1]) {
                Ok(txt) => protein(&genetic::protein_from_pdb(&txt), &args[1]),
                Err(e) => { eprintln!("vox pdb: {}: {}", args[1], e); 1 }
            } }
        }
        Some("compile") => {
            if args.len() < 2 { eprintln!("vox compile <seq> [--code standard|mitochondrial] [--pdb <path>]"); 1 }
            else { compile(&args[1..]) }
        }
        Some("glyco") => {
            if args.len() < 2 { eprintln!("vox glyco <seq | file.fasta | file.pdb>"); 1 }
            else {
                let a = &args[1];
                let loaded: Result<(String, String), String> = if a.ends_with(".pdb") {
                    std::fs::read_to_string(a).map(|t| (genetic::protein_from_pdb(&t), a.clone())).map_err(|e| e.to_string())
                } else if a.ends_with(".fasta") || a.ends_with(".fa") {
                    std::fs::read_to_string(a).map(|t| (genetic::protein_from_fasta(&t), a.clone())).map_err(|e| e.to_string())
                } else {
                    Ok((args[1..].join(""), "sequence".to_string()))
                };
                match loaded {
                    Ok((seq, src)) => glyco(&seq, &src),
                    Err(e) => { eprintln!("vox glyco: {a}: {e}"); 1 }
                }
            }
        }
        Some("self") => {
            let me = std::env::current_exe().map(|p| p.display().to_string())
                .unwrap_or_else(|_| "vox".into());
            selfread(&me)
        }
        Some("pyc") | Some("py") => {
            if args.len() < 2 { eprintln!("vox pyc <file.pyc>"); 1 } else { pyc_file(&args[1]) }
        }
        Some("findings") => {
            if args.len()<2 { eprintln!("vox findings <file>"); return; }
            let raw = read_or_exit(&args[1]);
            let l = loader::load(&raw);
            let image = vox_decode::Image { segments: l.code };
            let mut seeds: Vec<u64> = l.symbols.values().copied().collect(); seeds.push(l.entry);
            let mut w = vox_decode::walk(&image, l.entry, &seeds);
            vox_decode::mark_noreturn(&mut w.functions, &l.symbols);
            let mut claimed: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
            let mut b: Vec<(u64,String)> = Vec::new();
            for (start,f) in &w.functions {
                for ins in f { claimed.insert(ins.address); }
                let word = vox::recompile_function(f);
                if vox::verdict(&word)=='B' { b.push((*start, vox::glyphs(&word))); }
            }
            // sweep
            for (base,bytes) in &image.segments {
                let mut pos=0usize; let mut cur:Vec<x86::Insn>=Vec::new(); let mut fstart=*base;
                let flush=|cur:&mut Vec<x86::Insn>, fstart:u64, b:&mut Vec<(u64,String)>| {
                    if cur.is_empty(){return;} let word=alloc_word(cur);
                    if vox::verdict(&word)=='B' { b.push((fstart, vox::glyphs(&word))); } cur.clear();
                };
                while pos<bytes.len() {
                    let addr=base+pos as u64;
                    if claimed.contains(&addr){ flush(&mut cur,fstart,&mut b); if let Some(d)=x86::decode(&bytes[pos..],addr){pos+=d.len.max(1);}else{pos+=1;} continue; }
                    match x86::decode(&bytes[pos..],addr){
                        Some(d) if d.len>0 => { if cur.is_empty(){fstart=addr;} let mn=d.mnemonic.clone(); pos+=d.len; let term=mn.starts_with("ret")||matches!(mn.as_str(),"int3"|"ud2"|"hlt"|"jmp"); cur.push(d); if term{flush(&mut cur,fstart,&mut b);} }
                        _ => { flush(&mut cur,fstart,&mut b); pos+=1; }
                    }
                }
                flush(&mut cur,fstart,&mut b);
            }
            println!("{} B-finding(s): a fork held open across a commit or return.", b.len());
            println!("Inspect each; B is a candidate shape, not a proof.");
            for (a,word) in &b { println!("  0x{:x}  {}", a, word); }
            std::process::exit(0);
        }
        Some("circuit") => {
            if let Err(error) = circuit_cli::run(&args[1..]) {
                eprintln!("{error}");
                1
            } else { 0 }
        }
        Some("run") => {
            // vox run <file> [--argv a,b]            — run it: real process, real
            //                                           argv/envp/auxv stack, real
            //                                           syscalls, from its own entry.
            // vox run <symbol> <file> [--args 1,2]   — call one function directly:
            //                                           scalar int args in, one
            //                                           int back, no process at all.
            // A single bare token is the FILE, not the symbol — with no name given
            // there is no function to call, so the whole file runs as a process.
            // This is also the only thing a PE binary offers, since the loader
            // never populates a symbol table for PE at all (only ELF has one).
            let mut bare: Vec<String> = Vec::new();
            let mut argv_ints: Vec<i64> = Vec::new();
            let mut argv_strs: Vec<String> = Vec::new();
            let mut i=1;
            while i < args.len() {
                match args[i].as_str() {
                    "--args" => { i+=1; if i<args.len() { for a in args[i].split(',') { let a=a.trim(); if !a.is_empty() {
                        let v = if let Some(h)=a.strip_prefix("0x") { i64::from_str_radix(h,16).unwrap_or(0) } else { a.parse().unwrap_or(0) }; argv_ints.push(v);} } } }
                    "--argv" => { i+=1; if i<args.len() { for a in args[i].split(',') { argv_strs.push(a.to_string()); } } }
                    other => bare.push(other.to_string()),
                }
                i+=1;
            }
            let (sym, file): (String, String) = match bare.len() {
                0 => { eprintln!("vox run <file> [--argv a,b]   or   vox run <symbol> <file> [--args 1,2]"); return; }
                1 => (String::new(), bare[0].clone()),
                _ => (bare[0].clone(), bare[1..].join(" ")),
            };
            if file.is_empty() { eprintln!("vox run <file> [--argv a,b]   or   vox run <symbol> <file> [--args 1,2]"); return; }
            // A `.imasm` file is a saved module: text, already carrying its own
            // symbol table (`; sym NAME 0xADDR`), so it runs directly with no
            // second read of the original binary. Anything else is read as raw
            // bytes and lifted fresh, same as before.
            let raw = read_or_exit(&file);
            let is_glyph = file.ends_with(".glyphs") || raw.starts_with(::vox::glyph_module::PREFIX.as_bytes());
            let is_module = is_glyph || file.ends_with(".imasm")
                || raw.starts_with(b"; ");
            let mut m = if is_module {
                let text = match String::from_utf8(raw) {
                    Ok(t) => t,
                    Err(e) => { eprintln!("cannot read {}: {}", file, e); std::process::exit(1); }
                };
                let text = if is_glyph {
                    match ::vox::glyph_module::decode(&text) {
                        Ok(module) => module,
                        Err(error) => { eprintln!("invalid glyph module: {error}"); std::process::exit(2); }
                    }
                } else { text };
                imasm_vm::Machine::new(&text)
            } else {
                imasm_vm::Machine::new(&imasm_module::emit(&raw))
            };
            m.set_host(Box::new(StdHost::new()));
            let trace = std::env::var("VOX_TRACE").is_ok();
            if trace { m.trace_allocs(); }
            if let Ok(w) = std::env::var("VOX_WMEM") {
                m.wmem = u64::from_str_radix(w.trim().trim_start_matches("0x"), 16).unwrap_or(0);
            }
            if let Ok(r) = std::env::var("VOX_RANGE") {
                let p: Vec<&str> = r.split(',').collect();
                if p.len() == 2 {
                    m.trace_lo = u64::from_str_radix(p[0].trim_start_matches("0x"), 16).unwrap_or(0);
                    m.trace_hi = u64::from_str_radix(p[1].trim_start_matches("0x"), 16).unwrap_or(0);
                }
            }
            if sym.is_empty() {
                let mut argv = vec![file.clone()];
                argv.extend(argv_strs);
                let r = m.run_process(&argv, &[], 5_000_000_000);
                if let Err(imasm_vm::Stop::Halt(_)) = &r {
                    eprint!("regs at halt:");
                    for rn in ["rax","rbx","rcx","rdx","rsi","rdi","rbp","rsp","r12","r13","r14","r15"] {
                        eprint!(" {}={:x}", rn, m.reg(rn) as u64);
                    }
                    eprintln!();
                    // Walk saved rbp frames: each frame is [saved rbp][return addr].
                    let mut fp = m.reg("rbp") as u64;
                    eprint!("call chain:");
                    for _ in 0..12 {
                        if fp == 0 || fp < 0x1000 { break; }
                        let ret = { let b = m.peek(fp + 8, 8); (0..8).fold(0u64, |a,i| a | ((b[i] as u64) << (8*i))) };
                        let nfp = { let b = m.peek(fp, 8); (0..8).fold(0u64, |a,i| a | ((b[i] as u64) << (8*i))) };
                        eprint!(" {:x}", ret);
                        if nfp <= fp { break; }
                        fp = nfp;
                    }
                    eprintln!();
                }
                if trace || m.wmem != 0 { for line in &m.syslog { eprintln!("{}", line); } }
                if trace {
                    for rn in ["rax","rbx","rcx","rdx","rsi","rdi","rbp"] {
                        let a = m.reg(rn) as u64;
                        eprintln!("{} = {:x}  mem[{:x}..] = {:02x?}", rn, a, a, m.peek(a.wrapping_sub(4), 16));
                    }
                }
                match r {
                    Ok(()) => println!("entry(...) ran off the end with no exit call   [{} steps]", m.steps),
                    Err(imasm_vm::Stop::SysExit(c)) => println!("entry(...) exited({})   [{} steps in the twelve]", c, m.steps),
                    Err(imasm_vm::Stop::Halt(e)) => println!("entry(...) halted: {}   [{} steps]", e, m.steps),
                }
            } else {
                let addr = match m.resolve(&sym) {
                    Some(a) => a,
                    None => { eprintln!("no symbol '{}' in {}", sym, file); std::process::exit(1); }
                };
                match m.call(addr, &argv_ints, 50_000_000) {
                    Ok(r) => println!("{}({}) = {}   [{} steps in the twelve]", sym, argv_ints.iter().map(|a|a.to_string()).collect::<Vec<_>>().join(", "), r, m.steps),
                    Err(imasm_vm::Stop::SysExit(c)) => println!("{}(...) called exit({})   [{} steps in the twelve]", sym, c, m.steps),
                    Err(imasm_vm::Stop::Halt(e)) => println!("{}(...) halted: {}   [{} steps]", sym, e, m.steps),
                }
            }
            std::process::exit(0);
        }
        Some("glyphs") | Some("unglyphs") => {
            use std::io::Write;
            if args.len()!=3 { eprintln!("vox glyphs|unglyphs <input> <new-output>"); std::process::exit(2); }
            let source = std::fs::read_to_string(&args[1]).unwrap_or_else(|e| {
                eprintln!("cannot read {}: {e}",args[1]); std::process::exit(1);
            });
            let result = if args[0]=="glyphs" { ::vox::glyph_module::encode(&source) }
                         else { ::vox::glyph_module::decode(&source) };
            let result = result.unwrap_or_else(|e| { eprintln!("{e}"); std::process::exit(2); });
            let mut output = std::fs::OpenOptions::new().write(true).create_new(true).open(&args[2])
                .unwrap_or_else(|e| { eprintln!("cannot create {}: {e}",args[2]); std::process::exit(1); });
            output.write_all(result.as_bytes()).unwrap_or_else(|e| {
                eprintln!("cannot write {}: {e}",args[2]); std::process::exit(1);
            });
            println!("saved {} ({} glyphs/characters)",args[2],result.chars().count());
            std::process::exit(0);
        }
        Some("imasm") => { if args.len()<2 { eprintln!("vox imasm <file>"); return; }
            let raw=read_or_exit(&args[1]);
            let module = imasm_module::emit(&raw);
            print!("{}", module);
            let out_path = format!("{}.imasm", args[1]);
            match std::fs::write(&out_path, &module) {
                Ok(()) => eprintln!("saved {}", out_path),
                Err(e) => eprintln!("could not save {}: {}", out_path, e),
            }
            std::process::exit(0); }
        Some("word") | Some("words") => { if args.len()<2 { eprintln!("vox word <file>"); return; }
            let raw=read_or_exit(&args[1]); println!("{}", imasm_module::words(&raw)); std::process::exit(0); }
        Some("disasm") => {
            if args.len() < 2 { eprintln!("vox disasm <file> [symbol]"); return; }
            let raw = read_or_exit(&args[1]);
            let (entry, segments) = vox::parse_elf(&raw);
            let image = vox_decode::Image { segments };
            let seeds = vox::elf_function_symbols(&raw);
            let w = vox_decode::walk(&image, entry, &seeds);
            for (start, f) in &w.functions {
                if args.len() > 2 { /* filter by address later */ }
                let _ = start;
                for ins in f {
                    if let Some(bytes) = image.bytes_at(ins.address) {
                        if let Some(d) = x86::decode(bytes, ins.address) {
                            let ops: Vec<String> = d.ops.iter().map(|o| o.field()).collect();
                            println!("{:x}	{}	{}", d.addr, d.mnemonic, ops.join(" "));
                        } else {
                            println!("{:x}	??? (undecoded)", ins.address);
                        }
                    }
                }
            }
            std::process::exit(0);
        }
        Some("lift") => { if args.len() < 2 { eprintln!("vox lift <file>"); 1 } else { lift_file(&args[1]) } }
        Some("morphism-factor") => {
            if args.len() != 2 { eprintln!("vox morphism-factor <native-numeral-word>"); 1 }
            else { match ::vox::morphism_factor::factor(&args[1]) { Ok(w) => { println!("{}", w); 0 }, Err(e) => { eprintln!("{}", e); 2 } } }
        }
        Some("verify") => {
            if args.len() != 4 { eprintln!("vox verify <p-word> <q-word> <n-word>"); 1 }
            else { match ::vox::morphism_factor::verify(&args[1], &args[2], &args[3]) { Ok(w) => { println!("{}", w); 0 }, Err(e) => { eprintln!("{}", e); 2 } } }
        }
        Some("sieve") => {
            let parsed: Option<Vec<char>> = if args.len() != 2 {
                None
            } else if args[1].starts_with('⊢') {
                ::vox::morphism_factor::parse_numeral(&args[1]).ok()
            } else {
                ::vox::morphism_factor::decimal_to_tape(&args[1])
            };
            match parsed {
                Some(n) => { println!("{}", ::vox::sieve::repl_sieve(&n)); 0 }
                None => { eprintln!("vox sieve <N-word|decimal>   Dixon/QS for the HARD shape"); 1 }
            }
        }
        Some("factor") => {
            let parsed: Option<Vec<char>> = if args.len() != 2 {
                None
            } else if args[1].starts_with('⊢') {
                ::vox::morphism_factor::parse_numeral(&args[1]).ok()
            } else {
                ::vox::morphism_factor::decimal_to_tape(&args[1])
            };
            match parsed {
                Some(n) => { println!("{}", ::vox::morphism_factor::repl_smart_factor(&n)); 0 }
                None => { eprintln!("vox factor <N-word|decimal>   shape-routed full factorization"); 1 }
            }
        }
        Some("perfect") => {
            // Build the depth-n perfect membrane and read its closure with the
            // auditor: T means mu∘delta = id holds by the matched circuitry.
            let n = args.get(1).and_then(|a| a.parse::<usize>().ok()).unwrap_or(2);
            let (v, surplus, w) = ::vox::perfect_membrane::report(n);
            println!("depth {n}: verdict {v}  fork/fuse surplus {surplus}");
            println!("{}", ::vox::vox::glyphs(&w));
            0
        }
        Some("operculum") => {
            // Walk the vessel lifecycle: open, deposit, seal, run, extract.
            let n = args.get(2).and_then(|a| a.parse::<usize>().ok()).unwrap_or(3);
            match args.get(1).and_then(|a| ::vox::morphism_factor::decimal_to_tape(a)) {
                Some(v) => { print!("{}", ::vox::perfect_membrane::operculum_demo(&v, n)); 0 }
                None => { eprintln!("vox operculum <decimal> [depth]   load a value through the membrane's one lid"); 1 }
            }
        }
        Some("numeral") => {
            // Encode a decimal to its IMASM numeral word. This is the
            // pre-compilation step: the decimal is consumed here, and the word it
            // prints is baked into a program so the run itself takes no input.
            match args.get(1).and_then(|a| ::vox::morphism_factor::decimal_to_tape(a)) {
                Some(n) => { println!("{}", ::vox::morphism_factor::emit_numeral(&n)); 0 }
                None => { eprintln!("vox numeral <decimal>   emit the IMASM numeral word for N"); 1 }
            }
        }
        Some("mpqs") => {
            let parsed: Option<Vec<char>> = if args.len() < 2 {
                None
            } else {
                ::vox::morphism_factor::decimal_to_tape(&args[1])
            };
            match parsed {
                Some(n) => {
                    let (bound0, _m) = ::vox::sieve::sieve_params(&n);
                    let mh = args.get(2).and_then(|a| a.parse::<usize>().ok()).unwrap_or(32_768);
                    let bound = args.get(3).and_then(|a| a.parse::<usize>().ok()).unwrap_or(bound0);
                    match ::vox::sieve::mpqs(&n, bound, mh, 32) {
                        Some(f) => { println!("{} factor {}", ::vox::morphism_factor::dec_of(&n), ::vox::morphism_factor::dec_of(&f)); 0 }
                        None => { println!("mpqs: no factor (relations short of a dependency)"); 0 }
                    }
                }
                None => { eprintln!("vox mpqs <decimal N>   multiple-polynomial quadratic sieve arm, in isolation"); 1 }
            }
        }
        Some("scout") => {
            let parsed: Option<Vec<char>> = if args.len() != 2 {
                None
            } else if args[1].starts_with('⊢') {
                ::vox::morphism_factor::parse_numeral(&args[1]).ok()
            } else {
                ::vox::morphism_factor::decimal_to_tape(&args[1])
            };
            match parsed {
                Some(n) => { println!("{}", ::vox::morphism_factor::repl_scout(&n)); 0 }
                None => { eprintln!("vox scout <N-word|decimal>   read the shape of N and hand the factor"); 1 }
            }
        }
        Some("factor-operator") => {
            let rest: Vec<&str> = args[1..].iter().map(|x| x.as_str()).collect();
            println!("{}", ::vox::factor_operator::repl_factor_operator(&rest)); 0
        }
        Some("construct-carrier") => {
            if args.len() != 2 { eprintln!("vox construct-carrier <operator-word>   decompose a word into its factoring-morphism tower"); 1 }
            else { match ::vox::morphism_factor::construct_carrier(&args[1]) {
                Ok(tower) => {
                    let names: Vec<&str> = tower.iter().map(|t| ::vox::morphism_factor::morphism_name(t)).collect();
                    println!("tower ({} morphisms): {}", names.len(), names.join(" -> "));
                    0
                }
                Err(e) => { eprintln!("{}", e); 2 }
            } }
        }
        Some("factor-with") => {
            if args.len() != 3 { eprintln!("vox factor-with <operator-word> <n-word>   factor N on a carrier built from the operator word"); 1 }
            else { match ::vox::morphism_factor::factor_with(&args[1], &args[2]) { Ok(w) => { println!("{}", w); 0 }, Err(e) => { eprintln!("{}", e); 2 } } }
        }
        Some("membrane") | Some("divisor-membrane") | Some("divisor_membrane") => {
            let rest: Vec<&str> = args[1..].iter().map(|x| x.as_str()).collect();
            println!("{}", divisor_membrane::repl_divisor_membrane(&rest)); 0
        }
        Some("safetensors") | Some("safetensor") => {
            if args.len() < 2 { eprintln!("vox safetensors <file.safetensors>"); 1 }
            else {
                let raw = read_or_exit(&args[1]);
                match safetensors::lift_file(&raw) {
                    Ok(words) => {
                        for (name, w) in &words {
                            let v = vox::verdict(w);
                            println!("{:<20} {}  {}", name, v, vox::glyphs(w));
                        }
                        0
                    }
                    Err(e) => { eprintln!("safetensors: {}", e); 1 }
                }
            }
        }
        Some(flag) if flag.starts_with('-') => { eprintln!("vox: unknown option {}\n", flag); usage(); 2 }
        Some(path) => lift_file(path),
    };
    std::process::exit(code);
}
