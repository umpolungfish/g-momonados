//! The Stage 43/44 powerset unit and bind on retained arena coordinates.
//! A member names a complete correlated diagram in one common arena.
//! Union keeps alternative members distinct; it does not add amplitudes.
use alloc::collections::BTreeSet;

pub type Held<X> = BTreeSet<X>;

pub fn unit<X: Ord>(x: X) -> Held<X> {
    core::iter::once(x).collect()
}

pub fn multiply<X: Ord>(families: Held<Held<X>>) -> Held<X> {
    families.into_iter().flatten().collect()
}

pub fn bind<X: Ord, Y: Ord, E>(held: Held<X>, mut arrow: impl FnMut(X) -> Result<Held<Y>, E>)
    -> Result<Held<Y>, E>
{
    let mut result=Held::new();
    for member in held { result.extend(arrow(member)?); }
    Ok(result)
}

/// Lift a deterministic arrow without splitting its correlated diagram.
pub fn bind_one<X: Ord, E>(member:X, arrow:impl FnOnce(X)->Result<X,E>) -> Result<X,E> {
    let mut arrow=Some(arrow);
    let result=bind(unit(member),|member| arrow.take().expect("unit has exactly one member")(member).map(unit))?;
    Ok(result.into_iter().next().expect("deterministic arrow returns one successor"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ququart_decision::DecisionArena;
    use crate::phase_unbraid::{FixedComplex,FixedPointFormat};
    use crate::word_tape::WordTape;
    use num_bigint::{BigInt,BigUint};

    #[test]
    fn unit_bind_and_nested_multiplication_preserve_complex_retained_diagrams() {
        let source=WordTape::from_canonical_word(include_str!(
            "../measurements/resident_imasm_integration_20261012/inputs/fresh_balanced_512.imasm").trim()).unwrap();
        assert!(source.bit_len()>=200);
        let base=BigUint::from(1u8) << (source.bit_len()-1);
        assert!(&base+7u8<source.to_biguint());
        let mut arena=DecisionArena::new(source.bit_len());
        let format=FixedPointFormat::for_modulus_bits(source.bit_len() as u64).unwrap();
        let real=arena.basis(&base,BigInt::from(2));
        let imaginary=arena.basis(&(&base+3u8),BigInt::from(1));
        let imaginary=arena.scale(imaginary,&FixedComplex {re:BigInt::from(0),im:format.scale()},&format);
        let original=arena.sum(real,imaginary);
        let alternate=arena.flip(original,2);
        let held:Held<_>=[original,alternate].into_iter().collect();
        let before:(Vec<_>,Vec<_>)=(
            (0u8..8).map(|x|arena.amplitude(original,&(&base+x))).collect(),
            (0u8..8).map(|x|arena.amplitude(alternate,&(&base+x))).collect());
        let f=|a:&mut DecisionArena,x| -> Result<Held<usize>,()> {
            let y=a.flip(x,0); Ok([x,y].into_iter().collect())
        };
        let g=|a:&mut DecisionArena,x| -> Result<Held<usize>,()> {Ok(unit(a.flip(x,1)))};
        for x in held.iter().copied() {
            assert_eq!(bind(unit(x),|x|f(&mut arena,x)).unwrap(),f(&mut arena,x).unwrap());
        }
        assert_eq!(bind(held.clone(),|x|Ok::<_,()>(unit(x))).unwrap(),held);
        let first=bind(held.clone(),|x|f(&mut arena,x)).unwrap();
        assert_eq!(first.len(),4);
        let left=bind(first,|x|g(&mut arena,x)).unwrap();
        let right=bind(held.clone(),|x| {
            let fx=f(&mut arena,x)?; bind(fx,|y|g(&mut arena,y))
        }).unwrap();
        assert_eq!(left,right);
        assert!(bind(Held::<usize>::new(),|x|g(&mut arena,x)).unwrap().is_empty());
        let nested=unit(unit(held.clone()));
        let separated:Held<Held<usize>>=held.iter().copied().map(unit).collect();
        let grouped=unit(held.clone());
        assert_ne!(separated,grouped);
        assert_eq!(multiply(separated),multiply(grouped));
        assert_eq!(multiply(multiply(nested.clone())),
            multiply(nested.into_iter().map(multiply).collect()));
        assert_eq!(multiply(unit(held.clone())),held);
        assert_eq!(multiply(held.iter().copied().map(unit).collect()),held);
        for (root,expected) in [(original,before.0),(alternate,before.1)] {
            let changed=arena.flip(root,0);
            let recovered=arena.flip(changed,0);
            assert_eq!(recovered,root);
            for x in 0u8..8 {assert_eq!(arena.amplitude(recovered,&(&base+x)),expected[x as usize]);}
        }
    }
}
