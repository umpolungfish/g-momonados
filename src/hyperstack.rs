//! Heterogeneous hypernesting: load an encoded value through a stack of
//! different carrier types, each carrier's word becoming the payload of the
//! next. A value wrapped in a phase carrier, that wrapped in a Shor carrier,
//! that in a Fibonacci-anyon carrier, and so on. Unlike the homogeneous
//! ⊢∈-repeat, the carrier TYPE at each level imprints its own reading: the
//! phase carrier closes to T, the Shor carrier opens a held both-value B (the
//! superposition), the Fibonacci carrier holds it. The ⊡ winding still counts
//! the stack depth. Each layer is read live by the sixteen-value evaluator.
use crate::counterfactual::read;
use alloc::string::String;

/// The carrier registry: a name to the ob3ect glyph word for that operator type.
fn carrier(name: &str) -> Option<&'static str> {
    Some(match name {
        "phase"     => "⊢⊙∈≻⊤⋈≺⊥⊞∋⊡⋈⊙⊣",
        "shor"      => "⊢∈≻⋈⊞∈⊤≻⊥≺∋⊙⋈⊡⊣",
        "fib" | "fibonacci" | "anyon"
                    => "⊢⊙∈≻⋈⊤≻⊥⊞≺⋈∈⊤⊥∋⊡⋈≻⊙∋⊣",
        "arithmetic" => "⊢⊙∈≻⊤⋈≺⊥⊞∋⊡⋈⊙⊣",
        "branch"    => "⊢∈⊤⊥∋⊡⊣",
        _ => return None,
    })
}

/// Load a payload into a carrier: place it right after the carrier's first fork
/// ∈, so the carrier's own work still runs and the payload rides inside the
/// frame (free-lunch nesting), the inner word becoming this carrier's bulk.
fn frame_with(carrier: &str, payload: &str) -> String {
    match carrier.find('∈') {
        Some(byte_i) => {
            // byte_i is the byte index of '∈' (3 bytes); insert after it
            let cut = byte_i + '∈'.len_utf8();
            let mut s = String::with_capacity(carrier.len() + payload.len());
            s.push_str(&carrier[..cut]);
            s.push_str(payload);
            s.push_str(&carrier[cut..]);
            s
        }
        None => {
            let mut s = String::from(carrier);
            s.push_str(payload);
            s
        }
    }
}

fn read_line(out: &mut String, label: &str, word: &str, show_word: bool) {
    match read(word) {
        Some(r) => {
            let wind = word.matches('⊡').count();
            let tail = if show_word { alloc::format!("  {}", word) } else { alloc::format!("  len={}", word.chars().count()) };
            out.push_str(&alloc::format!(
                "  {label:11}  ⊡={wind}  reg {}  verdict {}  holds {}{}\n",
                r.register, r.verdict, r.holds, tail));
        }
        None => out.push_str(&alloc::format!("  {label:11}  (unparseable)\n")),
    }
}

pub fn hyperstack_main(args: &[&str]) -> String {
    if args.len() < 2 {
        return String::from(
            "hyperstack <value> <type> [<type> ...]\n\
             types: phase shor fib arithmetic branch\n\
             <value> is a decimal integer (encoded as an IMASM numeral) or an IMASM word.\n\
             loads the value into the first carrier, that into the next, and so on;\n\
             reads register, verdict and winding at each layer, and decodes the\n\
             decimal back out at the end.\n\
             e.g. hyperstack 91 phase shor fib\n");
    }
    let value = args[0];
    let types = &args[1..];
    for t in types {
        if carrier(t).is_none() {
            return alloc::format!("hyperstack: unknown carrier type '{}'\n", t);
        }
    }
    // A decimal value is encoded to a numeral payload and carried through the
    // stack; a glyph word is used as the payload directly.
    let numeric = !value.is_empty() && value.chars().all(|c| c.is_ascii_digit());
    let payload = if numeric { crate::native_numeral::encode(value) } else { String::from(value) };
    if numeric && payload.is_empty() {
        return alloc::format!("hyperstack: could not encode decimal '{}'\n", value);
    }
    let mut out = String::from("heterogeneous hypernest: value loaded through the carrier stack\n");
    if numeric { out.push_str(&alloc::format!("  N            {}\n", value)); }
    // Build the stacked carrier word: value in the first carrier, that in the
    // next, and so on. Read the layer structure as it climbs.
    let mut cur = payload.clone();
    read_line(&mut out, "value", &cur, !numeric);
    for t in types {
        let c = carrier(t).unwrap();
        cur = frame_with(c, &cur);
        read_line(&mut out, t, &cur, !numeric);
    }
    if !numeric {
        return out;
    }
    // Numeric: the stacked word is a factoring operator; factor N with it.
    use num_bigint::BigUint;
    use core::str::FromStr;
    let n = match BigUint::from_str(value) { Ok(v) => v, Err(_) => return out };
    let (found, ticks) = crate::membrane_family::run_membrane(&cur, &n, types.len() as u32);
    match found {
        Some((p, q)) if p > BigUint::from(1u8) && &p * &q == n => {
            out.push_str(&alloc::format!(
                "  factors      {} = {} × {}   [stacked membrane, {} ticks]\n",
                value, p.to_str_radix(10), q.to_str_radix(10), ticks));
        }
        _ => {
            // Fall back to the winding-order seed ladder, which the hypernest
            // depth indexes; report which nesting depth closes.
            let (depth, opt) = crate::dynamic_nesting_prime_finder::find_optimal_depth(value, 32);
            match opt {
                Some(p) => {
                    let q = &n / &p;
                    out.push_str(&alloc::format!(
                        "  factors      {} = {} × {}   [winding-order seed ladder, closure depth {}]\n",
                        value, p.to_str_radix(10), q.to_str_radix(10), depth));
                }
                None => out.push_str(&alloc::format!("  factors      {}: no closure (prime, or beyond the ladder)\n", value)),
            }
        }
    }
    out
}
