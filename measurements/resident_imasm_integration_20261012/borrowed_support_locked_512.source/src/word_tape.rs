//! WordTape — the numeral whose value IS its IMASM glyph word.
//!
//! `native_numeral::encode` writes n as ⊢ [≻⋈∈[⊤|⊥]∋]*(LSB first) ⊙⊡⊣ — one
//! closed parity branch per bit. `WordTape` carries that word verbatim, and
//! every arithmetic operation walks the word's own cells: read each cell's
//! parity arm (⊤ even, ⊥ odd), ripple the carry/borrow across cells, and
//! write cells back. No `BigUint` and no u64 limb vector appears inside any
//! operation — the glyph word is the only value. The bits an operation
//! reads are the cells' parity arms, not a conversion to another numeral
//! system; `from_bits` is the exact inverse walk that writes arms back.
//!
//! Layout (byte-identical to native_numeral::encode):
//!   ⊢            VINIT      open the value
//!   ≻⋈∈[⊤|⊥]∋   one bit    advance, chain, parity-branch, deposit, rejoin
//!   ...                      bits least-significant first
//!   ⊙⊡⊣         IMSCRIB/IFIX/TANCH  recognize, fix, close
//!   zero = ⊢⊙⊡⊣

use alloc::string::String;
use alloc::vec::Vec;
use num_bigint::BigUint;
use num_traits::{One, Zero};

const VINIT: char = '⊢';
const AFWD: char = '≻';
const CLINK: char = '⋈';
const FSPLIT: char = '∈';
const EVALT: char = '⊤';
const EVALF: char = '⊥';
const FFUSE: char = '∋';
const IMSCRIB: char = '⊙';
const IFIX: char = '⊡';
const TANCH: char = '⊣';

/// A numeral held as its own IMASM word.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WordTape(pub(crate) String);

impl Ord for WordTape {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering { self.compare(other) }
}
impl PartialOrd for WordTape {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> { Some(self.compare(other)) }
}

// ── entry/exit: the only two places BigUint appears ──────────────────────
fn biguint_to_bits(n: &BigUint) -> Vec<bool> {
    let mut v = Vec::new();
    let mut m = n.clone();
    let two = BigUint::from(2u32);
    while !m.is_zero() {
        let q = &m / &two;
        let r = &m % &two;
        v.push(!r.is_zero());
        m = q;
    }
    v
}

fn bits_to_biguint(bits: &[bool]) -> BigUint {
    let mut n = BigUint::zero();
    let mut p = BigUint::one();
    let two = BigUint::from(2u32);
    for &b in bits {
        if b { n += &p; }
        p *= &two;
    }
    n
}

// ── cell walks: read the word's parity arms, write them back ─────────────
fn trim(bits: &[bool]) -> &[bool] {
    let mut hi = bits.len();
    while hi > 0 && !bits[hi - 1] { hi -= 1; }
    &bits[..hi]
}

impl WordTape {
    fn numeral(&self) -> vox_core::fixed_point_imasm::ImasmNumeralWord {
        vox_core::fixed_point_imasm::ImasmNumeralWord::checked(self.as_word())
            .expect("WordTape retains canonical numeral framing")
    }

    pub(crate) fn parity_cells(&self) -> Vec<char> { self.vox_tape() }

    pub(crate) fn from_parity_cells(cells: &[char]) -> Option<Self> {
        if cells.iter().any(|cell| !matches!(cell, '⊤' | '⊥')) { return None; }
        Some(Self::from_vox_tape(cells))
    }
    fn vox_tape(&self) -> Vec<char> {
        self.bits_low_first().into_iter().map(|bit| if bit { EVALF } else { EVALT }).collect()
    }

    fn from_vox_tape(tape: &[char]) -> Self {
        let bits: Vec<bool> = tape.iter().map(|mark| *mark == EVALF).collect();
        Self::from_bits(&bits)
    }

