//! A batch of values baked into one sealed binary. Each is baked as its IMASM
//! numeral before compilation; the binary factors them all and prints the table
//! only when the whole batch is done. One operculum for the batch: sealed, run to
//! completion, opened once. No peeking at the interior mid-run.

fn main() {
    let words = option_env!("BENCH_WORDS").unwrap_or("");
    let labels = option_env!("BENCH_LABELS").unwrap_or("");
    let ws: Vec<&str> = words.split_whitespace().collect();
    let ls: Vec<&str> = labels.split_whitespace().collect();
    let mut out = String::new();
    for (i, w) in ws.iter().enumerate() {
        let label = ls.get(i).copied().unwrap_or("?");
        match ::vox::morphism_factor::parse_numeral(w) {
            Ok(n) => {
                let t = std::time::Instant::now();
                let res = ::vox::morphism_factor::repl_smart_factor(&n);
                let el = t.elapsed();
                out.push_str(&format!("{label}-bit  {res}  [{el:.2?}]\n"));
            }
            Err(e) => out.push_str(&format!("{label}-bit  parse error: {e}\n")),
        }
    }
    print!("{out}");
}
