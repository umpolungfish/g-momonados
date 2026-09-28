//! Baked executable for the 31-step resident factorization membrane.

fn main() {
    let n: u64 = option_env!("FACTOR_N_DEC").unwrap_or("0").parse().unwrap_or_else(|_| {
        eprintln!("FACTOR_N_DEC was not an unsigned decimal integer");
        std::process::exit(2);
    });
    match ::vox::factorization_31_membrane::dispatch_report(n) {
        Ok(report) => println!("{report}"),
        Err(e) => { eprintln!("factorization membrane failed: {e}"); std::process::exit(1); }
    }
}
