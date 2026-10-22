extern crate alloc;
extern crate self as g_momonados;
#[path = "src/lib.rs"]
#[allow(unused_attributes)]
mod membrane;
pub use membrane::*;
#[path = "src/bin/ququart_support/work.rs"]
#[allow(dead_code)]
mod resident_work;
mod resident_build;
mod point_build;
fn numeral(word: &str) -> Result<num_bigint::BigUint, String> {
    godel_calculus::decode_cell_biguint(word)
}
use std::{env, fs, path::PathBuf};


fn main() {
    println!("cargo:rerun-if-env-changed=QUQUART_PREPARED_FILE");
    let contents = match env::var("QUQUART_PREPARED_FILE") {
        Ok(path) => {
            let path = fs::canonicalize(path).expect("prepared ququart file must exist");
            println!("cargo:rerun-if-changed={}", path.display());
            let path_str = path.to_str().expect("valid UTF-8 path");
            format!("const PREPARED: &str = include_str!({path_str:?});\n")
        }
        Err(_) => "const PREPARED: &str = \"\";\n".into(),
    };
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::write(out.join("ququart_prepared.rs"), contents).unwrap();
    println!("cargo:rerun-if-env-changed=FINAL_POINT_SOURCE_FILE");
    println!("cargo:rerun-if-env-changed=POINT_SOURCE_FILE");
    let final_source=env::var_os("FINAL_POINT_SOURCE_FILE");
    let point_source=env::var_os("POINT_SOURCE_FILE");
    assert!(final_source.is_none() || point_source.is_none(),
        "select one N-only point source, not two independently selected inputs");
    match point_source.or(final_source) {
        Some(path)=>{
            let path=fs::canonicalize(path).expect("N-only final switch source must exist");
            println!("cargo:rerun-if-changed={}",path.display());
            let source=fs::read_to_string(path).expect("read N-only final source");
            let n=point_build::source(&source).expect("decode N-only final source");
            let evaluated=conventional_spectrum::compile_final_switch(n)
                .expect("complete numerical preparation before emitting the final switch");
            fs::write(out.join("point_final.json"),serde_json::to_vec(&evaluated).unwrap()).unwrap();
        },
        None=>{fs::write(out.join("point_final.json"),b"null").unwrap();}
    }
    println!("cargo:rerun-if-env-changed=POINT_CARRIER_SOURCE_FILE");
    println!("cargo:rerun-if-env-changed=POINT_CARRIER_CONTINUATION_FILE");
    println!("cargo:rerun-if-changed=point_build.rs");
    let carrier_source=env::var_os("POINT_CARRIER_SOURCE_FILE");
    let carrier_return=env::var_os("POINT_CARRIER_CONTINUATION_FILE");
    assert!(carrier_source.is_none()||carrier_return.is_none(),"select one source or its retained continuation");
    match (carrier_source,carrier_return) {
        (Some(path),None)=>{
            let path=fs::canonicalize(path).expect("N-only point source file must exist");
            println!("cargo:rerun-if-changed={}",path.display());
            let n=point_build::source(&fs::read_to_string(path).expect("read N-only point source")).expect("read exact point source");
            let prepared=conventional_spectrum::compile_switch_record(n).expect("prepare complete point circuit during Cargo compilation");
            point_build::emit(&prepared,&out).expect("emit prepared point state and operators");
        },
        (None,Some(path))=>{
            let path=fs::canonicalize(path).expect("held carrier file must exist");
            println!("cargo:rerun-if-changed={}",path.display());
            let record:serde_json::Value=serde_json::from_reader(std::io::BufReader::new(fs::File::open(path).unwrap()))
                .expect("read the retained carrier");
            let validated=conventional_spectrum::PreparedPointSwitch::from_record(&record)
                .expect("restore the complete source-bound continuation before baking");
            drop(validated);
            point_build::emit(&record,&out).expect("bake the retained carrier without preparation");
        },
        (None,None)=>fs::write(out.join("point_prepared.rs"),
            "fn compiled_point_switch()->Result<g_momonados::conventional_spectrum::PreparedPointSwitch,String>{Err(\"compile with POINT_CARRIER_SOURCE_FILE pointing to N alone\".into())}\n").unwrap(),
        _=>unreachable!(),
    }
    println!("cargo:rerun-if-env-changed=RESIDENT_SOURCE_FILE");
    println!("cargo:rerun-if-env-changed=RESIDENT_PREPARED_FILE");
    println!("cargo:rerun-if-env-changed=RESIDENT_CIRCUIT_OPERATOR_FILE");
    println!("cargo:rerun-if-env-changed=RESIDENT_COMPOSED_MODULAR");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=resident_build.rs");
    println!("cargo:rerun-if-changed=build.rs");
    match env::var_os("RESIDENT_SOURCE_FILE") {
        Some(path) => {
            let path = fs::canonicalize(path).expect("N-only IMASM source file must exist");
            println!("cargo:rerun-if-changed={}", path.display());
            if let Some(operator) = env::var_os("RESIDENT_CIRCUIT_OPERATOR_FILE") {
                let operator = fs::canonicalize(operator).expect("circuit operator receipt must exist");
                println!("cargo:rerun-if-changed={}", operator.display());
            }
            let source = fs::read_to_string(&path).expect("read N-only IMASM source");
            assert!(env::var_os("RESIDENT_PREPARED_FILE").is_none(),
                "Cargo constructs resident state from N alone; a separate prepared input is not admitted");
            let prepared = resident_build::compile(&source).expect("compile resident membrane");
            let file=fs::File::create(out.join("resident_prepared.rs")).expect("compiled resident output");
            let mut writer=std::io::BufWriter::new(file);
            resident_build::emit_baked(&prepared,&mut writer,&out).expect("emit typed resident word and register");
            let composed=env::var("RESIDENT_COMPOSED_MODULAR").as_deref()==Ok("1");
            std::io::Write::write_fmt(&mut writer,format_args!("const BAKED_COMPOSED_MODULAR:bool={composed};\n"))
                .expect("retain the compiled modular execution mode");
            std::io::Write::flush(&mut writer).expect("flush compiled resident tables");
        }
        None => {fs::write(out.join("resident_prepared.rs"),
            "const BAKED_SOURCE_WORD:&str=\"\";\nconst BAKED_PREPARATION_NS:u64=0;\nconst BAKED_COMPOSED_MODULAR:bool=false;\nfn baked_membrane()->Result<g_momonados::resident_glut::ResidentPhaseCarrier,String>{Err(\"compile with the N-only resident source\".into())}\n").unwrap();}
    }
}
