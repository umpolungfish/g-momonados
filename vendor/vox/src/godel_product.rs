//! Structural multiplication reads over cell-binary Gödel numerals.
//!
//! This layer keeps the arithmetic and the glyph geometry coupled: a claimed
//! product is checked both by exact `Nat` multiplication and by multiplying the
//! two binary-support polynomials, then normalizing the resulting coefficients
//! by base-2 carry propagation. The bundled RBD corpus contains 23 exact
//! product/factor triples mined from the repository's analyzer output.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::godel_analyzer::{bitlength, codec_assertions, residue_pow2, v2_plus_one, V2};
use crate::godel_calculus::{decode, encode_cell_binary, Family, Nat};

pub const RBD_PRODUCT_CORPUS: &str = include_str!("rbd_products.txt");

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProductAnalysis {
    pub product: Nat,
    pub left: Nat,
    pub right: Nat,
    pub exact_product: bool,
    pub codec_assertions: bool,
    pub convolution_normalizes: bool,
    pub carry_positions: Nat,
    pub carry_units: Nat,
    pub product_bitlength: Nat,
    pub left_bitlength: Nat,
    pub right_bitlength: Nat,
    pub low_width: Nat,
    pub low_product_residue: Nat,
    pub low_left_residue: Nat,
    pub low_right_residue: Nat,
    pub low_factor_product_residue: Nat,
    pub low_residue_relation: bool,
}

fn nat_from_index(mut value: usize) -> Nat {
    let mut bits = Vec::new();
    while value != 0 {
        bits.push(value & 1 == 1);
        value >>= 1;
    }
    Nat::from_bits_le(bits)
}

fn half(value: &Nat) -> Nat {
    if value.bits_le().len() <= 1 {
        Nat::zero()
    } else {
        Nat::from_bits_le(value.bits_le()[1..].to_vec())
    }
}

fn support_convolution(left: &Nat, right: &Nat) -> Vec<Nat> {
    if left.is_zero() || right.is_zero() {
        return Vec::new();
    }
    let len = left.bits_le().len() + right.bits_le().len() - 1;
    let mut coeffs = alloc::vec![Nat::zero(); len];
    let one = Nat::one();
    for (i, l) in left.bits_le().iter().copied().enumerate() {
        if !l {
            continue;
        }
        for (j, r) in right.bits_le().iter().copied().enumerate() {
            if r {
                coeffs[i + j] = coeffs[i + j].add(&one);
            }
        }
    }
    coeffs
}

/// Multiply the two support polynomials and normalize their coefficients in
/// base 2. This is deliberately independent of `Nat::mul`, so it checks the
/// same product through the glyph-support geometry rather than through the
/// arithmetic implementation itself.
pub fn normalize_support_product(left: &Nat, right: &Nat) -> (Nat, Nat, Nat) {
    if left.is_zero() || right.is_zero() {
        return (Nat::zero(), Nat::zero(), Nat::zero());
    }

    let mut coeffs = support_convolution(left, right);
    coeffs.push(Nat::zero());

    let mut bits = Vec::with_capacity(coeffs.len());
    let mut carry_positions = Nat::zero();
    let mut carry_units = Nat::zero();
    let one = Nat::one();

    for i in 0..coeffs.len() - 1 {
        bits.push(coeffs[i].bits_le().first().copied().unwrap_or(false));
        let carry = half(&coeffs[i]);
        if !carry.is_zero() {
            carry_positions = carry_positions.add(&one);
            carry_units = carry_units.add(&carry);
            coeffs[i + 1] = coeffs[i + 1].add(&carry);
        }
    }
    bits.push(
        coeffs
            .last()
            .and_then(|n| n.bits_le().first())
            .copied()
            .unwrap_or(false),
    );

    (Nat::from_bits_le(bits), carry_positions, carry_units)
}

fn parse_input(raw: &str) -> Result<Nat, String> {
    if raw.starts_with('⊢') {
        let reading = decode(raw).map_err(|e| e.to_string())?;
        if reading.family != Family::CellBinary {
            return Err("product accepts cell-binary words or decimal naturals".to_string());
        }
        Ok(reading.value)
    } else {
        Nat::from_decimal(raw)
            .ok_or_else(|| format!("not a natural number or cell-binary word: {raw}"))
    }
}

fn codec_ok(value: &Nat) -> Result<bool, String> {
    let word = encode_cell_binary(value);
    Ok(codec_assertions(value, &word)?.all())
}

