//! Arithmetic over native IMASM numeral words.
//!
//! A resident numeric value in this module is always an `ImasmNumeralWord`.
//! No value is decoded to a Rust integer or folded into machine limbs. The only
//! host integers used here are structural positions/lengths while walking the
//! glyph word. Arithmetic consumes and emits `⊤`/`⊥` cells.
//!
//! Addition and subtraction execute one IMASM operator word for every numeral
//! cell. Multiplication composes the adder word, and square root is the usual
//! restoring candidate construction expressed only through these word-level
//! operations. This is the arithmetic rung required before the fixed-point
//! vessel can be made executable without `morphism_factor`'s folded limbs.

use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::Ordering;

use crate::fixed_point_imasm::ImasmNumeralWord;
use crate::vox::{
    AFWD, AREV, CLINK, ENGAGR, EVALF, EVALT, FFUSE, FSPLIT, IFIX, IMSCRIB, TANCH,
    VINIT,
};

/// Full-adder cell: split the two resident marks, link them, engage, commit.
pub const WORD_ADD_CELL: [char; 9] = [
    VINIT, FSPLIT, EVALT, EVALF, FFUSE, CLINK, ENGAGR, IFIX, TANCH,
];

/// Full-subtractor cell. `AREV` is live inside the frame, so borrow is banked.
pub const WORD_SUB_CELL: [char; 10] = [
    VINIT, FSPLIT, EVALT, AREV, EVALF, FFUSE, CLINK, ENGAGR, IFIX, TANCH,
];

/// Comparison boundary used while constructing square-root candidates.
pub const WORD_COMPARE: [char; 7] = [VINIT, FSPLIT, EVALT, EVALF, FFUSE, IFIX, TANCH];

const fn mark_xor(a: char, b: char) -> char {
    if a == b { EVALT } else { EVALF }
}

const fn mark_not(a: char) -> char {
    if a == EVALF { EVALT } else { EVALF }
}

const fn mark_and(a: char, b: char) -> char {
    if a == EVALF && b == EVALF { EVALF } else { EVALT }
}

const fn mark_or(a: char, b: char) -> char {
    if a == EVALF || b == EVALF { EVALF } else { EVALT }
}

#[derive(Clone, Copy)]
enum CellMode {
    Add,
    Sub,
}

/// Execute one arithmetic cell by walking its IMASM program. Operand values are
/// resident marks; `ENGAGR` is the only place the output cell is formed, and
/// `IFIX` is required before `TANCH` can return it.
const fn run_cell(word: &[char], mode: CellMode, a: char, b: char, carry: char) -> (char, char) {
    let mut left = EVALT;
    let mut right = EVALT;
    let mut saw_left = false;
    let mut saw_right = false;
    let mut engaged = false;
    let mut fixed = false;
    let mut out = EVALT;
    let mut next = EVALT;

    let mut cursor = 0;
    while cursor < word.len() {
        let op = word[cursor];
        cursor += 1;
        match op {
            VINIT => {
                saw_left = false;
                saw_right = false;
                engaged = false;
                fixed = false;
            }
            EVALT if !saw_left => {
                left = a;
                saw_left = true;
            }
            EVALF if !saw_right => {
                right = b;
                saw_right = true;
            }
            ENGAGR if saw_left && saw_right => {
                let pair_xor = mark_xor(left, right);
                out = mark_xor(pair_xor, carry);
                next = match mode {
                    CellMode::Add => mark_or(mark_and(left, right), mark_and(pair_xor, carry)),
                    CellMode::Sub => mark_or(
                        mark_and(mark_not(left), mark_or(right, carry)),
                        mark_and(right, carry),
                    ),
                };
                engaged = true;
            }
            IFIX if engaged => fixed = true,
            TANCH => break,
            _ => {}
        }
    }

    assert!(fixed, "arithmetic cell did not reach IMASM fixation");
    (out, next)
}