    pub fn zero() -> Self { Self(String::from("⊢⊙⊡⊣")) }
    pub fn one() -> Self { Self::from_bits(&[true]) }
    pub fn from_small(mut n: u64) -> Self {
        let mut bits = Vec::new();
        while n != 0 { bits.push(n & 1 == 1); n >>= 1; }
        Self::from_bits(&bits)
    }
    /// Enter directly from the canonical IMASM numeral word. This boundary
    /// checks the word's own cells and never decodes through a host integer.
    pub fn from_canonical_word(word: &str) -> Option<Self> {
        let c: Vec<char> = word.chars().collect();
        if c == ['⊢', '⊙', '⊡', '⊣'] { return Some(Self(word.into())); }
        if c.len() < 9 || c[0] != VINIT || c[c.len()-3..] != [IMSCRIB, IFIX, TANCH] {
            return None;
        }
        let body = &c[1..c.len()-3];
        if body.is_empty() || body.len() % 5 != 0 { return None; }
        let mut bits = Vec::with_capacity(body.len()/5);
        for cell in body.chunks(5) {
            if cell[0] != AFWD || cell[1] != CLINK || cell[2] != FSPLIT || cell[4] != FFUSE {
                return None;
            }
            match cell[3] { EVALT => bits.push(false), EVALF => bits.push(true), _ => return None }
        }
        if !bits.last().copied().unwrap_or(false) { return None; }
        Some(Self(word.into()))
    }
    pub(crate) fn from_biguint(n: &BigUint) -> Self {
        if n.is_zero() { return Self::zero(); }
        let bits = biguint_to_bits(n);
        Self::from_bits(&bits)
    }
    pub(crate) fn to_biguint(&self) -> BigUint {
        bits_to_biguint(&self.bits_low_first())
    }
    pub fn is_zero(&self) -> bool { self.0 == "⊢⊙⊡⊣" }
    pub fn is_one(&self) -> bool { self.0 == "⊢≻⋈∈⊥∋⊙⊡⊣" }
    pub fn as_word(&self) -> &str { &self.0 }

    /// Walk the word's bit cells (⊢ prefix and ⊙⊡⊣ suffix dropped) and read
    /// each parity arm: ⊥ → true, ⊤ → false. Least-significant bit first.
    fn bits_low_first(&self) -> Vec<bool> {
        let c: Vec<char> = self.0.chars().collect();
        if c.len() == 4 && c[1] == IMSCRIB && c[2] == IFIX && c[3] == TANCH {
            return Vec::new();
        }
        let body = &c[1..c.len() - 3];
        let mut bits = Vec::with_capacity(body.len() / 5);
        for cell in body.chunks(5) {
            bits.push(cell[3] == EVALF);
        }
        bits
    }

    /// The inverse walk: write bits back as parity cells. Trailing high zero
    /// bits are dropped so the word is canonical (zero is ⊢⊙⊡⊣).
    fn from_bits(bits: &[bool]) -> Self {
        let bits = trim(bits);
        let mut w = String::new();
        w.push(VINIT);
        for &b in bits {
            w.push(AFWD);
            w.push(CLINK);
            w.push(FSPLIT);
            w.push(if b { EVALF } else { EVALT });
            w.push(FFUSE);
        }
        w.push(IMSCRIB);
        w.push(IFIX);
        w.push(TANCH);
        Self(w)
    }

    pub fn bit_len(&self) -> usize {
        // The validated canonical alphabet uses three UTF-8 bytes per mark:
        // one prefix, five marks per cell and three suffix marks.
        (self.0.len()-12)/15
    }
    pub fn is_odd(&self) -> bool {
        self.bit_at(0)
    }
    pub fn bit_at(&self, i: usize) -> bool {
        if i >= self.bit_len() { return false; }
        // Read the bound parity glyph at its cell address. No value is
        // reconstructed and no preceding cell needs to be walked again.
        &self.0.as_bytes()[12+15*i..15+15*i] == "⊥".as_bytes()
    }

    /// Low `k` bit-cells — x mod 2^k — read from the value's own word, not a
    /// conversion to another numeral system. `k` past the bit length returns
    /// the value unchanged (its low bits are all of it).
    pub fn truncate(&self, k: usize) -> Self {
        let bits = self.bits_low_first();
        let k = k.min(bits.len());
        Self::from_bits(&bits[..k])
    }
    pub(crate) fn interlace(&self, other: &Self) -> Self {
        let a = self.bits_low_first();
        let b = other.bits_low_first();
        let mut bits = Vec::with_capacity(a.len().max(b.len()) * 2);
        for i in 0..a.len().max(b.len()) {
            bits.push(a.get(i).copied().unwrap_or(false));
            bits.push(b.get(i).copied().unwrap_or(false));
        }
        Self::from_bits(&bits)
    }
    pub(crate) fn deinterlace(&self) -> (Self, Self) {
        let bits = self.bits_low_first();
        let mut a = Vec::with_capacity((bits.len()+1)/2);
        let mut b = Vec::with_capacity(bits.len()/2);
        for (i, bit) in bits.into_iter().enumerate() {
            if i % 2 == 0 { a.push(bit); } else { b.push(bit); }
        }
        (Self::from_bits(&a), Self::from_bits(&b))
    }
}