pub fn analyze_product(product: &Nat, left: &Nat, right: &Nat) -> Result<ProductAnalysis, String> {
    let exact = left.mul(right);
    let (normalized, carry_positions, carry_units) = normalize_support_product(left, right);

    let low_width = match v2_plus_one(product) {
        V2::Finite(k) => k,
        V2::Infinity => return Err("unexpected infinite v2(product+1)".to_string()),
    };
    let low_product_residue = residue_pow2(product, &low_width);
    let low_left_residue = residue_pow2(left, &low_width);
    let low_right_residue = residue_pow2(right, &low_width);
    let low_factor_product_residue =
        residue_pow2(&low_left_residue.mul(&low_right_residue), &low_width);

    Ok(ProductAnalysis {
        product: product.clone(),
        left: left.clone(),
        right: right.clone(),
        exact_product: exact == *product,
        codec_assertions: codec_ok(product)? && codec_ok(left)? && codec_ok(right)?,
        convolution_normalizes: normalized == *product,
        carry_positions,
        carry_units,
        product_bitlength: bitlength(product),
        left_bitlength: bitlength(left),
        right_bitlength: bitlength(right),
        low_width,
        low_product_residue: low_product_residue.clone(),
        low_left_residue,
        low_right_residue,
        low_factor_product_residue: low_factor_product_residue.clone(),
        low_residue_relation: low_factor_product_residue == low_product_residue,
    })
}

pub fn render(analysis: &ProductAnalysis) -> String {
    let pass = |x: bool| if x { "PASS" } else { "FAIL" };
    format!(
        "product                    {}\n\
         left                       {}\n\
         right                      {}\n\
         relation.exact-product     {}\n\
         relation.codec-assertions  {}\n\
         relation.support-carry     {}\n\
         carry.positions            {}\n\
         carry.units                {}\n\
         bitlength.product          {}\n\
         bitlength.left             {}\n\
         bitlength.right            {}\n\
         2adic.width=v2(product+1)  {}\n\
         2adic.product-residue      {}\n\
         2adic.left-residue         {}\n\
         2adic.right-residue        {}\n\
         2adic.factor-product       {}\n\
         2adic.residue-relation     {}\n",
        analysis.product,
        analysis.left,
        analysis.right,
        pass(analysis.exact_product),
        pass(analysis.codec_assertions),
        pass(analysis.convolution_normalizes),
        analysis.carry_positions,
        analysis.carry_units,
        analysis.product_bitlength,
        analysis.left_bitlength,
        analysis.right_bitlength,
        analysis.low_width,
        analysis.low_product_residue,
        analysis.low_left_residue,
        analysis.low_right_residue,
        analysis.low_factor_product_residue,
        pass(analysis.low_residue_relation),
    )
}

pub fn help_addendum() -> &'static str {
    "godel product <product> <left-factor> <right-factor>\n"
}

pub fn command(args: &[&str]) -> Result<String, String> {
    if args.first().copied() != Some("product") || args.len() != 4 {
        return Err("godel product <product> <left-factor> <right-factor>".to_string());
    }
    let product = parse_input(args[1])?;
    let left = parse_input(args[2])?;
    let right = parse_input(args[3])?;
    Ok(render(&analyze_product(&product, &left, &right)?))
}

fn corpus_triples() -> Result<Vec<(Nat, Nat, Nat)>, String> {
    let mut out = Vec::new();
    for (line_no, line) in RBD_PRODUCT_CORPUS.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let mut parts = line.split('|');
        let n = parts
            .next()
            .and_then(Nat::from_decimal)
            .ok_or_else(|| format!("invalid RBD product at line {}", line_no + 1))?;
        let p = parts
            .next()
            .and_then(Nat::from_decimal)
            .ok_or_else(|| format!("invalid RBD left factor at line {}", line_no + 1))?;
        let q = parts
            .next()
            .and_then(Nat::from_decimal)
            .ok_or_else(|| format!("invalid RBD right factor at line {}", line_no + 1))?;
        if parts.next().is_some() {
            return Err(format!("too many RBD fields at line {}", line_no + 1));
        }
        out.push((n, p, q));
    }
    Ok(out)
}

pub fn selftest_report() -> Result<String, String> {
    let triples = corpus_triples()?;
    let total = nat_from_index(triples.len());
    let mut passed = Nat::zero();
    let one = Nat::one();
    let mut all_ok = true;

    for (n, p, q) in triples {
        let analysis = analyze_product(&n, &p, &q)?;
        let ok = analysis.exact_product
            && analysis.codec_assertions
            && analysis.convolution_normalizes
            && analysis.low_residue_relation;
        if ok {
            passed = passed.add(&one);
        } else {
            all_ok = false;
        }
    }

    let report = format!(
        "rbd-product {passed}/{total} exact product+support-carry triples  {}\n",
        if all_ok { "PASS" } else { "FAIL" }
    );
    if all_ok {
        Ok(report)
    } else {
        Err(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_product_normalizes_support() {
        let n = Nat::from_u64(21);
        let p = Nat::from_u64(3);
        let q = Nat::from_u64(7);
        let analysis = analyze_product(&n, &p, &q).unwrap();
        assert!(analysis.exact_product);
        assert!(analysis.codec_assertions);
        assert!(analysis.convolution_normalizes);
        assert!(analysis.low_residue_relation);
    }

    #[test]
    fn rbd_corpus_is_23_exact_product_triples() {
        let triples = corpus_triples().unwrap();
        assert_eq!(triples.len(), 23);
        for (n, p, q) in triples {
            let analysis = analyze_product(&n, &p, &q).unwrap();
            assert!(analysis.exact_product);
            assert!(analysis.codec_assertions);
            assert!(analysis.convolution_normalizes);
            assert!(analysis.low_residue_relation);
        }
    }
}
