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

fn read_line(out: &mut String, label: &str, word: &str) {
    match read(word) {
        Some(r) => {
            let wind = word.matches('⊡').count();
            out.push_str(&alloc::format!(
                "  {label:11}  ⊡={wind}  reg {}  verdict {}  holds {}  {}\n",
                r.register, r.verdict, r.holds, word));
        }
        None => out.push_str(&alloc::format!("  {label:11}  (unparseable) {}\n", word)),
    }
}

pub fn hyperstack_main(args: &[&str]) -> String {
    if args.len() < 2 {
        return String::from(
            "hyperstack <value-word> <type> [<type> ...]\n\
             types: phase shor fib arithmetic branch\n\
             loads the value into the first carrier, that into the next, and so on;\n\
             reads register, verdict and winding at each layer.\n\
             e.g. hyperstack ⊤⊥ phase shor fib\n");
    }
    let payload = args[0];
    let types = &args[1..];
    // validate types first
    for t in types {
        if carrier(t).is_none() {
            return alloc::format!("hyperstack: unknown carrier type '{}'\n", t);
        }
    }
    let mut out = String::from("heterogeneous hypernest: value loaded through the carrier stack\n");
    let mut cur = String::from(payload);
    read_line(&mut out, "value", &cur);
    for t in types {
        let c = carrier(t).unwrap();
        cur = frame_with(c, &cur);
        read_line(&mut out, t, &cur);
    }
    out
}