// ── bit walks shared by the arithmetic (no BigUint, no limbs) ────────────
fn bits_add(a: &[bool], b: &[bool]) -> Vec<bool> {
    let n = a.len().max(b.len());
    let mut out = Vec::with_capacity(n + 1);
    let mut carry = false;
    for i in 0..n {
        let x = a.get(i).copied().unwrap_or(false);
        let y = b.get(i).copied().unwrap_or(false);
        out.push((x ^ y) ^ carry);
        carry = (x & y) | (x & carry) | (y & carry);
    }
    if carry { out.push(true); }
    out
}

/// a − b. None if a < b (borrow runs out).
fn bits_sub(a: &[bool], b: &[bool]) -> Option<Vec<bool>> {
    let n = a.len().max(b.len());
    let mut out = Vec::with_capacity(n);
    let mut borrow = false;
    for i in 0..n {
        let x = a.get(i).copied().unwrap_or(false);
        let y = b.get(i).copied().unwrap_or(false);
        let (d, bo) = match (x, y, borrow) {
            (false, false, false) => (false, false),
            (false, false, true) => (true, true),
            (false, true, false) => (true, true),
            (false, true, true) => (false, true),
            (true, false, false) => (true, false),
            (true, false, true) => (false, false),
            (true, true, false) => (false, false),
            (true, true, true) => (true, true),
        };
        out.push(d);
        borrow = bo;
    }
    if borrow { None } else { Some(out) }
}

fn bits_cmp(a: &[bool], b: &[bool]) -> core::cmp::Ordering {
    let a = trim(a);
    let b = trim(b);
    if a.len() != b.len() { return a.len().cmp(&b.len()); }
    for i in (0..a.len()).rev() {
        if a[i] != b[i] { return a[i].cmp(&b[i]); }
    }
    core::cmp::Ordering::Equal
}

/// Schoolbook shift-and-add on the bit list.
fn bits_mul(a: &[bool], b: &[bool]) -> Vec<bool> {
    let a = trim(a);
    let b = trim(b);
    let mut acc = vec![false; a.len() + b.len()];
    for i in 0..a.len() {
        if a[i] {
            let mut carry = false;
            for j in 0..b.len() {
                let idx = i + j;
                let x = acc[idx];
                let y = b[j];
                acc[idx] = (x ^ y) ^ carry;
                carry = (x & y) | (x & carry) | (y & carry);
            }
            let mut k = i + b.len();
            while carry {
                let x = acc[k];
                acc[k] = x ^ carry;
                carry = x & carry;
                k += 1;
            }
        }
    }
    trim(&acc).to_vec()
}

/// Restoring long division. None if b is zero.
fn bits_divmod(a: &[bool], b: &[bool]) -> Option<(Vec<bool>, Vec<bool>)> {
    let bt = trim(b);
    if bt.is_empty() { return None; }
    let at = trim(a);
    let mut rem: Vec<bool> = Vec::new();
    let mut q = Vec::with_capacity(at.len());
    for i in (0..at.len()).rev() {
        rem.insert(0, at[i]); // rem = rem·2 + bring-down bit
        match bits_sub(&rem, bt) {
            Some(r) => { rem = r; q.push(true); }
            None => q.push(false),
        }
    }
    q.reverse();
    Some((trim(&q).to_vec(), rem))
}

/// Floor square root by Newton descent from 2^ceil(bits/2), with a final
/// safety sweep so the result is the exact floor under floor division.
fn bits_isqrt(a: &[bool]) -> Vec<bool> {
    let at = trim(a);
    if at.is_empty() { return Vec::new(); }
    let bl = at.len();
    let mut x = vec![false; bl / 2 + 2];
    x[bl / 2 + 1] = true; // 2^ceil(bl/2), strictly above sqrt
    loop {
        let (q, _) = bits_divmod(&at, &x).unwrap();
        let y = bits_add(&x, &q)[1..].to_vec(); // (x + a/x) >> 1
        if bits_cmp(&y, &x) != core::cmp::Ordering::Less { break; }
        x = y;
    }
    while bits_cmp(&bits_mul(&x, &x), &at) == core::cmp::Ordering::Greater {
        x = bits_sub(&x, &[true]).unwrap();
    }
    while bits_cmp(&bits_mul(&bits_add(&x, &[true]), &bits_add(&x, &[true])), &at)
        != core::cmp::Ordering::Greater {
        x = bits_add(&x, &[true]);
    }
    trim(&x).to_vec()
}