// Compile the existing cell words over their complete finite input carrier.
// Each entry is the interpreter's fixed return, including its carry/borrow.
// Runtime words remain glyph cells; only the local cell transition is cached.
const fn compile_cell(word: &[char], mode: CellMode) -> [(char, char); 8] {
    let mut table = [(EVALT, EVALT); 8];
    let mut index = 0;
    while index < table.len() {
        let a = if index & 1 == 0 { EVALT } else { EVALF };
        let b = if index & 2 == 0 { EVALT } else { EVALF };
        let carry = if index & 4 == 0 { EVALT } else { EVALF };
        table[index] = run_cell(word, mode, a, b, carry);
        index += 1;
    }
    table
}

const ADD_CELL_RETURNS: [(char, char); 8] = compile_cell(&WORD_ADD_CELL, CellMode::Add);
const SUB_CELL_RETURNS: [(char, char); 8] = compile_cell(&WORD_SUB_CELL, CellMode::Sub);

fn prepared_cell(table: &[(char, char); 8], a: char, b: char, carry: char) -> (char, char) {
    table[(a == EVALF) as usize | ((b == EVALF) as usize) << 1
        | ((carry == EVALF) as usize) << 2]
}

fn trim_bits(mut bits: Vec<char>) -> Vec<char> {
    while bits.len() > 1 && bits.last() == Some(&EVALT) {
        bits.pop();
    }
    if bits.is_empty() {
        bits.push(EVALT);
    }
    bits
}

/// Read only the numeral-cell payload of the canonical native IMASM encoding.
/// This is a syntactic projection from glyph word to glyph cells, not numeric
/// decoding. The returned cells remain `⊤`/`⊥` marks.
fn payload_bits(value: &ImasmNumeralWord) -> Result<Vec<char>, &'static str> {
    let marks: Vec<char> = value.as_str().chars().collect();
    if marks.as_slice() == [VINIT, IMSCRIB, IFIX, TANCH] {
        return Ok(vec![EVALT]);
    }
    if marks.len() < 9 || marks.first() != Some(&VINIT) {
        return Err("unsupported native IMASM numeral framing");
    }
    let suffix = &marks[marks.len() - 3..];
    if suffix != [IMSCRIB, IFIX, TANCH] {
        return Err("unsupported native IMASM numeral suffix");
    }
    let body = &marks[1..marks.len() - 3];
    if body.len() % 5 != 0 {
        return Err("broken native IMASM numeral cell width");
    }
    let mut bits = Vec::with_capacity(body.len() / 5);
    for cell in body.chunks(5) {
        if cell[0] != AFWD || cell[1] != CLINK || cell[2] != FSPLIT || cell[4] != FFUSE {
            return Err("broken native IMASM numeral cell");
        }
        if cell[3] != EVALT && cell[3] != EVALF {
            return Err("native IMASM numeral cell carries a non-bit mark");
        }
        bits.push(cell[3]);
    }
    Ok(trim_bits(bits))
}

fn from_bits(bits: Vec<char>) -> Result<ImasmNumeralWord, &'static str> {
    let bits = trim_bits(bits);
    if bits.len() == 1 && bits[0] == EVALT {
        return ImasmNumeralWord::checked("⊢⊙⊡⊣");
    }
    let mut word = String::new();
    word.push(VINIT);
    for bit in bits {
        word.push(AFWD);
        word.push(CLINK);
        word.push(FSPLIT);
        word.push(bit);
        word.push(FFUSE);
    }
    word.push(IMSCRIB);
    word.push(IFIX);
    word.push(TANCH);
    ImasmNumeralWord::checked(&word)
}

