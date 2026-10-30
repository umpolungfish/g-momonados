//! Versioned, lossless glyph-only serialization of an executable IMASM module.
//! Work glyphs remain record heads. Nested numeral payloads carry exact UTF-8
//! lines, including operands and metadata; this is not a structural projection.
use alloc::{format, string::String, vec::Vec};

const ALPHABET: &str = "⊢⊣⊤⊥⊡≻≺∈∋⋈⊙⊞";
pub const PREFIX: &str = "⊢∈⊙∈⊢∈";
const VERSION: &str = "VOXGLYPH1";

fn checksum(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x100000001b3))
}

fn head(line: &str) -> char {
    line.chars().next().filter(|&c| ALPHABET.contains(c)).unwrap_or('⋈')
}

// Fixed-width UTF-8 bytes, LSB first, followed by a high one-bit sentinel.
// The sentinel preserves leading zero bytes as a numeral; its framing is the
// existing single-frame numeral form, with the live reversal inside the fuse.
fn record(out: &mut String, glyph: char, bytes: &[u8]) {
    out.push(glyph);
    out.push_str("∈⊢∈");
    for &byte in bytes {
        for bit in 0..8 { out.push(if byte & (1 << bit) == 0 {'⊤'} else {'⊥'}); }
    }
    out.push_str("⊥≺∋⊡⊣∋");
}

pub fn encode(module: &str) -> Result<String, String> {
    if !module.starts_with("; ⊙ module (") {
        return Err("expected a complete executable IMASM module".into());
    }
    let mut out = String::from("⊢∈");
    record(&mut out, '⊙', format!("{VERSION}:{}:{:016x}", module.len(), checksum(module.as_bytes())).as_bytes());
    for line in module.split_inclusive('\n') { record(&mut out, head(line), line.as_bytes()); }
    out.push_str("∋⊡⊣");
    Ok(out)
}

fn expect(chars: &mut core::str::Chars<'_>, expected: &str) -> Result<(), String> {
    for c in expected.chars() {
        if chars.next() != Some(c) { return Err(format!("broken glyph-module framing; expected {c}")); }
    }
    Ok(())
}

fn payload(chars: &mut core::str::Chars<'_>) -> Result<Vec<u8>, String> {
    expect(chars,"∈⊢∈")?;
    let mut bytes = Vec::new();
    let mut byte = 0u8;
    let mut bits = 0;
    loop {
        match chars.next() {
            Some(c @ ('⊤'|'⊥')) => {
                if c == '⊥' { byte |= 1 << bits; }
                bits += 1;
                if bits == 8 { bytes.push(byte); byte=0; bits=0; }
            }
            Some('≺') if bits == 1 && byte == 1 => break,
            _ => return Err("invalid byte payload or missing high-bit sentinel".into()),
        }
    }
    expect(chars,"∋⊡⊣∋")?;
    Ok(bytes)
}

pub fn decode(word: &str) -> Result<String, String> {
    let mut chars = word.chars();
    expect(&mut chars,"⊢∈⊙")?;
    let header = String::from_utf8(payload(&mut chars)?).map_err(|_| "invalid header UTF-8")?;
    let fields: Vec<_> = header.split(':').collect();
    if fields.len()!=3 || fields[0]!=VERSION { return Err("unsupported glyph-module version".into()); }
    let expected_len = fields[1].parse::<usize>().map_err(|_| "invalid module byte length")?;
    let expected_hash = u64::from_str_radix(fields[2],16).map_err(|_| "invalid module checksum")?;
    // Do not allocate from an untrusted declared length.
    let mut module = String::new();
    loop {
        let glyph = chars.next().ok_or("truncated glyph module")?;
        if glyph == '∋' {
            expect(&mut chars,"⊡⊣")?;
            if chars.next().is_some() { return Err("trailing content after glyph module".into()); }
            break;
        }
        if !ALPHABET.contains(glyph) { return Err("non-IMASM glyph in module".into()); }
        let line = String::from_utf8(payload(&mut chars)?).map_err(|_| "invalid payload UTF-8")?;
        if line.is_empty() || head(&line)!=glyph { return Err("record head does not match instruction glyph".into()); }
        if module.len().checked_add(line.len()).ok_or("module length overflow")? > expected_len {
            return Err("module exceeds declared byte length".into());
        }
        module.push_str(&line);
    }
    if module.len()!=expected_len || checksum(module.as_bytes())!=expected_hash {
        return Err("glyph module length/checksum mismatch".into());
    }
    if !module.starts_with("; ⊙ module (") { return Err("payload is not an executable IMASM module".into()); }
    Ok(module)
}

#[cfg(test)]
mod tests {
    use super::*;
    const MODULE: &str = "; ⊙ module (elf x86-64)\n; entry 0x1000\n; bits 64\n; sym answer 0x1000\n@0x1000\n⋈\tmov\tr:eax\ti:0x2a\n@0x1005\n⊣\tret\n";

    #[test]
    fn lossless_glyph_only_roundtrip() {
        for module in [MODULE, MODULE.trim_end_matches('\n')] {
            let word = encode(module).unwrap();
            assert!(word.chars().all(|c| ALPHABET.contains(c)));
            assert_eq!(decode(&word).unwrap(),module);
        }
        assert!(encode("⊢⊙⊡⊣").is_err());
    }

    #[test]
    fn nested_byte_payload_preserves_all_bytes() {
        let all: Vec<u8> = (0..=255).collect();
        let mut word = String::new(); record(&mut word,'⋈',&all);
        let mut chars=word.chars(); assert_eq!(chars.next(),Some('⋈'));
        assert_eq!(payload(&mut chars).unwrap(),all); assert_eq!(chars.next(),None);
    }

    #[test]
    fn malformed_words_fail_closed() {
        let word = encode(MODULE).unwrap();
        for bad in [String::new(),format!("{word}⊤"),word[..word.len()-3].into(),
            word.replacen('≺',"⊙",1),word.replacen("⊢∈⊙","⊢∈⋈",1)] {
            assert!(decode(&bad).is_err());
        }
        // Change a body payload bit without changing its length or framing.
        let mut bytes = word.clone();
        let pos = bytes.rfind('⊤').unwrap(); bytes.replace_range(pos..pos+3,"⊥");
        assert!(decode(&bytes).is_err());
    }

    #[test]
    fn execution_and_symbols_survive_roundtrip() {
        let decoded = decode(&encode(MODULE).unwrap()).unwrap();
        let mut m = crate::imasm_vm::Machine::new(&decoded);
        let address = m.symbols["answer"];
        assert_eq!(m.call(address,&[],100).ok(),Some(42));
    }
}