// ── arithmetic: word-walks over the cell bits ────────────────────────────
impl WordTape {
    pub fn add(&self, b: &Self) -> Self {
        if self.is_zero() { return b.clone(); }
        if b.is_zero() { return self.clone(); }
        Self(vox_core::fixed_point_word_arithmetic::add(&self.numeral(), &b.numeral())
            .expect("canonical word addition").as_str().into())
    }
    pub fn sub(&self, b: &Self) -> Option<Self> {
        if b.is_zero() { return Some(self.clone()); }
        if self == b { return Some(Self::zero()); }
        if !self.ge(b) { return None; }
        Some(Self(vox_core::fixed_point_word_arithmetic::subtract(&self.numeral(), &b.numeral())
            .expect("ordered canonical word subtraction").as_str().into()))
    }
    pub fn mul(&self, b: &Self) -> Self {
        if self.is_zero() || b.is_zero() { return Self::zero(); }
        if self.is_one() { return b.clone(); }
        if b.is_one() { return self.clone(); }
        Self(vox_core::fixed_point_word_arithmetic::multiply(&self.numeral(), &b.numeral())
            .expect("canonical word multiplication").as_str().into())
    }
    pub fn divmod(&self, b: &Self) -> Option<(Self, Self)> {
        if b.is_zero() { return None; }
        let (q, r) = vox_core::fixed_point_word_arithmetic::divide(&self.numeral(), &b.numeral()).ok()?;
        Some((Self(q.as_str().into()), Self(r.as_str().into())))
    }

    /// Modular powers consume the exponent's own numeral cells. Every product
    /// and reduction remains on the existing IMASM arithmetic rails.
    pub fn mod_pow(&self, exponent: &Self, modulus: &Self) -> Option<Self> {
        let mut result = Self::one().divmod(modulus)?.1;
        let mut base = self.divmod(modulus)?.1;
        for bit in 0..exponent.bit_len() {
            if exponent.bit_at(bit) { result = result.mul(&base).divmod(modulus)?.1; }
            base = base.mul(&base).divmod(modulus)?.1;
        }
        Some(result)
    }

    pub fn gcd(&self, other: &Self) -> Self {
        let (mut left, mut right) = (self.clone(),other.clone());
        while !right.is_zero() {
            let remainder = left.divmod(&right).expect("nonzero word divisor").1;
            left = right; right = remainder;
        }
        left
    }
    pub fn ge(&self, b: &Self) -> bool {
        self.compare(b) != core::cmp::Ordering::Less
    }
    pub fn gt(&self, b: &Self) -> bool {
        self.compare(b) == core::cmp::Ordering::Greater
    }

    pub fn compare(&self, b: &Self) -> core::cmp::Ordering {
        let width = self.bit_len();
        let other_width = b.bit_len();
        if width != other_width { return width.cmp(&other_width); }
        if self == b { return core::cmp::Ordering::Equal; }
        vox_core::fixed_point_word_arithmetic::compare(&self.numeral(), &b.numeral())
            .expect("canonical word comparison")
    }

    /// Shift left: insert n empty low bit-cells before the existing ones.
    pub fn shl(&self, bits: usize) -> Self {
        let mut v = self.bits_low_first();
        let mut out = vec![false; bits];
        out.append(&mut v);
        Self::from_bits(&out)
    }
    /// Shift right one: drop the least-significant bit-cell.
    pub fn shr1(&self) -> Self {
        let v = self.bits_low_first();
        if v.is_empty() { return Self::zero(); }
        Self::from_bits(&v[1..])
    }
    pub fn shr(&self, count: usize) -> Self {
        let cells = self.bits_low_first();
        Self::from_bits(&cells[count.min(cells.len())..])
    }
    /// A one-hot bit at a binary depth.
    pub fn one_at(bit: usize) -> Self {
        let mut v = vec![false; bit + 1];
        v[bit] = true;
        Self::from_bits(&v)
    }

    /// Both inputs canonical below m, so one compare/subtract is the full
    /// reduction walk.
    pub(crate) fn add_mod_canonical(&self, b: &Self, m: &Self) -> Self {
        let sum = self.add(b);
        if sum.ge(m) { sum.sub(m).unwrap() } else { sum }
    }

