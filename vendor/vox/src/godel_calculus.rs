//! Executable arithmetic readings for the twelve-glyph IMASM alphabet.
//!
//! The numeric layer is deliberately not backed by a fixed-width machine
//! integer. `Nat` is an exact, arbitrary-length little-endian bit word stored in
//! `alloc::vec::Vec`; its mathematical range has no compile-time numeric bound.
//! Encoding, decoding, addition, multiplication, subtraction, decimal parsing,
//! and rendering all operate directly on that representation.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cmp::Ordering;
use core::fmt;

use crate::vox::{
    AFWD, AREV, CLINK, ENGAGR, EVALF, EVALT, FFUSE, FSPLIT, IFIX, IMSCRIB, TANCH, VINIT,
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

/// Exact natural number with no fixed-width numeric ceiling.
///
/// The invariant is canonical little-endian binary: the final stored bit is
/// always `true`; zero is the empty vector. The only practical limit is the
/// memory available to hold the finite word being evaluated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Nat {
    bits_le: Vec<bool>,
}

impl Nat {
    pub fn zero() -> Self {
        Self {
            bits_le: Vec::new(),
        }
    }

    pub fn one() -> Self {
        Self {
            bits_le: alloc::vec![true],
        }
    }

    pub fn from_bits_le(mut bits_le: Vec<bool>) -> Self {
        while bits_le.last() == Some(&false) {
            bits_le.pop();
        }
        Self { bits_le }
    }

    pub fn from_u64(mut value: u64) -> Self {
        let mut bits = Vec::new();
        while value != 0 {
            bits.push(value & 1 == 1);
            value >>= 1;
        }
        Self { bits_le: bits }
    }

    pub fn from_decimal(raw: &str) -> Option<Self> {
        if raw.is_empty() {
            return None;
        }
        let mut out = Self::zero();
        for byte in raw.bytes() {
            if !byte.is_ascii_digit() {
                return None;
            }
            out = out.mul_small(10);
            out = out.add(&Self::from_u64((byte - b'0') as u64));
        }
        Some(out)
    }

    pub fn is_zero(&self) -> bool {
        self.bits_le.is_empty()
    }

    pub fn bits_le(&self) -> &[bool] {
        &self.bits_le
    }

    fn mod_small(&self, modulus: usize) -> usize {
        let mut residue = 0usize;
        for &bit in self.bits_le.iter().rev() {
            residue = if residue >= modulus - residue {
                residue - (modulus - residue)
            } else {
                residue + residue
            };
            if bit {
                residue = if residue == modulus - 1 {
                    0
                } else {
                    residue + 1
                };
            }
        }
        residue
    }

    fn bit(&self, position: usize) -> bool {
        self.bits_le.get(position).copied().unwrap_or(false)
    }

    fn shr(&self, places: usize) -> Self {
        Self::from_bits_le(self.bits_le.get(places..).unwrap_or(&[]).to_vec())
    }

    fn mod_nat(&self, modulus: &Self) -> Self {
        let mut remainder = Self::zero();
        for &bit in self.bits_le.iter().rev() {
            remainder = remainder.shl(1);
            if bit {
                remainder = remainder.add(&Self::one());
            }
            if remainder.cmp_nat(modulus) != Ordering::Less {
                remainder = remainder
                    .sub(modulus)
                    .expect("ordered remainder subtraction");
            }
        }
        remainder
    }

    fn cmp_nat(&self, other: &Self) -> Ordering {
        match self.bits_le.len().cmp(&other.bits_le.len()) {
            Ordering::Equal => {
                for i in (0..self.bits_le.len()).rev() {
                    match self.bits_le[i].cmp(&other.bits_le[i]) {
                        Ordering::Equal => {}
                        non_eq => return non_eq,
                    }
                }
                Ordering::Equal
            }
            non_eq => non_eq,
        }
    }

    pub fn add(&self, other: &Self) -> Self {
        let n = core::cmp::max(self.bits_le.len(), other.bits_le.len());
        let mut out = Vec::with_capacity(n + 1);
        let mut carry = false;
        for i in 0..n {
            let a = self.bits_le.get(i).copied().unwrap_or(false) as u8;
            let b = other.bits_le.get(i).copied().unwrap_or(false) as u8;
            let sum = a + b + carry as u8;
            out.push(sum & 1 == 1);
            carry = sum >= 2;
        }
        if carry {
            out.push(true);
        }
        Self::from_bits_le(out)
    }

    pub fn sub(&self, other: &Self) -> Option<Self> {
        if self.cmp_nat(other) == Ordering::Less {
            return None;
        }
        let mut out = Vec::with_capacity(self.bits_le.len());
        let mut borrow = 0i8;
        for i in 0..self.bits_le.len() {
            let a = self.bits_le[i] as i8;
            let b = other.bits_le.get(i).copied().unwrap_or(false) as i8;
            let mut d = a - b - borrow;
            if d < 0 {
                d += 2;
                borrow = 1;
            } else {
                borrow = 0;
            }
            out.push(d == 1);
        }
        Some(Self::from_bits_le(out))
    }

    pub fn shl(&self, places: usize) -> Self {
        if self.is_zero() {
            return Self::zero();
        }
        let mut bits = Vec::with_capacity(self.bits_le.len() + places);
        bits.resize(places, false);
        bits.extend_from_slice(&self.bits_le);
        Self { bits_le: bits }
    }

    pub fn mul(&self, other: &Self) -> Self {
        if self.is_zero() || other.is_zero() {
            return Self::zero();
        }
        let mut out = Self::zero();
        for (i, bit) in other.bits_le.iter().copied().enumerate() {
            if bit {
                out = out.add(&self.shl(i));
            }
        }
        out
    }

    pub fn div_rem(&self, divisor: &Self) -> Option<(Self, Self)> {
        if divisor.is_zero() {
            return None;
        }
        let mut quotient_bits = alloc::vec![false; self.bits_le.len()];
        let mut remainder = Self::zero();
        for position in (0..self.bits_le.len()).rev() {
            remainder = remainder.shl(1);
            if self.bit(position) {
                remainder = remainder.add(&Self::one());
            }
            if remainder.cmp_nat(divisor) != Ordering::Less {
                remainder = remainder
                    .sub(divisor)
                    .expect("ordered remainder subtraction");
                quotient_bits[position] = true;
            }
        }
        Some((Self::from_bits_le(quotient_bits), remainder))
    }

    fn mul_small(&self, factor: u8) -> Self {
        let mut out = Self::zero();
        for _ in 0..factor {
            out = out.add(self);
        }
        out
    }

    pub fn binary_string(&self) -> String {
        if self.is_zero() {
            return "0".to_string();
        }
        let mut out = String::with_capacity(self.bits_le.len());
        for bit in self.bits_le.iter().rev() {
            out.push(if *bit { '1' } else { '0' });
        }
        out
    }

    pub fn decimal_string(&self) -> String {
        if self.is_zero() {
            return "0".to_string();
        }
        let mut digits = alloc::vec![0u8];
        for bit in self.bits_le.iter().rev().copied() {
            let mut carry = if bit { 1u8 } else { 0u8 };
            for digit in &mut digits {
                let v = *digit * 2 + carry;
                *digit = v % 10;
                carry = v / 10;
            }
            if carry != 0 {
                digits.push(carry);
            }
        }
        let mut out = String::with_capacity(digits.len());
        for d in digits.iter().rev() {
            out.push((b'0' + *d) as char);
        }
        out
    }
}

