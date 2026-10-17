//! Executing numeral boundary of Imscribing.IdempotentGcd.idem_gcd_factor.
//! The source and candidate remain WordTapes through the idempotent equations,
//! complementary split, gcd, quotient and exact product closure.
use crate::word_tape::WordTape;
use alloc::{format, string::String};

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
    if !source.gt(&one) { return Err("idempotent source must exceed one".into()); }
    let e = candidate.divmod(source).unwrap().1;
    if e.is_zero() || e == one { return Err("trivial idempotent retains no factor split".into()); }
    if e.mul(&e).divmod(source).unwrap().1 != e {
        return Err("candidate does not satisfy e*e=e modulo the source".into());
    }
    let complement = source.add(&one).sub(&e).unwrap();
    if e.mul(&complement).divmod(source).unwrap().1 != WordTape::zero()
        || e.add(&complement).divmod(source).unwrap().1 != one
        || complement.mul(&complement).divmod(source).unwrap().1 != complement {
        return Err("idempotent complementary split failed".into());
    }
    let divisor = e.gcd(source);
    if !divisor.gt(&one) || !source.gt(&divisor) {
        return Err("idempotent gcd did not yield a proper divisor".into());
    }
    let (quotient, remainder) = source.divmod(&divisor).unwrap();
    if !remainder.is_zero() || divisor.mul(&quotient) != *source
        || !crate::native_numeral::syzygy_preserves(&source.to_biguint(), &divisor.to_biguint(), &quotient.to_biguint()) {
        return Err("idempotent factor/product boundary did not close".into());
    }
    Ok(IdempotentClosure { source:source.clone(), idempotent:e, complement, divisor, quotient })
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

pub fn report(args: &[String]) -> Result<String, String> {
    #[cfg(feature = "hosted")]
    if args.first().map(String::as_str)==Some("emit") {
        if args.len()!=3 {return Err("idempotent emit N OUTPUT".into());}
        return emit_membrane(&args[1], &args[2]);
    }
    if args.is_empty() || matches!(args[0].as_str(), "help" | "--help") {
        return Ok("idempotent emit N OUTPUT\nOUTPUT.exe selects the native Windows static GPU host; other paths select the GNU/Linux static host.\nidempotent <close|root> <N> <candidate>\nclose checks e*e=e; root transports s*s=1. Both inputs are decimal or canonical numeral words.\n".into());
    }
    if args.len()!=3 { return Err("idempotent requires a mode, source and candidate".into()); }
    let read = |text:&str| -> Result<WordTape,String> {
        if text.starts_with('⊢') { WordTape::from_canonical_word(text).ok_or_else(||String::from("invalid numeral word")) }
        else { text.parse().map(|n|WordTape::from_biguint(&n)).map_err(|_|String::from("invalid numeral")) }
    };
    let source=read(&args[1])?;
    let candidate=read(&args[2])?;
    let result=match args[0].as_str() {
        "close"=>close(&source,&candidate)?,
        "root"=>from_involution(&source,&candidate)?,
        _=>return Err("idempotent mode must be close or root".into()),
    };
    Ok(format!("source = {}\nidempotent = {}\ncomplement = {}\ndivisor = {}\nquotient = {}\nidempotency = true\northogonality = true\ncomplementary sum = 1 mod N\nremainder = 0\nexact word product = true\npaired boundary = true\n{}",
        result.source.to_biguint(),result.idempotent.to_biguint(),result.complement.to_biguint(),
        result.divisor.to_biguint(),result.quotient.to_biguint(),
        crate::native_numeral::factor_words_line(&result.divisor.to_biguint(),&result.quotient.to_biguint())))
}

