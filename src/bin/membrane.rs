// membrane.rs — a sub-millisecond CLI for the three membrane engines.
//
// Unlike the bash membrane script (which shells the REPL on every call and
// pays the full QEMU + kernel boot per dispatch), this binary links the
// kernel's public functions directly and runs each verb in-process.
//
// Public surface:
//     membrane encode <N>                       -> native_numeral word
//     membrane factor  <N>                      -> factor_membrane factor verdict
//     membrane make    <N> [TEMPLATE]           -> encode, embed at ⊙, run on N
//     membrane list                             -> family registry
//     membrane run    <WORD> <N>                -> run a membrane word on N
//     membrane family <N>                       -> pick + run across all family words
//     membrane help

use std::env;
use std::process::ExitCode;
use std::time::Instant;

use g_momonados::membrane_family as mf;
use g_momonados::native_numeral as nn;

fn main() -> ExitCode {
    let t0 = Instant::now();
    let args: Vec<String> = env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("help");

    let out = match cmd {
        "encode" => encode(&args[1..]),
        "factor" => factor(&args[1..]),
        "make" => make(&args[1..]),
        "list" => list(),
        "run" => run(&args[1..]),
        "family" => family(&args[1..]),
        "help" | "-h" | "--help" => help(),
        other => format!(
            "membrane: unknown command '{other}' (try 'membrane help')\n"
        ),
    };
    print!("{out}");
    let dt = t0.elapsed();
    eprintln!("[membrane] elapsed={:?}", dt);
    if cmd == "help" || cmd == "-h" || cmd == "--help" {
        ExitCode::SUCCESS
    } else if out.starts_with("membrane: unknown command") {
        ExitCode::from(2)
    } else if out.starts_with("usage:") || out.starts_with("membrane: ") {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    }
}

fn help() -> String {
    String::from(
        "membrane — sub-ms CLI for the three membrane engines (links g_momonados directly)\n\
         \n\
         factor_membrane  (Factor-Separating Imscription Membrane M_κ):\n\
           membrane encode  <N>                  decimal -> native-numeral word\n\
           membrane factor  <N>                  factor-separating walk\n\
         \n\
         membrane_family:\n\
           membrane list                         list registered membrane words\n\
           membrane run    <WORD> <N>            run a specific membrane word on N\n\
           membrane family <N>                   run every family word on N\n\
         \n\
         compile-and-run (encode -> embed at ⊙ -> run):\n\
           membrane make    <N> [TEMPLATE]       default template: carry_fuse_closure\n\
         \n\
           membrane help\n",
    )
}

fn encode(args: &[String]) -> String {
    let n = match args.first() {
        Some(s) => s,
        None => return "usage: membrane encode <N>\n".into(),
    };
    nn::encode_report(n)
}

fn factor(args: &[String]) -> String {
    let n_str = match args.first() {
        Some(s) => s,
        None => return "usage: membrane factor <N>\n".into(),
    };
    let n_word = match nn::encode(n_str) {
        w if !w.is_empty() => w,
        _ => return format!("INVALID: expected decimal N or canonical native_numeral encode word\n"),
    };
    let n = match nn::decode(&n_word) {
        Some(v) => v,
        None => return format!("INVALID: could not decode native word for N={n_str}\n"),
    };
    // The kernel verdict text lives inside the REPL handler. We re-implement the
    // factor verdict here by calling run_membrane against a known-working
    // family word, but the canonical path is the REPL. For a true in-process
    // factor we use the lowest-budget family template — carry_fuse_closure
    // closes composites quickly via the Fermat math register.
    let tpl = "⊢⊙∈≻⊤≺⊥⊞⋈∋⊡⊣";
    match nn::decode(&n_word) {
        Some(_) => match mf::run_membrane(tpl, &n, 64) {
            (Some((p, q)), ticks) => format!(
                "N={n}\n\
                 encode(N)={w}\n\
                 COMPOSITE-found (carry_fuse_closure)\n\
                 p={p}\n\
                 q={q}\n\
                 p*q==N: {ok}\n\
                 syzygy preserves [encode; Γ; Λ; μ]: {syz}\n\
                 nested mark ticks={ticks}\n",
                n = n,
                w = n_word,
                p = p,
                q = q,
                ok = &p * &q == n,
                syz = nn::syzygy_preserves(&n, &p, &q),
                ticks = ticks
            ),
            (None, ticks) => format!(
                "N={n}\n\
                 encode(N)={w}\n\
                 no pair fixed across the full band\n\
                 nested mark ticks={ticks}\n",
                n = n,
                w = n_word,
                ticks = ticks
            ),
        },
        None => String::from("INVALID: decode failed\n"),
    }
}

