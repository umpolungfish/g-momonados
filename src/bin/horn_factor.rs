fn main() {
    g_momonados::ordering::cli::intercept();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let refs: Vec<_> = args.iter().map(String::as_str).collect();
    match g_momonados::horn_torus_carrier::run(&refs) {
        Ok((report, produced)) => { print!("{report}"); if !produced { std::process::exit(3); } },
        Err(error) => { eprintln!("{error}"); std::process::exit(2); },
    }
}
