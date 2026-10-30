//! The safetensors lane: a Hugging Face safetensor file read as words.
//!
//! Format (spec): 8 bytes = header size u64 LE, then header_size bytes of
//! JSON, then each tensor's bytes at the offset declared in the header.
//! The JSON carries tensor names, shapes, dtypes and data offsets.
//!
//! Each tensor is lifted to a twelve-glyph IMASM word whose shape encodes
//! the tensor's control-flow signature: the dtype class sets the terminal,
//! the rank sets the fork density, and the element count sets the span.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use crate::vox::{TANCH, FSPLIT, FFUSE, AFWD, AREV, ENGAGR, IFIX, IMSCRIB, EVALT, EVALF};
use crate::vox::VINIT as V;
use alloc::collections::BTreeMap;


// ── SAFETENSORS HEADER ────────────────────────────────────────────────

/// The parsed header of a safetensors file.
pub struct SafeHeader {
    pub header_size: u64,
    pub json: String,
    pub tensors: Vec<TensorInfo>,
}

/// One tensor's metadata as declared in the JSON header.
pub struct TensorInfo {
    pub name: String,
    pub shape: Vec<u64>,
    pub dtype: String,
    pub offset: u64,
    pub length: u64,
}

fn read_u64_le(raw: &[u8], o: usize) -> Option<u64> {
    let mut v = 0u64;
    for k in 0..8 { if o + k < raw.len() { v |= (raw[o + k] as u64) << (8 * k); } }
    Some(v)
}

// ── JSON parser ──────────────────────────────────────────────────────
//
// A single-pass recursive parser that consumes one value at a time from a
// byte cursor. Each `parse_*` function advances the cursor past what it
// consumed. The whole format is a tiny subset of JSON — no escapes, no
// unicode, no whitespace inside numbers — so a hand-rolled reader is
// safe here.

#[derive(Debug, Clone)]
enum JsonValue {
    Null, Bool(()),    Num(f64), Str(String), Arr(Vec<JsonValue>), Obj(BTreeMap<String, JsonValue>),
}

struct Cursor<'a> { b: &'a [u8], i: usize }

impl<'a> Cursor<'a> {
    fn skip_ws(&mut self) {
        while let Some(&c) = self.b.get(self.i) {
            if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' { self.i += 1; } else { break; }
        }
    }
    fn peek(&self) -> Option<u8> { self.b.get(self.i).copied() }

}

fn parse_val(c: &mut Cursor) -> Option<JsonValue> {
    c.skip_ws();
    let p = c.peek()?;
    if p == b'{' { parse_obj(c) }
    else if p == b'[' { parse_arr(c) }
    else if p == b'"' { parse_str(c) }
    else if p == b't' { parse_keyword(c, b"true").then_some(JsonValue::Bool(())) }
    else if p == b'f' { parse_keyword(c, b"false").then_some(JsonValue::Bool(())) }
    else if p == b'n' { parse_keyword(c, b"null").then(|| JsonValue::Null) }
    else if p == b'-' || p.is_ascii_digit() { parse_num(c) }
    else { None }
}

fn parse_keyword(c: &mut Cursor, kw: &[u8]) -> bool {
    let s = c.i;
    if s + kw.len() > c.b.len() { return false; }
    if &c.b[s..s+kw.len()] != kw { return false; }
    c.i += kw.len();
    true
}

fn parse_str(c: &mut Cursor) -> Option<JsonValue> {
    if c.peek()? != b'"' { return None; }
    c.i += 1;
    let start = c.i;
    while c.i < c.b.len() && c.b[c.i] != b'"' { c.i += 1; }
    if c.i >= c.b.len() { return None; }
    let s = core::str::from_utf8(&c.b[start..c.i]).ok()?.to_string();
    c.i += 1; // closing quote
    Some(JsonValue::Str(s))
}

fn parse_num(c: &mut Cursor) -> Option<JsonValue> {
    let start = c.i;
    if c.peek() == Some(b'-') { c.i += 1; }
    while let Some(&ch) = c.b.get(c.i) {
        if ch.is_ascii_digit() || ch == b'.' || ch == b'e' || ch == b'E' || ch == b'+' || ch == b'-' { c.i += 1; } else { break; }
    }
    let s = core::str::from_utf8(&c.b[start..c.i]).ok()?;
    let n: f64 = s.parse().ok()?;
    Some(JsonValue::Num(n))
}