    /// Newton square root, entirely on the word.
    pub fn isqrt(&self) -> Self {
        Self(vox_core::fixed_point_word_arithmetic::sqrt_floor(&self.numeral())
            .expect("canonical word square root").as_str().into())
    }
    /// Concavity-bounded Newton continuation from a lower floor root.
    pub(crate) fn isqrt_from(&self, lower: &Self) -> Self {
        let bits = self.bits_low_first();
        let at = trim(&bits);
        if at.is_empty() { return Self::zero(); }
        let one = Self::one();
        let two = Self::from_small(2);
        let lower_sq = lower.mul(lower);
        let rise = self.sub(&lower_sq).unwrap_or_else(Self::zero);
        let denom = lower.add(lower).add(&one);
        let jump = rise.divmod(&denom).map(|x| x.0).unwrap_or_else(Self::zero);
        let mut x = lower.add(&jump).add(&one);
        loop {
            let q = self.divmod(&x).unwrap().0;
            let y = x.add(&q).divmod(&two).unwrap().0;
            if y.ge(&x) { break; }
            x = y;
        }
        while x.mul(&x).gt(self) { x = x.sub(&one).unwrap(); }
        x
    }

    /// Solve the nonlinear remainder of the carried Fermat root; statistics
    /// are word counts in the order q=u, q=u−1, general, bracket width,
    /// bounded refinements, divisions.
    pub(crate) fn root_deficit_from(base: &Self, excess: &Self) -> (Self, [Self; 6]) {
        let zero = Self::zero();
        let one = Self::one();
        let two_b = base.add(base);
        let (u, r) = excess.divmod(&two_b).unwrap();
        if u.is_zero() {
            return (zero, [one, Self::zero(), Self::zero(), Self::zero(), Self::zero(), Self::one()]);
        }
        let u2 = u.mul(&u);
        if !u2.gt(&r) {
            return (u, [one, Self::zero(), Self::zero(), Self::zero(), Self::zero(), Self::one()]);
        }
        let x = u2.sub(&r).unwrap();
        let two_u = u.add(&u);
        let d1_capacity = two_b.add(&two_u).sub(&one).unwrap();
        if !x.gt(&d1_capacity) {
            return (u.sub(&one).unwrap(), [Self::zero(), one, Self::zero(), Self::zero(), Self::zero(), Self::one()]);
        }
        let (mut lo, _) = excess.divmod(&two_b.add(&u)).unwrap();
        let mut hi = u;
        let mut lo_value = lo.mul(&two_b.add(&lo));
        debug_assert!(!lo_value.gt(excess));
        debug_assert!(hi.mul(&two_b.add(&hi)).gt(excess));
        let bracket = hi.sub(&lo).unwrap();
        let mut refinements = Self::zero();
        while hi.sub(&lo).unwrap().gt(&one) {
            let half = hi.sub(&lo).unwrap().shr1();
            let mid = lo.add(&half);
            let mid_value = lo_value.add(&half.mul(&two_b.add(&lo.add(&lo)).add(&half)));
            if mid_value.gt(excess) { hi = mid; } else { lo = mid; lo_value = mid_value; }
            debug_assert!(!lo_value.gt(excess));
            debug_assert!(hi.mul(&two_b.add(&hi)).gt(excess));
            refinements = refinements.add(&one);
        }
        (lo, [Self::zero(), Self::zero(), one, bracket, refinements, Self::from_small(2)])
    }

    /// Close the unnested excess into whole recursive square-shell blocks,
    /// by tape shifts, additions, comparisons and subtractions only.
    pub(crate) fn nest_into_square_frontier(excess: &Self, base: &Self) -> (Self, Self, Self) {
        let mut remainder = excess.clone();
        let mut nested_base = base.clone();
        let mut q = Self::zero();
        let mut blocks = Self::zero();
        for r in (0..=excess.bit_len() / 2).rev() {
            let width = Self::one_at(r);
            let capacity = nested_base.shl(r + 1).add(&Self::one_at(2 * r));
            if remainder.ge(&capacity) {
                remainder = remainder.sub(&capacity).unwrap();
                nested_base = nested_base.add(&width);
                q = q.add(&width);
                blocks = blocks.add(&Self::one());
            }
        }
        debug_assert_eq!(nested_base, base.add(&q));
        (q, remainder, blocks)
    }
}

/// Signed resident amplitude carried by positive and negative IMASM numeral
/// rails. Cancellation walks those words; host integers appear only at the
/// existing import/report boundaries.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct SignedWordTape {
    positive: WordTape,
    negative: WordTape,
}