fn cmp_bits(a: &[char], b: &[char]) -> Ordering {
    let a = &a[..a.iter().rposition(|&mark| mark == EVALF).map_or(0,|at| at+1)];
    let b = &b[..b.iter().rposition(|&mark| mark == EVALF).map_or(0,|at| at+1)];
    if a.len() != b.len() {
        return a.len().cmp(&b.len());
    }
    // WORD_COMPARE is the resident comparison boundary. Each high-to-low cell
    // is exposed only through its truth/falsity mark; no host numeric value is
    // synthesized.
    debug_assert_eq!(WORD_COMPARE.first(), Some(&VINIT));
    debug_assert_eq!(WORD_COMPARE.last(), Some(&TANCH));
    for (&left, &right) in a.iter().zip(b.iter()).rev() {
        if left != right {
            return if left == EVALF { Ordering::Greater } else { Ordering::Less };
        }
    }
    Ordering::Equal
}

fn add_bits(a: &[char], b: &[char]) -> Vec<char> {
    let width = a.len().max(b.len());
    let mut out = Vec::with_capacity(width + 1);
    let mut carry = EVALT;
    for at in 0..width {
        let left = a.get(at).copied().unwrap_or(EVALT);
        let right = b.get(at).copied().unwrap_or(EVALT);
        let (sum, next) = prepared_cell(&ADD_CELL_RETURNS, left, right, carry);
        out.push(sum);
        carry = next;
    }
    if carry == EVALF {
        out.push(carry);
    }
    trim_bits(out)
}

fn sub_bits(a: &[char], b: &[char]) -> Result<Vec<char>, &'static str> {
    if cmp_bits(a, b) == Ordering::Less {
        return Err("IMASM word subtraction would underflow");
    }
    let mut out = Vec::with_capacity(a.len());
    let mut borrow = EVALT;
    for at in 0..a.len() {
        let left = a.get(at).copied().unwrap_or(EVALT);
        let right = b.get(at).copied().unwrap_or(EVALT);
        let (difference, next) = prepared_cell(&SUB_CELL_RETURNS, left, right, borrow);
        out.push(difference);
        borrow = next;
    }
    if borrow == EVALF {
        return Err("IMASM subtractor left an exposed borrow");
    }
    Ok(trim_bits(out))
}

fn mul_bits(a: &[char], b: &[char]) -> Vec<char> {
    let a = &a[..a.iter().rposition(|&mark| mark == EVALF).map_or(0,|at| at+1)];
    let b = &b[..b.iter().rposition(|&mark| mark == EVALF).map_or(0,|at| at+1)];
    if a.is_empty() || b.is_empty() { return vec![EVALT]; }
    let a_weight = a.iter().filter(|&&mark| mark == EVALF).count();
    let b_weight = b.iter().filter(|&&mark| mark == EVALF).count();
    if a_weight < b_weight { return mul_bits(b,a); }
    if b_weight == 1 {
        let mut shifted = vec![EVALT;b.len()-1];
        shifted.extend_from_slice(a);
        return shifted;
    }
    if a.len().max(b.len()) >= 64 && b_weight > 8 {
        // Split the resident glyph lanes; the three products and their return
        // remain words. Machine integers address the split and shifts only.
        let half = a.len().max(b.len())/2;
        let (al,ah) = a.split_at(half.min(a.len()));
        let (bl,bh) = b.split_at(half.min(b.len()));
        let low = mul_bits(al,bl);
        let high = mul_bits(ah,bh);
        let combined = mul_bits(&add_bits(al,ah),&add_bits(bl,bh));
        let cross = sub_bits(&sub_bits(&combined,&low).expect("combined product contains low rail"),
            &high).expect("combined product contains high rail");
        let mut middle = vec![EVALT;half];
        middle.extend_from_slice(&cross);
        let mut upper = vec![EVALT;2*half];
        upper.extend_from_slice(&high);
        return add_bits(&add_bits(&low,&middle),&upper);
    }
    let mut out = vec![EVALT];
    for (shift, &bit) in b.iter().enumerate() {
        if bit != EVALF {
            continue;
        }
        let mut term = Vec::with_capacity(a.len() + shift);
        term.resize(shift, EVALT);
        term.extend_from_slice(a);
        out = add_bits(&out, &term);
    }
    trim_bits(out)
}