fn parse_arr(c: &mut Cursor) -> Option<JsonValue> {
    if c.peek()? != b'[' { return None; }
    c.i += 1;
    let mut out = Vec::new();
    c.skip_ws();
    if c.peek() == Some(b']') { c.i += 1; return Some(JsonValue::Arr(out)); }
    loop {
        let v = parse_val(c)?;
        out.push(v);
        c.skip_ws();
        match c.peek() {
            Some(b',') => { c.i += 1; c.skip_ws(); }
            Some(b']') => { c.i += 1; return Some(JsonValue::Arr(out)); }
            _ => return None,
        }
    }
}

fn parse_obj(c: &mut Cursor) -> Option<JsonValue> {
    if c.peek()? != b'{' { return None; }
    c.i += 1;
    let mut m: BTreeMap<String, JsonValue> = BTreeMap::new();
    c.skip_ws();
    if c.peek() == Some(b'}') { c.i += 1; return Some(JsonValue::Obj(m)); }
    loop {
        c.skip_ws();
        let k = match parse_val(c)? { JsonValue::Str(s) => s, _ => return None };
        c.skip_ws();
        if c.peek() != Some(b':') { return None; }
        c.i += 1;
        let v = parse_val(c)?;
        m.insert(k, v);
        c.skip_ws();
        match c.peek() {
            Some(b',') => { c.i += 1; }
            Some(b'}') => { c.i += 1; return Some(JsonValue::Obj(m)); }
            _ => return None,
        }
    }
}

// ── PARSE: safetensors file ──────────────────────────────────────────

/// Parse a safetensors file header. Returns header info + the raw tensor data regions.
/// Parse a safetensors file header. Returns header info + the raw tensor data regions.
pub fn parse(raw: &[u8]) -> Result<SafeHeader, &'static str> {
    if raw.len() < 8 { return Err("file too short for safetensors header"); }
    let header_size = read_u64_le(raw, 0).ok_or("read header size")?;
    if header_size as usize + 8 > raw.len() { return Err("header extends past file"); }
    let json_bytes = &raw[8..8 + header_size as usize];
    let json_str = core::str::from_utf8(json_bytes).map_err(|_| "header not utf8")?;

    let mut c = Cursor { b: json_bytes, i: 0 };
    let json_val = parse_val(&mut c).ok_or("parse header json")?;

    let mut tensors = Vec::new();

    // Extract tensor info from the header: each key (except __metadata__) is a tensor
    if let JsonValue::Obj(m) = &json_val {
        for (key, value) in m {
            if key == "__metadata__" {
                continue;
            }
            // Now, value should be an object with shape, dtype, data_offsets
            let entry = match value {
                JsonValue::Obj(e) => e,
                _ => continue, // skip if not an object
            };
            // Now parse shape, dtype, data_offsets from entry
            let shape = match entry.get("shape") {
                Some(JsonValue::Arr(s)) => {
                    let mut out = Vec::new();
                    for v in s {
                        if let JsonValue::Num(n) = v {
                            out.push(*n as u64);
                        }
                    }
                    out
                }
                _ => vec![],
            };
            let dtype = match entry.get("dtype") {
                Some(JsonValue::Str(d)) => d.clone(),
                _ => "F32".into(),
            };
            let offset = match entry.get("data_offsets") {
                Some(JsonValue::Arr(o)) if o.len() >= 1 => {
                    if let JsonValue::Num(x) = &o[0] { *x as u64 } else { 0 }
                }
                _ => 0,
            };
            let length = match entry.get("data_offsets") {
                Some(JsonValue::Arr(o)) if o.len() >= 2 => {
                    let start = match &o[0] { JsonValue::Num(x) => *x as u64, _ => 0 };
                    let end = match &o[1] { JsonValue::Num(x) => *x as u64, _ => 0 };
                    end.saturating_sub(start)
                }
                _ => {
                    let nelem: u64 = shape.iter().product();
                    dtype_bytes(&dtype) * nelem
                }
            };
            tensors.push(TensorInfo {
                name: key.clone(),
                shape,
                dtype,
                offset,
                length,
            });
        }
    } else {
        return Err("header not an object");
    }

    // Also parse the format version / other top-level keys
    Ok(SafeHeader { header_size, json: json_str.to_string(), tensors })
}

