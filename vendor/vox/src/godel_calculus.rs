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

    fn bit(&self, position: usize) -> bool {
        self.bits_le.get(position).copied().unwrap_or(false)
    }

    fn shr(&self, places: usize) -> Self {
        Self::from_bits_le(self.bits_le.get(places..).unwrap_or(&[]).to_vec())
    }

    fn mod_small(&self, modulus: usize) -> usize {
        let mut residue = 0usize;
        for &bit in self.bits_le.iter().rev() {
            residue = (residue * 2 + usize::from(bit)) % modulus;
        }
        residue
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

fn smallest_odd_factor_through(value: &Nat, bound: usize) -> Option<usize> {
    if bound < 3 || value.is_zero() {
        return None;
    }
    let mut composite = alloc::vec![false; bound + 1];
    let mut candidate = 3usize;
    while candidate <= bound {
        if !composite[candidate] {
            if value.mod_small(candidate) == 0 {
                return Some(candidate);
            }
            if candidate <= bound / candidate {
                let mut multiple = candidate * candidate;
                while multiple <= bound {
                    composite[multiple] = true;
                    multiple += candidate;
                }
            }
        }
        candidate += 2;
    }
    None
}

/// Structural reads from a decoded cell-binary value. Periodicity and the
/// prime-divisor bound are separate coordinates: the latter is certified by
/// testing every odd prime through the aperture against the exact bit support.
pub fn analyze(value: &Nat, window: usize) -> Result<String, String> {
    if !(2..=20).contains(&window) {
        return Err("window must be in 2..=20".to_string());
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
    let aperture = 1usize << window;
    let period = support_period(value, window);
    let bound_factor = smallest_odd_factor_through(value, aperture);
    let bound_read = match bound_factor {
        Some(prime) => format!("odd divisor {prime} present at aperture 2^{window}={aperture}"),
        None => {
            format!("smallest odd prime factor > {aperture} (all odd primes ≤ aperture tested)")
        }
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
     godel analyze <natural-number> [window=8]\n\
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
                return Err("godel analyze <natural-number> [window=8]".to_string());
            }
            let raw = args[1];
            let value =
                Nat::from_decimal(raw).ok_or_else(|| format!("not a natural number: {raw}"))?;
            let window = args
                .get(2)
                .map(|s| s.parse::<usize>())
                .transpose()
                .map_err(|_| "window must be an integer in 2..=20".to_string())?
                .unwrap_or(8);
            analyze(&value, window)
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
