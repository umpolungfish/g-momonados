//! Source-sized positive/negative tensor-projector calibration, no factors input.
use g_momonados::{idempotent_projector::Projector,ququart_decision::DecisionArena};
use num_bigint::{BigInt,BigUint};
use num_traits::{One,Zero};
use serde_json::json;
fn main()->Result<(),String>{
    let n:BigUint=std::env::args().nth(1).ok_or("projector_control <N>")?.parse().map_err(|_|"invalid N")?;
    if n.bits()<200{return Err("projector calibration requires at least 200 bits".into());}
    let mut receipts=Vec::new();
    for value in [1u8,2] {
        let started=std::time::Instant::now();let width=n.bits() as usize;
        let mut arena=DecisionArena::new(width);let initial=arena.basis(&BigUint::from(value),BigInt::one());
        let mut projector=Projector::compile(&mut arena,initial,&(0..width).collect::<Vec<_>>(),&n)?;
        let constraints=projector.constraints;
        while projector.internal_count()!=0 {
            projector.advance(&mut arena)?;
            if projector.contractions%32==0 {
                let mut roots=projector.roots();roots.push(initial);arena.fold_roots(&roots);
            }
        }
        let contractions=projector.contractions;let result=projector.finish(&mut arena)?;
        let mass=arena.mass(result);
        if (value==1 && mass!=BigUint::one()) || (value==2 && !mass.is_zero()) {
            return Err("contracted projector disagrees with the exact residual".into());
        }
        receipts.push(json!({"point":value,"support_mass":mass.to_string(),"constraints":constraints,
            "contractions":contractions,"seconds":started.elapsed().as_secs_f64()}));
    }
    println!("{}",json!({"N":n.to_string(),"purpose":"projector calibration, not factor production","checks":receipts}));
    Ok(())
}