impl SignedWordTape {
    #[cfg(feature = "hosted")]
    pub(crate) fn prepared_record(&self) -> serde_json::Value {
        serde_json::json!({"direction":if self.is_negative() { "≺" } else { "≻" },
            "magnitude_word":self.magnitude().as_word()})
    }
    #[cfg(feature = "hosted")]
    pub(crate) fn from_prepared_record(record: &serde_json::Value) -> Result<Self,String> {
        let magnitude = WordTape::from_canonical_word(record["magnitude_word"].as_str()
            .ok_or("prepared amplitude lost its IMASM word")?).ok_or("invalid prepared amplitude word")?;
        let negative = match record["direction"].as_str() {
            Some("≺") => true, Some("≻") => false, _ => return Err("invalid prepared amplitude direction".into()),
        };
        Ok(Self::from_magnitude(magnitude,negative))
    }
    pub(crate) fn from_magnitude(magnitude: WordTape, negative: bool) -> Self {
        if negative {
            Self { positive: WordTape::zero(), negative: magnitude }
        } else { Self { positive: magnitude, negative: WordTape::zero() } }
    }
    pub(crate) fn from_small(value: u64) -> Self {
        Self::from_magnitude(WordTape::from_small(value), false)
    }
    pub(crate) fn abs(&self) -> Self { Self::from_magnitude(self.magnitude().clone(), false) }
    fn rails(positive: WordTape, negative: WordTape) -> Self {
        if negative.is_zero() { return Self { positive,negative }; }
        if positive.is_zero() { return Self { positive,negative }; }
        if positive.ge(&negative) {
            Self { positive: positive.sub(&negative).unwrap(), negative: WordTape::zero() }
        } else {
            Self { positive: WordTape::zero(), negative: negative.sub(&positive).unwrap() }
        }
    }
    pub(crate) fn zero() -> Self {
        Self { positive: WordTape::zero(), negative: WordTape::zero() }
    }
    pub(crate) fn from_bigint(value: &num_bigint::BigInt) -> Self {
        let magnitude = WordTape::from_biguint(value.magnitude());
        if value.sign() == num_bigint::Sign::Minus {
            Self { positive: WordTape::zero(), negative: magnitude }
        } else { Self { positive: magnitude, negative: WordTape::zero() } }
    }
    pub(crate) fn to_bigint(&self) -> num_bigint::BigInt {
        let sign = if self.is_negative() { num_bigint::Sign::Minus } else { num_bigint::Sign::Plus };
        num_bigint::BigInt::from_biguint(sign, self.magnitude().to_biguint())
    }
    pub(crate) fn is_zero(&self) -> bool { self.positive.is_zero() && self.negative.is_zero() }
    pub(crate) fn is_negative(&self) -> bool { !self.negative.is_zero() }
    pub(crate) fn magnitude(&self) -> &WordTape {
        if self.is_negative() { &self.negative } else { &self.positive }
    }
    pub(crate) fn add(&self, other: &Self) -> Self {
        Self::rails(self.positive.add(&other.positive), self.negative.add(&other.negative))
    }
    pub(crate) fn negate(&self) -> Self {
        Self { positive: self.negative.clone(), negative: self.positive.clone() }
    }
    pub(crate) fn sub(&self, other: &Self) -> Self { self.add(&other.negate()) }
    pub(crate) fn mul(&self, other: &Self) -> Self {
        let magnitude = self.magnitude().mul(other.magnitude());
        if self.is_negative() ^ other.is_negative() {
            Self { positive: WordTape::zero(), negative: magnitude }
        } else { Self { positive: magnitude, negative: WordTape::zero() } }
    }
    pub(crate) fn div(&self, other: &Self) -> Self {
        let magnitude = self.magnitude().divmod(other.magnitude())
            .expect("signed word division requires a nonzero divisor").0;
        if self.is_negative() ^ other.is_negative() {
            Self { positive: WordTape::zero(), negative: magnitude }
        } else { Self { positive: magnitude, negative: WordTape::zero() } }
    }
    pub(crate) fn rem(&self, other: &Self) -> Self {
        let remainder = self.magnitude().divmod(other.magnitude())
            .expect("signed word remainder requires a nonzero divisor").1;
        Self::from_magnitude(remainder, self.is_negative())
    }
    /// Signed right shift retains the floor convention of the existing fixed
    /// precision interface, including a negative value with low residue cells.
    pub(crate) fn shr(&self, count: usize) -> Self {
        let mut magnitude = self.magnitude().shr(count);
        if self.is_negative() && !self.magnitude().truncate(count).is_zero() {
            magnitude = magnitude.add(&WordTape::one());
        }
        Self::from_magnitude(magnitude, self.is_negative())
    }
    pub(crate) fn shl(&self, count: usize) -> Self {
        Self { positive: self.positive.shl(count), negative: self.negative.shl(count) }
    }
    pub(crate) fn rounded_shr(&self, count: usize) -> Self {
        if count == 0 { return self.clone(); }
        let magnitude = self.magnitude().add(&WordTape::one_at(count - 1)).shr(count);
        if self.is_negative() {
            Self { positive: WordTape::zero(), negative: magnitude }
        } else { Self { positive: magnitude, negative: WordTape::zero() } }
    }
}

