//! Complex-coordinate actions executed inside the source-bound SRFP/DNN graph.
//! Finite Weyl commutation and phase winding have their explicit domains here.
use num_bigint::{BigInt,BigUint};
use num_traits::Zero;
use serde_json::{json,Value};
use crate::{ququart_decision::DecisionArena,ququart4::QuquartMatrix,
    phase_unbraid::{FixedComplex,FixedPointFormat},word_tape::SignedWordTape,
    anyon_ququart::FixedQuquartSic};

type Matrix=[(SignedWordTape,SignedWordTape);16];
fn words(matrix:&QuquartMatrix,format:&FixedPointFormat)->Result<Matrix,String>{
    let scale=format.scale();
    if matrix.0.iter().any(|z| &z.re%&scale!=BigInt::zero() || &z.im%&scale!=BigInt::zero()){
        return Err("protocol matrix is not an exact Gaussian integer operator".into());
    }
    Ok(core::array::from_fn(|i| (SignedWordTape::from_bigint(&(&matrix.0[i].re/&scale)),
        SignedWordTape::from_bigint(&(&matrix.0[i].im/&scale)))))
}
fn entries(matrix:&Matrix)->Value{json!(matrix.iter().map(|(re,im)|
    json!({"re":re.to_bigint().to_string(),"im":im.to_bigint().to_string()})).collect::<Vec<_>>())}

/// Four nonzero complex coordinates with a retained relative phase. The
/// caller correlates their displacement with the source-point register.
pub(crate) fn prepare(arena:&mut DecisionArena,root:usize,wires:[usize;2])->Result<usize,String>{
    let mut matrix:Matrix=core::array::from_fn(|_|(SignedWordTape::zero(),SignedWordTape::zero()));
    for row in 0..3{matrix[4*row]=(SignedWordTape::from_small(1),SignedWordTape::zero());}
    matrix[12]=(SignedWordTape::zero(),SignedWordTape::from_small(1));
    arena.apply_control_matrix_numerator(root,wires,&matrix)
}

/// Restrict the actual carried coordinate support to a native flag mask.
pub(crate) fn restrict(arena:&mut DecisionArena,root:usize,wires:[usize;2],mask:u8)->usize{
    let zero=arena.zero();let mut result=zero;
    for label in 0..4{if mask&(1<<label)!=0{
        let arm=arena.conditional_literals(zero,root,&[(wires[0],label&1!=0),(wires[1],label&2!=0)]);
        result=arena.sum(result,arm);
    }}result
}
/// Linear retraction of the tagged direct sum: erase the branch tag only
/// after selecting each sector, then coherently add the complete spectators.
/// On the image of delta the coordinate sectors are disjoint, so mu delta = I.
pub(crate) fn fuse(arena:&mut DecisionArena,tagged:usize,history:usize)->usize{
    let zero=arena.zero();
    let low=arena.conditional_literals(zero,tagged,&[(history,false)]);
    let high=arena.conditional_literals(zero,tagged,&[(history,true)]);
    let high=arena.flip(high,history);
    arena.sum(low,high)
}

/// delta(psi) = P_truth psi |0> + P_falsity psi |1>.
/// Check the special law on the actual complete complex register, including
/// every source, arithmetic and bank spectator. The history starts at zero.
pub(crate) fn split(arena:&mut DecisionArena,root:usize,wires:[usize;2],history:usize)
    ->Result<([usize;2],usize,Value),String>{
    if wires.contains(&history) || !arena.wire_is_zero(root,history){
        return Err("comultiplication requires a fresh, separate ancestry wire".into());
    }
    let truth=restrict(arena,root,wires,5);
    let falsity=restrict(arena,root,wires,10);
    let tagged=arena.flip(falsity,history);
    let joined=arena.sum(truth,tagged);
    let recovered=fuse(arena,joined,history);
    if recovered!=root || arena.mass(joined)!=arena.mass(root){
        return Err("executing comultiplication failed mu delta = identity".into());
    }
    Ok(([truth,tagged],joined,json!({"operation":"source-bound tagged direct-sum comultiplication",
        "input_root":root,"arm_roots":[truth,tagged],"tagged_root":joined,
        "recovered_root":recovered,"history_wire":history,"coordinate_masks":[5,10],
        "mu_delta_identity":true,"complex_mass_preserved":true,
        "identity_domain":"complete resident register with zero ancestry wire"})))
}
fn gram(arena:&mut DecisionArena,root:usize,wires:[usize;2])->[(BigInt,BigInt);16]{
    let zero=arena.zero();let mut aligned=[zero;4];
    for (label,arm) in aligned.iter_mut().enumerate(){
        *arm=restrict(arena,root,wires,1<<label);
        for (bit,&wire) in wires.iter().enumerate(){if label&(1<<bit)!=0{*arm=arena.flip(*arm,wire);}}
    }
    arena.control_gram(&aligned)
}
fn sic(arena:&mut DecisionArena,root:usize,wires:[usize;2],format:&FixedPointFormat)->Result<Value,String>{
    let gram=gram(arena,root,wires);let frame=FixedQuquartSic::new(format)?;
    let masses=frame.gram_mass_numerators(&gram)?;
    let result=frame.certify_gram_frame_unrounded_masses(&gram,&masses)?;
    Ok(json!({"dimension":4,"nonempty_state":!arena.mass(root).is_zero(),
        "gram":gram.iter().map(|(re,im)|json!({"re":re.to_string(),"im":im.to_string()})).collect::<Vec<_>>(),
        "mass_numerators":masses.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "signed_rational_residuals":result.signed_residual_numerators.iter().map(|(re,im)|
            json!({"re":re.to_string(),"im":im.to_string()})).collect::<Vec<_>>(),
        "rational_denominator":result.denominator.to_string(),
        "maximum_integer_residual":result.rounded.maximum_residual.to_string(),
        "tolerance":result.rounded.tolerance.to_string(),"full_complex_dual_recovery":true}))
}