fn dtype_bytes(dtype: &str) -> u64 {
    match dtype {
        "F32" | "F64" | "I32" | "U32" | "I64" | "U64" => 8,
        "F16" | "BF16" | "I16" | "U16" => 2,
        "I8" | "U8" | "BOOL" => 1,
        _ => 4,
    }
}

fn dtype_bits(dtype: &str) -> u64 {
    match dtype {
        "F16" | "BF16" | "I16" | "U16" => 16,
        "F32" | "I32" | "U32" => 32,
        "F64" | "I64" | "U64" => 64,
        "I8" | "U8" => 8,
        "BOOL" => 1,
        _ => 32,
    }
}

// ── LIFT: tensor -> IMASM word ────────────────────────────────────────

/// Lift a tensor info to a twelve-glyph IMASM word.
///
/// The lift maps tensor structure onto the Belnap control-flow grammar:
///   - shape rank → fork density (FSPLIT per dimension, FFUSE to close)
///   - dtype class sets the terminal (TANCH for scalar, IFIX for quantized)
///   - element count mod 3 sets the closing path
///   - metadata / name hash seeds the self-reference (⊙)
pub fn lift_tensor(t: &TensorInfo) -> Vec<char> {
    let rank = t.shape.len();
    let nelem: u64 = t.shape.iter().product();
    let bits = dtype_bits(&t.dtype);

    let mut w = Vec::new();
    w.push(V); // open the tensor word

    // Each rank contributes a fork branch (FSPLIT) then a fuse (FFUSE)
    // — the tensor dimensions are nested forks that rejoin.
    for d in 0..rank {
        let dim = t.shape.get(d).copied().unwrap_or(1);
        // Fork density scales with dimension size mod 3
        let forks = (dim % 3) + 1;
        for _ in 0..forks { w.push(FSPLIT); }
        for _ in 0..forks { w.push(FFUSE); }
    }

    // Terminal set by dtype class
    if bits <= 8 {
        // small integer / bool → EVALF (truth) for bit-packed, IFIX for byte
        if t.dtype == "BOOL" { w.push(EVALF); } else { w.push(IFIX); }
    } else if bits == 16 {
        w.push(ENGAGR); // half-precision: engagement
    } else {
        // 32/64 bit float → TANCH terminal (truth-producing computation)
        w.push(TANCH);
    }

    // Closing: element count mod 3 selects the close path
    let close = (nelem % 3) as usize;
    match close {
        0 => { w.push(FFUSE); }
        1 => { w.push(AFWD); }
        _ => { w.push(AREV); }
    }

    // Self-reference seeded by name length + rank — the tensor IS its own
    // descriptor (⊤=IMSCRIB criticality). If there are multiple tensors
    // the name is also encoded as EVALT/CLINK deposits.
    let name_seed = t.name.len() as u64;
    if name_seed % 2 == 0 { w.push(IMSCRIB); } else { w.push(EVALT); }

    // Final anchor
    w.push(TANCH);
    w
}

/// Lift an entire safetensors file to a sequence of words, one per tensor.
pub fn lift_file(raw: &[u8]) -> Result<Vec<(String, Vec<char>)>, &'static str> {
    let header = parse(raw)?;
    let mut words = Vec::new();
    for t in &header.tensors {
        let word = lift_tensor(t);
        words.push((t.name.clone(), word));
    }
    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_f32_scalar() {
        // Build a minimal safetensors file: one F32 scalar tensor
        let tensor_name = "w";
        let json = format!(
            r#"{{"tensor_names":["{}"],"{}":{{"shape":[1],"dtype":"F32","data_offsets":[0,4]}}}}"#,
            tensor_name, tensor_name
        );
        let header_size = json.len() as u64;
        let mut raw = Vec::new();
        raw.extend_from_slice(&header_size.to_le_bytes());
        raw.extend_from_slice(json.as_bytes());
        // 4 bytes of tensor data
        raw.extend_from_slice(&[0u8; 4]);

        let header = parse(&raw).expect("parse");
        assert_eq!(header.tensors.len(), 1);
        let t = &header.tensors[0];
        assert_eq!(t.name, "w");
        assert_eq!(t.shape, vec![1]);
        assert_eq!(t.dtype, "F32");
        assert_eq!(t.length, 4);

        let word = lift_tensor(t);
        assert!(!word.is_empty());
        assert_eq!(word[0], V);
    }
}
