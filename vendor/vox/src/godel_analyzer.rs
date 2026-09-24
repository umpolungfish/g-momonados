//! Forward structural reads over the `cell-binary` Gödel numeral family.
//!
//! The codec is fixed by `godel_calculus`: `⊥=1`, `⊤=0`, and the leftmost
//! payload cell is bit 0. This module reads arithmetic structure directly from
//! that support geometry. It never treats absence as a factor proof by itself:
//! a divisor lower bound is reported only when supplied as an explicit sieve
//! certificate.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

use crate::godel_calculus::{
    bit_support, decode, encode_cell_binary, polynomial_string, Family, Nat, Structure,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum V2 {
    Finite(Nat),
    Infinity,
}

impl fmt::Display for V2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Finite(n) => write!(f, "{n}"),
            Self::Infinity => f.write_str("∞"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BitInterval {
    pub start: Nat,
    pub end_exclusive: Nat,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decomp2k {
    pub k: Nat,
    pub prefix: Nat,
    pub remainder: Nat,
}

/// A factor lower bound is not inferred from a blank support window. It enters
/// the analyzer only as a certificate from an actual sieve lane.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DivisorBoundCertificate {
    pub bound: Nat,
    pub tested_primes: Nat,
    pub aperture_width: Nat,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodecAssertions {
    pub support: bool,
    pub binary_reverse: bool,
    pub word_glyph_map: bool,
    pub roundtrip: bool,
    pub bitlength: bool,
}

impl CodecAssertions {
    pub fn all(&self) -> bool {
        self.support
            && self.binary_reverse
            && self.word_glyph_map
            && self.roundtrip
            && self.bitlength
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructuralAnalysis {
    pub value: Nat,
    pub word: String,
    pub binary: String,
    pub bits_le: String,
    pub support: Vec<Nat>,
    pub polynomial: String,
    pub assertions: CodecAssertions,
    pub v2: V2,
    pub v2_plus_one: V2,
    pub v2_minus_one: Option<V2>,
    pub popcount: Nat,
    pub bitlength: Nat,
    pub runs: Vec<BitInterval>,
    pub gaps: Vec<BitInterval>,
    pub odd_part: Nat,
    pub shift_factor: V2,
    pub decomp_2k: Decomp2k,
    pub residue_2k: Nat,
    pub window_width: Nat,
    pub period: Option<Nat>,
    pub divisor_bound: Option<DivisorBoundCertificate>,
}

fn nat_from_index(mut value: usize) -> Nat {
    let mut bits = Vec::new();
    while value != 0 {
        bits.push(value & 1 == 1);
        value >>= 1;
    }
    Nat::from_bits_le(bits)
}

fn bits_le_string(bits: &[bool]) -> String {
    let mut out = String::with_capacity(bits.len());
    for bit in bits {
        out.push(if *bit { '1' } else { '0' });
    }
    out
}

fn support_string(support: &[Nat]) -> String {
    let mut out = String::from("{");
    for (i, p) in support.iter().enumerate() {
        if i != 0 {
            out.push(',');
        }
        out.push_str(&p.to_string());
    }
    out.push('}');
    out
}

fn intervals_string(intervals: &[BitInterval]) -> String {
    let mut out = String::from("[");
    for (i, interval) in intervals.iter().enumerate() {
        if i != 0 {
            out.push(',');
        }
        out.push('[');
        out.push_str(&interval.start.to_string());
        out.push(',');
        out.push_str(&interval.end_exclusive.to_string());
        out.push(')');
    }
    out.push(']');
    out
}

fn count_prefix(bits: &[bool], target: bool) -> usize {
    bits.iter().take_while(|bit| **bit == target).count()
}

fn shift_right(value: &Nat, places: usize) -> Nat {
    if places >= value.bits_le().len() {
        Nat::zero()
    } else {
        Nat::from_bits_le(value.bits_le()[places..].to_vec())
    }
}

fn shift_left(value: &Nat, places: usize) -> Nat {
    if value.is_zero() {
        return Nat::zero();
    }
    let mut bits = Vec::with_capacity(value.bits_le().len() + places);
    bits.resize(places, false);
    bits.extend_from_slice(value.bits_le());
    Nat::from_bits_le(bits)
}

/// `v₂(n)`: number of low zero bits, therefore the initial `⊤` run under the
/// fixed codec. `v₂(0)=∞`.
pub fn v2(value: &Nat) -> V2 {
    if value.is_zero() {
        V2::Infinity
    } else {
        V2::Finite(nat_from_index(count_prefix(value.bits_le(), false)))
    }
}

/// `v₂(n+1)`: for finite binary words this is the number of low one bits, the
/// initial `⊥` run.
pub fn v2_plus_one(value: &Nat) -> V2 {
    V2::Finite(nat_from_index(count_prefix(value.bits_le(), true)))
}

pub fn v2_minus_one(value: &Nat) -> Option<V2> {
    if value.is_zero() {
        return None;
    }
    let minus = value.sub(&Nat::one()).expect("positive natural minus one");
    Some(v2(&minus))
}

pub fn popcount(value: &Nat) -> Nat {
    nat_from_index(value.bits_le().iter().filter(|bit| **bit).count())
}

/// Codec bitlength. The current canonical zero word has one `⊤` payload cell,
/// so zero has codec bitlength 1; nonzero values use the ordinary bitlength.
pub fn bitlength(value: &Nat) -> Nat {
    if value.is_zero() {
        Nat::one()
    } else {
        nat_from_index(value.bits_le().len())
    }
}

pub fn odd_part(value: &Nat) -> Nat {
    if value.is_zero() {
        return Nat::zero();
    }
    shift_right(value, count_prefix(value.bits_le(), false))
}

fn intervals(bits: &[bool], target: bool) -> Vec<BitInterval> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bits.len() {
        if bits[i] != target {
            i += 1;
            continue;
        }
        let start = i;
        while i < bits.len() && bits[i] == target {
            i += 1;
        }
        out.push(BitInterval {
            start: nat_from_index(start),
            end_exclusive: nat_from_index(i),
        });
    }
    out
}

pub fn runs(value: &Nat) -> Vec<BitInterval> {
    intervals(value.bits_le(), true)
}

pub fn gaps(value: &Nat) -> Vec<BitInterval> {
    intervals(value.bits_le(), false)
}

/// `n = (2^k - 1) + 2^k*m`, with `k = v₂(n+1)`.
pub fn decomp_2k(value: &Nat) -> Decomp2k {
    let k_index = count_prefix(value.bits_le(), true);
    let k = nat_from_index(k_index);
    let prefix = Nat::from_bits_le(alloc::vec![true; k_index]);
    let n_plus_one = value.add(&Nat::one());
    let quotient = shift_right(&n_plus_one, k_index);
    let remainder = quotient
        .sub(&Nat::one())
        .expect("(n+1)/2^v2(n+1) is positive");
    let reconstructed = prefix.add(&shift_left(&remainder, k_index));
    debug_assert_eq!(&reconstructed, value);
    Decomp2k {
        k,
        prefix,
        remainder,
    }
}

/// Direct low-cell read of `n mod 2^k`. `width` is arbitrary precision; the
/// loop stops when the finite numeral runs out of cells.
pub fn residue_pow2(value: &Nat, width: &Nat) -> Nat {
    let mut kept = Vec::new();
    let mut position = Nat::zero();
    for bit in value.bits_le().iter().copied() {
        if &position == width {
            break;
        }
        kept.push(bit);
        position = position.add(&Nat::one());
    }
    Nat::from_bits_le(kept)
}

/// Smallest exact period having at least two repetitions in the materialized
/// low-bit window. `None` is a negative structural read, not a factor proof.
fn exact_window_period(bits: &[bool]) -> Option<Nat> {
    let width = bits.len();
    if width < 2 {
        return None;
    }
    for p in 1..=width / 2 {
        if (p..width).all(|i| bits[i] == bits[i - p]) {
            return Some(nat_from_index(p));
        }
    }
    None
}

fn cell_bits(word: &str) -> Result<Vec<bool>, String> {
    let reading = decode(word).map_err(|e| e.to_string())?;
    match reading.structure {
        Structure::CellBinary { bits_le } => Ok(bits_le),
        _ => Err("structural analyzer requires cell-binary input".to_string()),
    }
}

pub fn codec_assertions(value: &Nat, word: &str) -> Result<CodecAssertions, String> {
    let bits = cell_bits(word)?;
    let support_from_bits: Vec<Nat> = bits
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(i, bit)| bit.then(|| nat_from_index(i)))
        .collect();

    let bits_text = bits_le_string(&bits);
    let reversed = if bits_text.is_empty() {
        "0".to_string()
    } else {
        bits_text.chars().rev().collect()
    };

    let decoded = decode(word).map_err(|e| e.to_string())?;
    let encoded_bitlength = nat_from_index(bits.len());

    Ok(CodecAssertions {
        support: support_from_bits == bit_support(value),
        binary_reverse: reversed == value.binary_string(),
        word_glyph_map: encode_cell_binary(value) == word,
        roundtrip: decoded.value == *value && encode_cell_binary(&decoded.value) == word,
        bitlength: bitlength(value) == encoded_bitlength,
    })
}

pub fn analyze(
    value: &Nat,
    divisor_bound: Option<DivisorBoundCertificate>,
) -> Result<StructuralAnalysis, String> {
    let word = encode_cell_binary(value);
    let bits = cell_bits(&word)?;
    let assertions = codec_assertions(value, &word)?;
    if !assertions.all() {
        return Err("cell-binary codec assertion failed".to_string());
    }

    let decomp = decomp_2k(value);
    let residue = residue_pow2(value, &decomp.k);
    if residue != decomp.prefix {
        return Err("decomp_2k residue assertion failed".to_string());
    }

    let v2_read = v2(value);
    Ok(StructuralAnalysis {
        value: value.clone(),
        word,
        binary: value.binary_string(),
        bits_le: bits_le_string(&bits),
        support: bit_support(value),
        polynomial: polynomial_string(value),
        assertions,
        v2: v2_read.clone(),
        v2_plus_one: v2_plus_one(value),
        v2_minus_one: v2_minus_one(value),
        popcount: popcount(value),
        bitlength: bitlength(value),
        runs: runs(value),
        gaps: gaps(value),
        odd_part: odd_part(value),
        shift_factor: v2_read,
        decomp_2k: decomp,
        residue_2k: residue,
        window_width: nat_from_index(bits.len()),
        period: exact_window_period(&bits),
        divisor_bound,
    })
}

fn optional_v2(value: &Option<V2>) -> String {
    value
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_else(|| "undefined".to_string())
}

pub fn render(analysis: &StructuralAnalysis) -> String {
    let pass = |x: bool| if x { "PASS" } else { "FAIL" };
    let period = analysis
        .period
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_else(|| "none".to_string());
    let factor_bound = analysis
        .divisor_bound
        .as_ref()
        .map(|c| c.bound.to_string())
        .unwrap_or_else(|| "uncertified".to_string());
    let tested = analysis
        .divisor_bound
        .as_ref()
        .map(|c| c.tested_primes.to_string())
        .unwrap_or_else(|| "none".to_string());
    let cert_aperture = analysis
        .divisor_bound
        .as_ref()
        .map(|c| format!("2^{}", c.aperture_width))
        .unwrap_or_else(|| "none".to_string());

    format!(
        "family                     cell-binary\n\
         value                      {}\n\
         word                       {}\n\
         binary                     {}\n\
         bits-le                    {}\n\
         support                    {}\n\
         polynomial                 {}\n\
         assert.support             {}\n\
         assert.binary-reverse      {}\n\
         assert.word-glyph-map      {}\n\
         assert.roundtrip           {}\n\
         assert.bitlength           {}\n\
         primitive.v2               {}\n\
         primitive.v2+1             {}\n\
         primitive.v2-1             {}\n\
         primitive.popcount         {}\n\
         primitive.bitlength        {}\n\
         primitive.runs             {}\n\
         primitive.gaps             {}\n\
         primitive.odd-part         {}\n\
         primitive.shift-factor     {}\n\
         composite.decomp-k         {}\n\
         composite.decomp-prefix    {}\n\
         composite.decomp-remainder {}\n\
         composite.residue-2^k      {}\n\
         window.width               {}\n\
         window.aperture            2^{}\n\
         window.period              {}\n\
         negative.factor-bound      {}\n\
         negative.tested-primes     {}\n\
         negative.cert-aperture     {}\n",
        analysis.value,
        analysis.word,
        analysis.binary,
        analysis.bits_le,
        support_string(&analysis.support),
        analysis.polynomial,
        pass(analysis.assertions.support),
        pass(analysis.assertions.binary_reverse),
        pass(analysis.assertions.word_glyph_map),
        pass(analysis.assertions.roundtrip),
        pass(analysis.assertions.bitlength),
        analysis.v2,
        analysis.v2_plus_one,
        optional_v2(&analysis.v2_minus_one),
        analysis.popcount,
        analysis.bitlength,
        intervals_string(&analysis.runs),
        intervals_string(&analysis.gaps),
        analysis.odd_part,
        analysis.shift_factor,
        analysis.decomp_2k.k,
        analysis.decomp_2k.prefix,
        analysis.decomp_2k.remainder,
        analysis.residue_2k,
        analysis.window_width,
        analysis.window_width,
        period,
        factor_bound,
        tested,
        cert_aperture,
    )
}

/// LTE read for odd `a > 1` and positive even `m`:
/// `v₂(a^m-1)=v₂(a-1)+v₂(a+1)+v₂(m)-1`.
pub fn lte_2(a: &Nat, m: &Nat) -> Result<Nat, String> {
    if a.is_zero() || !a.bits_le().first().copied().unwrap_or(false) {
        return Err("lte2 requires positive odd a".to_string());
    }
    if a == &Nat::one() {
        return Err("lte2 finite read requires a > 1".to_string());
    }
    if m.is_zero() || m.bits_le().first().copied().unwrap_or(false) {
        return Err("lte2 requires positive even m".to_string());
    }

    let a_minus = a.sub(&Nat::one()).expect("a > 1");
    let a_plus = a.add(&Nat::one());
    let va = match v2(&a_minus) {
        V2::Finite(n) => n,
        V2::Infinity => return Err("unexpected infinite v2(a-1)".to_string()),
    };
    let vb = match v2(&a_plus) {
        V2::Finite(n) => n,
        V2::Infinity => return Err("unexpected infinite v2(a+1)".to_string()),
    };
    let vm = match v2(m) {
        V2::Finite(n) => n,
        V2::Infinity => return Err("unexpected infinite v2(m)".to_string()),
    };
    va.add(&vb)
        .add(&vm)
        .sub(&Nat::one())
        .ok_or_else(|| "lte2 underflow".to_string())
}

fn parse_input(raw: &str) -> Result<Nat, String> {
    if raw.starts_with('⊢') {
        let reading = decode(raw).map_err(|e| e.to_string())?;
        if reading.family != Family::CellBinary {
            return Err("analyze accepts cell-binary words or decimal naturals".to_string());
        }
        Ok(reading.value)
    } else {
        Nat::from_decimal(raw)
            .ok_or_else(|| format!("not a natural number or cell-binary word: {raw}"))
    }
}

pub fn selftest_report() -> Result<String, String> {
    let mut out = String::new();
    let mut ok = true;

    let n = Nat::from_u64(21);
    let analysis = analyze(&n, None)?;
    let structural = analysis.period == Some(Nat::from_u64(2))
        && analysis.popcount == Nat::from_u64(3)
        && analysis.assertions.all();
    ok &= structural;
    out.push_str(&format!(
        "analyzer   21 structural reads  {}\n",
        if structural { "PASS" } else { "FAIL" }
    ));

    let nines = Nat::from_decimal("999999")
        .ok_or_else(|| "internal analyzer decimal parse failure".to_string())?;
    let d = decomp_2k(&nines);
    let decomp = d.k == Nat::from_u64(6)
        && d.prefix == Nat::from_u64(63)
        && d.remainder == Nat::from_u64(15624);
    ok &= decomp;
    out.push_str(&format!(
        "decomp     10^6-1 split  {}\n",
        if decomp { "PASS" } else { "FAIL" }
    ));

    let lte = lte_2(&Nat::from_u64(5), &Nat::from_u64(144))? == Nat::from_u64(6);
    ok &= lte;
    out.push_str(&format!(
        "lte2       v2(5^144-1)=6  {}\n",
        if lte { "PASS" } else { "FAIL" }
    ));

    if ok { Ok(out) } else { Err(out) }
}

pub fn help_addendum() -> &'static str {
    "godel analyze <natural-number|cell-binary-word>\n\
     godel lte2 <odd-a> <even-m>\n"
}

pub fn command(args: &[&str]) -> Result<String, String> {
    match args.first().copied() {
        Some("analyze") => {
            let raw = args
                .get(1)
                .ok_or_else(|| "godel analyze <natural-number|cell-binary-word>".to_string())?;
            let value = parse_input(raw)?;
            Ok(render(&analyze(&value, None)?))
        }
        Some("lte2") => {
            if args.len() != 3 {
                return Err("godel lte2 <odd-a> <even-m>".to_string());
            }
            let a = Nat::from_decimal(args[1])
                .ok_or_else(|| format!("not a natural number: {}", args[1]))?;
            let m = Nat::from_decimal(args[2])
                .ok_or_else(|| format!("not a natural number: {}", args[2]))?;
            let value = lte_2(&a, &m)?;
            Ok(format!(
                "a          {a}\n\
                 m          {m}\n\
                 rule       v2(a^m-1)=v2(a-1)+v2(a+1)+v2(m)-1\n\
                 value      {value}\n"
            ))
        }
        _ => Err("godel analyzer command must be analyze or lte2".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitive_reads_match_bits() {
        let n = Nat::from_u64(40); // 101000₂
        assert_eq!(v2(&n), V2::Finite(Nat::from_u64(3)));
        assert_eq!(v2_plus_one(&n), V2::Finite(Nat::zero()));
        assert_eq!(v2_minus_one(&n), Some(V2::Finite(Nat::zero())));
        assert_eq!(popcount(&n), Nat::from_u64(2));
        assert_eq!(bitlength(&n), Nat::from_u64(6));
        assert_eq!(odd_part(&n), Nat::from_u64(5));
        assert_eq!(runs(&n).len(), 2);
        assert_eq!(gaps(&n).len(), 2);
    }

    #[test]
    fn codec_assertions_hold_for_examples_and_huge_value() {
        for raw in [
            "3",
            "7",
            "10",
            "21",
            "115792089237316195423570985008687907853269984665640564039457584007913129639936",
        ] {
            let n = Nat::from_decimal(raw).unwrap();
            let word = encode_cell_binary(&n);
            assert!(codec_assertions(&n, &word).unwrap().all());
        }
    }

    #[test]
    fn decimal_nines_expose_2adic_decomposition() {
        let n = Nat::from_decimal("999999").unwrap();
        let d = decomp_2k(&n);
        assert_eq!(d.k, Nat::from_u64(6));
        assert_eq!(d.prefix, Nat::from_u64(63));
        assert_eq!(d.remainder, Nat::from_u64(15624));
        assert_eq!(residue_pow2(&n, &d.k), d.prefix);
    }

    #[test]
    fn periodicity_reads_21() {
        let n = Nat::from_u64(21); // bits-le 10101
        let analysis = analyze(&n, None).unwrap();
        assert_eq!(analysis.period, Some(Nat::from_u64(2)));
        assert_eq!(analysis.popcount, Nat::from_u64(3));
        assert_eq!(analysis.divisor_bound, None);
    }

    #[test]
    fn lte_read_matches_5_pow_144_case() {
        assert_eq!(
            lte_2(&Nat::from_u64(5), &Nat::from_u64(144)).unwrap(),
            Nat::from_u64(6)
        );
    }

    #[test]
    fn factor_bound_requires_explicit_certificate() {
        let n = Nat::from_u64(21);
        assert!(analyze(&n, None).unwrap().divisor_bound.is_none());
        let cert = DivisorBoundCertificate {
            bound: Nat::from_u64(97),
            tested_primes: Nat::from_u64(25),
            aperture_width: Nat::from_u64(7),
        };
        assert_eq!(
            analyze(&n, Some(cert.clone())).unwrap().divisor_bound,
            Some(cert)
        );
    }
}
