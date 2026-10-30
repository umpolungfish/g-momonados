//! Centered second moment on the carried point register, recovered from SIC masses.
use crate::{anyon_ququart::FixedQuquartSic, phase_unbraid::FixedPointFormat,
    ququart_decision::DecisionArena, word_tape::SignedWordTape};
use num_bigint::{BigInt, BigUint};
use num_traits::{One, Zero};
use serde_json::{json, Value};

pub(crate) struct PreparedMoment {
    pub offset:SignedWordTape,
    pub weights:Vec<SignedWordTape>,
}
impl PreparedMoment {
    pub(crate) fn prepare(n:&BigUint,width:usize)->Self {
        Self{offset:SignedWordTape::from_bigint(&-BigInt::from(n+BigUint::one())),
            weights:(0..width).map(|i|SignedWordTape::from_bigint(&(BigInt::one()<<(i+1)))).collect()}
    }
}

pub struct MomentReturn {
    pub weighted_root: usize,
    pub squared_root: usize,
    pub point: Option<BigUint>,
    pub evidence: Value,
}

pub fn read(arena: &mut DecisionArena, root: usize, wires: &[usize], n: &BigUint)
    -> Result<MomentReturn, String>
{
    let frame=FixedQuquartSic::new(&FixedPointFormat::for_modulus_bits(n.bits())?)?;
    read_prepared(arena,root,wires,n,&frame)
}
pub(crate) fn read_prepared(arena:&mut DecisionArena,root:usize,wires:&[usize],n:&BigUint,frame:&FixedQuquartSic)
    ->Result<MomentReturn,String>
{
    let operator=PreparedMoment::prepare(n,wires.len());
    read_prepared_operator(arena,root,wires,n,frame,&operator,None)
}
pub(crate) fn read_prepared_operator(arena:&mut DecisionArena,root:usize,wires:&[usize],n:&BigUint,
    frame:&FixedQuquartSic,operator:&PreparedMoment,dual:Option<&crate::anyon_ququart::PreparedSicDual>)->Result<MomentReturn,String>
{
    if operator.weights.len()!=wires.len(){return Err("compiled moment lost its point coordinates".into());}
    if wires.iter().any(|&w| w >= arena.cells) || wires.iter().enumerate().any(|(i,w)|wires[..i].contains(w)) {
        return Err("point wire lies outside the retained register".into());
    }
    fn apply(arena: &mut DecisionArena, root: usize, wires: &[usize], operator:&PreparedMoment) -> usize {
        let zero = arena.zero();
        let mut weighted = arena.scale_words(root, &(operator.offset.clone(),SignedWordTape::zero()), 0);
        // X = -(N+1)I + sum_i 2^(i+1) P(E_i=1), with all spectators.
        for (i, &wire) in wires.iter().enumerate() {
            let projected = arena.conditional_literals(zero, root, &[(wire, true)]);
            let term = arena.scale_words(projected, &(operator.weights[i].clone(),SignedWordTape::zero()), 0);
            weighted = arena.sum(weighted, term);
        }
        weighted
    }
    let weighted = apply(arena,root,wires,operator);
    let squared = apply(arena,weighted,wires,operator);
    let zero = arena.zero();
    let gram = arena.control_gram(&[root, weighted, squared, zero]);
    let masses = frame.gram_mass_numerators(&gram)?;
    let recovered=match dual{Some(dual)=>dual.apply(frame,&masses)?,None=>frame.exact_gram_from_masses(&masses)?};
    let population = &recovered.numerators[0].0;
    let moment = &recovered.numerators[5].0;
    let fourth = &recovered.numerators[10].0;
    let variance_numerator = fourth * population - moment * moment;
    if variance_numerator < BigInt::zero() {return Err("negative spectral variance".into());}
    let definite_separation = variance_numerator.is_zero();
    if population <= &BigInt::zero() || moment < &BigInt::zero() {
        return Err("second moment requires positive support and nonnegative weighted mass".into());
    }
    let integral = (moment % population).is_zero();
    let d = (moment / population).to_biguint().ok_or("negative second moment")?;
    let separation = d.sqrt();
    let square = integral && &separation * &separation == d;
    let sum = n + BigUint::one();
    let point = if definite_separation && square && separation < sum && ((&sum + &separation) % 2u8).is_zero() {
        Some((&sum + &separation) / 2u8)
    } else { None };
    Ok(MomentReturn { weighted_root: weighted, squared_root: squared, point,
        evidence: json!({"operator":"X(E)=2E-(N+1)",
            "construction":"bit-projector weighted sum on the same register",
            "point_enumeration":false,"point_wire_count":wires.len(),
            "sic_masses":masses.map(|v|v.to_string()),
            "reconstruction_denominator":recovered.denominator.to_string(),
            "support_population_numerator":population.to_string(),
            "weighted_population_numerator":moment.to_string(),
            "fourth_moment_population_numerator":fourth.to_string(),
            "spectral_variance_numerator":variance_numerator.to_string(),
            "definite_complementary_separation":definite_separation,
            "second_moment_integral":integral,"second_moment":d.to_string(),
            "perfect_square":square,"separation":separation.to_string()}) })
}