impl fmt::Display for Nat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.decimal_string())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    CellBinary,
    AffineEdit,
    ProductFusion,
}

impl Family {
    pub fn label(self) -> &'static str {
        match self {
            Family::CellBinary => "cell-binary",
            Family::AffineEdit => "affine-edit",
            Family::ProductFusion => "product-fusion",
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
    pub value: Nat,
    pub family: Family,
    pub structure: Structure,
}

impl Reading {
    pub fn binary(&self) -> String {
        self.value.binary_string()
    }

    pub fn support(&self) -> Vec<Nat> {
        bit_support(&self.value)
    }

    pub fn polynomial(&self) -> String {
        polynomial_string(&self.value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operator {
    Add,
    Mul,
}

impl Operator {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "+" | "add" | "plus" => Some(Operator::Add),
            "*" | "x" | "mul" | "times" => Some(Operator::Mul),
            _ => None,
        }
    }

    pub fn glyph(self) -> char {
        match self {
            Operator::Add => '+',
            Operator::Mul => '*',
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EquationCheck {
    pub lhs: Nat,
    pub rhs: Nat,
    pub out: Nat,
    pub expected: Nat,
    pub operator: Operator,
    pub valid: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InsertionRelation {
    pub glyph: char,
    pub position: Nat,
    pub delta: Nat,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    Empty,
    InvalidGlyph(char),
    Unrecognized,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::Empty => f.write_str("empty glyph word"),
            DecodeError::InvalidGlyph(c) => write!(f, "not an IMASM glyph: {c}"),
            DecodeError::Unrecognized => {
                f.write_str("well-formed glyph alphabet, but no registered numeral family matches")
            }
        }
    }
}

fn is_glyph(c: char) -> bool {
    matches!(
        c,
        VINIT
            | TANCH
            | AFWD
            | AREV
            | FSPLIT
            | FFUSE
            | IMSCRIB
            | IFIX
            | CLINK
            | EVALT
            | EVALF
            | ENGAGR
    )
}

fn validate(word: &str) -> Result<Vec<char>, DecodeError> {
    if word.is_empty() {
        return Err(DecodeError::Empty);
    }
    let chars: Vec<char> = word.chars().collect();
    for &c in &chars {
        if !is_glyph(c) {
            return Err(DecodeError::InvalidGlyph(c));
        }
    }
    Ok(chars)
}

fn decode_cell(chars: &[char]) -> Option<Reading> {
    if chars.len() < 9 || chars[0] != VINIT {
        return None;
    }
    let n = chars.len();
    if chars[n - 3] != IMSCRIB || chars[n - 2] != IFIX || chars[n - 1] != TANCH {
        return None;
    }
    let body_len = n - 4;
    if body_len == 0 || body_len % 5 != 0 {
        return None;
    }

    let cells = body_len / 5;
    let mut bits = Vec::with_capacity(cells);
    for bit in 0..cells {
        let i = 1 + bit * 5;
        if chars[i] != AFWD
            || chars[i + 1] != CLINK
            || chars[i + 2] != FSPLIT
            || chars[i + 4] != FFUSE
        {
            return None;
        }
        let one = match chars[i + 3] {
            EVALF => true,
            EVALT => false,
            _ => return None,
        };
        bits.push(one);
    }

    Some(Reading {
        value: Nat::from_bits_le(bits.clone()),
        family: Family::CellBinary,
        structure: Structure::CellBinary { bits_le: bits },
    })
}

fn decode_affine(chars: &[char]) -> Option<Reading> {
    if chars.len() < 8
        || chars[0] != VINIT
        || chars[1] != EVALT
        || chars[2] != AFWD
        || chars[3] != CLINK
    {
        return None;
    }

    let mut i = 4usize;
    let unit = if chars.get(i) == Some(&EVALF) {
        i += 1;
        true
    } else {
        false
    };
    if chars.get(i) != Some(&AREV) {
        return None;
    }
    i += 1;

    let branch = if chars.get(i) == Some(&ENGAGR) {
        i += 1;
        true
    } else {
        false
    };

    if chars.get(i) != Some(&IMSCRIB)
        || chars.get(i + 1) != Some(&IFIX)
        || chars.get(i + 2) != Some(&TANCH)
        || i + 3 != chars.len()
    {
        return None;
    }

    let value = 2u64 + unit as u64 + 2u64 * branch as u64;
    Some(Reading {
        value: Nat::from_u64(value),
        family: Family::AffineEdit,
        structure: Structure::AffineEdit { unit, branch },
    })
}

fn decode_fusion(chars: &[char]) -> Option<Reading> {
    let pattern = [
        VINIT, FSPLIT, AFWD, CLINK, EVALF, AREV, CLINK, FFUSE, IMSCRIB, IFIX, TANCH,
    ];
    if chars == pattern.as_slice() {
        Some(Reading {
            value: Nat::from_u64(8),
            family: Family::ProductFusion,
            structure: Structure::ProductFusion,
        })
    } else {
        None
    }
}

pub fn decode(word: &str) -> Result<Reading, DecodeError> {
    let chars = validate(word)?;
    if let Some(r) = decode_cell(&chars) {
        return Ok(r);
    }
    if let Some(r) = decode_affine(&chars) {
        return Ok(r);
    }
    if let Some(r) = decode_fusion(&chars) {
        return Ok(r);
    }
    Err(DecodeError::Unrecognized)
}

/// Canonical repeated-cell binary encoding for an arbitrary-length natural.
pub fn encode_cell_binary(value: &Nat) -> String {
    let mut out = String::new();
    out.push(VINIT);

    if value.is_zero() {
        push_cell(&mut out, false);
    } else {
        for bit in value.bits_le().iter().copied() {
            push_cell(&mut out, bit);
        }
    }

    out.push(IMSCRIB);
    out.push(IFIX);
    out.push(TANCH);
    out
}

pub fn encode_decimal(raw: &str) -> Option<String> {
    Nat::from_decimal(raw).map(|n| encode_cell_binary(&n))
}

/// Γ: interleave two canonical LSB-first numeral streams, padding the shorter
/// stream with zeroes up to the longer stream's width.
pub fn braid_values(left: &Nat, right: &Nat) -> Nat {
    let width = left.bits_le().len().max(right.bits_le().len());
    let mut braided = Vec::with_capacity(width.saturating_mul(2));
    for index in 0..width {
        braided.push(left.bit(index));
        braided.push(right.bit(index));
    }
    Nat::from_bits_le(braided)
}

/// Λ: recover the even-cell and odd-cell lanes of an LSB-first numeral stream.
pub fn unbraid_value(value: &Nat) -> (Nat, Nat) {
    let mut left = Vec::with_capacity(value.bits_le().len().div_ceil(2));
    let mut right = Vec::with_capacity(value.bits_le().len() / 2);
    for (index, &bit) in value.bits_le().iter().enumerate() {
        if index % 2 == 0 {
            left.push(bit);
        } else {
            right.push(bit);
        }
    }
    (Nat::from_bits_le(left), Nat::from_bits_le(right))
}

pub fn braid_report(left_raw: &str, right_raw: &str) -> Result<String, String> {
    let left =
        Nat::from_decimal(left_raw).ok_or_else(|| format!("not a natural number: {left_raw}"))?;
    let right =
        Nat::from_decimal(right_raw).ok_or_else(|| format!("not a natural number: {right_raw}"))?;
    let braided = braid_values(&left, &right);
    let (left_roundtrip, right_roundtrip) = unbraid_value(&braided);
    let closed = left_roundtrip == left && right_roundtrip == right;
    Ok(format!(
        "left-value       {left}\nleft-word        {}\n\
         right-value      {right}\nright-word       {}\n\
         braid-value      {braided}\nbraid-word       {}\n\
         unbraid-left     {left_roundtrip}\nunbraid-left-word  {}\n\
         unbraid-right    {right_roundtrip}\nunbraid-right-word {}\n\
         ΓΛ-closure       {}\n",
        encode_cell_binary(&left),
        encode_cell_binary(&right),
        encode_cell_binary(&braided),
        encode_cell_binary(&left_roundtrip),
        encode_cell_binary(&right_roundtrip),
        if closed { "closed" } else { "open" },
    ))
}

pub fn unbraid_report(raw: &str) -> Result<String, String> {
    let value = Nat::from_decimal(raw).ok_or_else(|| format!("not a natural number: {raw}"))?;
    let (left, right) = unbraid_value(&value);
    let recomposed = braid_values(&left, &right);
    Ok(format!(
        "source-value     {value}\nsource-word      {}\n\
         left-lane        {left}\nleft-word        {}\n\
         right-lane       {right}\nright-word       {}\n\
         ΓΛ-value         {recomposed}\nΓΛ-word          {}\n\
         ΓΛ-closure       {}\n\
         factor-closure   {}\n",
        encode_cell_binary(&value),
        encode_cell_binary(&left),
        encode_cell_binary(&right),
        encode_cell_binary(&recomposed),
        if recomposed == value {
            "closed"
        } else {
            "open"
        },
        if left.mul(&right) == value {
            "closed"
        } else {
            "open"
        },
    ))
}

/// Check the five codec identities at the point where a decimal enters the
/// analyzer. The returned checks are also used by the CLI contract and tests.
pub fn codec_assertions(value: &Nat) -> Result<(), String> {
    let word = encode_cell_binary(value);
    let decoded = decode(&word).map_err(|error| error.to_string())?;
    let expected_bits = if value.is_zero() {
        alloc::vec![false]
    } else {
        value.bits_le().to_vec()
    };
    let support_from_value: Vec<usize> = value
        .bits_le()
        .iter()
        .enumerate()
        .filter_map(|(i, bit)| bit.then_some(i))
        .collect();
    let support_from_reading: Vec<usize> = bit_support(value)
        .iter()
        .map(|position| position.to_string().parse::<usize>().unwrap_or(usize::MAX))
        .collect();
    let binary = value.binary_string();
    let bits_le: String = expected_bits
        .iter()
        .map(|bit| if *bit { '1' } else { '0' })
        .collect();
    let suffix: String = [IMSCRIB, IFIX, TANCH].iter().collect();
    let word_body = word
        .strip_prefix(VINIT)
        .and_then(|w| w.strip_suffix(&suffix));
    let body_matches = word_body
        .map(|body| {
            let mut expected = String::new();
            for bit in &expected_bits {
                push_cell(&mut expected, *bit);
            }
            body == expected
        })
        .unwrap_or(false);
    let decoded_word = encode_cell_binary(&decoded.value);
    let assertions = [
        support_from_value == support_from_reading,
        binary.chars().rev().collect::<String>() == bits_le,
        body_matches,
        decoded.value == *value && decoded_word == word,
        expected_bits.len() == bits_le.len(),
    ];
    if assertions.iter().all(|pass| *pass) {
        Ok(())
    } else {
        Err(format!("codec assertions failed: {assertions:?}"))
    }
}

fn trailing_zero_bits(value: &Nat) -> usize {
    value.bits_le().iter().take_while(|bit| !**bit).count()
}

fn pow_nat(base: &Nat, exponent: &Nat) -> Nat {
    let mut result = Nat::one();
    let mut power = base.clone();
    for (position, bit) in exponent.bits_le().iter().copied().enumerate() {
        if bit {
            result = result.mul(&power);
        }
        if position + 1 < exponent.bits_le().len() {
            power = power.mul(&power);
        }
    }
    result
}

pub fn lte_2_report(a_raw: &str, m_raw: &str) -> Result<String, String> {
    let a = Nat::from_decimal(a_raw).ok_or_else(|| format!("not a natural number: {a_raw}"))?;
    let m = Nat::from_decimal(m_raw).ok_or_else(|| format!("not a natural number: {m_raw}"))?;
    if a.cmp_nat(&Nat::one()) != Ordering::Greater || !a.bit(0) || m.is_zero() || m.bit(0) {
        return Err("lte_2 requires odd a > 1 and positive even m".to_string());
    }
    let lhs_value = pow_nat(&a, &m)
        .sub(&Nat::one())
        .ok_or_else(|| "a^m - 1 underflow".to_string())?;
    let lhs = trailing_zero_bits(&lhs_value);
    let a_minus = a
        .sub(&Nat::one())
        .ok_or_else(|| "a - 1 underflow".to_string())?;
    let a_plus = a.add(&Nat::one());
    let rhs =
        trailing_zero_bits(&a_minus) + trailing_zero_bits(&a_plus) + trailing_zero_bits(&m) - 1;
    Ok(format!(
        "lte_2      v2(a^m - 1) = v2(a - 1) + v2(a + 1) + v2(m) - 1\na            {a}\nm            {m}\nleft         {lhs}\nright        {rhs}\nmethod       exact support valuation\nstatus       {}\n",
        if lhs == rhs { "PASS" } else { "FAIL" },
    ))
}

fn intervals(value: &Nat, ones: bool, width: usize) -> String {
    let mut ranges = Vec::new();
    let mut start = None;
    for position in 0..width {
        let matches = value.bit(position) == ones;
        match (start, matches) {
            (None, true) => start = Some(position),
            (Some(begin), false) => {
                ranges.push(format!("{begin}..{}", position - 1));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(begin) = start {
        ranges.push(format!("{begin}..{}", width.saturating_sub(1)));
    }
    format!("[{}]", ranges.join(","))
}

fn support_period(value: &Nat, width: usize) -> Option<usize> {
    (1..=width).find(|period| {
        (0..width.saturating_sub(*period)).all(|i| value.bit(i) == value.bit(i + period))
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrimeSieveRead {
    pub aperture_width: usize,
    pub aperture: Nat,
    pub tested_primes: Nat,
    pub divisor: Option<Nat>,
    pub lower_bound: Option<Nat>,
}

fn nat_from_usize(mut value: usize) -> Nat {
    let mut bits = Vec::new();
    while value != 0 {
        bits.push(value & 1 == 1);
        value >>= 1;
    }
    Nat::from_bits_le(bits)
}

fn power_of_two(exponent: usize) -> Result<Nat, String> {
    let bit_count = exponent
        .checked_add(1)
        .ok_or_else(|| "sieve aperture exceeds addressable memory".to_string())?;
    let mut bits = Vec::new();
    bits.try_reserve_exact(bit_count)
        .map_err(|_| "insufficient memory for sieve aperture".to_string())?;
    bits.resize(exponent, false);
    bits.push(true);
    Ok(Nat::from_bits_le(bits))
}

fn integer_sqrt(value: usize) -> usize {
    let mut low = 0usize;
    let mut high = value;
    while low < high {
        let distance = high - low;
        let middle = low + distance / 2 + distance % 2;
        if middle <= value / middle {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    low
}

fn segmented_odd_sieve(value: &Nat, aperture: usize) -> Result<(Nat, Option<Nat>), String> {
    const SEGMENT_ODDS: usize = 32 * 1024;
    let root = integer_sqrt(aperture);
    let base_len = root
        .checked_add(1)
        .ok_or_else(|| "sieve base range exceeds addressable memory".to_string())?;
    let mut composite = Vec::new();
    composite
        .try_reserve_exact(base_len)
        .map_err(|_| "insufficient memory for sieve base primes".to_string())?;
    composite.resize(base_len, false);
    let mut primes = Vec::new();
    for candidate in 2..=root {
        if composite[candidate] {
            continue;
        }
        if primes.len() == primes.capacity() {
            primes
                .try_reserve(1)
                .map_err(|_| "insufficient memory for sieve base-prime list".to_string())?;
        }
        primes.push(candidate);
        if candidate <= root / candidate {
            let mut multiple = candidate * candidate;
            while multiple <= root {
                composite[multiple] = true;
                multiple += candidate;
            }
        }
    }

    let mut tested = Nat::zero();
    let mut low = 3usize;
    let mut segment = Vec::new();
    segment
        .try_reserve_exact(SEGMENT_ODDS)
        .map_err(|_| "insufficient memory for sieve segment".to_string())?;
    while low <= aperture {
        let span = 2usize.saturating_mul(SEGMENT_ODDS - 1);
        let mut high = low.saturating_add(span).min(aperture);
        if high & 1 == 0 {
            high -= 1;
        }
        let count = (high - low) / 2 + 1;
        segment.clear();
        segment.resize(count, false);
        for &prime in &primes {
            if prime > high / prime {
                break;
            }
            if prime == 2 {
                continue;
            }
            let square = prime * prime;
            let quotient = low / prime;
            let first_multiple = if low % prime == 0 {
                low
            } else {
                quotient
                    .checked_add(1)
                    .and_then(|next| next.checked_mul(prime))
                    .unwrap_or(usize::MAX)
            };
            let first = square.max(first_multiple);
            let first = if first & 1 == 0 {
                first.checked_add(prime).unwrap_or(usize::MAX)
            } else {
                first
            };
            let step = prime * 2;
            let mut multiple = first;
            while multiple <= high {
                segment[(multiple - low) / 2] = true;
                let Some(next) = multiple.checked_add(step) else {
                    break;
                };
                multiple = next;
            }
        }
        for (offset, marked) in segment.iter().copied().enumerate() {
            if !marked {
                let candidate = low + offset * 2;
                tested = tested.add(&Nat::one());
                if value.mod_small(candidate) == 0 {
                    return Ok((tested, Some(nat_from_usize(candidate))));
                }
            }
        }
        if high >= aperture - 1 {
            break;
        }
        low = high.saturating_add(2);
    }
    Ok((tested, None))
}

fn arbitrary_nat_sieve(value: &Nat, aperture: &Nat) -> Result<(Nat, Option<Nat>), String> {
    let two = Nat::from_u64(2);
    let mut candidate = Nat::from_u64(3);
    let mut tested_primes = Nat::zero();
    let mut primes_and_squares: Vec<(Nat, Nat)> = Vec::new();
    while candidate.cmp_nat(aperture) != Ordering::Greater {
        let mut is_prime = true;
        for (prime, square) in &primes_and_squares {
            if square.cmp_nat(&candidate) == Ordering::Greater {
                break;
            }
            if candidate.mod_nat(prime).is_zero() {
                is_prime = false;
                break;
            }
        }
        if is_prime {
            tested_primes = tested_primes.add(&Nat::one());
            if value.mod_nat(&candidate).is_zero() {
                return Ok((tested_primes, Some(candidate)));
            }
            if primes_and_squares.len() == primes_and_squares.capacity() {
                primes_and_squares
                    .try_reserve(1)
                    .map_err(|_| "insufficient memory for arbitrary sieve primes".to_string())?;
            }
            primes_and_squares.push((candidate.clone(), candidate.mul(&candidate)));
        }
        candidate = candidate.add(&two);
    }
    Ok((tested_primes, None))
}

/// Read the odd-prime sieve lane directly from the canonical bit support.
/// A returned lower bound is backed by testing every odd prime through 2^width.
pub fn prime_sieve_read(value: &Nat, width: usize) -> Result<PrimeSieveRead, String> {
    if width < 2 {
        return Err("sieve width must be at least 2".to_string());
    }
    let aperture = power_of_two(width)?;
    let fast_aperture = (width < usize::BITS as usize).then(|| 1usize << width);
    let (tested_primes, divisor) = if let Some(fast_aperture) = fast_aperture {
        let (tested, divisor) = segmented_odd_sieve(value, fast_aperture)?;
        (tested, divisor)
    } else {
        arbitrary_nat_sieve(value, &aperture)?
    };
    let odd_part = Nat::from_bits_le(
        value
            .bits_le()
            .iter()
            .skip(trailing_zero_bits(value))
            .copied()
            .collect(),
    );
    let lower_bound = if divisor.is_none() && odd_part.cmp_nat(&Nat::one()) == Ordering::Greater {
        Some(aperture.clone())
    } else {
        None
    };
    Ok(PrimeSieveRead {
        aperture_width: width,
        aperture,
        tested_primes,
        divisor,
        lower_bound,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FactorPairRead {
    pub factor: Nat,
    pub cofactor: Nat,
    pub product_closed: bool,
    pub semiprime_closed: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameArithmeticRead {
    pub source: Nat,
    pub left_width: usize,
    pub left_index: usize,
    pub left: Nat,
    pub operator: &'static str,
    pub right_width: usize,
    pub right_index: usize,
    pub right: Nat,
    pub result: Nat,
    pub remainder: Option<Nat>,
    pub return_frames: Vec<FrameReturnRead>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameReturnRead {
    pub width: usize,
    pub groups: usize,
    pub recovered: Nat,
    pub closed: bool,
}

fn frame_bits(value: &Nat) -> Vec<bool> {
    if value.is_zero() {
        alloc::vec![false]
    } else {
        value.bits_le().to_vec()
    }
}

fn frame_group(value: &Nat, width: usize, index: usize) -> Result<Nat, String> {
    let bits = frame_bits(value);
    let start = index
        .checked_mul(width)
        .ok_or_else(|| "frame group index exceeds addressable memory".to_string())?;
    if start >= bits.len() {
        return Err(format!(
            "frame group {index} is outside width-{width} frame"
        ));
    }
    let end = start.saturating_add(width).min(bits.len());
    Ok(Nat::from_bits_le(bits[start..end].to_vec()))
}

fn frame_return(value: &Nat, width: usize) -> Result<FrameReturnRead, String> {
    let bits = frame_bits(value);
    let groups = bits.len().div_ceil(width);
    let digits = (0..groups)
        .map(|index| frame_group(value, width, index))
        .collect::<Result<Vec<_>, _>>()?;
    let recovered = Nat::from_bits_le(
        (0..bits.len())
            .map(|position| digits[position / width].bit(position % width))
            .collect(),
    );
    Ok(FrameReturnRead {
        width,
        groups,
        closed: recovered == *value,
        recovered,
    })
}

pub fn frame_arithmetic_read(
    value: &Nat,
    left_width: usize,
    left_index: usize,
    operator: &str,
    right_width: usize,
    right_index: usize,
) -> Result<FrameArithmeticRead, String> {
    if left_width < 2 || right_width < 2 {
        return Err("frame widths must be at least 2".to_string());
    }
    if left_width == right_width {
        return Err("frame arithmetic requires values from different widths".to_string());
    }
    let left = frame_group(value, left_width, left_index)?;
    let right = frame_group(value, right_width, right_index)?;
    let (operator, result, remainder) = match operator {
        "add" => ("add", left.add(&right), None),
        "mul" => ("mul", left.mul(&right), None),
        "sub" => (
            "sub",
            left.sub(&right)
                .ok_or_else(|| "frame subtraction underflow".to_string())?,
            None,
        ),
        "mod" => {
            let (_, remainder) = left
                .div_rem(&right)
                .ok_or_else(|| "frame modulo by zero".to_string())?;
            ("mod", remainder.clone(), Some(remainder))
        }
        "divmod" => {
            let (quotient, remainder) = left
                .div_rem(&right)
                .ok_or_else(|| "frame divmod by zero".to_string())?;
            ("divmod", quotient, Some(remainder))
        }
        _ => return Err("frame operator must be add|mul|sub|mod|divmod".to_string()),
    };
    let return_frames = (2..=8)
        .map(|width| frame_return(&result, width))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(FrameArithmeticRead {
        source: value.clone(),
        left_width,
        left_index,
        left,
        operator,
        right_width,
        right_index,
        right,
        result,
        remainder,
        return_frames,
    })
}

pub fn render_frame_arithmetic(read: &FrameArithmeticRead) -> String {
    let remainder = read
        .remainder
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_else(|| "none".to_string());
    let mut output = format!(
        "frame-left   width={} group={} value={} word={}\n\
         frame-right  width={} group={} value={} word={}\n\
         operation    {}\n\
         result       {}\n\
         result-word  {}\n\
         remainder    {}\n",
        read.left_width,
        read.left_index,
        read.left,
        encode_cell_binary(&read.left),
        read.right_width,
        read.right_index,
        read.right,
        encode_cell_binary(&read.right),
        read.operator,
        read.result,
        encode_cell_binary(&read.result),
        remainder,
    );
    for returned in &read.return_frames {
        output.push_str(&format!(
            "frame-return width={} groups={} recovered={} closure={} word={}\n",
            returned.width,
            returned.groups,
            returned.recovered,
            if returned.closed { "closed" } else { "open" },
            encode_cell_binary(&returned.recovered),
        ));
    }
    output
}

pub fn frame_arithmetic_from_args(args: &[&str]) -> Result<FrameArithmeticRead, String> {
    if args.len() != 7 || args.first().copied() != Some("frame-op") {
        return Err(
            "godel frame-op <natural-number> <left-width> <left-group> <add|mul|sub|mod|divmod> <right-width> <right-group>".to_string(),
        );
    }
    let value =
        Nat::from_decimal(args[1]).ok_or_else(|| format!("not a natural number: {}", args[1]))?;
    let parse_address = |raw: &str, label: &str| {
        raw.parse::<usize>()
            .map_err(|_| format!("{label} must be a nonnegative frame address"))
    };
    frame_arithmetic_read(
        &value,
        parse_address(args[2], "left width")?,
        parse_address(args[3], "left group")?,
        args[4],
        parse_address(args[5], "right width")?,
        parse_address(args[6], "right group")?,
    )
}

/// Turn a prime-divisor witness into an exact factor-pair read and independently
/// check whether the cofactor is prime within the same sieve aperture.
pub fn factor_pair_read(
    value: &Nat,
    sieve: &PrimeSieveRead,
    width: usize,
) -> Result<Option<FactorPairRead>, String> {
    let Some(factor) = sieve.divisor.as_ref() else {
        return Ok(None);
    };
    let Some((cofactor, remainder)) = value.div_rem(factor) else {
        return Err("zero divisor in factor-pair closure".to_string());
    };
    let nontrivial = factor.cmp_nat(&Nat::one()) == Ordering::Greater
        && cofactor.cmp_nat(&Nat::one()) == Ordering::Greater;
    let product_closed = nontrivial && factor.mul(&cofactor) == *value && remainder.is_zero();
    if !product_closed {
        return Ok(Some(FactorPairRead {
            factor: factor.clone(),
            cofactor,
            product_closed,
            semiprime_closed: Some(false),
        }));
    }

    let cofactor_is_prime = if cofactor == Nat::from_u64(2) {
        Some(true)
    } else if !cofactor.bit(0) {
        Some(false)
    } else {
        let sqrt_aperture_width = cofactor.bits_le().len().div_ceil(2).max(2);
        let cofactor_width = width.min(sqrt_aperture_width);
        let cofactor_sieve = prime_sieve_read(&cofactor, cofactor_width)?;
        match cofactor_sieve.divisor.as_ref() {
            Some(witness) => Some(witness == &cofactor),
            None => cofactor_sieve.lower_bound.as_ref().and_then(|bound| {
                let bound_squared = bound.mul(bound);
                (bound_squared.cmp_nat(&cofactor) != Ordering::Less).then_some(true)
            }),
        }
    };
    Ok(Some(FactorPairRead {
        factor: factor.clone(),
        cofactor,
        product_closed,
        semiprime_closed: cofactor_is_prime,
    }))
}

/// Structural reads from a decoded cell-binary value. Periodicity and the
/// prime-divisor bound are separate coordinates: the latter is certified by
/// testing every odd prime through the aperture against the exact bit support.
pub fn analyze(value: &Nat, window: usize) -> Result<String, String> {
    if window < 2 {
        return Err("window must be at least 2".to_string());
    }
    codec_assertions(value)?;
    let width = value.bits_le().len().max(1);
    let support = bit_support(value);
    let gaps = intervals(value, false, width);
    let runs = intervals(value, true, width);
    let v2_n = if value.is_zero() {
        "∞".to_string()
    } else {
        trailing_zero_bits(value).to_string()
    };
    let plus_one = value.add(&Nat::one());
    let v2_plus = trailing_zero_bits(&plus_one);
    let minus_one = value.sub(&Nat::one());
    let v2_minus = match minus_one.as_ref() {
        Some(value) if value.is_zero() => "∞".to_string(),
        Some(value) => trailing_zero_bits(value).to_string(),
        None => "undefined".to_string(),
    };
    let odd_part = value.shr(trailing_zero_bits(value));
    let k = v2_plus;
    let decomposition_m = plus_one.shr(k).sub(&Nat::one()).unwrap_or_else(Nat::zero);
    let residue = Nat::from_bits_le(value.bits_le().iter().take(k).copied().collect());
    let sieve = prime_sieve_read(value, window)?;
    let aperture = sieve.aperture;
    let period = support_period(value, window);
    let bound_read = match (&sieve.divisor, &sieve.lower_bound) {
        (Some(prime), _) => {
            format!("odd divisor {prime} present at aperture 2^{window}={aperture}")
        }
        (_, Some(bound)) => {
            format!("smallest odd prime factor > {bound} (all odd primes ≤ aperture tested)")
        }
        _ => "no odd prime factor in the support".to_string(),
    };
    let periodicity = period
        .map(|p| format!("period={p} on [0,{window})"))
        .unwrap_or_else(|| format!("aperiodic on [0,{window})"));
    let binary = value.binary_string();
    let bits_le: String = if value.is_zero() {
        "0".to_string()
    } else {
        value
            .bits_le()
            .iter()
            .map(|bit| if *bit { '1' } else { '0' })
            .collect()
    };
    let signature = format!(
        "(runs={runs},gaps={gaps},period={})",
        period
            .map(|p| p.to_string())
            .unwrap_or_else(|| "none".into())
    );
    Ok(format!(
        "value        {value}\nbinary       {binary}\nbits-le      {bits_le}\nsupport      {}\npolynomial   {}\nword         {}\nbitlength    {}\ncodec        support={} binary={} body={} decode={} bitlength={}\n\nprimitive reads\nv2(n)        {v2_n}\nv2(n+1)      {v2_plus}\nv2(n-1)      {v2_minus}\npopcount     {}\nbitlength    {}\nruns         {runs}\ngaps         {gaps}\nodd_part     {odd_part}\nshift_factor {}\n\ncomposite reads\ndecomp_2k    n=(2^{k}-1)+2^{k}*{decomposition_m}  (k={k})\nresidue_read n mod 2^{k}={residue}\nperiodicity  {periodicity}\naperture     2^{window}={aperture}\nwindow_prime_bound {bound_read}\nsignature    {signature}\n",
        support_string(&support), polynomial_string(value), encode_cell_binary(value),
        width, true, binary.chars().rev().collect::<String>() == bits_le,
        true, true, true, support.len(), width, trailing_zero_bits(value),
    ))
}

fn push_cell(out: &mut String, one: bool) {
    out.push(AFWD);
    out.push(CLINK);
    out.push(FSPLIT);
    out.push(if one { EVALF } else { EVALT });
    out.push(FFUSE);
}

pub fn check(
    lhs: &str,
    operator: Operator,
    rhs: &str,
    out: &str,
) -> Result<EquationCheck, DecodeError> {
    let lhs = decode(lhs)?.value;
    let rhs = decode(rhs)?.value;
    let out = decode(out)?.value;
    let expected = match operator {
        Operator::Add => lhs.add(&rhs),
        Operator::Mul => lhs.mul(&rhs),
    };
    let valid = expected == out;

    Ok(EquationCheck {
        lhs,
        rhs,
        out,
        expected,
        operator,
        valid,
    })
}

/// If `to` is exactly `from` with one glyph inserted, report the insertion and
/// its exact induced numeric delta under the registered readings.
pub fn insertion_relation(from: &str, to: &str) -> Result<Option<InsertionRelation>, DecodeError> {
    let from_chars = validate(from)?;
    let to_chars = validate(to)?;
    if to_chars.len() != from_chars.len() + 1 {
        return Ok(None);
    }

    let mut i = 0usize;
    while i < from_chars.len() && from_chars[i] == to_chars[i] {
        i += 1;
    }
    if from_chars[i..] != to_chars[i + 1..] {
        return Ok(None);
    }

    let a = decode(from)?.value;
    let b = decode(to)?.value;
    let delta = match b.sub(&a) {
        Some(delta) => delta,
        None => return Ok(None),
    };
    Ok(Some(InsertionRelation {
        glyph: to_chars[i],
        position: Nat::from_decimal(&i.to_string()).expect("usize decimal is a natural"),
        delta,
    }))
}

pub fn bit_support(value: &Nat) -> Vec<Nat> {
    let mut out = Vec::new();
    let mut position = Nat::zero();
    for bit in value.bits_le().iter().copied() {
        if bit {
            out.push(position.clone());
        }
        position = position.add(&Nat::one());
    }
    out
}

pub fn polynomial_string(value: &Nat) -> String {
    let support = bit_support(value);
    if support.is_empty() {
        return "0".to_string();
    }
    let zero = Nat::zero();
    let one = Nat::one();
    let mut out = String::new();
    for (n, power) in support.iter().enumerate() {
        if n != 0 {
            out.push_str(" + ");
        }
        if power == &zero {
            out.push('1');
        } else if power == &one {
            out.push('x');
        } else {
            out.push_str(&format!("x^{power}"));
        }
    }
    out
}

fn support_string(support: &[Nat]) -> String {
    let mut out = String::from("{");
    for (i, p) in support.iter().enumerate() {
        if i != 0 {
            out.push_str(",");
        }
        out.push_str(&p.to_string());
    }
    out.push('}');
    out
}

pub fn render(reading: &Reading) -> String {
    let mut out = format!(
        "family     {}\nvalue      {}\nbinary     {}\nsupport    {}\npolynomial {}\n",
        reading.family.label(),
        reading.value,
        reading.binary(),
        support_string(&reading.support()),
        reading.polynomial(),
    );

    match &reading.structure {
        Structure::CellBinary { bits_le } => {
            let mut bits = String::with_capacity(bits_le.len());
            for &b in bits_le {
                bits.push(if b { '1' } else { '0' });
            }
            out.push_str(&format!(
                "bits-le    {bits}\nrule       ⊥=1  ⊤=0  leftmost=2^0\n"
            ));
        }
        Structure::AffineEdit { unit, branch } => {
            out.push_str(&format!(
                "base       2\nunit ⊥     {} (+1)\nbranch ⊞   {} (+2)\n",
                if *unit { "present" } else { "absent" },
                if *branch { "present" } else { "absent" },
            ));
        }
        Structure::ProductFusion => {
            out.push_str("fusion     3+5 = 2*4 = 8\n");
        }
    }
    out
}

pub fn help() -> &'static str {
    "godel — exact, unbounded readings of the IMASM glyph calculus\n\
     \n\
     godel decode <word>\n\
     godel encode <natural-number>\n\
     godel analyze <natural-number|cell-binary-word> [sieve-window=8]\n\
     godel braid <left-natural> <right-natural>\n\
     godel unbraid <natural-number>\n\
     godel frame-op <natural-number> <left-width> <left-group> <add|mul|sub|mod|divmod> <right-width> <right-group>\n\
     godel lte2 <odd-a> <positive-even-m>\n\
     godel check add|mul <lhs-word> <rhs-word> <out-word>\n\
     godel relation <from-word> <to-word>\n\
     godel selftest\n"
}

pub fn command(args: &[&str]) -> Result<String, String> {
    match args.first().copied().unwrap_or("help") {
        "help" | "-h" | "--help" => Ok(help().to_string()),
        "decode" => {
            let word = args
                .get(1)
                .ok_or_else(|| "godel decode <word>".to_string())?;
            let r = decode(word).map_err(|e| e.to_string())?;
            Ok(format!("word       {word}\n{}", render(&r)))
        }
        "encode" => {
            let raw = args
                .get(1)
                .ok_or_else(|| "godel encode <natural-number>".to_string())?;
            let n = Nat::from_decimal(raw).ok_or_else(|| format!("not a natural number: {raw}"))?;
            codec_assertions(&n)?;
            let word = encode_cell_binary(&n);
            Ok(format!(
                "value      {n}\nword       {word}\ncodec      all five assertions PASS\n{}",
                render(&decode(&word).map_err(|e| e.to_string())?)
            ))
        }
        "analyze" => {
            if !(2..=3).contains(&args.len()) {
                return Err("godel analyze <natural-number> [sieve-window=8]".to_string());
            }
            let raw = args[1];
            let value =
                Nat::from_decimal(raw).ok_or_else(|| format!("not a natural number: {raw}"))?;
            let window = args
                .get(2)
                .map(|s| s.parse::<usize>())
                .transpose()
                .map_err(|_| "window must be a nonnegative integer of at least 2".to_string())?
                .unwrap_or(8);
            analyze(&value, window)
        }
        "braid" => {
            if args.len() != 3 {
                return Err("godel braid <left-natural> <right-natural>".to_string());
            }
            braid_report(args[1], args[2])
        }
        "unbraid" => {
            if args.len() != 2 {
                return Err("godel unbraid <natural-number>".to_string());
            }
            unbraid_report(args[1])
        }
        "frame-op" => {
            let read = frame_arithmetic_from_args(args)?;
            Ok(format!(
                "source-value  {}\nsource-word   {}\n{}",
                read.source,
                encode_cell_binary(&read.source),
                render_frame_arithmetic(&read),
            ))
        }
        "lte2" => {
            if args.len() != 3 {
                return Err("godel lte2 <odd-a> <positive-even-m>".to_string());
            }
            lte_2_report(args[1], args[2])
        }
        "check" => {
            if args.len() != 5 {
                return Err("godel check add|mul <lhs-word> <rhs-word> <out-word>".to_string());
            }
            let op =
                Operator::parse(args[1]).ok_or_else(|| format!("unknown operator: {}", args[1]))?;
            let c = check(args[2], op, args[3], args[4]).map_err(|e| e.to_string())?;
            Ok(format!(
                "equation   {} {} {} = {}\nexpected   {}\nstatus     {}\n",
                c.lhs,
                c.operator.glyph(),
                c.rhs,
                c.out,
                c.expected,
                if c.valid { "PASS" } else { "FAIL" },
            ))
        }
        "relation" => {
            if args.len() != 3 {
                return Err("godel relation <from-word> <to-word>".to_string());
            }
            match insertion_relation(args[1], args[2]).map_err(|e| e.to_string())? {
                Some(r) => Ok(format!(
                    "relation   insert {} at glyph position {}\ndelta      +{}\n",
                    r.glyph, r.position, r.delta
                )),
                None => Ok(
                    "relation   not a one-glyph insertion between registered numeral forms\n"
                        .to_string(),
                ),
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
        out.push_str(&format!(
            "{name:<9} {} {} {} = {}  {}\n",
            c.lhs,
            c.operator.glyph(),
            c.rhs,
            c.out,
            if c.valid { "PASS" } else { "FAIL" },
        ));
    }

    for (name, from, to, glyph, delta) in [
        ("unit-2→3", D1, A, EVALF, 1u64),
        ("unit-4→5", D2, B, EVALF, 1u64),
        ("branch-2→4", D1, D2, ENGAGR, 2u64),
        ("branch-3→5", A, B, ENGAGR, 2u64),
    ] {
        let relation = insertion_relation(from, to).map_err(|e| e.to_string())?;
        let expected = Nat::from_u64(delta);
        let pass = relation
            .as_ref()
            .map(|r| r.glyph == glyph && r.delta == expected)
            .unwrap_or(false);
        ok &= pass;
        out.push_str(&format!(
            "{name:<9} insert {glyph} => +{delta}  {}\n",
            if pass { "PASS" } else { "FAIL" }
        ));
    }

    let huge = "115792089237316195423570985008687907853269984665640564039457584007913129639936";
    let n =
        Nat::from_decimal(huge).ok_or_else(|| "internal unbounded parse failure".to_string())?;
    let word = encode_cell_binary(&n);
    let back = decode(&word).map_err(|e| e.to_string())?.value;
    let unbounded_pass = back == n && back.decimal_string() == huge;
    ok &= unbounded_pass;
    out.push_str(&format!(
        "unbounded  2^256 roundtrip  {}\n",
        if unbounded_pass { "PASS" } else { "FAIL" }
    ));

    if ok {
        Ok(out)
    } else {
        Err(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_binary_examples() {
        assert_eq!(decode(CELL_3).unwrap().value, Nat::from_u64(3));
        assert_eq!(decode(CELL_7).unwrap().value, Nat::from_u64(7));
        assert_eq!(decode(CELL_10).unwrap().value, Nat::from_u64(10));
        assert_eq!(decode(CELL_21).unwrap().value, Nat::from_u64(21));
        assert!(check(CELL_3, Operator::Add, CELL_7, CELL_10).unwrap().valid);
        assert!(check(CELL_3, Operator::Mul, CELL_7, CELL_21).unwrap().valid);
    }

    #[test]
    fn edit_square_examples() {
        assert_eq!(decode(D1).unwrap().value, Nat::from_u64(2));
        assert_eq!(decode(A).unwrap().value, Nat::from_u64(3));
        assert_eq!(decode(D2).unwrap().value, Nat::from_u64(4));
        assert_eq!(decode(B).unwrap().value, Nat::from_u64(5));
        assert_eq!(decode(C).unwrap().value, Nat::from_u64(8));
        assert!(check(A, Operator::Add, B, C).unwrap().valid);
        assert!(check(D1, Operator::Mul, D2, C).unwrap().valid);
    }

    #[test]
    fn edit_relations_are_structural() {
        let unit = insertion_relation(D1, A).unwrap().unwrap();
        assert_eq!((unit.glyph, unit.delta), (EVALF, Nat::from_u64(1)));
        let branch = insertion_relation(D1, D2).unwrap().unwrap();
        assert_eq!((branch.glyph, branch.delta), (ENGAGR, Nat::from_u64(2)));
    }

    #[test]
    fn arbitrary_length_roundtrip_and_arithmetic() {
        let two_256 =
            "115792089237316195423570985008687907853269984665640564039457584007913129639936";
        let n = Nat::from_decimal(two_256).unwrap();
        assert!(n.bits_le().len() > 128);
        let word = encode_cell_binary(&n);
        let round = decode(&word).unwrap().value;
        assert_eq!(round, n);
        assert_eq!(round.decimal_string(), two_256);

        let doubled = n.add(&n);
        assert_eq!(
            doubled.decimal_string(),
            "231584178474632390847141970017375815706539969331281128078915168015826259279872"
        );
        let tripled = n.mul(&Nat::from_u64(3));
        assert_eq!(
            tripled.decimal_string(),
            "347376267711948586270712955026063723559809953996921692118372752023739388919808"
        );
    }

    #[test]
    fn analyzer_reads_support_and_keeps_periodicity_separate_from_bound() {
        let n = Nat::from_decimal("8051").unwrap();
        let narrow = analyze(&n, 4).unwrap();
        assert!(narrow.contains("v2(n)        0"));
        assert!(narrow.contains("v2(n+1)      2"));
        assert!(narrow.contains("aperture     2^4=16"));
        assert!(narrow.contains("smallest odd prime factor > 16"));
        let wide = analyze(&n, 7).unwrap();
        assert!(wide.contains("odd divisor 83 present at aperture 2^7=128"));
    }

    #[test]
    fn prime_sieve_scales_past_native_aperture_width() {
        let read = prime_sieve_read(&Nat::from_u64(21), usize::BITS as usize).unwrap();
        assert_eq!(read.aperture, power_of_two(usize::BITS as usize).unwrap());
        assert_eq!(read.tested_primes, Nat::one());
        assert_eq!(read.divisor, Some(Nat::from_u64(3)));
        assert_eq!(read.lower_bound, None);
    }

    #[test]
    fn factor_pair_closure_requires_exact_product_and_prime_cofactor() {
        let semiprime = Nat::from_u64(21);
        let sieve = prime_sieve_read(&semiprime, 8).unwrap();
        let pair = factor_pair_read(&semiprime, &sieve, 8).unwrap().unwrap();
        assert_eq!(pair.factor, Nat::from_u64(3));
        assert_eq!(pair.cofactor, Nat::from_u64(7));
        assert!(pair.product_closed);
        assert_eq!(pair.semiprime_closed, Some(true));

        let composite = Nat::from_u64(999_999);
        let sieve = prime_sieve_read(&composite, 8).unwrap();
        let pair = factor_pair_read(&composite, &sieve, 8).unwrap().unwrap();
        assert_eq!(pair.cofactor, Nat::from_u64(333_333));
        assert!(pair.product_closed);
        assert_eq!(pair.semiprime_closed, Some(false));
    }

    #[test]
    fn arithmetic_combines_values_from_distinct_frames() {
        // 45 has LSB-first bits 101101. Width-2 group 2 is 2, and
        // width-3 group 0 is 5, so the cross-frame product is 10.
        let value = Nat::from_u64(45);
        let product = frame_arithmetic_read(&value, 2, 2, "mul", 3, 0).unwrap();
        assert_eq!(product.left, Nat::from_u64(2));
        assert_eq!(product.right, Nat::from_u64(5));
        assert_eq!(product.result, Nat::from_u64(10));
        assert_eq!(product.remainder, None);

        let sum = frame_arithmetic_read(&value, 2, 2, "add", 3, 0).unwrap();
        assert_eq!(sum.result, Nat::from_u64(7));
        let difference = frame_arithmetic_read(&value, 3, 0, "sub", 2, 2).unwrap();
        assert_eq!(difference.result, Nat::from_u64(3));
        let remainder = frame_arithmetic_read(&value, 3, 0, "mod", 2, 2).unwrap();
        assert_eq!(remainder.result, Nat::from_u64(1));
        assert_eq!(remainder.remainder, Some(Nat::from_u64(1)));

        let divmod = frame_arithmetic_read(&value, 3, 0, "divmod", 2, 2).unwrap();
        assert_eq!(divmod.left, Nat::from_u64(5));
        assert_eq!(divmod.right, Nat::from_u64(2));
        assert_eq!(divmod.result, Nat::from_u64(2));
        assert_eq!(divmod.remainder, Some(Nat::from_u64(1)));
        let rendered = render_frame_arithmetic(&product);
        assert!(rendered.contains("frame-left   width=2 group=2 value=2 word=⊢≻⋈∈⊤∋≻⋈∈⊥∋⊙⊡⊣"));
        assert!(rendered.contains("result-word  ⊢≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋⊙⊡⊣"));
        assert_eq!(product.return_frames.len(), 7);
        assert!(product
            .return_frames
            .iter()
            .all(|frame| frame.closed && frame.recovered == Nat::from_u64(10)));
    }

    #[test]
    fn braid_unbraid_roundtrip_is_exact_for_unequal_and_unbounded_lanes() {
        for (left, right) in [(0, 0), (0, 37), (37, 0), (13, 17), (5, 1025)] {
            let left = Nat::from_u64(left);
            let right = Nat::from_u64(right);
            let braided = braid_values(&left, &right);
            assert_eq!(unbraid_value(&braided), (left, right));
        }

        let left = Nat::from_decimal(
            "115792089237316195423570985008687907853269984665640564039457584007913129639936",
        )
        .unwrap();
        let right = Nat::from_decimal("1000000007").unwrap();
        let braided = braid_values(&left, &right);
        assert_eq!(unbraid_value(&braided), (left.clone(), right.clone()));
        assert!(braided.bits_le().len() > left.bits_le().len());

        let report = braid_report("5", "1025").unwrap();
        assert!(report.contains("ΓΛ-closure       closed\n"));
        let split = unbraid_report("45").unwrap();
        assert!(split.contains("left-lane        3\n"));
        assert!(split.contains("right-lane       6\n"));
        assert!(split.contains("ΓΛ-closure       closed\n"));
        assert!(split.contains("factor-closure   open\n"));
    }

    #[test]
    fn frame_arithmetic_uses_unbounded_values_and_checks_frame_addresses() {
        let value = Nat::from_decimal(
            "115792089237316195423570985008687907853269984665640564039457584007913129639936",
        )
        .unwrap();
        let high_width = value.bits_le().len();
        let read = frame_arithmetic_read(&value, high_width, 0, "add", 2, 0).unwrap();
        assert!(read.result.bits_le().len() > 1);
        assert!(read.return_frames.iter().all(|frame| frame.closed));
        assert!(frame_arithmetic_read(&value, 2, 0, "mul", 2, 1).is_err());
        assert!(frame_arithmetic_read(&value, 2, usize::MAX, "mul", 3, 0).is_err());

        let zero = Nat::zero();
        let zero_read = frame_arithmetic_read(&zero, 2, 0, "add", 3, 0).unwrap();
        assert_eq!(zero_read.left, Nat::zero());
        assert_eq!(zero_read.right, Nat::zero());
        assert!(zero_read.return_frames.iter().all(|frame| frame.closed));
    }

    #[test]
    fn codec_assertions_cover_zero_and_unbounded_values() {
        codec_assertions(&Nat::zero()).unwrap();
        let n = Nat::from_decimal(
            "115792089237316195423570985008687907853269984665640564039457584007913129639936",
        )
        .unwrap();
        codec_assertions(&n).unwrap();
    }

    #[test]
    fn lte2_reads_the_binary_valuation_identity() {
        assert!(lte_2_report("3", "2")
            .unwrap()
            .contains("status       PASS"));
        assert!(lte_2_report("4", "2").is_err());
        assert!(lte_2_report("3", "3").is_err());
    }
}
