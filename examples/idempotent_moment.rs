//! N-only contraction, or source-sized observable calibration.
use g_momonados::{idempotent_moment, idempotent_point, ququart_decision::DecisionArena,
    word_tape::WordTape};
use num_bigint::{BigInt, BigUint};
use num_traits::One;
use serde_json::json;
fn main() -> Result<(), String> {
    let raw=std::env::args().nth(1).ok_or("idempotent_moment <N>")?;
    if raw=="--resume" {
        let input=std::env::args().nth(2).ok_or("--resume <continuation.json> <steps> <return.json>")?;
        let steps=std::env::args().nth(3).ok_or("missing resume steps")?.parse::<u64>().map_err(|e|e.to_string())?;
        let output=std::env::args().nth(4).ok_or("missing return path")?;
        let reader=std::io::BufReader::new(std::fs::File::open(&input).map_err(|e|e.to_string())?);
        let record=serde_json::from_reader(reader).map_err(|e|e.to_string())?;
        let held=g_momonados::conventional_spectrum::PreparedPointSwitch::from_record(&record)?;
        drop(record);
        println!("{}",held.hold(steps,std::path::Path::new(&output))?);
        return Ok(());
    }
    let n=BigUint::parse_bytes(raw.as_bytes(),10).ok_or("invalid N")?;
    if std::env::args().nth(2).as_deref()==Some("--hold") {
        let steps=std::env::args().nth(3).ok_or("--hold <steps> <return.json>")?.parse::<u64>().map_err(|e|e.to_string())?;
        let output=std::env::args().nth(4).ok_or("missing held return path")?;
        println!("{}",g_momonados::conventional_spectrum::prepare_held(n,steps,std::path::Path::new(&output))?);
        return Ok(());
    }
    if std::env::args().nth(2).as_deref()==Some("--contract") {
        let seconds=std::env::args().nth(3).map(|v|v.parse::<u64>()
            .map_err(|_|"optional contraction seconds must be a nonnegative integer".to_owned()))
            .transpose()?.unwrap_or(0);
        println!("{}",g_momonados::conventional_spectrum::prepare_contracted(n,seconds)?);
        return Ok(());
    }
    if std::env::args().nth(2).as_deref()==Some("--project") {
        if n.bits()<100 {return Err("projection check requires at least 100 bits".into());}
        println!("{}",g_momonados::conventional_spectrum::prepare_for("srfp",n)?);
        return Ok(());
    }
    if n.bits()<200 { return Err("calibration requires at least 200 bits".into()); }
    let width=n.bits() as usize;
    let mut arena=DecisionArena::new(width);
    // Complementary N-derived points 2,N-1, with unequal amplitudes 1,3.
    // This checks the operator and its weight independence, without reference factors.
    let a=arena.basis(&BigUint::from(2u8),BigInt::one());
    let b=arena.basis(&(&n-BigUint::one()),BigInt::from(3));
    let root=arena.sum(a,b);
    let result=idempotent_moment::read(&mut arena,root,&(0..width).collect::<Vec<_>>(),&n)?;
    let point=result.point.ok_or("complementary pair moment lost its point")?;
    if point!=&n-BigUint::one() {return Err("moment returned the wrong complementary point".into());}
    let rejected=idempotent_point::close(&WordTape::from_canonical_word(&vox_core::combined_factor::native_word(&n)).ok_or("source word")?,&WordTape::from_canonical_word(&vox_core::combined_factor::native_word(&point)).ok_or("point word")?).is_err();
    if !rejected {return Err("non-idempotent calibration passed the factor boundary".into());}
    let extra=arena.basis(&BigUint::from(3u8),BigInt::one());
    let broad=arena.sum(root,extra);
    let broad_return=idempotent_moment::read(&mut arena,broad,&(0..width).collect::<Vec<_>>(),&n)?;
    if broad_return.point.is_some() || broad_return.evidence["definite_complementary_separation"]!=false {
        return Err("broad support passed the definite-separation observable".into());
    }
    println!("{}",json!({"N":n.to_string(),"input_boundary":"N-only",
        "purpose":"observable calibration, not factor production",
        "unequal_amplitudes":[1,3],"point":point.to_string(),
        "non_idempotent_rejected":rejected,"observable":result.evidence,
        "broad_support_rejected":true,"broad_support_observable":broad_return.evidence,
        "retained_nodes":arena.retained_nodes(),"retained_roots":[root,result.weighted_root,result.squared_root,broad,broad_return.weighted_root,broad_return.squared_root]}));
    Ok(())
}
