//! Exact numerical point-to-ideal-generator boundary shared by native carriers.
use crate::word_tape::WordTape;
use alloc::string::String;

#[derive(Clone, Debug)]
pub struct IdempotentClosure {
    pub source: WordTape,
    pub idempotent: WordTape,
    pub complement: WordTape,
    pub divisor: WordTape,
    pub quotient: WordTape,
}

pub fn close(source: &WordTape, candidate: &WordTape) -> Result<IdempotentClosure, String> {
    let one = WordTape::one();
    if !source.gt(&one) {
        return Err("idempotent source must exceed one".into());
    }
    let e = candidate.divmod(source).unwrap().1;
    if e.is_zero() || e == one {
        return Err("trivial idempotent retains no factor split".into());
    }
    if e.mul(&e).divmod(source).unwrap().1 != e {
        return Err("candidate does not satisfy e*e=e modulo the source".into());
    }
    let complement = source.add(&one).sub(&e).unwrap();
    if e.mul(&complement).divmod(source).unwrap().1 != WordTape::zero()
        || e.add(&complement).divmod(source).unwrap().1 != one
        || complement.mul(&complement).divmod(source).unwrap().1 != complement
    {
        return Err("idempotent complementary split failed".into());
    }
    let divisor = e.gcd(source);
    if !divisor.gt(&one) || !source.gt(&divisor) {
        return Err("idempotent gcd did not yield a proper divisor".into());
    }
    let (quotient, remainder) = source.divmod(&divisor).unwrap();
    if !remainder.is_zero()
        || divisor.mul(&quotient) != *source
        || !crate::native_numeral::syzygy_preserves(
            &source.to_biguint(),
            &divisor.to_biguint(),
            &quotient.to_biguint(),
        )
    {
        return Err("idempotent factor/product boundary did not close".into());
    }
    Ok(IdempotentClosure {
        source: source.clone(),
        idempotent: e,
        complement,
        divisor,
        quotient,
    })
}

/// For odd N, transport a checked involution s*s=1 to e=(s+1)/2 mod N.
/// This acts before extracting a divisor; neither factor is an input.
pub fn from_involution(source: &WordTape, root: &WordTape) -> Result<IdempotentClosure, String> {
    let one = WordTape::one();
    if !source.gt(&one) || !source.bit_at(0) {
        return Err("involution-to-idempotent transport requires an odd source above one".into());
    }
    let s = root.divmod(source).unwrap().1;
    if s.mul(&s).divmod(source).unwrap().1 != one {
        return Err("phase half-return is not an involution modulo the source".into());
    }
    let inverse_two = source.add(&one).shr1();
    close(source, &s.add(&one).mul(&inverse_two))
}

