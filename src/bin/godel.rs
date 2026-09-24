use vox_core::{godel_analyzer, godel_calculus};

fn dispatch(args: &[&str]) -> Result<String, String> {
    match args.first().copied().unwrap_or("help") {
        "analyze" | "lte2" => godel_analyzer::command(args),
        "selftest" | "verify" => {
            let mut out = godel_calculus::selftest_report()?;
            out.push_str(&godel_analyzer::selftest_report()?);
            Ok(out)
        }
        "help" | "-h" | "--help" => Ok(format!(
            "{}{}",
            godel_calculus::help(),
            godel_analyzer::help_addendum()
        )),
        _ => godel_calculus::command(args),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    match dispatch(&refs) {
        Ok(report) => print!("{report}"),
        Err(error) => {
            eprint!("{error}");
            if !error.ends_with('\n') {
                eprintln!();
            }
            std::process::exit(1);
        }
    }
}