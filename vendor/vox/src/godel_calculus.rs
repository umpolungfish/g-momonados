//! Executable arithmetic readings for the twelve-glyph IMASM alphabet.
//!
//! Vendored from Vox's arithmetic layer so g-mOMonadOS uses the same glyph
//! constants and the same structural numeral families as the standalone tool.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

use crate::vox::{
    AFWD, AREV, CLINK, ENGAGR, EVALF, EVALT, FFUSE, FSPLIT, IFIX, IMSCRIB,
    TANCH, VINIT,
};

pub const CELL_3: &str = "⊢≻⋈∈⊥∋≻⋈∈⊥∋⊙⊡⊣";
pub const CELL_7: &str = "⊢≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋⊙⊡⊣";
pub const CELL_10: &str = "⊢≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋⊙⊡⊣";
pub const CELL_21: &str = "⊢≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋⊙⊡⊣";

pub const D1: &str = "⊢⊤≻⋈≺⊙⊡⊣";
pub const A: &str = "⊢⊤≻⋈⊥≺⊙⊡⊣";
pub const D2: &str = "⊢⊤≻⋈≺⊞⊙⊡⊣";
pub const B: &str = "⊢⊤≻⋈⊥≺⊞⊙⊡⊣";
pub const C: &str = "⊢∈≻⋈⊥≺⋈∋⊙⊡⊣";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    CellBinary,
    AffineEdit,
    ProductFusion,
}

