use membranes::{factor_radix4_with_stats, from_word, mul, to_word};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args.len() > 2 {
        eprintln!("usage: membrane_radix4_factor <canonical-IMASM-N> [node-cap]");
        std::process::exit(2);
    }
    let Some(n) = from_word(&args[0]) else {
        eprintln!("membrane_radix4_factor: expected a canonical IMASM numeral word");
        std::process::exit(2);
    };
    let cap = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(2_000_000u64);
    let (pair, nodes, stats) = factor_radix4_with_stats(&n, cap);
    println!("radix-four input bits={}", n.bit_len());
    let profile = stats.prefix_frames.iter().enumerate().map(|(digit, frames)| {
        format!("{digit}:{}:{}:{}", stats.recursive_entries[digit], frames,
            stats.prefix_closures[digit])
    }).collect::<Vec<_>>().join(",");
    println!("digit:entries:frames:closures={profile}");
    let balanced_profile = stats.balanced_frames.iter().enumerate().map(|(digit, frames)| {
        format!("{digit}:{}:{}:{}", stats.balanced_entries[digit], frames,
            stats.balanced_closures[digit])
    }).collect::<Vec<_>>().join(",");
    println!("balanced_digit:entries:frames:closures={balanced_profile}");
    match pair {
        Some((p, q)) => println!("factor={}\ncofactor={}\nproduct_closes={}\nnodes={nodes}",
            to_word(&p), to_word(&q), mul(&p, &q) == n),
        None => println!("no factor within {nodes} radix-four states (cap={cap})"),
    }
}