/// Apply the untruncated polynomial operators X and P=-i d/dX to
/// coefficients that are complete correlated arena diagrams. Degree four
/// is retained when X raises a degree-three input; it is never wrapped mod 4.
pub(crate) fn clink(arena:&mut DecisionArena,root:usize,wires:[usize;2],bits:u64)
    ->Result<(usize,Vec<usize>,Value),String>{
    let format=FixedPointFormat::for_modulus_bits(bits)?;let zero=arena.zero();
    let mut polynomial=Vec::new();
    for label in 0..4{
        let mut coefficient=restrict(arena,root,wires,1<<label);
        for (bit,&wire) in wires.iter().enumerate(){if label&(1<<bit)!=0{coefficient=arena.flip(coefficient,wire);}}
        polynomial.push(coefficient);
    }
    let position=|p:&[usize]| {let mut out=vec![zero];out.extend_from_slice(p);out};
    let momentum=|arena:&mut DecisionArena,p:&[usize]| ->Vec<usize>{
        if p.len()<2{return vec![zero];}
        p.iter().enumerate().skip(1).map(|(k,&coefficient)|
            arena.scale_words_numerator(coefficient,&(SignedWordTape::zero(),
                SignedWordTape::from_bigint(&-BigInt::from(k))))).collect()
    };
    let ppsi=momentum(arena,&polynomial);let xp=position(&ppsi);
    let xpsi=position(&polynomial);let px=momentum(arena,&xpsi);
    let before=sic(arena,root,wires,&format)?;let mut changed=zero;
    let mut coefficients=Vec::new();
    for k in 0..polynomial.len(){
        let negative=arena.scale_words_numerator(px[k],&(SignedWordTape::from_small(1).negate(),SignedWordTape::zero()));
        let delta=arena.sum(xp[k],negative);
        let expected=arena.scale_words_numerator(polynomial[k],&(SignedWordTape::zero(),SignedWordTape::from_small(1)));
        if delta!=expected{return Err("carried polynomial commutator failed an exact coefficient".into());}
        coefficients.push(delta);let mut arm=delta;
        for (bit,&wire) in wires.iter().enumerate(){if k&(1<<bit)!=0{arm=arena.flip(arm,wire);}}
        changed=arena.sum(changed,arm);
    }
    let expected=arena.scale_words_numerator(root,&(SignedWordTape::zero(),SignedWordTape::from_small(1)));
    let recovered=arena.scale_words_numerator(changed,&(SignedWordTape::zero(),SignedWordTape::from_small(1).negate()));
    if changed!=expected || recovered!=root || arena.mass(changed)!=arena.mass(root){
        return Err("CLINK lost its complete correlated commutator return".into());
    }
    let after=sic(arena,changed,wires,&format)?;
    let mut banks=polynomial.clone();banks.extend_from_slice(&ppsi);banks.extend_from_slice(&xp);
    banks.extend_from_slice(&xpsi);banks.extend_from_slice(&px);banks.extend_from_slice(&coefficients);
    Ok((changed,banks,json!({"operation":"XP-PX","position":"multiply by formal X",
        "momentum":"-i d/dX","relation":"[X,P]=iI","hbar":1,
        "domain":"arbitrary finite Gaussian-complex polynomials with resident diagram coefficients",
        "maximum_intermediate_degree":xpsi.len()-1,"degree_wraparound":false,
        "input_coefficients":polynomial,"P_input":ppsi,"X_P_input":xp,"X_input":xpsi,
        "P_X_input":px,"commutator_coefficients":coefficients,
        "input_root":root,"output_root":changed,"state_changed":root!=changed,
        "coefficient_identity":"(k+1)-k=1","exact_scalar_CCR":true,
        "exact_dual_return":true,"before_SIC":before,"after_SIC":after})))
}