impl Ord for SignedWordTape {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        match (self.is_negative(), other.is_negative()) {
            (true, false) => core::cmp::Ordering::Less,
            (false, true) => core::cmp::Ordering::Greater,
            (true, true) => other.magnitude().compare(self.magnitude()),
            (false, false) => self.magnitude().compare(other.magnitude()),
        }
    }
}
impl PartialOrd for SignedWordTape {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> { Some(self.cmp(other)) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_traits::One;

    fn b(n: u64) -> BigUint { BigUint::from(n) }

    #[test]
    fn word_product_split_matches_dense_and_unequal_width_operands() {
        let mut random = 1729u64;
        for (left_bits,right_bits) in [(63,64),(64,65),(127,200),(512,513),(1024,65)] {
            let mut make_operand = |bits: usize| {
                let mut value = BigUint::zero();
                for bit in 0..bits {
                    random ^= random<<13; random ^= random>>7; random ^= random<<17;
                    if random&1 != 0 { value.set_bit(bit as u64,true); }
                }
                value.set_bit((bits-1) as u64,true);
                value
            };
            let left = make_operand(left_bits);
            let right = make_operand(right_bits);
            assert_eq!(WordTape::from_biguint(&left).mul(&WordTape::from_biguint(&right)).to_biguint(), &left*&right);
        }
    }

    #[test]
    fn word_square_root_retains_wide_square_and_neighbor_boundaries() {
        let root = (BigUint::one()<<256usize) + (BigUint::one()<<127usize) + BigUint::from(37u8);
        let square = &root*&root;
        for (input, expected) in [
            (square.clone(),root.clone()),
            (&square+BigUint::one(),root.clone()),
            (&square-BigUint::one(),&root-BigUint::one()),
        ] {
            assert_eq!(WordTape::from_biguint(&input).isqrt().to_biguint(), expected);
        }
        let word = WordTape::from_biguint(&root);
        for bit in 0..300 { assert_eq!(word.bit_at(bit),root.bit(bit as u64)); }
        assert_eq!(word.bit_len(),257);
        assert!(!WordTape::zero().bit_at(0));
    }

    #[test]
    fn signed_residual_rails_preserve_wide_arithmetic_and_rounding() {
        use num_bigint::BigInt;
        let a = -(BigInt::one() << 223usize) + BigInt::from(17);
        let b = (BigInt::one() << 219usize) + BigInt::from(31);
        let left = SignedWordTape::from_bigint(&a);
        let right = SignedWordTape::from_bigint(&b);
        assert_eq!(left.add(&right).to_bigint(), &a + &b);
        assert_eq!(left.sub(&right).to_bigint(), &a - &b);
        assert_eq!(left.mul(&right).to_bigint(), &a * &b);
        assert_eq!(left.div(&right).to_bigint(), &a / &b);
        assert!(left.add(&left.negate()).is_zero());
        assert!(WordTape::from_canonical_word(left.magnitude().as_word()).is_some());
        for value in [-7i32, -6, -5, 0, 5, 6, 7] {
            let word = SignedWordTape::from_bigint(&BigInt::from(value));
            let rounded = if value < 0 { -((-value + 2) / 4) } else { (value + 2) / 4 };
            assert_eq!(word.rounded_shr(2).to_bigint(), BigInt::from(rounded));
        }
    }

    #[test]
    fn word_division_closes_a_wide_operand_and_remainder() {
        let dividend = WordTape::one_at(223).add(&WordTape::from_small(19));
        let divisor = WordTape::one_at(111).add(&WordTape::from_small(7));
        let (quotient, remainder) = dividend.divmod(&divisor).unwrap();
        assert_eq!(quotient.mul(&divisor).add(&remainder), dividend);
        assert!(!remainder.ge(&divisor));
        // Independent host arithmetic is confined to this comparison instrument.
        let expected = dividend.to_biguint();
        let d = divisor.to_biguint();
        assert_eq!(quotient.to_biguint(), &expected / &d);
        assert_eq!(remainder.to_biguint(), &expected % &d);
    }