/// Compile the source and complete prepared search continuation into a static
/// membrane. No instance input or external worker is used by the emitted file.
#[cfg(feature = "hosted")]
pub fn emit_membrane(text: &str, destination: &str) -> Result<String,String> {
    use std::{fs, path::Path, process::Command};
    let compilation_clock=std::time::Instant::now();
    let n:num_bigint::BigUint=text.parse().map_err(|_|"invalid N")?;
    let source=WordTape::from_biguint(&n);
    if source.bit_len()<200 {return Err("membrane compilation requires a source of at least 200 bits".into());}
    let output=std::env::current_dir().map_err(|e|e.to_string())?.join(destination);
    if output.exists() {return Err("output already exists".into());}
    let project=output.with_extension("membrane-source");
    if project.exists() {return Err("membrane source directory already exists".into());}
    fs::create_dir_all(project.join("src")).map_err(|e|e.to_string())?;
    let repository=Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest=String::from("[package]\nname=\"idempotent-membrane\"\nversion=\"0.1.0\"\nedition=\"2021\"\n[workspace]\n[dependencies]\ncudarc={version=\"=0.19.9\",default-features=false,features=[\"driver\",\"nvrtc\",\"cuda-12040\",\"dynamic-loading\",\"std\"]}\n");
    fs::write(project.join("Cargo.toml"),manifest).map_err(|e|e.to_string())?;
    // Preparation fixes the source, initial candidate, residual, and complete
    // Y program. The transition carries e and e(e-1) modulo N together.
    let ptx=crate::idempotent_gpu_prepare::compile(&source,&project)?;
    fs::write(project.join("membrane.ptx"),ptx).map_err(|e|e.to_string())?;
    let program=include_str!("idempotent_gpu_runner.in.rs")
        .replace("BAKED_SOURCE_LITERAL",&format!("{:?}",crate::native_numeral::encode(text)))
        .replace("BAKED_WIDTH",&(source.bit_len()+2).to_string());
    fs::write(project.join("src/main.rs"),program).map_err(|e|e.to_string())?;
    let windows=output.extension().is_some_and(|s|s=="exe");
    let target=if windows{"x86_64-pc-windows-gnu"}else{"x86_64-unknown-linux-gnu"};
    let status=Command::new("cargo").args(["build","--offline","--release","--target",target,"--manifest-path"])
        .arg(project.join("Cargo.toml")).env("CARGO_TARGET_DIR",repository.join("target"))
        .env("RUSTFLAGS","-C target-feature=+crt-static -D warnings")
        .status().map_err(|e|e.to_string())?;
    if !status.success() {return Err(format!("membrane compilation failed: {status}"));}
    fs::copy(repository.join("target").join(target).join("release")
        .join(if windows{"idempotent-membrane.exe"}else{"idempotent-membrane"}),&output)
        .map_err(|e|e.to_string())?;
    Ok(format!("baked membrane = {}\nsource bits = {}\ninstance inputs at execution = none\nexecution device = GPU 1 / RTX 4070\ncompilation_seconds = {:.9}\n",output.display(),source.bit_len(),compilation_clock.elapsed().as_secs_f64()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigUint;

    #[test]
    fn source_bound_idempotents_close_through_the_executing_phase_paths() {
        // Roots are supplied by the reference control boundary. This measures
        // extraction and state retention, independently of phase production.
        let mut controls: alloc::vec::Vec<serde_json::Value> = serde_json::from_str(
            include_str!("../../Vox/measurements/integer_word_map_20261014/closure_controls.json")).unwrap();
        controls.push(serde_json::from_str(include_str!("../measurements/idempotent_closure_20261015/fresh_control.json")).unwrap());
        for row in controls {
            let n:BigUint=row["N"].as_str().unwrap().parse().unwrap();
            assert!(n.bits()>=200);
            let root:BigUint=row.get("root").unwrap_or(&row["base"]).as_str().unwrap().parse().unwrap();
            let source=WordTape::from_biguint(&n);
            let root_word=WordTape::from_biguint(&root);
            let split=from_involution(&source,&root_word).unwrap();
            assert_eq!(split.divisor.mul(&split.quotient),source);
            assert_eq!(split.idempotent.mul(&split.complement).divmod(&source).unwrap().1,WordTape::zero());
            assert_eq!(close(&source,&split.idempotent.add(&source)).unwrap().idempotent,split.idempotent);
            for candidate in [WordTape::zero(),WordTape::one(),WordTape::from_small(2)] {
                assert!(close(&source,&candidate).is_err());
            }
            for candidate in [WordTape::one(),source.sub(&WordTape::one()).unwrap(),WordTape::from_small(2)] {
                assert!(from_involution(&source,&candidate).is_err());
            }
            let mut word_phase=crate::phase_unbraid::PhaseReadoutAccumulator::default();
            let result=word_phase.close_word_fraction(&WordTape::one(),&WordTape::from_small(2),&root_word,&source).unwrap().unwrap();
            assert_eq!(result.1.mul(&result.2),source);
            assert_eq!(word_phase.idempotent().unwrap().idempotent,split.idempotent);
            let mut phase=crate::phase_unbraid::PhaseReadoutAccumulator::default();
            let result=phase.close_fraction(&BigUint::from(1u8),&BigUint::from(2u8),&root,&n).unwrap().unwrap();
            assert_eq!(&result.1*&result.2,n);
            assert_eq!(phase.idempotent().unwrap().idempotent,split.idempotent);
            let ququart=crate::ququart_factor::extract_certified_winding_for_base(&n,&root,&BigUint::from(2u8)).unwrap();
            assert_eq!(ququart.idempotent().idempotent,split.idempotent);
            assert_eq!(ququart.factors().0*ququart.factors().1,n);
        }
    }
}
