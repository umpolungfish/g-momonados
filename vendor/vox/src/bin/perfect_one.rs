//! A transformation baked into a perfect membrane, run as pure execution.
//!
//! The value is baked as its IMASM numeral before compilation, so the built
//! artifact IS this one transformation and the run takes no input. The membrane
//! splits the value (delta), carries a real transform at the core, fuses it back
//! (mu), and recovers the input exactly: mu∘delta = id by the matched circuitry.

fn main() {
    let word: &str = option_env!("PERFECT_N_WORD").unwrap_or("⊢⊙⊡⊣");
    let depth: usize = option_env!("PERFECT_DEPTH")
        .and_then(|s| s.parse().ok())
        .unwrap_or(2);
    match ::vox::morphism_factor::parse_numeral(word) {
        Ok(v) => {
            let (verdict, surplus, w) = ::vox::perfect_membrane::report(depth);
            println!(
                "perfect membrane depth {depth}: verdict {verdict}  surplus {surplus}"
            );
            println!("{}", ::vox::vox::glyphs(&w));
            print!("{}", ::vox::perfect_membrane::run(&v, depth));
        }
        Err(e) => {
            eprintln!("PERFECT_N_WORD was not an IMASM numeral: {e}");
            std::process::exit(2);
        }
    }
}