    fn check_roundtrip(vals: &[u64]) {
        for &v in vals {
            let t = WordTape::from_small(v);
            assert_eq!(t.to_biguint(), b(v), "roundtrip {v}");
            // canonical zero is the bare word, everything else has cells
            if v == 0 { assert_eq!(t.0, "⊢⊙⊡⊣"); }
        }
    }

    #[test]
    fn canonical_input_stays_in_imasm_word_form() {
        let w = "⊢≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋⊙⊡⊣";
        let n = WordTape::from_canonical_word(w).unwrap();
        assert_eq!(n.as_word(), w);
        assert_eq!(n.bit_len(), 7);
        assert_eq!(n.mul(&WordTape::from_small(1)).as_word(), w);
        assert!(WordTape::from_canonical_word("91").is_none());
        assert!(WordTape::from_canonical_word("⊢≻⋈∈⊤∋⊙⊡⊣").is_none());
    }

    #[test]
    fn roundtrip_through_the_word() {
        check_roundtrip(&[0, 1, 2, 3, 4, 5, 7, 8, 15, 16, 31, 32, 63, 64, 127, 255, 256, 1024, 4032, 65535]);
    }

    #[test]
    fn add_is_exact() {
        for a in 0..40u64 {
            for c in 0..40u64 {
                let r = WordTape::from_small(a).add(&WordTape::from_small(c));
                assert_eq!(r.to_biguint(), b(a + c), "add {a}+{c}");
            }
        }
    }

    #[test]
    fn sub_is_exact_or_none() {
        for a in 0..40u64 {
            for c in 0..40u64 {
                let r = WordTape::from_small(a).sub(&WordTape::from_small(c));
                if c > a { assert!(r.is_none(), "sub {a}-{c} should be none"); }
                else { assert_eq!(r.unwrap().to_biguint(), b(a - c), "sub {a}-{c}"); }
            }
        }
    }

    #[test]
    fn mul_is_exact() {
        for a in 0..25u64 {
            for c in 0..25u64 {
                let r = WordTape::from_small(a).mul(&WordTape::from_small(c));
                assert_eq!(r.to_biguint(), b(a * c), "mul {a}*{c}");
            }
        }
    }

    #[test]
    fn divmod_is_exact() {
        for a in 0..50u64 {
            for m in 1..50u64 {
                let (q, r) = WordTape::from_small(a).divmod(&WordTape::from_small(m)).unwrap();
                assert_eq!(q.to_biguint(), b(a / m), "div {a}/{m}");
                assert_eq!(r.to_biguint(), b(a % m), "mod {a}/{m}");
            }
        }
    }

    #[test]
    fn isqrt_is_floor_sqrt() {
        for v in 0..200u64 {
            let r = WordTape::from_small(v).isqrt();
            let x = r.to_biguint();
            let xs = x.clone() * x.clone();
            let xp1 = (x.clone() + BigUint::one()) * (x.clone() + BigUint::one());
            assert!(xs <= b(v), "isqrt({v})^2 <= v");
            assert!(xp1 > b(v), "(isqrt({v})+1)^2 > v");
        }
        for &sq in &[1u64, 4, 9, 16, 25, 36, 49, 64, 81, 100, 4032 * 4032 / 1] {
            let r = WordTape::from_small(sq).isqrt();
            // only assert exact for perfect squares
            if sq < 400 { assert_eq!(r.to_biguint(), BigUint::from((sq as f64).sqrt() as u64)); }
        }
    }

    #[test]
    fn add_mod_canonical_stays_below_modulus() {
        let m = WordTape::from_small(64);
        for a in 0..64u64 {
            for c in 0..64u64 {
                let r = WordTape::from_small(a).add_mod_canonical(&WordTape::from_small(c), &m);
                assert_eq!(r.to_biguint(), b((a + c) % 64), "amc {a}+{c} mod 64");
            }
        }
    }

    #[test]
    fn wide_roundtrip_and_arithmetic() {
        // a 128-bit-scale value exercised through add/mul/divmod/isqrt
        let big = BigUint::from(0xFFFFFFFFFFFFFFFFu64) * BigUint::from(0x123456789ABCDEFu64);
        let t = WordTape::from_biguint(&big);
        assert_eq!(t.to_biguint(), big);
        let doubled = t.add(&t);
        assert_eq!(doubled.to_biguint(), &big + &big);
        let squared = t.mul(&t);
        assert_eq!(squared.to_biguint(), &big * &big);
        let root = squared.isqrt();
        assert_eq!(root.to_biguint(), big, "isqrt of a perfect square recovers the root");
    }
}
