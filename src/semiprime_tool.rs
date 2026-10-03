use g_momonados::semiprime_tool_logic::{process_semiprime, render_report};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(input) = args.first() else {
        eprintln!("Usage: semiprime-tool <natural-number|cell-binary-word>");
        std::process::exit(1);
    };
    match process_semiprime(input) {
        Ok(report) => {
            print!("{}", render_report(&report));
            if !report.protocol_match {
                std::process::exit(2);
            }
        }
        Err(error) => {
            eprintln!("semiprime-tool: {error}");
            std::process::exit(1);
        }
    }
}
