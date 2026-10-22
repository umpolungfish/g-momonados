//! Typed split/transform/fuse on one exact complex-amplitude diagram.
//! A SIXTEEN_3 flag denotes presence of the corresponding ququart basis
//! coordinate; each projected arm retains all other resident work cells.
use alloc::string::String;
#[cfg(feature = "hosted")]
use alloc::vec::Vec;
use imasm_core::{check::Graph, classic::Token, flow::gate_out, imasm16_3::Reg16_3};
use num_bigint::{BigInt, BigUint};
use num_traits::{One, Zero};
use crate::{ququart_decision::DecisionArena, word_tape::SignedWordTape};

pub struct ResidentFrameWitness {
    pub graph: Graph,
    pub pair: (usize, usize),
    pub partition_masks: [u8; 3],
    pub split_mass: [BigUint; 3],
    pub exact_split_return: bool,
    pub live_register_cleared: bool,
    pub exact_banked_return: bool,
    pub exact_transformed_return: bool,
    pub exact_dual_return: bool,
    pub information_arm_loss_detected: Option<bool>,
    pub cross_arm_gram: [(BigInt, BigInt); 16],
}

impl ResidentFrameWitness {
    #[cfg(feature = "hosted")]
    pub fn record(&self) -> serde_json::Value {
        serde_json::json!({
            "carrier":"source-bound DecisionArena with exact signed complex amplitudes",
            "nodes":self.graph.nodes.iter().map(|token|token.name()).collect::<Vec<_>>(),
            "edges":self.graph.edges,
            "ancestry_pair":self.pair,
            "partition_masks":self.partition_masks,
            "split_mass":self.split_mass.each_ref().map(|mass|mass.to_string()),
            "exact_split_return":self.exact_split_return,
            "live_clear_position":5,
            "live_register_cleared":self.live_register_cleared,
            "exact_banked_return":self.exact_banked_return,
            "exact_transformed_return":self.exact_transformed_return,
            "exact_dual_return_after_determinant_normalization":self.exact_dual_return,
            "information_arm_loss_detected":self.information_arm_loss_detected,
            "cross_arm_gram":self.cross_arm_gram.iter().map(|(re,im)|
                serde_json::json!({"re":re.to_string(),"im":im.to_string()})).collect::<Vec<_>>()
        })
    }
}

pub(crate) fn lift_matrix(operator: &[(BigInt, BigInt); 4])
    -> [(SignedWordTape, SignedWordTape); 16] {
    core::array::from_fn(|entry| {
        let (row, col) = (entry / 4, entry % 4);
        if row & 2 != col & 2 { (SignedWordTape::zero(), SignedWordTape::zero()) }
        else {
            let (re, im) = &operator[2 * (row & 1) + (col & 1)];
            (SignedWordTape::from_bigint(re), SignedWordTape::from_bigint(im))
        }
    })
}

