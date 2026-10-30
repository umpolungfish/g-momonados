//! Standalone `ququart_word` binary.
//!
//! argv comes straight from the shell. Each argument is trimmed of ASCII
//! whitespace plus NBSP, ZWSP, and BOM before parsing, so no invisible
//! separator survives into a positional slot.

#[cfg(not(feature = "hosted"))]
fn main() {
    #[cfg(feature = "hosted")]
    vox_core::ordering::cli::intercept();
    eprintln!("ququart_word: rebuild with --features hosted");
    std::process::exit(2);
}

#[cfg(feature = "hosted")]
fn main() {
    #[cfg(feature = "hosted")]
    vox_core::ordering::cli::intercept();
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

    // The compiler and its kernel services live in the main executable.
    // Use its existing argv entry instead of naming a binary-only module
    // as though it were exported by the library.
    let result = (|| -> std::io::Result<std::process::ExitStatus> {
        let executable = std::env::current_exe()?.with_file_name("g-momonados");
        let mut command = std::process::Command::new(executable);
        command.arg("anyon_ququart_word").args(&cleaned);
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            Err(command.exec())
        }
        #[cfg(not(unix))]
        command.status()
    })();
    match result {
        Ok(status) => std::process::exit(status.code().unwrap_or(1)),
        Err(error) => { eprintln!("ququart_word: {error}"); std::process::exit(1); }
    }
}
