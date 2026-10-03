use g_momonados::prime_tool_logic::{process_prime, render_report};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(input) = args.first() else {
        eprintln!("Usage: prime-tool <natural-number|cell-binary-word>");
        std::process::exit(1);
    };
    match process_prime(input) {
        Ok(report) => print!("{}", render_report(&report)),
        Err(error) => {
            eprintln!("prime-tool: {error}");
            std::process::exit(1);
        }
    }
}
