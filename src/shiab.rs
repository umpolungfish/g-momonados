//! The SHIAB operator as an executable tool.
//!
//! SHIAB is a holographic scale-collapse: the boundary marks ⊢⊙ … ⊡⊣ encode a
//! bulk word, the ∈/∋ pair splits and stitches it, and the ⊙ gate holds it at
//! criticality where μ∘δ = id. This runs that operator live. `run` walks the
//! canonical word; `apply` wraps an input word as the bulk inside the boundary
//! and runs the collapse, so the tool transforms an input rather than describing
//! one. Every reading is computed at call time by the sixteen-value evaluator,
//! not stored.
use crate::counterfactual::read;

/// The canonical SHIAB operator word.
pub const SHIAB_WORD: &str = "⊢⊙∈≻⊤≺⊥⊞⋈∋⊡⊣";

/// The integer winding invariant: the count of ⊡ (ZWIND) fixations. A word with
/// n ≠ 0 sits in a protected class that cannot deform to the trivial state,
/// ∮_γ A = 2πn.
fn winding(word: &str) -> usize {
    word.chars().filter(|&c| c == '⊡').count()
}

/// Apply the SHIAB operator to an input word: place it as the bulk between the
/// split ⊢⊙∈ and the stitch-fix-anchor ⋈∋⊡⊣, so the boundary encodes the bulk.
fn frame(input: &str) -> String {
    let mut s = String::from("⊢⊙∈");
    s.push_str(input);
    s.push_str("⋈∋⊡⊣");
    s
}

fn read_block(out: &mut String, title: &str, word: &str) {
    match read(word) {
        Some(r) => {
            let n = winding(word);
            let prot = if n != 0 { "protected (∮=2πn, n≠0)" } else { "trivial winding" };
            out.push_str(&format!("  {title}\n"));
            out.push_str(&format!("    word      {}\n", r.word));
            out.push_str(&format!("    register  {}   verdict {} ({})\n", r.register, r.verdict, r.verdict_why));
            out.push_str(&format!("    winding   n = {}   {}\n", n, prot));
            out.push_str(&format!("    banked    holds={} vacuous={} live_clears={} deposits={} exposed={}\n",
                r.holds, r.vacuous, r.live_clears, r.deposits, r.exposed));
        }
        None => out.push_str(&format!("  {title}: empty or unparseable word\n")),
    }
}

fn help() -> String {
    let mut s = String::new();
    s.push_str("SHIAB operator — holographic scale-collapse, run live\n");
    s.push_str("  shiab run                run the canonical operator ⊢⊙∈≻⊤≺⊥⊞⋈∋⊡⊣\n");
    s.push_str("  shiab apply <word>       wrap a word as bulk and run the collapse\n");
    s.push_str("  shiab winding <word>     the integer winding invariant ⊡ (∮=2πn)\n");
    s
}

pub fn shiab_main(args: &[&str]) -> String {
    match args.first().copied() {
        None | Some("run") => {
            let mut s = String::from("SHIAB operator — canonical collapse, computed live\n");
            read_block(&mut s, "canonical", SHIAB_WORD);
            s
        }
        Some("apply") => match args.get(1) {
            Some(inp) => {
                let framed = frame(inp);
                let mut s = String::from("SHIAB apply — the boundary encodes the bulk word as its collapse\n");
                read_block(&mut s, "input (bulk)", inp);
                read_block(&mut s, "collapsed", &framed);
                s
            }
            None => String::from("shiab apply <word>   — wrap a word as bulk and run the collapse\n"),
        },
        Some("winding") => match args.get(1) {
            Some(w) => {
                let n = winding(w);
                format!("winding n = {}  ({})\n", n, if n != 0 { "protected" } else { "trivial" })
            }
            None => String::from("shiab winding <word>\n"),
        },
        _ => help(),
    }
}
