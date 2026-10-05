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
}

