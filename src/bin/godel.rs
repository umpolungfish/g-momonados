use vox_core::godel_calculus;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    match godel_calculus::command(&refs) {
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
