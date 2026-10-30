//! semiprime-winding <N> [sieve_width] [plain_kmax] [factorial_bound]
use g_momonados::godel_semiprime3::{process_semiprime_with, render_report};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 || args.len() > 5 {
        eprintln!("usage: semiprime-winding <N> [sieve_width=16] [plain_kmax=200000] [factorial_bound=20000]");
        std::process::exit(1);
    }
    let num = |i: usize, d: u64| args.get(i).and_then(|s| s.parse::<u64>().ok()).unwrap_or(d);
    match process_semiprime_with(&args[1], num(2, 16) as usize, num(3, 200_000), num(4, 20_000)) {
        Ok(r) => print!("{}", render_report(&r)),
        Err(e) => { eprintln!("ERROR: {e}"); std::process::exit(1); }
    }
}