impl Family {
    pub fn label(self) -> &'static str {
        match self {
            Self::CellBinary => "cell-binary",
            Self::AffineEdit => "affine-edit",
            Self::ProductFusion => "product-fusion",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Structure {
    CellBinary { bits_le: Vec<bool> },
    AffineEdit { unit: bool, branch: bool },
    ProductFusion,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reading {
    pub value: u128,
    pub family: Family,
    pub structure: Structure,
}

impl Reading {
    pub fn binary(&self) -> String { binary_string(self.value) }
    pub fn support(&self) -> Vec<usize> { bit_support(self.value) }
    pub fn polynomial(&self) -> String { polynomial_string(self.value) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operator { Add, Mul }

impl Operator {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "+" | "add" | "plus" => Some(Self::Add),
            "*" | "x" | "mul" | "times" => Some(Self::Mul),
            _ => None,
        }
    }
    pub fn glyph(self) -> char { if self == Self::Add { '+' } else { '*' } }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EquationCheck {
    pub lhs: u128,
    pub rhs: u128,
    pub out: u128,
    pub expected: u128,
    pub operator: Operator,
    pub valid: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InsertionRelation {
    pub glyph: char,
    pub index: usize,
    pub delta: u128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    Empty,
    InvalidGlyph(char),
    Unrecognized,
    Overflow,
    ArithmeticOverflow,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("empty glyph word"),
            Self::InvalidGlyph(c) => write!(f, "not an IMASM glyph: {c}"),
            Self::Unrecognized => f.write_str("well-formed glyph alphabet, but no registered numeral family matches"),
            Self::Overflow => f.write_str("numeral exceeds u128"),
            Self::ArithmeticOverflow => f.write_str("arithmetic result exceeds u128"),
        }
    }
}

fn is_glyph(c: char) -> bool {
    matches!(c, VINIT | TANCH | AFWD | AREV | FSPLIT | FFUSE | IMSCRIB | IFIX | CLINK | EVALT | EVALF | ENGAGR)
}

fn validate(word: &str) -> Result<Vec<char>, DecodeError> {
    if word.is_empty() { return Err(DecodeError::Empty); }
    let chars: Vec<char> = word.chars().collect();
    for &c in &chars {
        if !is_glyph(c) { return Err(DecodeError::InvalidGlyph(c)); }
    }
    Ok(chars)
}

fn decode_cell(chars: &[char]) -> Result<Option<Reading>, DecodeError> {
    if chars.len() < 9 || chars[0] != VINIT { return Ok(None); }
    let n = chars.len();
    if chars[n - 3] != IMSCRIB || chars[n - 2] != IFIX || chars[n - 1] != TANCH { return Ok(None); }
    let body_len = n - 4;
    if body_len == 0 || body_len % 5 != 0 { return Ok(None); }
    let cells = body_len / 5;
    if cells > 128 { return Err(DecodeError::Overflow); }

    let mut bits = Vec::with_capacity(cells);
    let mut value = 0u128;
    for bit in 0..cells {
        let i = 1 + bit * 5;
        if chars[i] != AFWD || chars[i + 1] != CLINK || chars[i + 2] != FSPLIT || chars[i + 4] != FFUSE {
            return Ok(None);
        }
        let one = match chars[i + 3] {
            EVALF => true,
            EVALT => false,
            _ => return Ok(None),
        };
        bits.push(one);
        if one { value |= 1u128 << bit; }
    }
    Ok(Some(Reading { value, family: Family::CellBinary, structure: Structure::CellBinary { bits_le: bits } }))
}

fn decode_affine(chars: &[char]) -> Option<Reading> {
    if chars.len() < 8 || chars[0] != VINIT || chars[1] != EVALT || chars[2] != AFWD || chars[3] != CLINK {
        return None;
    }
    let mut i = 4usize;
    let unit = if chars.get(i) == Some(&EVALF) { i += 1; true } else { false };
    if chars.get(i) != Some(&AREV) { return None; }
    i += 1;
    let branch = if chars.get(i) == Some(&ENGAGR) { i += 1; true } else { false };
    if chars.get(i) != Some(&IMSCRIB)
        || chars.get(i + 1) != Some(&IFIX)
        || chars.get(i + 2) != Some(&TANCH)
        || i + 3 != chars.len()
    {
        return None;
    }
    let value = 2u128 + u128::from(unit) + 2u128 * u128::from(branch);
    Some(Reading { value, family: Family::AffineEdit, structure: Structure::AffineEdit { unit, branch } })
}

fn decode_fusion(chars: &[char]) -> Option<Reading> {
    let pattern = [VINIT, FSPLIT, AFWD, CLINK, EVALF, AREV, CLINK, FFUSE, IMSCRIB, IFIX, TANCH];
    if chars == pattern.as_slice() {
        Some(Reading { value: 8, family: Family::ProductFusion, structure: Structure::ProductFusion })
    } else {
        None
    }
}

pub fn decode(word: &str) -> Result<Reading, DecodeError> {
    let chars = validate(word)?;
    if let Some(r) = decode_cell(&chars)? { return Ok(r); }
    if let Some(r) = decode_affine(&chars) { return Ok(r); }
    if let Some(r) = decode_fusion(&chars) { return Ok(r); }
    Err(DecodeError::Unrecognized)
}

pub fn encode_cell_binary(mut value: u128) -> String {
    let mut out = String::new();
    out.push(VINIT);
    if value == 0 {
        push_cell(&mut out, false);
    } else {
        while value != 0 {
            push_cell(&mut out, value & 1 == 1);
            value >>= 1;
        }
    }
    out.push(IMSCRIB);
    out.push(IFIX);
    out.push(TANCH);
    out
}

fn push_cell(out: &mut String, one: bool) {
    out.push(AFWD);
    out.push(CLINK);
    out.push(FSPLIT);
    out.push(if one { EVALF } else { EVALT });
    out.push(FFUSE);
}

pub fn check(lhs: &str, operator: Operator, rhs: &str, out: &str) -> Result<EquationCheck, DecodeError> {
    let lhs = decode(lhs)?.value;
    let rhs = decode(rhs)?.value;
    let out = decode(out)?.value;
    let expected = match operator {
        Operator::Add => lhs.checked_add(rhs),
        Operator::Mul => lhs.checked_mul(rhs),
    }.ok_or(DecodeError::ArithmeticOverflow)?;
    Ok(EquationCheck { lhs, rhs, out, expected, operator, valid: expected == out })
}

pub fn insertion_relation(from: &str, to: &str) -> Result<Option<InsertionRelation>, DecodeError> {
    let from_chars = validate(from)?;
    let to_chars = validate(to)?;
    if to_chars.len() != from_chars.len() + 1 { return Ok(None); }
    let mut i = 0usize;
    while i < from_chars.len() && from_chars[i] == to_chars[i] { i += 1; }
    if from_chars[i..] != to_chars[i + 1..] { return Ok(None); }
    let a = decode(from)?.value;
    let b = decode(to)?.value;
    if b < a { return Ok(None); }
    Ok(Some(InsertionRelation { glyph: to_chars[i], index: i, delta: b - a }))
}

pub fn binary_string(value: u128) -> String {
    if value == 0 { return "0".to_string(); }
    let top = 127usize - value.leading_zeros() as usize;
    let mut out = String::with_capacity(top + 1);
    for i in (0..=top).rev() { out.push(if (value >> i) & 1 == 1 { '1' } else { '0' }); }
    out
}

pub fn bit_support(mut value: u128) -> Vec<usize> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while value != 0 {
        if value & 1 == 1 { out.push(i); }
        value >>= 1;
        i += 1;
    }
    out
}

pub fn polynomial_string(value: u128) -> String {
    let support = bit_support(value);
    if support.is_empty() { return "0".to_string(); }
    let mut out = String::new();
    for (n, power) in support.iter().enumerate() {
        if n != 0 { out.push_str(" + "); }
        match *power {
            0 => out.push('1'),
            1 => out.push('x'),
            p => out.push_str(&format!("x^{p}")),
        }
    }
    out
}

fn support_string(support: &[usize]) -> String {
    let mut out = String::from("{");
    for (i, p) in support.iter().enumerate() {
        if i != 0 { out.push(','); }
        out.push_str(&p.to_string());
    }
    out.push('}');
    out
}

pub fn render(reading: &Reading) -> String {
    let mut out = format!(
        "family     {}\nvalue      {}\nbinary     {}\nsupport    {}\npolynomial {}\n",
        reading.family.label(), reading.value, reading.binary(), support_string(&reading.support()), reading.polynomial(),
    );
    match &reading.structure {
        Structure::CellBinary { bits_le } => {
            let mut bits = String::with_capacity(bits_le.len());
            for &b in bits_le { bits.push(if b { '1' } else { '0' }); }
            out.push_str(&format!("bits-le    {bits}\nrule       ⊥=1  ⊤=0  leftmost=2^0\n"));
        }
        Structure::AffineEdit { unit, branch } => {
            out.push_str(&format!(
                "base       2\nunit ⊥     {} (+1)\nbranch ⊞   {} (+2)\n",
                if *unit { "present" } else { "absent" },
                if *branch { "present" } else { "absent" },
            ));
        }
        Structure::ProductFusion => out.push_str("fusion     3+5 = 2*4 = 8\n"),
    }
    out
}

pub fn help() -> &'static str {
    "godel — executable readings of the IMASM glyph calculus\n\
     \n\
     godel decode <word>\n\
     godel encode <u128>\n\
     godel check add|mul <lhs-word> <rhs-word> <out-word>\n\
     godel relation <from-word> <to-word>\n\
     godel selftest\n"
}

pub fn command(args: &[&str]) -> Result<String, String> {
    match args.first().copied().unwrap_or("help") {
        "help" | "-h" | "--help" => Ok(help().to_string()),
        "decode" => {
            let word = args.get(1).ok_or_else(|| "godel decode <word>".to_string())?;
            let r = decode(word).map_err(|e| e.to_string())?;
            Ok(format!("word       {word}\n{}", render(&r)))
        }
        "encode" => {
            let raw = args.get(1).ok_or_else(|| "godel encode <u128>".to_string())?;
            let n = raw.parse::<u128>().map_err(|_| format!("not a u128: {raw}"))?;
            let word = encode_cell_binary(n);
            Ok(format!("value      {n}\nword       {word}\n{}", render(&decode(&word).map_err(|e| e.to_string())?)))
        }
        "check" => {
            if args.len() != 5 { return Err("godel check add|mul <lhs-word> <rhs-word> <out-word>".to_string()); }
            let op = Operator::parse(args[1]).ok_or_else(|| format!("unknown operator: {}", args[1]))?;
            let c = check(args[2], op, args[3], args[4]).map_err(|e| e.to_string())?;
            Ok(format!(
                "equation   {} {} {} = {}\nexpected   {}\nstatus     {}\n",
                c.lhs, c.operator.glyph(), c.rhs, c.out, c.expected, if c.valid { "PASS" } else { "FAIL" },
            ))
        }
        "relation" => {
            if args.len() != 3 { return Err("godel relation <from-word> <to-word>".to_string()); }
            match insertion_relation(args[1], args[2]).map_err(|e| e.to_string())? {
                Some(r) => Ok(format!("relation   insert {} at glyph index {}\ndelta      +{}\n", r.glyph, r.index, r.delta)),
                None => Ok("relation   not a one-glyph insertion between registered numeral forms\n".to_string()),
            }
        }
        "selftest" | "verify" => selftest_report(),
        other => Err(format!("unknown godel command: {other}\n{}", help())),
    }
}

pub fn selftest_report() -> Result<String, String> {
    let checks = [
        ("cell-add", CELL_3, Operator::Add, CELL_7, CELL_10),
        ("cell-mul", CELL_3, Operator::Mul, CELL_7, CELL_21),
        ("edit-add", A, Operator::Add, B, C),
        ("edit-mul", D1, Operator::Mul, D2, C),
    ];
    let mut out = String::new();
    let mut ok = true;
    for (name, lhs, op, rhs, result) in checks {
        let c = check(lhs, op, rhs, result).map_err(|e| e.to_string())?;
        ok &= c.valid;
        out.push_str(&format!("{name:<9} {} {} {} = {}  {}\n", c.lhs, c.operator.glyph(), c.rhs, c.out, if c.valid { "PASS" } else { "FAIL" }));
    }
    for (name, from, to, glyph, delta) in [
        ("unit-2→3", D1, A, EVALF, 1u128),
        ("unit-4→5", D2, B, EVALF, 1u128),
        ("branch-2→4", D1, D2, ENGAGR, 2u128),
        ("branch-3→5", A, B, ENGAGR, 2u128),
    ] {
        let relation = insertion_relation(from, to).map_err(|e| e.to_string())?;
        let pass = relation.as_ref().map(|r| r.glyph == glyph && r.delta == delta).unwrap_or(false);
        ok &= pass;
        out.push_str(&format!("{name:<9} insert {glyph} => +{delta}  {}\n", if pass { "PASS" } else { "FAIL" }));
    }
    if ok { Ok(out) } else { Err(out) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equations_hold() {
        assert!(check(CELL_3, Operator::Add, CELL_7, CELL_10).unwrap().valid);
        assert!(check(CELL_3, Operator::Mul, CELL_7, CELL_21).unwrap().valid);
        assert!(check(A, Operator::Add, B, C).unwrap().valid);
        assert!(check(D1, Operator::Mul, D2, C).unwrap().valid);
    }

    #[test]
    fn edit_relations_hold() {
        assert_eq!(insertion_relation(D1, A).unwrap().unwrap().delta, 1);
        assert_eq!(insertion_relation(D1, D2).unwrap().unwrap().delta, 2);
    }

    #[test]
    fn cell_roundtrip() {
        for n in [0u128, 1, 2, 3, 5, 7, 8, 10, 21, 255, 1024, u64::MAX as u128] {
            assert_eq!(decode(&encode_cell_binary(n)).unwrap().value, n);
        }
    }
}
