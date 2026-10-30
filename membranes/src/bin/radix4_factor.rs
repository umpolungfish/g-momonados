use membranes::{divmod, factor_radix4_with_stats, from_word, is_zero, to_word};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut input = None;
    let mut cap = 2_000_000u64;
    let mut index = 0;
    while index < args.len() {
        let value = args.get(index + 1).unwrap_or_else(|| { eprintln!("each flag requires a value"); std::process::exit(2); });
        match args[index].as_str() {
            "--number" => input = Some(value.as_str()),
            "--node-cap" => cap = value.parse().unwrap_or_else(|_| { eprintln!("--node-cap must be an integer"); std::process::exit(2); }),
            flag => { eprintln!("unknown flag {flag}"); std::process::exit(2); }
        }
        index += 2;
    }
    let input = input.unwrap_or_else(|| { eprintln!("usage: membrane_radix4_factor --number CANONICAL_IMASM_N [--node-cap COUNT]"); std::process::exit(2); });
    let Some(n) = from_word(input) else {
        eprintln!("membrane_radix4_factor: expected a canonical IMASM numeral word");
        std::process::exit(2);
    };
    let (factor, nodes, stats) = factor_radix4_with_stats(&n, cap);
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
    match factor {
        Some(p) => {
            let (_, remainder) = divmod(&n, &p);
            println!("factor={}\nproduct_closes={}\nnodes={nodes}",
                to_word(&p), is_zero(&remainder));
        }
        None => println!("no factor within {nodes} radix-four states (cap={cap})"),
    }
}