/// Retain the old coordinate as ancestry before adjoining both information
/// flags. Distinct incoming complex amplitudes cannot cancel through erasure.
pub(crate) fn engage(arena:&mut DecisionArena,root:usize,wires:[usize;2],history:[usize;2])->Result<(usize,Value),String>{
    let mut copied=root;
    for bit in 0..2{
        if !arena.wire_is_zero(copied,history[bit]){return Err("engagement ancestry bank was reused".into());}
        copied=arena.controlled_flip(copied,history[bit],&[wires[bit]]);
    }
    let mut matrix:Matrix=core::array::from_fn(|_|(SignedWordTape::zero(),SignedWordTape::zero()));
    for col in 0..4{
        for row in 0..4{if row==col || row>=2{matrix[4*row+col]=(SignedWordTape::from_small(1),SignedWordTape::zero());}}
    }
    let changed=arena.apply_control_matrix_numerator(copied,wires,&matrix)?;
    let zero=arena.zero();let mut recovered=zero;
    for label in 0..4{
        let selected=restrict(arena,changed,wires,1<<label);
        let mut selected=arena.conditional_literals(zero,selected,&[(history[0],label&1!=0),(history[1],label&2!=0)]);
        for bit in 0..2{if label&(1<<bit)!=0{selected=arena.flip(selected,history[bit]);}}
        recovered=arena.sum(recovered,selected);
    }
    if recovered!=root{return Err("engagement lost its complete complex source ancestry".into());}
    Ok((changed,json!({"operation":"adjoin t,f with retained coordinate ancestry",
        "input_root":root,"output_root":changed,"history_wires":history,
        "matrix":entries(&matrix),"ancestry_retained":true,"exact_ancestry_return":true})))
}

/// Four actual central Weyl phase steps close on the same state. The path's
/// lifted quarter-turn count is retained separately from endpoint equality.
pub(crate) fn fix_phase(arena:&mut DecisionArena,root:usize,wires:[usize;2],bits:u64)->Result<(usize,Vec<usize>,Value),String>{
    let format=FixedPointFormat::for_modulus_bits(bits)?;
    let x=QuquartMatrix::x(&format);let z=QuquartMatrix::z(&format);
    let loop_operator=z.multiply(&x,&format).multiply(&z.adjoint(),&format).multiply(&x.adjoint(),&format);
    let mut expected=QuquartMatrix::identity(&format);
    let phase=FixedComplex{re:BigInt::zero(),im:format.scale()};
    for entry in &mut expected.0{*entry=entry.mul(&phase,&format);}
    if loop_operator!=expected{return Err("central Weyl phase is not iI".into());}
    let matrix=words(&loop_operator,&format)?;let mut live=root;let mut banks=Vec::new();
    for _ in 0..4{live=arena.apply_control_matrix_numerator(live,wires,&matrix)?;banks.push(live);}
    if live!=root{return Err("four-quarter-turn phase did not return the complete state".into());}
    let witness=json!({"operation":"(ZXZ_dagger X_dagger)^4","central_step":"iI",
        "executed_steps":4,"lifted_quarter_turns":4,"operator_winding":1,
        "nonempty_state":!arena.mass(root).is_zero(),"complete_state_return":true,
        "scope":"executed finite central phase loop; first Chern class not established","phase_banks":banks});
    Ok((live,banks,witness))
}

pub(crate) fn source_reflect(arena:&DecisionArena,root:usize,n:&BigUint,word:&str)->Result<Value,String>{
    let model=crate::parasm::SelfModel::prepare()?;
    let payload=json!({"N":n.to_string(),"word":word,"register":arena.prepared_record(root)});
    let bytes=serde_json::to_vec(&payload).map_err(|e|e.to_string())?;
    if !model.contains(&bytes)||model.reflect(&bytes)!=bytes{return Err("IMSCRIB failed complete carried-state self-return".into());}
    Ok(json!({"operation":"ParaVM compile/recompile on source, word and carried register",
        "bytes_returned":bytes.len(),"executed_trie_steps":model.executed_trie_steps,
        "program_self_return":model.program_self_return,"complete_payload_return":true,
        "scope":"native code/payload reflection; full constructible-universe reflection not established"}))
}

pub(crate) fn phase_coherence_control(arena:&mut DecisionArena,root:usize,wires:[usize;2],bits:u64)
    ->Result<(usize,Value),String>{
    let format=FixedPointFormat::for_modulus_bits(bits)?;let frame=FixedQuquartSic::new(&format)?;
    let before=gram(arena,root,wires);let old=frame.gram_mass_numerators(&before)?;
    let changed=arena.apply_control_matrix_numerator(root,wires,&words(&QuquartMatrix::z(&format),&format)?)?;
    let after=gram(arena,changed,wires);
    if before==after || arena.mass(root)!=arena.mass(changed){return Err("coherence control failed to isolate a phase change".into());}
    let fresh=frame.gram_mass_numerators(&after)?;
    frame.certify_gram_frame_unrounded_masses(&after,&fresh)?;
    if frame.certify_gram_frame_unrounded_masses(&after,&old).is_ok(){return Err("SIC dual admitted stale masses after a phase change".into());}
    Ok((changed,json!({"phase_only_change":true,"mass_preserved":true,
        "complex_gram_changed":true,"fresh_mass_recovery":true,"stale_masses_rejected":true})))
}
