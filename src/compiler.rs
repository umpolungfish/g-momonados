// ─── compiler.rs ───────────────────────────────────────────────────────
// Compile between mathematical representations (build.txt §349).
//
// The one clean bridge is δ/μ between braids and IMASM. This compiles a braid
// (source) to IMASM, to its Jones invariant, or to the Lean closure statement;
// and compiles an IMASM word back to a braid (μ, read_tangle). The IMASM side
// accepts token NAMES (VINIT), short forms (VI), and the GLYPHS themselves (⊢) —
// including a whole glyph word typed as one token (⊢∈≻⊤∋⊣), which is split one
// character at a time. Names, short forms and glyphs are all one parser
// (Token::parse); a multi-character token that is not a name is read as a run
// of glyphs.
#![allow(dead_code)]
extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use crate::braid_protocol::{braid_to_imasm, parse_token_name, read_tangle, token_glyph};
use crate::fibonacci_qc::jones_polynomial;
use crate::tokens::Token;

fn parse_ints(items: &[&str]) -> Vec<i32> {
    items.iter().filter_map(|s| s.parse::<i32>().ok()).collect()
}

/// Expand one whitespace-separated source token into a run of IMASM tokens.
/// A full name (VINIT), a short form (VI) or a single glyph (⊢) parses whole.
/// A contiguous glyph word (⊢∈≻⊤∋⊣) splits one character at a time; every
/// character must itself parse as a token, or the whole expansion fails.
fn expand_tokens(w: &str) -> Option<Vec<Token>> {
    if let Some(t) = parse_token_name(w) {
        return Some(vec![t]);
    }
    let chars: Vec<char> = w.chars().collect();
    if chars.len() >= 2 {
        let mut out = Vec::new();
        for c in chars {
            out.push(parse_token_name(&c.to_string())?);
        }
        return Some(out);
    }
    None
}

pub fn compiler_main(args: &[&str]) -> String {
    let flat: Vec<&str> = args.iter().flat_map(|s| s.split_whitespace()).collect();
    // Split at "--to".
    let to_pos = flat.iter().position(|&s| s == "--to");
    let target = to_pos.and_then(|i| flat.get(i + 1).copied());
    let src_end = to_pos.unwrap_or(flat.len());
    let src = &flat[..src_end];

    if src.len() < 2 || target.is_none() {
        return "compiler <source> --to <target>\n\n\
                sources:  braid <ints...>        e.g.  braid 1 2 1\n\
                          imasm <WORD>           e.g.  imasm VINIT FSPLIT FFUSE TANCH\n\
                                                       or  imasm ⊢∈≻⊤∋⊣\n\
                targets:  imasm | jones | lean | braid\n\n\
                Braids compile to imasm/jones/lean; an IMASM word compiles\n\
                to a braid (μ, read_tangle). The imasm source takes names,\n\
                short forms or glyphs — a glyph word may be one token or\n\
                space-separated.\n\n\
                Try:  compiler braid 1 2 1 --to imasm\n\
                      compiler imasm ⊢∈⊤⊥∋⊣ --to braid\n".to_string();
    }
    let kind = src[0];
    let target = target.unwrap();
    let mut out = String::from("COMPILER\n========\n\n");

    match (kind, target) {
        ("braid", "imasm") => {
            let gens = parse_ints(&src[1..]);
            let prog = braid_to_imasm(&gens, 1, false);
            // The IMASM side speaks glyphs, not names: the word is the symbols.
            let word: String = prog.iter().map(|t| token_glyph(t)).collect();
            out.push_str(&format!("braid → imasm:  {}\n", word));
            out.push_str("certificate:    δ (braid_to_imasm), closure-preserving\n");
        }
        ("braid", "jones") => {
            let gens = parse_ints(&src[1..]);
            let j = jones_polynomial(3, &gens);
            out.push_str(&format!("braid → Jones:  {:.6} + {:.6}i   |·|={:.6}\n", j.re, j.im, j.norm()));
        }
        ("braid", "lean") => {
            let gens = parse_ints(&src[1..]);
            let prog = braid_to_imasm(&gens, 1, false);
            let closes = read_tangle(&prog, gens.len() + 2, 1).map(|t| t.closes).unwrap_or(false);
            out.push_str("braid → Lean:   theorem: μ∘δ = id\n");
            out.push_str(&format!("closure:        {}\n", if closes { "PASS (tangle closes)" } else { "FAIL (tangle open)" }));
        }
        ("imasm", "braid") => {
            // Each whitespace-separated source token may be a name, a short form,
            // a single glyph, or a contiguous glyph word — expand them all.
            let mut prog: Vec<Token> = Vec::new();
            let mut bad: Option<String> = None;
            for n in &src[1..] {
                match expand_tokens(n) {
                    Some(toks) => prog.extend(toks),
                    None => { bad = Some(n.to_string()); break; }
                }
            }
            match bad {
                Some(w) => out.push_str(&format!("one token did not parse: '{}'\n", w)),
                None => match read_tangle(&prog, prog.len() + 1, 1) {
                    Ok(tr) => {
                        let g: Vec<String> = tr.generators.iter().map(|x| format!("{}", x)).collect();
                        out.push_str(&format!("imasm → braid:  [{}]   ({} crossings)\n", g.join(" "), tr.crossings));
                        out.push_str(&format!("certificate:    μ (read_tangle), closes: {}\n", tr.closes));
                    }
                    Err(e) => out.push_str(&format!("imasm → braid:  FAIL ({})\n", e)),
                },
            }
        }
        _ => out.push_str(&format!("no route: {} --to {}\n", kind, target)),
    }
    out
}