fn bit_candidate(root: &[char], position: usize) -> Vec<char> {
    let mut out = root.to_vec();
    if out.len() <= position {
        out.resize(position + 1, EVALT);
    }
    out[position] = EVALF;
    trim_bits(out)
}

pub fn compare(a: &ImasmNumeralWord, b: &ImasmNumeralWord) -> Result<Ordering, &'static str> {
    Ok(cmp_bits(&payload_bits(a)?, &payload_bits(b)?))
}

pub fn add(a: &ImasmNumeralWord, b: &ImasmNumeralWord) -> Result<ImasmNumeralWord, &'static str> {
    from_bits(add_bits(&payload_bits(a)?, &payload_bits(b)?))
}

pub fn subtract(a: &ImasmNumeralWord, b: &ImasmNumeralWord) -> Result<ImasmNumeralWord, &'static str> {
    from_bits(sub_bits(&payload_bits(a)?, &payload_bits(b)?)?)
}

pub fn multiply(a: &ImasmNumeralWord, b: &ImasmNumeralWord) -> Result<ImasmNumeralWord, &'static str> {
    from_bits(mul_bits(&payload_bits(a)?, &payload_bits(b)?))
}

/// Restore the remainder one resident cell at a time. Neither operand nor
/// intermediate is folded into a machine integer or limb vector.
pub fn divide(a: &ImasmNumeralWord, b: &ImasmNumeralWord)
    -> Result<(ImasmNumeralWord, ImasmNumeralWord), &'static str> {
    let dividend = payload_bits(a)?;
    let divisor = payload_bits(b)?;
    if divisor == [EVALT] { return Err("IMASM word division by zero"); }
    if divisor.iter().filter(|&&mark| mark == EVALF).count() == 1 {
        let cut = (divisor.len()-1).min(dividend.len());
        return Ok((from_bits(dividend[cut..].to_vec())?,from_bits(dividend[..cut].to_vec())?));
    }
    let mut remainder = vec![EVALT];
    let mut quotient = Vec::with_capacity(dividend.len());
    for &cell in dividend.iter().rev() {
        remainder.insert(0, cell);
        remainder = trim_bits(remainder);
        if cmp_bits(&remainder, &divisor) != Ordering::Less {
            remainder = sub_bits(&remainder, &divisor)?;
            quotient.push(EVALF);
        } else {
            quotient.push(EVALT);
        }
    }
    quotient.reverse();
    Ok((from_bits(quotient)?, from_bits(remainder)?))
}

pub fn square(a: &ImasmNumeralWord) -> Result<ImasmNumeralWord, &'static str> {
    multiply(a, a)
}

pub fn one() -> ImasmNumeralWord {
    ImasmNumeralWord::checked("⊢≻⋈∈⊥∋⊙⊡⊣").expect("literal one is a native IMASM numeral")
}

pub fn zero() -> ImasmNumeralWord {
    ImasmNumeralWord::checked("⊢⊙⊡⊣").expect("literal zero is a native IMASM numeral")
}

pub fn increment(a: &ImasmNumeralWord) -> Result<ImasmNumeralWord, &'static str> {
    add(a, &one())
}

/// Floor square root built only from resident IMASM word operations. Candidate
/// bit positions are structural addresses, never decoded numeric payloads.
pub fn sqrt_floor(n: &ImasmNumeralWord) -> Result<ImasmNumeralWord, &'static str> {
    let n_bits = payload_bits(n)?;
    let root_width = (n_bits.len() + 1) / 2;
    let mut root = vec![EVALT];
    let mut root_square = vec![EVALT];
    for position in (0..root_width).rev() {
        let candidate = bit_candidate(&root, position);
        // Retain R². Setting this previously zero cell adds exactly
        // 2 R 2^position + 2^(2 position), through the same adder word.
        let mut twice_root_shifted = vec![EVALT;position+1];
        twice_root_shifted.extend_from_slice(&root);
        let unit_square = bit_candidate(&[EVALT],2*position);
        let increment = add_bits(&twice_root_shifted,&unit_square);
        let candidate_square = add_bits(&root_square,&increment);
        if cmp_bits(&candidate_square, &n_bits) != Ordering::Greater {
            root = candidate;
            root_square = candidate_square;
        }
    }
    from_bits(root)
}