/// Return the transformed root, with an audit of delta/mu on its complete
/// correlated state. No projection measurements or amplitude rounding occur.
/// AREV clears the live cursor only after the transformed arms are banked;
/// FFUSE retrieves those exact deposits, including their interference.
pub(crate) fn apply(arena: &mut DecisionArena, root: usize, wires: [usize; 2],
    operator: &[(BigInt, BigInt); 4]) -> Result<(usize, ResidentFrameWitness), String> {
    let graph = Graph {
        nodes: alloc::vec![Token::Vinit, Token::Fsplit3, Token::Afwd,
            Token::Afwd, Token::Afwd, Token::Arev, Token::Ffuse3, Token::Tanch],
        // The three transformed rails reconverge at the same ancestry fuse.
        // The active rail crosses AREV while the other two retain their banks.
        edges: alloc::vec![(0,1),(1,2),(1,3),(1,4),(2,5),(5,6),(3,6),(4,6),(6,7)],
    };
    let (pairs, fully_closed) = graph.frobenius_closures();
    if !graph.validate().is_empty() || !fully_closed {
        return Err("resident split/fuse graph failed its typed ancestry check".into());
    }
    if pairs.as_slice() != [(1,6)] {
        return Err("resident frame lost its common split ancestry".into());
    }
    let zero = arena.zero();
    let unit = (BigInt::one(), BigInt::zero());
    let labels = [
        Reg16_3 { big_t:true, ..Reg16_3::default() },
        Reg16_3 { big_f:true, ..Reg16_3::default() },
        Reg16_3 { small_t:true, ..Reg16_3::default() },
        Reg16_3 { small_f:true, ..Reg16_3::default() },
    ];
    let partition_masks: [u8;3] = core::array::from_fn(|slot| {
        labels.iter().enumerate().fold(0, |mask, (label, &value)| {
            let projected = gate_out(Token::Fsplit3, value, Reg16_3::default(), slot, 3);
            if projected == value { mask | 1 << label } else { mask }
        })
    });
    if partition_masks != [1,2,12] {
        return Err("typed split no longer partitions T, F, and independent t/f".into());
    }
    let mut split = [zero;3];
    for (slot, &mask) in partition_masks.iter().enumerate() {
        for label in 0..4 {
            if mask & (1 << label) != 0 {
                let controls = [(wires[0],label & 1 != 0),(wires[1],label & 2 != 0)];
                let selected = arena.conditional_literals(zero, root, &controls);
                split[slot] = arena.sum(split[slot], selected);
            }
        }
    }
    let split_mass = split.map(|arm| arena.mass_word(arm).to_biguint());
    let mut split_join = zero;
    for &arm in &split { split_join = arena.sum(split_join,arm); }
    let exact_split_return = arena.equals_complex_scaled(root,split_join,&unit);
    let matrix = lift_matrix(operator);
    let expected = arena.apply_control_matrix_numerator(root,wires,&matrix)?;
    // Apply the same source-bound morphism to each retained arm. Arms can
    // overlap after transformation; their complex amplitudes fuse coherently.
    let mut banked = [zero;3];
    for (bank, arm) in banked.iter_mut().zip(split) {
        *bank = arena.apply_control_matrix_numerator(arm,wires,&matrix)?;
    }
    // Observe inter-arm coherences before fuse rather than replacing banks
    // with probabilities. The fourth slot pads this three-arm Gram table.
    let cross_arm_gram = arena.control_gram(&[banked[0],banked[1],banked[2],zero]);
    let mut live = arena.flip(banked[0],wires[0]);
    let reversed_mass = arena.mass_word(live);
    live = zero;
    let live_register_cleared = arena.mass_word(live).is_zero();
    // Banked recovery reads every deposit once; it never adds a source copy.
    for &arm in &banked { live = arena.sum(live,arm); }
    let exact_banked_return = arena.equals_complex_scaled(expected,live,&unit);
    let information_arm_loss_detected = if split_mass[2].is_zero() { None }
        else {
            let lost = arena.sum(banked[0],banked[1]);
            Some(!arena.equals_complex_scaled(expected,lost,&unit))
        };
    let exact_transformed_return = exact_banked_return
        && reversed_mass == arena.mass_word(banked[0]);
    let [a,b,c,d] = operator;
    let product = |x: &(BigInt,BigInt), y: &(BigInt,BigInt)|
        (&x.0*&y.0-&x.1*&y.1, &x.0*&y.1+&x.1*&y.0);
    let (ad,bc) = (product(a,d),product(b,c));
    let determinant = (ad.0-bc.0,ad.1-bc.1);
    if determinant.0.is_zero() && determinant.1.is_zero() {
        return Err("resident frame operator has a zero determinant".into());
    }
    let adjugate = [d.clone(),(-b.0.clone(),-b.1.clone()),
        (-c.0.clone(),-c.1.clone()),a.clone()];
    let returned = arena.apply_control_matrix_numerator(live,wires,&lift_matrix(&adjugate))?;
    let exact_dual_return = arena.equals_complex_scaled(root,returned,&determinant);
    if !(exact_split_return && live_register_cleared && exact_banked_return
        && exact_transformed_return && exact_dual_return) {
        return Err("resident banked circuit failed its complete-amplitude frame return".into());
    }
    Ok((live, ResidentFrameWitness { graph, pair:pairs[0], partition_masks,split_mass,
        exact_split_return,live_register_cleared,exact_banked_return,
        exact_transformed_return,exact_dual_return,information_arm_loss_detected,cross_arm_gram }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::word_tape::WordTape;

    #[test]
    fn source_bound_signed_coherence_survives_banked_clear() {
        for text in [
            std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"),
                "/measurements/resident_imasm_integration_20261012/inputs/htsx_balanced_control_208.imasm"))
                .expect("restore the archived 208-bit source fixture before this test"),
            include_str!("../measurements/resident_imasm_integration_20261012/inputs/fresh_balanced_512.imasm").to_owned(),
        ] {
            let source = WordTape::from_canonical_word(text.trim()).unwrap().to_biguint();
            assert!(source.bits() >= 200);
            let mut arena = DecisionArena::new(source.bits() as usize);
            let base = (&source >> 2usize) << 2usize;
            let mut root = arena.zero();
            for (label, (re,im)) in [(1,2),(-3,1),(2,-4),(-1,-2)].into_iter().enumerate() {
                let arm = arena.basis(&(&base + BigUint::from(label)),BigInt::one());
                let arm = arena.scale_words_numerator(arm,&(
                    SignedWordTape::from_bigint(&BigInt::from(re)),
                    SignedWordTape::from_bigint(&BigInt::from(im))));
                root = arena.sum(root,arm);
            }
            // Exact complex mixing creates interference between T/F and t/f.
            let matrix = [(BigInt::one(),BigInt::one()),(BigInt::from(-2),BigInt::one()),
                (BigInt::from(3),BigInt::from(-1)),(BigInt::from(-1),BigInt::from(-2))];
            let (returned, frame) = apply(&mut arena,root,[0,1],&matrix).unwrap();
            assert!(frame.exact_split_return && frame.exact_banked_return && frame.exact_dual_return);
            assert_eq!(frame.information_arm_loss_detected,Some(true));
            assert!(frame.cross_arm_gram.iter().any(|(_,im)|!im.is_zero()));
            // Losing a relative sign must also be visible in complete-state return.
            let zero = arena.zero();
            let arm = arena.conditional_literals(zero,returned,&[(1,true)]);
            let negative = arena.scale_words_numerator(arm,&(
                SignedWordTape::from_bigint(&-BigInt::one()),SignedWordTape::zero()));
            let corrupted = arena.conditional_literals(returned,negative,&[(1,true)]);
            assert!(!arena.equals_complex_scaled(returned,corrupted,&(BigInt::one(),BigInt::zero())));
        }
    }
}
