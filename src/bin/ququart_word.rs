//! Standalone `ququart_word` binary.
//!
//! argv comes straight from the shell. Each argument is trimmed of ASCII
//! whitespace plus NBSP, ZWSP, and BOM before parsing, so no invisible
//! separator survives into a positional slot.

#[cfg(not(feature = "hosted"))]
fn main() {
    eprintln!("ququart_word: rebuild with --features hosted");
    std::process::exit(2);
}

#[cfg(feature = "hosted")]
fn main() {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    if raw.is_empty() {
        eprintln!(
            "usage: ququart_word <N> [sk=4] [net=7] [capacity=20000] [refinement=2] [accuracy=4] [inverse]"
        );
        std::process::exit(2);
    }

    let cleaned: Vec<String> = raw
        .iter()
        .map(|s| {
            s.trim_matches(|c: char| {
                c.is_ascii_whitespace()
                    || c == '\u{00A0}'
                    || c == '\u{200B}'
                    || c == '\u{FEFF}'
            })
            .to_string()
        })
        .collect();

    let refs: Vec<&str> = cleaned.iter().map(String::as_str).collect();

    match g_momonados::anyon_braid_cnot::compile_ququart_fourier(&refs) {
        Ok(report) => println!("{}", report),
        Err(error) => {
            eprintln!("ququart_word: {}", error);
            std::process::exit(1);
        }
    }
}