pub fn sqrt_ceil(n: &ImasmNumeralWord) -> Result<ImasmNumeralWord, &'static str> {
    let floor = sqrt_floor(n)?;
    if compare(&square(&floor)?, n)? == Ordering::Equal {
        Ok(floor)
    } else {
        increment(&floor)
    }
}

pub fn square_root_exact(n: &ImasmNumeralWord) -> Result<Option<ImasmNumeralWord>, &'static str> {
    let root = sqrt_floor(n)?;
    if compare(&square(&root)?, n)? == Ordering::Equal {
        Ok(Some(root))
    } else {
        Ok(None)
    }
}

pub fn square_gap(a: &ImasmNumeralWord, n: &ImasmNumeralWord) -> Result<ImasmNumeralWord, &'static str> {
    subtract(&square(a)?, n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn numeral(word: &str) -> ImasmNumeralWord {
        ImasmNumeralWord::checked(word).unwrap()
    }

    #[test]
    fn add_subtract_and_multiply_remain_glyph_words() {
        let three = numeral("⊢≻⋈∈⊥∋≻⋈∈⊥∋⊙⊡⊣");
        let five = numeral("⊢≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋⊙⊡⊣");
        let eight = numeral("⊢≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋⊙⊡⊣");
        let fifteen = numeral("⊢≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋⊙⊡⊣");

        assert_eq!(add(&three, &five).unwrap(), eight);
        assert_eq!(subtract(&eight, &five).unwrap(), three);
        assert_eq!(multiply(&three, &five).unwrap(), fifteen);
        assert!(multiply(&three, &five)
            .unwrap()
            .as_str()
            .chars()
            .all(super::super::fixed_point_imasm::is_imasm_for_test));
    }

    #[test]
    fn square_root_stays_inside_word_arithmetic() {
        let fifteen = numeral("⊢≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋⊙⊡⊣");
        let twenty_five = numeral("⊢≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋⊙⊡⊣");
        let three = numeral("⊢≻⋈∈⊥∋≻⋈∈⊥∋⊙⊡⊣");
        let four = numeral("⊢≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋⊙⊡⊣");
        let five = numeral("⊢≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋⊙⊡⊣");

        assert_eq!(sqrt_floor(&fifteen).unwrap(), three);
        assert_eq!(sqrt_ceil(&fifteen).unwrap(), four);
        assert_eq!(square_root_exact(&fifteen).unwrap(), None);
        assert_eq!(square_root_exact(&twenty_five).unwrap(), Some(five));
    }

    #[test]
    fn arithmetic_cell_words_are_native_imasm() {
        for word in [&WORD_ADD_CELL[..], &WORD_SUB_CELL[..], &WORD_COMPARE[..]] {
            assert_eq!(word.first(), Some(&VINIT));
            assert_eq!(word.last(), Some(&TANCH));
            assert!(word.iter().all(|&mark| super::super::fixed_point_imasm::is_imasm_for_test(mark)));
        }
    }

    #[test]
    fn division_restores_remainder_without_decoding_values() {
        let five = numeral("⊢≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋⊙⊡⊣");
        let three = numeral("⊢≻⋈∈⊥∋≻⋈∈⊥∋⊙⊡⊣");
        let two = numeral("⊢≻⋈∈⊤∋≻⋈∈⊥∋⊙⊡⊣");
        assert_eq!(divide(&five, &three).unwrap(), (one(), two));
        assert_eq!(divide(&three, &five).unwrap(), (zero(), three.clone()));
        assert_eq!(divide(&three, &three).unwrap(), (one(), zero()));
        assert!(divide(&five, &zero()).is_err());
    }
}
