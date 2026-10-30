//! Single-execution factorizer with N baked in as its IMASM numeral.
//!
//! The number is part of the program, not an argument. The decimal is consumed
//! before compilation by `vox numeral`, which emits the IMASM numeral word; that
//! word is baked in here at compile by option_env!, so the built artifact IS the
//! factorization of that one N and the run takes no input. Build and run in one
//! step with factor_one.sh, which does the encode-then-bake.
//!
//!   FACTOR_N_WORD="$(vox numeral 8051)" cargo build --release --bin factor_one
//!   ./target/release/factor_one

fn main() {
    // Baked at compile time as the IMASM numeral word. The default is the numeral
    // for 0 so the crate still builds normally when unset; the wrapper sets it.
    let word: &str = option_env!("FACTOR_N_WORD").unwrap_or("⊢⊙⊡⊣");
    match ::vox::morphism_factor::parse_numeral(word) {
        Ok(n) => println!("{}", ::vox::morphism_factor::repl_smart_factor(&n)),
        Err(e) => {
            eprintln!("FACTOR_N_WORD was not an IMASM numeral: {e}");
            std::process::exit(2);
        }
    }
}