fn list() -> String {
    let mut s = String::from("membrane family (name : word):\n");
    for (name, word) in mf::family() {
        s.push_str(&format!("  {name:28} {word}\n"));
    }
    s
}

fn run(args: &[String]) -> String {
    if args.len() < 2 {
        return "usage: membrane run <WORD> <N>\n".into();
    }
    let word = &args[0];
    let n_str = &args[1];
    let n = match n_str.parse::<num_bigint::BigUint>() {
        Ok(v) => v,
        Err(_) => return "N must be decimal\n".into(),
    };
    match mf::run_membrane(word, &n, 64) {
        (Some((p, q)), ticks) => format!(
            "membrane {word}\n\
             N={n}\n\
             {n} = {p} x {q} (verified={})\n\
             nested mark ticks={ticks}\n",
            &p * &q == n
        ),
        (None, ticks) => format!(
            "membrane {word}\n\
             N={n}\n\
             no pair fixed across the full band\n\
             nested mark ticks={ticks}\n",
        ),
    }
}

fn family(args: &[String]) -> String {
    let n_str = match args.first() {
        Some(s) => s,
        None => return "usage: membrane family <N>\n".into(),
    };
    let n = match n_str.parse::<num_bigint::BigUint>() {
        Ok(v) => v,
        Err(_) => return "N must be decimal\n".into(),
    };
    let mut s = format!("math-register membrane family on N={n}\n");
    s.push_str(&format!(
        "{:28} {:>10} {:>12} {}\n",
        "MEMBRANE", "TICKS", "FACTORED", "PAIR"
    ));
    for (name, word) in mf::family() {
        match mf::run_membrane(word, &n, 64) {
            (Some((p, q)), ticks) => {
                let ok = &p * &q == n;
                s.push_str(&format!(
                    "{name:28} {ticks:>10} {:>12} {p} x {q}\n",
                    if ok { "yes" } else { "BADVERIFY" }
                ));
            }
            (None, ticks) => {
                s.push_str(&format!("{name:28} {ticks:>10} {:>12} -\n", "no"));
            }
        }
    }
    s
}

fn make(args: &[String]) -> String {
    if args.is_empty() {
        return "usage: membrane make <N> [TEMPLATE]\n".into();
    }
    let n_str = &args[0];
    let template = args.get(1).map(String::as_str).unwrap_or("carry_fuse_closure");

    // 1. encode N
    let enc = match !nn::encode(n_str).is_empty() {
        true => nn::encode(n_str),
        false => return format!("INVALID: could not encode N={n_str}\n"),
    };
    let n = match nn::decode(&enc) {
        Some(v) => v,
        None => return format!("INVALID: could not decode native word for N={n_str}\n"),
    };

    // 2. look up the template word
    let tpl = match mf::family().iter().find(|(name, _)| *name == template) {
        Some((_, w)) => w.to_string(),
        None => {
            return format!(
                "make: unknown template '{template}' (try: membrane list)\n"
            );
        }
    };

    // 3. embed the value word inside the template at the ⊙ slot
    let compiled = if let Some(pos) = tpl.find('⊙') {
        let mut c = String::with_capacity(tpl.len() + enc.len());
        c.push_str(&tpl[..pos]);
        c.push_str(&enc);
        c.push_str(&tpl[pos + '⊙'.len_utf8()..]);
        c
    } else if let Some(pos) = tpl.find('⊣') {
        let mut c = String::with_capacity(tpl.len() + enc.len());
        c.push_str(&tpl[..pos]);
        c.push_str(&enc);
        c.push_str(&tpl[pos..]);
        c
    } else {
        tpl.clone()
    };

    let mut s = format!(
        "make: N={n}  template={template}\n\
         value-word : {enc}\n\
         template    : {tpl}\n\
         compiled    : {compiled}\n"
    );

    // 4. run the compiled membrane
    match mf::run_membrane(&compiled, &n, 64) {
        (Some((p, q)), ticks) => {
            let ok = &p * &q == n;
            s.push_str(&format!(
                "membrane {compiled}\n  N={n}\n  {n} = {p} x {q} (verified={ok})\n  nested mark ticks={ticks}\n"
            ));
        }
        (None, ticks) => {
            s.push_str(&format!(
                "membrane {compiled}\n  N={n}\n  no pair fixed across the full band\n  nested mark ticks={ticks}\n"
            ));
        }
    }
    s
}
