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
    println!("cargo:rerun-if-env-changed=RESIDENT_SOURCE_FILE");
    println!("cargo:rerun-if-env-changed=RESIDENT_PREPARED_FILE");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=resident_build.rs");
    println!("cargo:rerun-if-changed=build.rs");
    let contents = match env::var_os("RESIDENT_SOURCE_FILE") {
        Some(path) => {
            let path = fs::canonicalize(path).expect("N-only IMASM source file must exist");
            println!("cargo:rerun-if-changed={}", path.display());
            let source = fs::read_to_string(&path).expect("read N-only IMASM source");
            let state = out.join("resident_state.json");
            assert!(env::var_os("RESIDENT_PREPARED_FILE").is_none(),
                "Cargo constructs resident state from N alone; a separate prepared input is not admitted");
            let record = resident_build::compile(&source).expect("compile resident membrane");
            fs::write(&state, serde_json::to_vec(&record).unwrap()).unwrap();
            format!("const PREPARED_RESIDENT: &str = include_str!({:?});\n", state)
        }
        None => "const PREPARED_RESIDENT: &str = \"\";\n".into(),
    };
    fs::write(out.join("resident_prepared.rs"), contents).unwrap();
}
